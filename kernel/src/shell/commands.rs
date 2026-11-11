//! Built-in shell commands and the command registry.
//!
//! Each command is a [`Command`] entry in [`COMMANDS`] (the Command pattern
//! from Docs/09-lld.md): the shell looks a command up by name, validates the
//! argument count against the entry, asks for confirmation if the entry
//! demands it, and only then calls the handler. Adding a command means adding
//! one entry and one handler function; the parser and the shell loop do not
//! change.

use super::Shell;
use super::parser::ParsedCommand;
use crate::logger::{self, Level};
use crate::{print, println};
use core::fmt;

/// Why a command did not complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    /// The arguments have the right count but a wrong value.
    InvalidArguments(&'static str),
    /// The command needs something that is not available right now.
    Unavailable(&'static str),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::InvalidArguments(reason) => {
                write!(f, "Invalid command arguments: {}", reason)
            }
            CommandError::Unavailable(reason) => f.write_str(reason),
        }
    }
}

pub type CommandResult = Result<(), CommandError>;

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
        handler: help,
    },
    Command {
        name: "clear",
        usage: "clear",
        summary: "Clear the screen",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        handler: clear,
    },
    Command {
        name: "info",
        usage: "info",
        summary: "Show kernel and system information",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        handler: info,
    },
    Command {
        name: "meminfo",
        usage: "meminfo",
        summary: "Show physical memory and heap usage",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        handler: meminfo,
    },
    Command {
        name: "uptime",
        usage: "uptime",
        summary: "Show the time since boot",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        handler: uptime,
    },
    Command {
        name: "echo",
        usage: "echo [text...]",
        summary: "Print the given text",
        min_args: 0,
        max_args: usize::MAX,
        confirmation: None,
        handler: echo,
    },
    Command {
        name: "history",
        usage: "history",
        summary: "Show previously entered commands",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        handler: history,
    },
    Command {
        name: "logs",
        usage: "logs [count] [info|warn|error|panic]",
        summary: "Show recent kernel log records",
        min_args: 0,
        max_args: 2,
        confirmation: None,
        handler: logs,
    },
    Command {
        name: "ai",
        usage: "ai <question...>",
        summary: "Ask the AI assistant (needs the host AI Bridge)",
        min_args: 1,
        max_args: usize::MAX,
        confirmation: None,
        handler: ai,
    },
    Command {
        name: "panic-test",
        usage: "panic-test",
        summary: "Trigger a controlled kernel panic",
        min_args: 0,
        max_args: 0,
        confirmation: Some("This deliberately crashes the kernel. A restart is required afterwards."),
        handler: panic_test,
    },
    Command {
        name: "shutdown",
        usage: "shutdown",
        summary: "Stop the system (exits QEMU)",
        min_args: 0,
        max_args: 0,
        confirmation: None,
        handler: shutdown,
    },
];

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

fn ai(_shell: &mut Shell, _command: &ParsedCommand) -> CommandResult {
    // The kernel never talks to an AI service itself. Requests will travel
    // over the serial diagnostic channel to the host-side AI Bridge; until
    // that exists the command fails cleanly and the shell stays usable.
    Err(CommandError::Unavailable(
        "AI assistance unavailable: the host AI Bridge is not connected.",
    ))
}

fn panic_test(_shell: &mut Shell, _command: &ParsedCommand) -> CommandResult {
    panic!("controlled panic requested with the 'panic-test' shell command");
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
    assert!(find("help").unwrap().confirmation.is_none());
}

#[test_case]
fn uptime_formatting() {
    use core::fmt::Write;
    let mut text: crate::fixed::FixedString<16> = crate::fixed::FixedString::new();
    let _ = write!(text, "{}", Uptime(12_345));
    assert_eq!(text, "12.34 s");
}
