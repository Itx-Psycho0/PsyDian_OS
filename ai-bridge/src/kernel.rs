//! The serial link to the kernel: either a QEMU process started by the
//! bridge (serial on its stdin/stdout) or a TCP socket that QEMU's serial
//! port listens on (`cargo run -- --serial-tcp <port>`).

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type SharedWriter = Arc<Mutex<Box<dyn Write + Send>>>;

pub struct Link {
    pub reader: Box<dyn Read + Send>,
    pub writer: SharedWriter,
    pub child: Option<Child>,
}

/// Starts `command` (QEMU through the launcher) with its stdio as the link.
pub fn spawn(command: &[String], cwd: &Path) -> io::Result<Link> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| io::Error::other("empty kernel command"))?;
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("no stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("no stdout"))?;
    Ok(Link {
        reader: Box::new(stdout),
        writer: Arc::new(Mutex::new(Box::new(stdin))),
        child: Some(child),
    })
}

/// Starts `command` in the background (its output stays on the terminal)
/// and connects to the serial port it opens at `address`.
pub fn spawn_and_connect(
    command: &[String],
    cwd: &Path,
    address: &str,
    wait: Duration,
) -> io::Result<Link> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| io::Error::other("empty kernel command"))?;
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .spawn()?;
    let started = Instant::now();
    loop {
        match TcpStream::connect(address) {
            Ok(stream) => {
                let mut link = from_stream(stream)?;
                link.child = Some(child);
                return Ok(link);
            }
            Err(error) => {
                if let Some(status) = child.try_wait()? {
                    return Err(io::Error::other(format!(
                        "the kernel exited ({status}) before its serial port opened"
                    )));
                }
                if started.elapsed() > wait {
                    let _ = child.kill();
                    return Err(error);
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        }
    }
}

/// Connects to a QEMU serial port that is already listening.
pub fn connect(address: &str) -> io::Result<Link> {
    from_stream(TcpStream::connect(address)?)
}

fn from_stream(stream: TcpStream) -> io::Result<Link> {
    stream.set_nodelay(true)?;
    let reader = stream.try_clone()?;
    Ok(Link {
        reader: Box::new(reader),
        writer: Arc::new(Mutex::new(Box::new(stream))),
        child: None,
    })
}

/// Writes bytes to the kernel's serial input.
pub fn send(writer: &SharedWriter, bytes: &[u8]) {
    if let Ok(mut w) = writer.lock() {
        let _ = w.write_all(bytes);
        let _ = w.flush();
    }
}

/// Sends a frame. `fresh_line` puts a line break first so the kernel sees
/// the marker at the start of a line even if a key was pressed meanwhile;
/// it is left out when the kernel sits at its prompt (it would show an
/// empty command line).
pub fn send_frame(writer: &SharedWriter, frame: &str, fresh_line: bool) {
    let mut bytes = Vec::with_capacity(frame.len() + 2);
    if fresh_line {
        bytes.push(b'\n');
    }
    bytes.extend_from_slice(frame.as_bytes());
    bytes.push(b'\n');
    send(writer, &bytes);
}
