//! Psydian AI Bridge: the host-side program between the kernel and an AI
//! service (Docs/10-genai-architecture.md).
//!
//! ```text
//! Psydian kernel ─ serial (@PSY frames) ─► AI Bridge ─ HTTPS ─► AI service
//! ```
//!
//! The kernel never talks to the network and never sees the API key. The
//! bridge reads the kernel's `diagnostic` and `ai_request` frames, asks the
//! configured AI provider, validates the answer and sends back one
//! `ai_response` or `error` frame per request.

mod bridge;
mod config;
mod context;
mod diagnostics;
mod kernel;
mod prompt;
mod protocol;
mod provider;
mod safety;
mod telemetry;
mod validation;

use bridge::{Answerer, RateLimiter};
use config::Config;
use diagnostics::DiagnosticStore;
use protocol::{AiRequest, KernelMessage};
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use telemetry::Telemetry;

const USAGE: &str = "usage: ai-bridge [options]

Boots Psydian and answers its `ai` questions with an AI service.

  (default)            boot the kernel headless; type shell commands here
  --window             boot with a QEMU window (type there or here)
  --connect HOST:PORT  attach to a kernel already started with
                       `cargo run -- --serial-tcp PORT`
  --show-frames        also print the raw @PSY protocol frames
  --check              end-to-end self-test with the mock provider
  --repo PATH          Psydian checkout (default: the parent of ai-bridge/)
  -h, --help           this text

Configuration (environment):
  AI_PROVIDER    gemini, omniroute, openrouter, ollama / openai,
                 anthropic or mock; unset: picked from the key that is set
                 (gemini when none is)
  GEMINI_API_KEY, OMNIROUTE_API_KEY, OPENROUTER_API_KEY,
  ANTHROPIC_API_KEY or AI_API_KEY   the API key (never logged)
  AI_MODEL       default gemini-flash-latest (gemini), llama3.2 (ollama),
                 claude-opus-5-5 (anthropic); omniroute and openrouter
                 need one
  AI_ENDPOINT    API URL (https, or http://localhost for local services);
                 a base URL such as http://localhost:20128/v1 is fine
  AI_EFFORT      low (default), medium, high, xhigh, max
  AI_TIMEOUT     seconds per AI call, 1-28 (default 25)
  AI_MAX_REQUESTS_PER_MINUTE   default 10
  PSYDIAN_KERNEL_CMD           command that boots the kernel (headless mode)";

struct Options {
    mode: Mode,
    show_frames: bool,
    check: bool,
    repo: PathBuf,
}

enum Mode {
    Headless,
    Window,
    Connect(String),
}

fn parse_args() -> Result<Options, String> {
    let mut options = Options {
        mode: Mode::Headless,
        show_frames: false,
        check: false,
        repo: Path::new(env!("CARGO_MANIFEST_DIR")).join(".."),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--window" => options.mode = Mode::Window,
            "--connect" => {
                options.mode = Mode::Connect(args.next().ok_or("--connect needs HOST:PORT")?)
            }
            "--show-frames" => options.show_frames = true,
            "--check" => options.check = true,
            "--repo" => options.repo = PathBuf::from(args.next().ok_or("--repo needs a path")?),
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown option '{other}'")),
        }
    }
    Ok(options)
}

/// Terminal output: kernel text plus the bridge's own status lines.
#[derive(Clone)]
struct Screen {
    color: bool,
    show_frames: bool,
    quiet: bool,
}

impl Screen {
    fn bridge(&self, text: &str) {
        if self.quiet {
            return;
        }
        let (on, off) = if self.color {
            ("\x1b[36m", "\x1b[0m")
        } else {
            ("", "")
        };
        let mut out = io::stdout().lock();
        let _ = writeln!(out, "{on}[bridge] {text}{off}");
        let _ = out.flush();
    }

    fn kernel(&self, text: &str, newline: bool) {
        if self.quiet {
            return;
        }
        let mut out = io::stdout().lock();
        let _ = if newline {
            writeln!(out, "{text}")
        } else {
            write!(out, "{text}")
        };
        let _ = out.flush();
    }

    fn frame(&self, text: &str) {
        if self.show_frames && !self.quiet {
            let (on, off) = if self.color {
                ("\x1b[2m", "\x1b[0m")
            } else {
                ("", "")
            };
            self.kernel(&format!("{on}{text}{off}"), true);
        }
    }
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(options) => options,
        Err(message) => {
            if message.is_empty() {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            eprintln!("ai-bridge: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let config = if options.check {
        Config::from_lookup(|name| (name == "AI_PROVIDER").then(|| "mock".to_string()))
    } else {
        Config::from_env()
    };
    let config = match config {
        Ok(config) => config,
        Err(error) => {
            eprintln!("ai-bridge: configuration error: {error}");
            return ExitCode::from(2);
        }
    };

    let screen = Screen {
        color: io::stdout().is_terminal(),
        show_frames: options.show_frames,
        quiet: options.check,
    };
    let log_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ai-bridge.log");
    let telemetry = Arc::new(if options.check {
        Telemetry::disabled()
    } else {
        Telemetry::open(&log_path)
    });

    let provider = provider::from_config(&config);
    let problem = config.problem();
    screen.bridge(&format!("AI provider: {}", provider.describe()));
    if let Some(problem) = &problem {
        screen.bridge(&format!(
            "WARNING: {problem}. The kernel will get an AI_NOT_CONFIGURED error for every question."
        ));
    }
    if let Some(path) = telemetry.path() {
        screen.bridge(&format!("request log: {}", path.display()));
    }
    telemetry.event(&format!(
        "bridge started provider=\"{}\" problem={:?}",
        provider.describe(),
        problem
    ));

    let link = match connect(&options, &screen) {
        Ok(link) => link,
        Err(error) => {
            eprintln!("ai-bridge: could not reach the kernel: {error}");
            return ExitCode::FAILURE;
        }
    };

    let answerer = Answerer {
        provider,
        config_problem: problem,
        limiter: RateLimiter::per_minute(config.max_requests_per_minute),
    };
    let session = Session::start(link, answerer, screen.clone(), telemetry);

    let code = if options.check {
        run_check(&session)
    } else {
        session.relay_stdin()
    };
    session.finish();
    code
}

fn connect(options: &Options, screen: &Screen) -> io::Result<kernel::Link> {
    let repo = options.repo.canonicalize()?;
    match &options.mode {
        Mode::Headless => {
            let command = match std::env::var("PSYDIAN_KERNEL_CMD") {
                Ok(cmd) if !options.check && !cmd.trim().is_empty() => {
                    cmd.split_whitespace().map(str::to_string).collect()
                }
                _ => ["cargo", "run", "-q", "--", "--headless"]
                    .map(str::to_string)
                    .to_vec(),
            };
            screen.bridge("booting the kernel (the first run also builds it)...");
            kernel::spawn(&command, &repo)
        }
        Mode::Window => {
            let port = free_port()?;
            let command = [
                "cargo",
                "run",
                "-q",
                "--",
                "--serial-tcp",
                &port.to_string(),
            ]
            .map(str::to_string)
            .to_vec();
            screen.bridge("booting the kernel in a QEMU window (the first run also builds it)...");
            kernel::spawn_and_connect(
                &command,
                &repo,
                &format!("127.0.0.1:{port}"),
                Duration::from_secs(900),
            )
        }
        Mode::Connect(address) => kernel::connect(address),
    }
}

fn free_port() -> io::Result<u16> {
    Ok(std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port())
}

/// A running bridge: a reader thread for the kernel's output, a worker
/// thread for AI requests, and the caller's thread for typed input.
struct Session {
    writer: kernel::SharedWriter,
    output: Arc<Mutex<String>>,
    finished: Arc<Mutex<bool>>,
    child: Mutex<Option<std::process::Child>>,
}

impl Session {
    fn start(
        link: kernel::Link,
        mut answerer: Answerer,
        screen: Screen,
        telemetry: Arc<Telemetry>,
    ) -> Self {
        let store = Arc::new(Mutex::new(DiagnosticStore::new()));
        let output = Arc::new(Mutex::new(String::new()));
        let finished = Arc::new(Mutex::new(false));
        let (requests, queue) = mpsc::channel::<AiRequest>();

        // Worker: one AI request at a time.
        {
            let writer = link.writer.clone();
            let store = store.clone();
            let screen = screen.clone();
            let describe = answerer.provider.describe();
            std::thread::spawn(move || {
                for request in queue {
                    let snapshot = store.lock().map(|s| s.clone()).unwrap_or_default();
                    let outcome = answerer.answer(&request, &snapshot);
                    kernel::send_frame(&writer, &outcome.frame, true);
                    let seconds = outcome.latency.as_secs_f32();
                    match &outcome.detail {
                        Some(detail) if outcome.result == "answered" => screen.bridge(&format!(
                            "{}: answered in {seconds:.1} s ({detail})",
                            request.id
                        )),
                        Some(detail) if *detail != outcome.result => screen.bridge(&format!(
                            "{}: {} after {seconds:.1} s - {detail}",
                            request.id, outcome.result
                        )),
                        Some(_) => screen.bridge(&format!(
                            "{}: {} after {seconds:.1} s",
                            request.id, outcome.result
                        )),
                        None => {
                            screen.bridge(&format!("{}: answered in {seconds:.1} s", request.id))
                        }
                    }
                    telemetry.request(&request.id, &describe, outcome.latency, &outcome.result);
                    if let Some(detail) = &outcome.detail {
                        telemetry.event(&format!("request={} detail=\"{detail}\"", request.id));
                    }
                }
            });
        }

        // Reader: kernel output, frames and prompts.
        {
            let writer = link.writer.clone();
            let output = output.clone();
            let finished = finished.clone();
            let mut reader = link.reader;
            std::thread::spawn(move || {
                let mut pending: Vec<u8> = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    let n = match reader.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    if let Ok(mut out) = output.lock() {
                        out.push_str(&String::from_utf8_lossy(&chunk[..n]));
                    }
                    pending.extend_from_slice(&chunk[..n]);
                    while let Some(end) = pending.iter().position(|&b| b == b'\n') {
                        let line: Vec<u8> = pending.drain(..=end).collect();
                        let line = String::from_utf8_lossy(&line);
                        let line = line.trim_end_matches(['\r', '\n']);
                        handle_line(line, &writer, &store, &requests, &screen);
                    }
                    // Prompts such as "psydian> " end without a line break.
                    if pending.ends_with(b" ") && !pending.starts_with(b"@") {
                        screen.kernel(&String::from_utf8_lossy(&pending), false);
                        pending.clear();
                    }
                }
                if let Ok(mut done) = finished.lock() {
                    *done = true;
                }
                screen.bridge("kernel stopped.");
            });
        }

        Self {
            writer: link.writer,
            output,
            finished,
            child: Mutex::new(link.child),
        }
    }

    /// Forwards typed lines to the kernel until stdin closes or it stops.
    /// `cancel` sends Ctrl+C (Ctrl+C itself would stop the bridge).
    fn relay_stdin(&self) -> ExitCode {
        let (lines, typed) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            for line in io::stdin().lock().lines() {
                let Ok(line) = line else { break };
                if lines.send(line).is_err() {
                    break;
                }
            }
        });
        loop {
            if self.kernel_stopped() {
                return ExitCode::SUCCESS;
            }
            match typed.recv_timeout(Duration::from_millis(200)) {
                Ok(line) => self.type_line(&line),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Input closed (e.g. piped commands): wait for the kernel.
                    while !self.kernel_stopped() {
                        std::thread::sleep(Duration::from_millis(200));
                    }
                    return ExitCode::SUCCESS;
                }
            }
        }
    }

    fn type_line(&self, line: &str) {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("cancel") || trimmed == "^C" {
            kernel::send(&self.writer, b"\x03");
        } else {
            let mut bytes: Vec<u8> = line
                .bytes()
                .filter(|b| b.is_ascii() && !b.is_ascii_control())
                .collect();
            bytes.push(b'\r');
            kernel::send(&self.writer, &bytes);
        }
    }

    fn kernel_stopped(&self) -> bool {
        self.finished.lock().map(|done| *done).unwrap_or(true)
    }

    fn mark(&self) -> usize {
        self.output.lock().map(|o| o.len()).unwrap_or(0)
    }

    fn wait_for(&self, needle: &str, since: usize, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self
                .output
                .lock()
                .map(|o| o.get(since..).unwrap_or("").contains(needle))
                .unwrap_or(false)
            {
                return true;
            }
            if self.kernel_stopped() {
                return false;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn finish(&self) {
        if let Ok(mut child) = self.child.lock()
            && let Some(child) = child.as_mut()
        {
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if let Ok(Some(_)) = child.try_wait() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn handle_line(
    line: &str,
    writer: &kernel::SharedWriter,
    store: &Mutex<DiagnosticStore>,
    requests: &mpsc::Sender<AiRequest>,
    screen: &Screen,
) {
    let Some(body) = protocol::frame_body(line) else {
        screen.kernel(line, true);
        return;
    };
    screen.frame(line);
    match protocol::parse_kernel_frame(body) {
        Ok(KernelMessage::Ready) => {
            kernel::send_frame(writer, &protocol::bridge_ready_frame(), false);
            screen.bridge("connected - the kernel is ready. Ask with: ai <question>");
        }
        Ok(KernelMessage::Diagnostic(record)) => {
            if let Ok(mut store) = store.lock() {
                store.add(record);
            }
        }
        Ok(KernelMessage::AiRequest(request)) => {
            let _ = requests.send(request);
        }
        Ok(KernelMessage::Other) => {}
        Err(error) => screen.bridge(&format!(
            "ignored an invalid frame from the kernel: {}",
            error.0
        )),
    }
}

/// End-to-end self-test against the real kernel with the mock provider.
fn run_check(session: &Session) -> ExitCode {
    // (line to type, text that must appear, text that must not appear)
    const STEPS: &[(Option<&str>, &str, &[&str])] = &[
        (None, "psydian> ", &[]),
        (Some("ai hello"), "Hello from the AI Bridge", &[]),
        (
            Some("run 1"),
            "[AI] Running suggested command: meminfo",
            &[],
        ),
        (Some("fault-test heap"), "Proceed? [y/N]", &[]),
        (Some("y"), "[MEM-003] Heap allocation failed", &[]),
        (Some("ai why did the heap fail?"), "64 MiB", &[]),
        // The bridge dropped panic-test and rm -rf before the kernel saw them.
        (
            Some("ai can you crash it?"),
            "1. shutdown",
            &["panic-test", "rm -rf"],
        ),
        (Some("run 1"), "Proceed? [y/N]", &[]),
        (Some("n"), "Cancelled.", &[]),
        (Some("ai trigger a timeout"), "AI_TIMEOUT", &[]),
        (Some("ai simulate an error"), "AI_SERVICE_UNAVAILABLE", &[]),
        (Some("ai return garbage"), "AI_INVALID_RESPONSE", &[]),
        (Some("help"), "Available commands:", &[]),
    ];
    let color = io::stdout().is_terminal();
    let (green, red, off) = if color {
        ("\x1b[32m", "\x1b[31m", "\x1b[0m")
    } else {
        ("", "", "")
    };
    println!("AI Bridge self-test: real kernel + bridge + mock provider");
    for (number, (line, expected, forbidden)) in STEPS.iter().enumerate() {
        let since = session.mark();
        if let Some(line) = line {
            session.type_line(line);
        }
        let timeout = if line.is_none() {
            Duration::from_secs(900)
        } else {
            Duration::from_secs(20)
        };
        let mut ok = session.wait_for(expected, since, timeout);
        if ok && !forbidden.is_empty() {
            // Let the rest of the answer arrive, then look only at it.
            session.wait_for("psydian> ", since, Duration::from_secs(5));
            let output = session
                .output
                .lock()
                .map(|o| o[since..].to_string())
                .unwrap_or_default();
            let answer = output
                .rfind("[AI] Answer to")
                .map_or("", |at| &output[at..]);
            ok = forbidden.iter().all(|text| !answer.contains(text));
        }
        let label = line.unwrap_or("(boot)");
        println!(
            "  {}  {:>2}. {label:<28} expects {expected:?}",
            if ok {
                format!("{green}pass{off}")
            } else {
                format!("{red}FAIL{off}")
            },
            number + 1
        );
        if !ok {
            let output = session.output.lock().map(|o| o.clone()).unwrap_or_default();
            let tail = &output[output.len().saturating_sub(1500)..];
            println!("{red}Self-test FAILED.{off} Last kernel output:\n{tail}");
            return ExitCode::FAILURE;
        }
    }
    session.type_line("shutdown");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !session.kernel_stopped() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    if !session.kernel_stopped() {
        println!("  {red}FAIL{off}  the kernel did not shut down");
        return ExitCode::FAILURE;
    }
    println!("  {green}pass{off}  {:>2}. shutdown", STEPS.len() + 1);
    println!("{green}AI Bridge self-test passed.{off}");
    ExitCode::SUCCESS
}
