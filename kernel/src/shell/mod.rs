//! Interactive command-line shell.
//!
//! ```text
//! Keyboard / serial ─► input queue ─► line editor ─► parser ─► validation
//!                                                                 │
//!                         output ◄─ command handler ◄─ confirmation (if required)
//! ```
//!
//! The shell uses only fixed-size buffers, so it keeps working when the heap
//! is unavailable or exhausted.

pub mod autocomplete;
pub mod commands;
pub mod history;
pub mod parser;

use crate::fixed::FixedString;
use crate::input::{self, Key};
use crate::logger::{self, Level, Subsystem};
use crate::{print, println};
use autocomplete::Completion;
use history::History;
use parser::MAX_LINE_LENGTH;

const PROMPT: &str = "psydian> ";

type Line = FixedString<MAX_LINE_LENGTH>;

/// What happened to a submitted line. Returned by [`Shell::execute`] so the
/// behaviour can be tested without inspecting console output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The line was blank.
    Empty,
    /// The command ran and succeeded.
    Executed,
    /// The line could not be parsed.
    ParseError,
    /// No command has this name.
    UnknownCommand,
    /// The command exists but was given a wrong number of arguments.
    InvalidArguments,
    /// The user did not confirm a command that requires confirmation.
    Cancelled,
    /// The command ran and reported an error.
    Failed,
}

pub struct Shell {
    /// The line being edited.
    line: Line,
    history: History,
}

impl Shell {
    pub const fn new() -> Self {
        Self {
            line: FixedString::new(),
            history: History::new(),
        }
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    /// Runs the shell forever: prompt, read a line, execute it.
    pub fn run(&mut self) -> ! {
        println!();
        println!(
            "Psydian OS v{} - AI-assisted operating system prototype",
            env!("CARGO_PKG_VERSION")
        );
        println!("Type 'help' to list the available commands.");
        println!();

        loop {
            print!("{}", PROMPT);
            let line = self.read_line(true);
            self.history.push(line.as_str());
            self.execute(line.as_str());
        }
    }

    /// Reads one line with editing. `interactive` enables history browsing
    /// and tab completion (off while answering a yes/no question).
    fn read_line(&mut self, interactive: bool) -> Line {
        self.line.clear();
        self.history.reset_cursor();

        loop {
            match input::read_key() {
                Key::Char(c) => {
                    // A full line ignores further characters instead of
                    // silently dropping the end of a command.
                    if self.line.push(c) {
                        print!("{}", c);
                    }
                }
                Key::Backspace => {
                    if self.line.pop().is_some() {
                        print!("\u{8}");
                    }
                }
                Key::Enter => {
                    println!();
                    return self.line;
                }
                Key::Interrupt => {
                    println!("^C");
                    self.line.clear();
                    return self.line;
                }
                Key::ClearScreen => {
                    crate::console::clear();
                    print!("{}{}", PROMPT, self.line);
                }
                Key::Tab if interactive => self.complete(),
                Key::Up if interactive => {
                    if let Some(entry) = self.history.previous() {
                        let entry = Line::from_str_truncated(entry);
                        self.replace_line(entry.as_str());
                    }
                }
                Key::Down if interactive => {
                    let entry = Line::from_str_truncated(self.history.next().unwrap_or(""));
                    self.replace_line(entry.as_str());
                }
                // No cursor movement inside the line yet.
                _ => {}
            }
        }
    }

    /// Replaces the edited line on screen and in the buffer.
    fn replace_line(&mut self, text: &str) {
        for _ in 0..self.line.as_str().chars().count() {
            print!("\u{8}");
        }
        self.line.clear();
        self.line.push_str(text);
        print!("{}", self.line);
    }

    /// Tab: completes the command name (also after `help `).
    fn complete(&mut self) {
        let typed = self.line;
        let typed = typed.as_str();
        let (head, word) = match typed.strip_prefix("help ") {
            Some(argument) => ("help ", argument),
            None => ("", typed),
        };
        if word.contains(' ') {
            return; // arguments of other commands are free text
        }

        let completed = match autocomplete::complete(word, commands::names()) {
            Completion::NoMatch => return,
            Completion::Unique(name) => {
                let mut line = Line::new();
                line.push_str(head);
                line.push_str(name);
                line.push(' ');
                line
            }
            Completion::Ambiguous { common_prefix, .. } => {
                if common_prefix.len() == word.len() {
                    // Nothing more to add: show the choices instead.
                    println!();
                    for name in commands::names().filter(|name| name.starts_with(word)) {
                        print!("{}  ", name);
                    }
                    println!();
                    print!("{}{}", PROMPT, typed);
                    return;
                }
                let mut line = Line::new();
                line.push_str(head);
                line.push_str(common_prefix);
                line
            }
        };
        self.replace_line(completed.as_str());
    }

    /// Asks a yes/no question. Only an explicit "y" or "yes" counts as yes.
    fn confirm(&mut self, warning: &str) -> bool {
        println!("[WARN] {}", warning);
        print!("Proceed? [y/N] ");
        let answer = self.read_line(false);
        let answer = answer.as_str().trim();
        answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes")
    }

    /// Parses, validates and runs one command line.
    ///
    /// Every command, whoever suggests it (the user today, the AI assistant
    /// later), goes through this single path, so validation and confirmation
    /// cannot be bypassed.
    pub fn execute(&mut self, line: &str) -> Outcome {
        let command = match parser::parse(line) {
            Ok(Some(command)) => command,
            Ok(None) => return Outcome::Empty,
            Err(err) => {
                println!("[ERROR] Invalid command: {}.", err);
                logger::record(
                    Level::Warn,
                    Subsystem::Shell,
                    "CMD-001",
                    format_args!("Invalid command: {}", err),
                );
                return Outcome::ParseError;
            }
        };

        let Some(entry) = commands::find(command.name) else {
            println!("[ERROR] Unknown command: {}", command.name);
            println!("Type 'help' to list the available commands.");
            logger::record(
                Level::Warn,
                Subsystem::Shell,
                "CMD-001",
                format_args!("Unknown command: {}", command.name),
            );
            return Outcome::UnknownCommand;
        };

        let arg_count = command.args().len();
        if arg_count < entry.min_args || arg_count > entry.max_args {
            println!("[ERROR] Invalid command arguments.");
            println!("Usage: {}", entry.usage);
            logger::record(
                Level::Warn,
                Subsystem::Shell,
                "CMD-002",
                format_args!("Invalid arguments for '{}'", entry.name),
            );
            return Outcome::InvalidArguments;
        }

        if let Some(warning) = entry.confirmation {
            if !self.confirm(warning) {
                println!("Cancelled.");
                logger::record(
                    Level::Info,
                    Subsystem::Shell,
                    "CMD-004",
                    format_args!("Command '{}' cancelled at confirmation", entry.name),
                );
                return Outcome::Cancelled;
            }
        }

        match (entry.handler)(self, &command) {
            Ok(()) => {
                logger::record(
                    Level::Info,
                    Subsystem::Shell,
                    "CMD-010",
                    format_args!("Command executed: {}", entry.name),
                );
                Outcome::Executed
            }
            Err(err) => {
                println!("[ERROR] {}", err);
                if matches!(err, commands::CommandError::InvalidArguments(_)) {
                    println!("Usage: {}", entry.usage);
                }
                logger::record(
                    Level::Warn,
                    Subsystem::Shell,
                    "CMD-003",
                    format_args!("Command '{}' failed: {}", entry.name, err),
                );
                Outcome::Failed
            }
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
fn type_text(text: &str) {
    for c in text.chars() {
        input::push_key(Key::from_char(c).expect("test input must be typable"));
    }
}

#[test_case]
fn blank_line_does_nothing() {
    let mut shell = Shell::new();
    assert_eq!(shell.execute(""), Outcome::Empty);
    assert_eq!(shell.execute("    "), Outcome::Empty);
}

#[test_case]
fn valid_commands_execute() {
    let mut shell = Shell::new();
    for line in ["help", "help meminfo", "info", "meminfo", "uptime", "echo hi there", "history", "logs 3 warn"] {
        assert_eq!(shell.execute(line), Outcome::Executed);
    }
}

#[test_case]
fn unknown_command_is_reported() {
    let mut shell = Shell::new();
    assert_eq!(shell.execute("xyz"), Outcome::UnknownCommand);
    // Commands are case-sensitive and never matched by prefix.
    assert_eq!(shell.execute("HELP"), Outcome::UnknownCommand);
    assert_eq!(shell.execute("hel"), Outcome::UnknownCommand);
}

#[test_case]
fn malformed_input_is_rejected() {
    let mut shell = Shell::new();
    assert_eq!(shell.execute("echo \"unterminated"), Outcome::ParseError);
    assert_eq!(shell.execute("help;shutdown"), Outcome::ParseError);
}

#[test_case]
fn wrong_argument_count_is_rejected() {
    let mut shell = Shell::new();
    assert_eq!(shell.execute("meminfo now"), Outcome::InvalidArguments);
    assert_eq!(shell.execute("help a b"), Outcome::InvalidArguments);
    assert_eq!(shell.execute("ai"), Outcome::InvalidArguments);
}

#[test_case]
fn bad_argument_values_fail_cleanly() {
    let mut shell = Shell::new();
    assert_eq!(shell.execute("help nosuchcommand"), Outcome::Failed);
    assert_eq!(shell.execute("logs verbose"), Outcome::Failed);
    assert_eq!(shell.execute("logs 0"), Outcome::Failed);
}

#[test_case]
fn ai_fails_without_breaking_the_shell() {
    let mut shell = Shell::new();
    assert_eq!(shell.execute("ai why did the kernel panic?"), Outcome::Failed);
    assert_eq!(shell.execute("help"), Outcome::Executed);
}

#[test_case]
fn destructive_command_needs_explicit_yes() {
    let mut shell = Shell::new();
    input::clear();
    // Just pressing Enter means "no".
    type_text("\n");
    assert_eq!(shell.execute("panic-test"), Outcome::Cancelled);
    type_text("n\n");
    assert_eq!(shell.execute("panic-test"), Outcome::Cancelled);
    type_text("maybe\n");
    assert_eq!(shell.execute("panic-test"), Outcome::Cancelled);
}

#[test_case]
fn line_editing_and_history_recall() {
    let mut shell = Shell::new();
    input::clear();

    // Typing with a correction: "helx" <backspace> "p".
    type_text("helx");
    input::push_key(Key::Backspace);
    type_text("p\n");
    let line = shell.read_line(true);
    assert_eq!(line, "help");
    shell.history.push(line.as_str());

    // Up recalls it, Down returns to an empty line.
    input::push_key(Key::Up);
    type_text("\n");
    assert_eq!(shell.read_line(true), "help");
    input::push_key(Key::Up);
    input::push_key(Key::Down);
    type_text("\n");
    assert_eq!(shell.read_line(true), "");
}

#[test_case]
fn tab_completes_command_names() {
    let mut shell = Shell::new();
    input::clear();

    type_text("mem\t\n");
    assert_eq!(shell.read_line(true), "meminfo ");

    type_text("help pan\t\n");
    assert_eq!(shell.read_line(true), "help panic-test ");

    // Ambiguous ("help", "history"): the line is left as typed.
    type_text("h\t\n");
    assert_eq!(shell.read_line(true), "h");
}

#[test_case]
fn ctrl_c_abandons_the_line() {
    let mut shell = Shell::new();
    input::clear();
    type_text("shutdown");
    input::push_key(Key::Interrupt);
    assert_eq!(shell.read_line(true), "");
}

#[test_case]
fn long_line_is_read_completely() {
    let mut shell = Shell::new();
    input::clear();
    // Fits both the input queue and the line buffer.
    for _ in 0..MAX_LINE_LENGTH - 8 {
        type_text("a");
    }
    type_text("\n");
    assert_eq!(shell.read_line(true).len(), MAX_LINE_LENGTH - 8);
}
