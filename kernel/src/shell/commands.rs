//! Built-in shell commands and the command registry.
//!
//! Each command is a [`Command`] entry in [`COMMANDS`] (the Command pattern
//! from Docs/09-lld.md): the shell looks a command up by name, validates the
//! argument count against the entry, asks for confirmation if the entry
//! demands it, and only then calls the handler. Adding a command means adding
//! one entry and one handler function; the parser and the shell loop do not
//! change.
//!
//! Every entry also carries a [`Risk`] level. It decides what happens when the
//! AI assistant suggests the command (see [`run`]): the assistant never runs
//! anything itself, and the riskier a suggestion, the more the user has to do
//! before it runs.

use super::Shell;
use super::parser::ParsedCommand;
use crate::diagnostics::{self, Diagnostic, DiagnosticId};
use crate::logger::{self, Level, Subsystem};
use crate::fixed::FixedString;
use crate::protocol::RequestId;
use crate::protocol::inbound::{self, AiResponse, Printable, Reply, WaitError, MAX_COMMANDS, MAX_COMMAND_LENGTH};
use crate::{print, println};
use core::fmt;

/// Why a command did not complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    /// The arguments have the right count but a wrong value.
    InvalidArguments(&'static str),
    /// The command needs something that is not available right now.
    Unavailable(&'static str),
    /// The user said no at a confirmation prompt inside the command.
    Cancelled,
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::InvalidArguments(reason) => {
                write!(f, "Invalid command arguments: {}", reason)
            }
            CommandError::Unavailable(reason) => f.write_str(reason),
            CommandError::Cancelled => f.write_str("Cancelled."),
        }
    }
}

pub type CommandResult = Result<(), CommandError>;

/// How much harm a command can do. This is the risk classification from
/// Docs/11-security-design.md (section 5.2).
///
/// | Level         | Typed by the user           | Suggested by the AI (`run <n>`) |
/// |---------------|-----------------------------|---------------------------------|
/// | `Safe`        | runs                        | runs                            |
/// | `Sensitive`   | runs                        | runs after a y/N confirmation   |
/// | `Privileged`  | runs                        | runs after a y/N confirmation   |
/// | `Destructive` | runs after its confirmation | never runs: blocked             |
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    /// Only reads and prints kernel state.
    Safe,
    /// Sends information out of the kernel (to the host / AI Bridge).
    Sensitive,
    /// Changes the state of the whole system, e.g. stops it.
    Privileged,
    /// Deliberately crashes or damages the running kernel.
    Destructive,
}

impl Risk {
    pub fn as_str(self) -> &'static str {
        match self {
            Risk::Safe => "safe",
            Risk::Sensitive => "sensitive",
            Risk::Privileged => "privileged",
            Risk::Destructive => "destructive",
        }
    }

    /// Why the level was given, shown next to a suggested command.
    pub fn explanation(self) -> &'static str {
        match self {
            Risk::Safe => "only shows information",
            Risk::Sensitive => "sends data to the host",
            Risk::Privileged => "changes the state of the system",
            Risk::Destructive => "crashes or damages the kernel",
        }
    }
}

impl fmt::Display for Risk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub struct Command {
    pub name: &'static str,
    /// Usage line shown by `help <command>` and on argument errors.
    pub usage: &'static str,
    /// One-line description shown by `help`.
    pub summary: &'static str,
    pub min_args: usize,
    pub max_args: usize,
    /// `Some(warning)` marks a command that crashes or stops the system. The
    /// shell shows the warning and runs the command only after an explicit
    /// "y" from the user.
    pub confirmation: Option<&'static str>,
    pub risk: Risk,
    pub handler: fn(&mut Shell, &ParsedCommand) -> CommandResult,
}

/// The command registry, in the order `help` lists it.
pub static COMMANDS: &[Command] = &[
    Command {
        name: "help",
        usage: "help [command]",
        summary: "List commands, or show how to use one",
        min_args: 0,
        max_args: 1,
        confirmation: None,
        risk: Risk::Safe,
        handler: help,
    },
    Command {
        name: "clear",
        usage: "clear",
        summary: "Clear the screen",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        risk: Risk::Safe,
        handler: clear,
    },
    Command {
        name: "info",
        usage: "info",
        summary: "Show kernel and system information",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        risk: Risk::Safe,
        handler: info,
    },
    Command {
        name: "meminfo",
        usage: "meminfo",
        summary: "Show physical memory and heap usage",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        risk: Risk::Safe,
        handler: meminfo,
    },
    Command {
        name: "uptime",
        usage: "uptime",
        summary: "Show the time since boot",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        risk: Risk::Safe,
        handler: uptime,
    },
    Command {
        name: "echo",
        usage: "echo [text...]",
        summary: "Print the given text",
        min_args: 0,
        max_args: usize::MAX,
        confirmation: None,
        risk: Risk::Safe,
        handler: echo,
    },
    Command {
        name: "history",
        usage: "history",
        summary: "Show previously entered commands",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        risk: Risk::Safe,
        handler: history,
    },
    Command {
        name: "logs",
        usage: "logs [count] [info|warn|error|panic]",
        summary: "Show recent kernel log records",
        min_args: 0,
        max_args: 2,
        confirmation: None,
        risk: Risk::Safe,
        handler: logs,
    },
    Command {
        name: "diag",
        usage: "diag [list | last | <number> | send <number|last>]",
        summary: "Show or re-send structured diagnostic records",
        min_args: 0,
        max_args: 2,
        confirmation: None,
        risk: Risk::Safe,
        handler: diag,
    },
    Command {
        name: "ai",
        usage: "ai <question...>",
        summary: "Ask the AI assistant (needs the host AI Bridge)",
        min_args: 1,
        max_args: usize::MAX,
        confirmation: None,
        risk: Risk::Sensitive,
        handler: ai,
    },
    Command {
        name: "run",
        usage: "run [number]",
        summary: "Run a command the AI assistant suggested (after safety checks)",
        min_args: 0,
        max_args: 1,
        confirmation: None,
        risk: Risk::Safe,
        handler: run,
    },
    Command {
        name: "panic-test",
        usage: "panic-test",
        summary: "Trigger a controlled kernel panic",
        min_args: 0,
        max_args: 0,
        confirmation: Some("This deliberately crashes the kernel. A restart is required afterwards."),
        risk: Risk::Destructive,
        handler: panic_test,
    },
    Command {
        name: "fault-test",
        usage: "fault-test <breakpoint|heap|page-fault|divide-by-zero|invalid-opcode|stack-overflow>",
        summary: "Trigger a controlled failure to exercise diagnostics",
        min_args: 1,
        max_args: 1,
        confirmation: Some(
            "This injects a real fault. Every scenario except 'breakpoint' and 'heap' halts the kernel.",
        ),
        risk: Risk::Destructive,
        handler: fault_test,
    },
    Command {
        name: "shutdown",
        usage: "shutdown",
        summary: "Stop the system (exits QEMU)",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        risk: Risk::Privileged,
        handler: shutdown,
    },
];

/// The commands suggested in the most recent AI answer, kept by the shell so
/// `run <n>` can pick one. A new `ai` request replaces them.
#[derive(Clone, Copy)]
pub struct Suggested {
    request: Option<RequestId>,
    commands: [FixedString<MAX_COMMAND_LENGTH>; MAX_COMMANDS],
    count: usize,
    /// The answer asked for confirmation before any of its commands run.
    requires_confirmation: bool,
}

impl Suggested {
    pub const fn new() -> Self {
        Self {
            request: None,
            commands: [FixedString::new(); MAX_COMMANDS],
            count: 0,
            requires_confirmation: false,
        }
    }

    fn from_response(response: &AiResponse) -> Self {
        let mut suggested = Self::new();
        suggested.request = Some(response.request);
        suggested.requires_confirmation = response.requires_confirmation;
        for command in response.commands() {
            suggested.commands[suggested.count].push_str(command);
            suggested.count += 1;
        }
        suggested
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The suggestion shown as number `number` (1-based).
    pub fn get(&self, number: usize) -> Option<&str> {
        number
            .checked_sub(1)
            .filter(|&index| index < self.count)
            .map(|index| self.commands[index].as_str())
    }
}

impl Default for Suggested {
    fn default() -> Self {
        Self::new()
    }
}

/// Why a suggested command may not run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocked {
    /// The text does not parse as a command line.
    Invalid,
    /// No command has this name.
    Unknown,
    /// The command exists but the argument count is wrong.
    BadArguments,
    /// `run` itself: a suggestion may not trigger other suggestions.
    Recursive,
    /// The command is classified [`Risk::Destructive`].
    Destructive,
}

impl Blocked {
    pub fn reason(self) -> &'static str {
        match self {
            Blocked::Invalid => "it is not a valid command line",
            Blocked::Unknown => "it is not a Psydian command",
            Blocked::BadArguments => "it has the wrong number of arguments",
            Blocked::Recursive => "a suggestion cannot run other suggestions",
            Blocked::Destructive => "it is destructive; type it yourself if you really mean it",
        }
    }
}

/// Validates a suggested command line exactly as the shell would validate a
/// typed one, then applies the AI policy on top. Returns the command entry
/// the line would run.
pub fn check_suggestion(line: &str) -> Result<&'static Command, Blocked> {
    let parsed = match super::parser::parse(line) {
        Ok(Some(parsed)) => parsed,
        Ok(None) | Err(_) => return Err(Blocked::Invalid),
    };
    let entry = find(parsed.name).ok_or(Blocked::Unknown)?;
    let arg_count = parsed.args().len();
    if arg_count < entry.min_args || arg_count > entry.max_args {
        return Err(Blocked::BadArguments);
    }
    if entry.name == "run" {
        return Err(Blocked::Recursive);
    }
    if entry.risk == Risk::Destructive {
        return Err(Blocked::Destructive);
    }
    Ok(entry)
}

/// Looks a command up by its exact name.
pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.name == name)
}

/// All command names, for tab completion.
pub fn names() -> impl Iterator<Item = &'static str> {
    COMMANDS.iter().map(|command| command.name)
}

fn help(_shell: &mut Shell, command: &ParsedCommand) -> CommandResult {
    if let Some(name) = command.arg(0) {
        let entry = find(name).ok_or(CommandError::InvalidArguments("no such command"))?;
        println!("{}", entry.summary);
        println!("Usage: {}", entry.usage);
        println!("Risk:  {} ({})", entry.risk, entry.risk.explanation());
        if entry.confirmation.is_some() {
            println!("Asks for confirmation before running.");
        }
        return Ok(());
    }

    println!("Available commands:");
    for entry in COMMANDS {
        println!("  {:<12} {}", entry.name, entry.summary);
    }
    println!("Type 'help <command>' for details. Tab completes, Up/Down browse history.");
    Ok(())
}

fn clear(_shell: &mut Shell, _command: &ParsedCommand) -> CommandResult {
    crate::console::clear();
    Ok(())
}

fn info(_shell: &mut Shell, _command: &ParsedCommand) -> CommandResult {
    println!("System information");
    println!("  Name:          Psydian OS");
    println!("  Version:       {}", env!("CARGO_PKG_VERSION"));
    println!("  Architecture:  x86_64");
    println!("  CPU vendor:    {}", cpu_vendor().as_str());
    println!("  Boot method:   BIOS (bootloader 0.11)");
    match crate::framebuffer::resolution() {
        Some((width, height)) => println!("  Display:       {}x{} framebuffer", width, height),
        None => println!("  Display:       none (serial console only)"),
    }
    println!("  Timer:         {} Hz", crate::time::TICK_HZ);
    println!("  Uptime:        {}", Uptime(crate::time::uptime_ms()));
    println!(
        "  Heap:          {}",
        if crate::memory::is_initialized() {
            "available"
        } else {
            "unavailable"
        }
    );
    println!(
        "  Log records:   {} info, {} warn, {} error, {} panic",
        logger::count(Level::Info),
        logger::count(Level::Warn),
        logger::count(Level::Error),
        logger::count(Level::Panic)
    );
    Ok(())
}

/// The 12-character vendor string from CPUID leaf 0 (e.g. "GenuineIntel").
fn cpu_vendor() -> crate::fixed::FixedString<12> {
    let leaf = core::arch::x86_64::__cpuid(0);
    let mut vendor = crate::fixed::FixedString::new();
    for register in [leaf.ebx, leaf.edx, leaf.ecx] {
        for byte in register.to_le_bytes() {
            if byte.is_ascii_graphic() || byte == b' ' {
                vendor.push(byte as char);
            }
        }
    }
    vendor
}

fn meminfo(_shell: &mut Shell, _command: &ParsedCommand) -> CommandResult {
    let stats = crate::memory::stats().ok_or(CommandError::Unavailable(
        "Memory information unavailable: memory subsystem is not initialized.",
    ))?;
    const KIB: u64 = 1024;

    println!("Physical memory");
    println!(
        "  Usable:           {} KiB ({} MiB)",
        stats.usable_bytes / KIB,
        stats.usable_bytes / (KIB * KIB)
    );
    println!(
        "  Memory regions:   {} usable of {}",
        stats.usable_regions, stats.total_regions
    );
    println!(
        "  Frames (4 KiB):   {} allocated of {}",
        stats.allocated_frames, stats.total_frames
    );
    println!("  Mapped at offset: {:#x}", stats.physical_memory_offset);
    println!("Kernel heap");
    println!("  Start address:    {:#x}", crate::memory::heap::HEAP_START);
    println!("  Size:             {} KiB", stats.heap.size as u64 / KIB);
    println!("  Used:             {} bytes", stats.heap.used);
    println!("  Free:             {} bytes", stats.heap.free());
    println!("  Peak used:        {} bytes", stats.heap.peak_used);
    println!(
        "  Allocations:      {} total, {} live, {} failed",
        stats.heap.allocations, stats.heap.live_allocations, stats.heap.failed_allocations
    );
    Ok(())
}

/// Formats milliseconds as seconds with two decimals, e.g. `12.34 s`.
struct Uptime(u64);

impl fmt::Display for Uptime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:02} s", self.0 / 1000, (self.0 % 1000) / 10)
    }
}

fn uptime(_shell: &mut Shell, _command: &ParsedCommand) -> CommandResult {
    println!(
        "Uptime: {} ({} timer ticks)",
        Uptime(crate::time::uptime_ms()),
        crate::time::ticks()
    );
    Ok(())
}

fn echo(_shell: &mut Shell, command: &ParsedCommand) -> CommandResult {
    println!("{}", command.rest);
    Ok(())
}

fn history(shell: &mut Shell, _command: &ParsedCommand) -> CommandResult {
    let history = shell.history();
    if history.is_empty() {
        println!("No command history available.");
        return Ok(());
    }
    for index in 0..history.len() {
        if let Some(line) = history.get(index) {
            println!("  {:>2}  {}", index + 1, line);
        }
    }
    Ok(())
}

fn logs(_shell: &mut Shell, command: &ParsedCommand) -> CommandResult {
    const DEFAULT_COUNT: usize = 20;
    let mut count = DEFAULT_COUNT;
    let mut minimum = Level::Info;

    // The count and the level may be given in either order.
    for argument in command.args() {
        if let Ok(number) = argument.parse::<usize>() {
            if number == 0 {
                return Err(CommandError::InvalidArguments("count must be at least 1"));
            }
            count = number;
        } else if let Some(level) = Level::parse(argument) {
            minimum = level;
        } else {
            return Err(CommandError::InvalidArguments(
                "expected a count or one of info, warn, error, panic",
            ));
        }
    }

    let total = logger::history_len();
    let matching = (0..total)
        .filter_map(logger::history_get)
        .filter(|record| record.level >= minimum)
        .count();
    if matching == 0 {
        println!("No log records available.");
        return Ok(());
    }

    let mut skip = matching.saturating_sub(count);
    for record in (0..total)
        .filter_map(logger::history_get)
        .filter(|record| record.level >= minimum)
    {
        if skip > 0 {
            skip -= 1;
            continue;
        }
        println!(
            "{:>4} {:>9} {}",
            record.sequence,
            Uptime(record.uptime_ms),
            record
        );
    }
    Ok(())
}

/// Parses `last` or a diagnostic number (`3`, `diag-003`).
fn select_diagnostic(selector: &str) -> Result<Diagnostic, CommandError> {
    if selector == "last" {
        return diagnostics::last().ok_or(CommandError::Unavailable(
            "No diagnostic information available.",
        ));
    }
    let number = selector
        .strip_prefix("diag-")
        .unwrap_or(selector)
        .parse::<u32>()
        .map_err(|_| CommandError::InvalidArguments("expected 'last' or a diagnostic number"))?;
    diagnostics::find(number).ok_or(CommandError::Unavailable(
        "No such diagnostic (never reported, or no longer in the history).",
    ))
}

fn diag(_shell: &mut Shell, command: &ParsedCommand) -> CommandResult {
    match (command.arg(0), command.arg(1)) {
        (None, None) | (Some("list"), None) => {
            let stored = diagnostics::history_len();
            if stored == 0 {
                println!("No diagnostic information available.");
                return Ok(());
            }
            println!(
                "{} diagnostic record(s) reported since boot, {} stored:",
                diagnostics::total_reported(),
                stored
            );
            for record in (0..stored).filter_map(diagnostics::history_get) {
                println!(
                    "  {}  {:<5} {:<10} {:<9} {}",
                    DiagnosticId(record.id),
                    record.severity,
                    record.subsystem,
                    record.code,
                    record.message
                );
            }
            println!("Type 'diag <number>' for details.");
            Ok(())
        }
        (Some("send"), Some(selector)) => {
            let record = select_diagnostic(selector)?;
            if !crate::protocol::send_diagnostic(&record) {
                return Err(CommandError::Unavailable(
                    "Diagnostic communication failed: record does not fit into a frame.",
                ));
            }
            println!("{} sent to the diagnostic channel.", DiagnosticId(record.id));
            Ok(())
        }
        (Some(selector), None) => {
            println!("{}", select_diagnostic(selector)?);
            Ok(())
        }
        _ => Err(CommandError::InvalidArguments("unexpected extra argument")),
    }
}

fn ai(shell: &mut Shell, command: &ParsedCommand) -> CommandResult {
    // The kernel never talks to an AI service itself. The question goes out
    // as an `ai_request` frame on the serial diagnostic channel, where the
    // host-side AI Bridge picks it up. The most recent diagnostic is
    // referenced so "why did that happen?" has something to refer to.
    let related = diagnostics::last().map(|record| DiagnosticId(record.id));
    let request = crate::protocol::send_ai_request(command.rest, related).ok_or(
        CommandError::Unavailable("Diagnostic communication failed: request is too long."),
    )?;
    println!("[AI] Request {} sent to the AI Bridge. Waiting for the answer (Ctrl+C cancels)...", request);
    // Suggestions from an older answer must not be run in reply to this one.
    shell.suggested = Suggested::new();

    // The answer comes back as an `ai_response` frame on the serial line.
    // Whatever arrives is only displayed: suggested commands are shown, never
    // run, and any failure leaves the shell working (fail closed).
    match inbound::wait_for_reply(request) {
        Ok(Reply::Answer(response)) => {
            shell.suggested = Suggested::from_response(&response);
            print_ai_response(&response);
            Ok(())
        }
        Ok(Reply::BridgeError(error)) => {
            logger::record(
                Level::Warn,
                Subsystem::Ai,
                "AI-001",
                format_args!("AI Bridge error for {}: {}", request, error.code),
            );
            if error.message.as_str().is_empty() {
                println!("[AI] The AI Bridge reported {}.", Printable(error.code.as_str()));
            } else {
                println!(
                    "[AI] The AI Bridge reported {}: {}",
                    Printable(error.code.as_str()),
                    Printable(error.message.as_str())
                );
            }
            Err(CommandError::Unavailable("AI assistance unavailable for this request."))
        }
        Err(WaitError::Invalid(reason)) => {
            logger::record(
                Level::Warn,
                Subsystem::Ai,
                "AI-003",
                format_args!("Invalid AI response for {}: {}", request, reason),
            );
            println!("[AI] Answer rejected: {}.", reason);
            Err(CommandError::Unavailable(
                "AI assistance unavailable: the AI Bridge sent an invalid response.",
            ))
        }
        Err(WaitError::Cancelled) => Err(CommandError::Unavailable("AI request cancelled.")),
        Err(WaitError::Timeout { bridge_seen: false }) => Err(CommandError::Unavailable(
            "AI assistance unavailable: no response from the AI Bridge.",
        )),
        Err(WaitError::Timeout { bridge_seen: true }) => {
            logger::record(
                Level::Warn,
                Subsystem::Ai,
                "AI-002",
                format_args!("No answer for {} before the timeout", request),
            );
            Err(CommandError::Unavailable(
                "AI assistance unavailable: the AI Bridge did not answer in time.",
            ))
        }
    }
}

fn print_ai_response(response: &AiResponse) {
    println!("[AI] Answer to {}:", response.request);
    println!("  {}", Printable(response.summary.as_str()));
    if response.suggestions().next().is_some() {
        println!("  Suggestions:");
        for (index, suggestion) in response.suggestions().enumerate() {
            println!("    {}. {}", index + 1, Printable(suggestion));
        }
    }
    if response.commands().next().is_some() {
        println!("  Suggested commands (nothing runs until you type 'run <number>'):");
        print_suggested(&Suggested::from_response(response));
    }
}

/// One line per suggested command, with the check result the user will get.
fn print_suggested(suggested: &Suggested) {
    const COLUMN: usize = 24;
    for number in 1..=suggested.len() {
        let line = suggested.get(number).unwrap_or("");
        // `Printable` writes character by character, so pad by hand.
        print!("    {}. {}", number, Printable(line));
        for _ in line.chars().count()..COLUMN {
            print!(" ");
        }
        match check_suggestion(line) {
            Ok(entry) => println!(" [{}]", entry.risk),
            Err(blocked) => println!(" [blocked: {}]", blocked.reason()),
        }
    }
}

/// `run [number]`: runs one command from the latest AI answer.
///
/// ```text
/// suggestion ─► parser ─► registry lookup ─► argument check ─► risk policy
///                                                                  │
///                 execution (same path as typed) ◄─ confirmation ◄─┘
/// ```
///
/// The AI never gets here on its own: only the user typing `run` does.
fn run(shell: &mut Shell, command: &ParsedCommand) -> CommandResult {
    let Some(selector) = command.arg(0) else {
        if shell.suggested.is_empty() {
            println!("No suggested commands. Ask the assistant first: ai <question>");
        } else {
            if let Some(request) = shell.suggested.request {
                println!("Commands suggested in the answer to {}:", request);
            }
            print_suggested(&shell.suggested);
            println!("Type 'run <number>' to run one.");
        }
        return Ok(());
    };

    if shell.suggested.is_empty() {
        return Err(CommandError::Unavailable(
            "No suggested commands. Ask the assistant first: ai <question>",
        ));
    }
    let number = selector
        .parse::<usize>()
        .map_err(|_| CommandError::InvalidArguments("expected the number of a suggested command"))?;
    // Copied out so the suggestion survives a command that replaces the list.
    let line: FixedString<MAX_COMMAND_LENGTH> = FixedString::from_str_truncated(
        shell
            .suggested
            .get(number)
            .ok_or(CommandError::InvalidArguments("no suggested command has this number"))?,
    );
    let request = shell.suggested.request;

    let entry = match check_suggestion(line.as_str()) {
        Ok(entry) => entry,
        Err(blocked) => {
            logger::record(
                Level::Warn,
                Subsystem::Ai,
                "AI-011",
                format_args!("Blocked AI-suggested command '{}': {}", Printable(line.as_str()), blocked.reason()),
            );
            println!("[AI] Not running '{}': {}.", Printable(line.as_str()), blocked.reason());
            return Err(CommandError::Unavailable("The suggested command was blocked."));
        }
    };

    if entry.risk > Risk::Safe || shell.suggested.requires_confirmation {
        println!(
            "[AI] Suggested command: {} (risk: {}, {})",
            Printable(line.as_str()),
            entry.risk,
            entry.risk.explanation()
        );
        if !shell.confirm("This command was suggested by the AI assistant, not typed by you.") {
            logger::record(
                Level::Info,
                Subsystem::Ai,
                "AI-012",
                format_args!("AI-suggested command '{}' declined", Printable(line.as_str())),
            );
            return Err(CommandError::Cancelled);
        }
    }

    logger::record(
        Level::Info,
        Subsystem::Ai,
        "AI-010",
        format_args!(
            "Running AI-suggested command '{}'{}{}",
            Printable(line.as_str()),
            if request.is_some() { " from " } else { "" },
            OptionalId(request)
        ),
    );
    println!("[AI] Running suggested command: {}", Printable(line.as_str()));
    // Exactly the path a typed line takes: parse, validate, confirm, run.
    match shell.execute(line.as_str()) {
        super::Outcome::Executed => Ok(()),
        super::Outcome::Cancelled => Err(CommandError::Cancelled),
        _ => Err(CommandError::Unavailable("The suggested command did not complete.")),
    }
}

/// Prints a request id, or nothing.
struct OptionalId(Option<RequestId>);

impl fmt::Display for OptionalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(id) => write!(f, "{}", id),
            None => Ok(()),
        }
    }
}

fn panic_test(_shell: &mut Shell, _command: &ParsedCommand) -> CommandResult {
    panic!("controlled panic requested with the 'panic-test' shell command");
}

fn fault_test(_shell: &mut Shell, command: &ParsedCommand) -> CommandResult {
    match command.arg(0).unwrap_or("") {
        // Recoverable: the handler records a diagnostic and execution continues.
        "breakpoint" => {
            x86_64::instructions::interrupts::int3();
            println!("Breakpoint handled; the kernel kept running.");
            Ok(())
        }
        // Recoverable: a request far larger than the heap fails with a null
        // pointer, which is reported as a diagnostic instead of a crash.
        "heap" => {
            const REQUEST: usize = 64 * 1024 * 1024;
            let layout = core::alloc::Layout::from_size_align(REQUEST, 8)
                .map_err(|_| CommandError::Unavailable("Could not build the test allocation."))?;
            let pointer = unsafe { alloc::alloc::alloc(layout) };
            if !pointer.is_null() {
                unsafe { alloc::alloc::dealloc(pointer, layout) };
                println!("Allocation unexpectedly succeeded; no failure to report.");
                return Ok(());
            }
            let heap = crate::allocator::stats();
            diagnostics::report(
                Diagnostic::new(
                    Level::Error,
                    crate::logger::Subsystem::Memory,
                    "MEM-003",
                    format_args!("Heap allocation failed"),
                )
                .with("size", REQUEST)
                .with("align", 8)
                .with("heap_size", heap.size)
                .with("heap_used", heap.used),
            );
            println!("Allocation failure handled; the kernel kept running.");
            Ok(())
        }
        "page-fault" => {
            // Nothing is mapped at this address.
            let unmapped = 0xdead_beef_000_u64 as *mut u8;
            unsafe { unmapped.write_volatile(42) };
            Ok(())
        }
        "divide-by-zero" => {
            // Rust checks integer division itself, so the CPU exception has
            // to be provoked with the instruction directly.
            unsafe {
                core::arch::asm!(
                    "div {divisor}",
                    divisor = in(reg) 0u64,
                    inout("rax") 1u64 => _,
                    inout("rdx") 0u64 => _,
                );
            }
            Ok(())
        }
        "invalid-opcode" => {
            unsafe { core::arch::asm!("ud2") };
            Ok(())
        }
        "stack-overflow" => {
            overflow_stack(0);
            Ok(())
        }
        _ => Err(CommandError::InvalidArguments("unknown fault scenario")),
    }
}

/// Recurses until the kernel stack hits its guard page.
#[allow(unconditional_recursion)]
fn overflow_stack(depth: u64) -> u64 {
    let padding = [depth; 64];
    // The use after the call keeps the compiler from turning this into a loop.
    overflow_stack(depth + 1) + core::hint::black_box(padding)[0]
}

fn shutdown(_shell: &mut Shell, _command: &ParsedCommand) -> CommandResult {
    println!("Shutting down.");
    crate::exit_qemu(crate::QemuExitCode::Success);
    // Not running under QEMU (or without the debug-exit device): the best
    // remaining option is to stop the CPU.
    print!("It is now safe to turn off the machine.");
    x86_64::instructions::interrupts::disable();
    crate::hlt_loop();
}

#[test_case]
fn command_names_are_unique_and_valid() {
    for (index, command) in COMMANDS.iter().enumerate() {
        assert!(!command.name.is_empty());
        assert!(command.min_args <= command.max_args);
        assert!(command.usage.starts_with(command.name));
        assert!(
            COMMANDS[index + 1..].iter().all(|other| other.name != command.name),
            "duplicate command name"
        );
        // Every name must survive the parser's own validation.
        let parsed = super::parser::parse(command.name).unwrap().unwrap();
        assert_eq!(parsed.name, command.name);
    }
}

#[test_case]
fn find_is_exact() {
    assert!(find("help").is_some());
    assert!(find("hel").is_none());
    assert!(find("HELP").is_none());
    assert!(find("").is_none());
}

#[test_case]
fn roadmap_commands_exist() {
    for name in ["help", "clear", "info", "meminfo", "panic-test", "ai"] {
        assert!(find(name).is_some());
    }
}

#[test_case]
fn destructive_commands_require_confirmation() {
    assert!(find("panic-test").unwrap().confirmation.is_some());
    assert!(find("fault-test").unwrap().confirmation.is_some());
    assert!(find("help").unwrap().confirmation.is_none());
}

#[test_case]
fn risk_levels_match_the_policy() {
    // Everything destructive also asks the user when typed by hand ...
    for command in COMMANDS {
        if command.risk == Risk::Destructive {
            assert!(command.confirmation.is_some(), "{} needs a confirmation", command.name);
        }
    }
    // ... and the classification of the existing commands is fixed.
    for (name, risk) in [
        ("help", Risk::Safe),
        ("meminfo", Risk::Safe),
        ("diag", Risk::Safe),
        ("ai", Risk::Sensitive),
        ("shutdown", Risk::Privileged),
        ("panic-test", Risk::Destructive),
        ("fault-test", Risk::Destructive),
    ] {
        assert_eq!(find(name).unwrap().risk, risk, "{}", name);
    }
}

#[test_case]
fn suggestions_are_checked_like_typed_commands() {
    assert_eq!(check_suggestion("meminfo").map(|c| c.name), Ok("meminfo"));
    assert_eq!(check_suggestion("  diag last ").map(|c| c.name), Ok("diag"));
    assert_eq!(check_suggestion("shutdown").map(|c| c.risk), Ok(Risk::Privileged));
    assert_eq!(check_suggestion("ai hi").map(|c| c.risk), Ok(Risk::Sensitive));
    assert_eq!(check_suggestion("").err(), Some(Blocked::Invalid));
    assert_eq!(check_suggestion("help;shutdown").err(), Some(Blocked::Invalid));
    assert_eq!(check_suggestion("echo \"open").err(), Some(Blocked::Invalid));
    assert_eq!(check_suggestion("rm -rf /").err(), Some(Blocked::Unknown));
    assert_eq!(check_suggestion("MEMINFO").err(), Some(Blocked::Unknown));
    assert_eq!(check_suggestion("meminfo now").err(), Some(Blocked::BadArguments));
    assert_eq!(check_suggestion("run 1").err(), Some(Blocked::Recursive));
    assert_eq!(check_suggestion("panic-test").err(), Some(Blocked::Destructive));
    assert_eq!(check_suggestion("fault-test heap").err(), Some(Blocked::Destructive));
}

#[test_case]
fn suggested_numbers_are_one_based_and_bounded() {
    let suggested = Suggested::new();
    assert!(suggested.is_empty());
    assert_eq!(suggested.get(1), None);

    let mut suggested = Suggested::new();
    suggested.commands[0].push_str("meminfo");
    suggested.commands[1].push_str("diag last");
    suggested.count = 2;
    assert_eq!(suggested.get(0), None);
    assert_eq!(suggested.get(1), Some("meminfo"));
    assert_eq!(suggested.get(2), Some("diag last"));
    assert_eq!(suggested.get(3), None);
    assert_eq!(suggested.get(usize::MAX), None);
}

#[test_case]
fn uptime_formatting() {
    use core::fmt::Write;
    let mut text: crate::fixed::FixedString<16> = crate::fixed::FixedString::new();
    let _ = write!(text, "{}", Uptime(12_345));
    assert_eq!(text, "12.34 s");
}
