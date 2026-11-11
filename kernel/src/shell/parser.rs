//! Command-line parser: turns a raw input line into a command name and
//! arguments, without allocating.
//!
//! ```text
//! "echo hello world"      -> name "echo",  args ["hello", "world"]
//! "echo \"hello world\""  -> name "echo",  args ["hello world"]
//! "   "                   -> nothing to run
//! ```

use core::fmt;

/// Longest accepted input line, in bytes.
pub const MAX_LINE_LENGTH: usize = 128;

/// Most arguments a command line may have (not counting the command name).
pub const MAX_ARGS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// More than [`MAX_ARGS`] arguments.
    TooManyArguments,
    /// A `"` was opened but never closed.
    UnterminatedQuote,
    /// The command name contains a character no command can have.
    InvalidCommandName,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::TooManyArguments => {
                write!(f, "too many arguments (at most {} allowed)", MAX_ARGS)
            }
            ParseError::UnterminatedQuote => f.write_str("unterminated quote"),
            ParseError::InvalidCommandName => {
                f.write_str("command names may only contain letters, digits and '-'")
            }
        }
    }
}

/// A parsed command line. All strings borrow from the input line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedCommand<'a> {
    /// The first word of the line.
    pub name: &'a str,
    /// Everything after the command name, trimmed but otherwise untouched.
    /// Commands that take free text (`echo`, `ai`) use this.
    pub rest: &'a str,
    args: [&'a str; MAX_ARGS],
    arg_count: usize,
}

impl<'a> ParsedCommand<'a> {
    pub fn args(&self) -> &[&'a str] {
        &self.args[..self.arg_count]
    }

    pub fn arg(&self, index: usize) -> Option<&'a str> {
        self.args().get(index).copied()
    }
}

/// Parses one input line. Returns `Ok(None)` for a blank line.
pub fn parse(line: &str) -> Result<Option<ParsedCommand<'_>>, ParseError> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }

    let (name, rest) = match line.find(char::is_whitespace) {
        Some(end) => (&line[..end], line[end..].trim_start()),
        None => (line, ""),
    };
    if !name.chars().all(is_command_name_char) {
        return Err(ParseError::InvalidCommandName);
    }

    let mut args = [""; MAX_ARGS];
    let mut arg_count = 0;
    let mut remaining = rest;
    while let Some((token, after)) = next_token(remaining)? {
        if arg_count == MAX_ARGS {
            return Err(ParseError::TooManyArguments);
        }
        args[arg_count] = token;
        arg_count += 1;
        remaining = after;
    }

    Ok(Some(ParsedCommand {
        name,
        rest,
        args,
        arg_count,
    }))
}

fn is_command_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '?'
}

/// Splits the next argument off `input`. Returns the argument and the text
/// after it, or `None` when only whitespace is left.
fn next_token(input: &str) -> Result<Option<(&str, &str)>, ParseError> {
    let input = input.trim_start();
    if input.is_empty() {
        return Ok(None);
    }

    if let Some(quoted) = input.strip_prefix('"') {
        // A quoted argument runs to the closing quote and may contain spaces.
        let end = quoted.find('"').ok_or(ParseError::UnterminatedQuote)?;
        return Ok(Some((&quoted[..end], &quoted[end + 1..])));
    }

    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    Ok(Some((&input[..end], &input[end..])))
}

#[test_case]
fn blank_lines_parse_to_nothing() {
    assert_eq!(parse(""), Ok(None));
    assert_eq!(parse("   \t "), Ok(None));
}

#[test_case]
fn command_without_arguments() {
    let command = parse("meminfo").unwrap().unwrap();
    assert_eq!(command.name, "meminfo");
    assert_eq!(command.args(), &[] as &[&str]);
    assert_eq!(command.rest, "");
}

#[test_case]
fn command_with_arguments() {
    let command = parse("  echo   hello   world  ").unwrap().unwrap();
    assert_eq!(command.name, "echo");
    assert_eq!(command.args(), &["hello", "world"]);
    assert_eq!(command.arg(1), Some("world"));
    assert_eq!(command.arg(2), None);
    assert_eq!(command.rest, "hello   world");
}

#[test_case]
fn quoted_argument_keeps_spaces() {
    let command = parse("echo \"hello world\" again").unwrap().unwrap();
    assert_eq!(command.args(), &["hello world", "again"]);
}

#[test_case]
fn unterminated_quote_is_rejected() {
    assert_eq!(parse("echo \"oops"), Err(ParseError::UnterminatedQuote));
}

#[test_case]
fn too_many_arguments_are_rejected() {
    assert!(parse("echo 1 2 3 4 5 6 7 8").is_ok());
    assert_eq!(
        parse("echo 1 2 3 4 5 6 7 8 9"),
        Err(ParseError::TooManyArguments)
    );
}

#[test_case]
fn invalid_command_name_is_rejected() {
    assert_eq!(parse("rm;reboot"), Err(ParseError::InvalidCommandName));
    assert_eq!(parse("../x"), Err(ParseError::InvalidCommandName));
    assert!(parse("panic-test").is_ok());
}
