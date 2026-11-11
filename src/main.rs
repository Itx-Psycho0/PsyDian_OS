//! Host-side launcher for Psydian.
//!
//! * `cargo run`                 boots the kernel image in a QEMU window, with the
//!                               serial port attached to this terminal.
//! * `cargo run -- --headless`   the same without a window (serial console only).
//! * `cargo run -- --debug`      also logs CPU exceptions/resets to `qemu.log`.
//! * `cargo ktest`               cargo calls this binary once per kernel test
//!                               executable (see `.cargo/config.toml`). The ELF
//!                               is wrapped in a bootable image, run headless,
//!                               and QEMU's exit code is turned into pass/fail.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

/// Image of the real kernel, produced by `build.rs`.
const KERNEL_IMAGE: &str = env!("PSYDIAN_BIOS_IMAGE");

/// A kernel test writes 0x10 to the `isa-debug-exit` port on success; QEMU then
/// exits with `(0x10 << 1) | 1`.
const TEST_SUCCESS_EXIT_CODE: i32 = 33;

/// A test that has not finished after this long is treated as hung.
const TEST_TIMEOUT: Duration = Duration::from_secs(60);

fn main() -> ExitCode {
    let mut elf: Option<PathBuf> = None;
    let mut headless = false;
    let mut debug = false;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--headless" => headless = true,
            "--debug" => debug = true,
            "-h" | "--help" => {
                println!("usage: cargo run [-- [--headless] [--debug] [KERNEL_ELF]]");
                return ExitCode::SUCCESS;
            }
            other if elf.is_none() && Path::new(other).is_file() => {
                elf = Some(PathBuf::from(other));
            }
            // Extra arguments (e.g. test filters passed by cargo) mean nothing
            // to a bare-metal test binary.
            _ => {}
        }
    }

    match elf {
        Some(elf) if is_test_executable(&elf) => run_test(&elf),
        Some(elf) => {
            let image = match create_image(&elf) {
                Ok(image) => image,
                Err(code) => return code,
            };
            run_interactive(&image, headless, debug)
        }
        None => run_interactive(Path::new(KERNEL_IMAGE), headless, debug),
    }
}

/// Cargo names test executables `<name>-<16 hex digits>`; the real kernel
/// binary is plain `kernel`.
fn is_test_executable(elf: &Path) -> bool {
    elf.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.rsplit_once('-'))
        .is_some_and(|(_, hash)| hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn create_image(elf: &Path) -> Result<PathBuf, ExitCode> {
    let image = elf.with_extension("img");
    let mut boot_config = bootloader::BootConfig::default();
    boot_config.serial_logging = false;
    boot_config.frame_buffer_logging = false;

    bootloader::BiosBoot::new(elf)
        .set_boot_config(&boot_config)
        .create_disk_image(&image)
        .map_err(|err| {
            eprintln!("failed to create a boot image for {}: {err:?}", elf.display());
            ExitCode::FAILURE
        })?;
    Ok(image)
}

fn qemu(image: &Path) -> Command {
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.arg("-drive")
        .arg(format!("format=raw,file={}", image.display()))
        .args(["-device", "isa-debug-exit,iobase=0xf4,iosize=0x04"])
        .args(["-serial", "stdio"]);
    cmd
}

fn run_interactive(image: &Path, headless: bool, debug: bool) -> ExitCode {
    let mut cmd = qemu(image);
    if headless {
        cmd.args(["-display", "none"]);
    }
    if debug {
        // -no-reboot: a triple fault makes QEMU stop instead of silently
        // restarting forever.
        cmd.args(["-no-reboot", "-no-shutdown"])
            .args(["-d", "int,cpu_reset,guest_errors", "-D", "qemu.log"]);
        eprintln!("debug mode: CPU events are logged to qemu.log");
    }

    match cmd.status() {
        // The `shutdown` shell command leaves through isa-debug-exit, which
        // reports the same code as a passing test.
        Ok(status) if status.success() || status.code() == Some(TEST_SUCCESS_EXIT_CODE) => {
            ExitCode::SUCCESS
        }
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(err) => {
            eprintln!("failed to start qemu-system-x86_64: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_test(elf: &Path) -> ExitCode {
    let image = match create_image(elf) {
        Ok(image) => image,
        Err(code) => return code,
    };

    let mut cmd = qemu(&image);
    cmd.args(["-display", "none", "-no-reboot"]).stdin(Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            eprintln!("failed to start qemu-system-x86_64: {err}");
            return ExitCode::FAILURE;
        }
    };

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > TEST_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                eprintln!("\n[timed out after {}s]", TEST_TIMEOUT.as_secs());
                return ExitCode::FAILURE;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(err) => {
                eprintln!("failed to wait for QEMU: {err}");
                return ExitCode::FAILURE;
            }
        }
    };

    let _ = std::fs::remove_file(&image);

    if status.code() == Some(TEST_SUCCESS_EXIT_CODE) {
        ExitCode::SUCCESS
    } else {
        eprintln!("[QEMU exited with {status}]");
        ExitCode::FAILURE
    }
}
