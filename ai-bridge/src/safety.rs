//! Checks the AI's command suggestions before they reach the kernel.
//!
//! This is a second line of defence: the kernel checks every suggestion
//! again (`run <n>`), and the AI never gets execution authority. The table
//! mirrors the kernel's command registry and risk levels
//! (`kernel/src/shell/commands.rs`, Docs/11-security-design.md section 5.2).

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    Safe,
    Sensitive,
    Privileged,
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
}

pub struct Command {
    pub name: &'static str,
    pub usage: &'static str,
    pub summary: &'static str,
    pub risk: Risk,
}

pub const COMMANDS: &[Command] = &[
    Command {
        name: "help",
        usage: "help [command]",
        summary: "list commands or show how to use one",
        risk: Risk::Safe,
    },
    Command {
        name: "clear",
        usage: "clear",
        summary: "clear the screen",
        risk: Risk::Safe,
    },
    Command {
        name: "info",
        usage: "info",
        summary: "kernel and system information",
        risk: Risk::Safe,
    },
    Command {
        name: "meminfo",
        usage: "meminfo",
        summary: "physical memory, frame and heap usage",
        risk: Risk::Safe,
    },
    Command {
        name: "uptime",
        usage: "uptime",
        summary: "time since boot",
        risk: Risk::Safe,
    },
    Command {
        name: "echo",
        usage: "echo [text...]",
        summary: "print text",
        risk: Risk::Safe,
    },
    Command {
        name: "history",
        usage: "history",
        summary: "previously entered commands",
        risk: Risk::Safe,
    },
    Command {
        name: "logs",
        usage: "logs [count] [info|warn|error|panic]",
        summary: "recent kernel log records",
        risk: Risk::Safe,
    },
    Command {
        name: "diag",
        usage: "diag [list | last | <number>]",
        summary: "show diagnostic records",
        risk: Risk::Safe,
    },
    Command {
        name: "ai",
        usage: "ai <question...>",
        summary: "ask the assistant again",
        risk: Risk::Sensitive,
    },
    Command {
        name: "shutdown",
        usage: "shutdown",
        summary: "stop the system",
        risk: Risk::Privileged,
    },
    Command {
        name: "panic-test",
        usage: "panic-test",
        summary: "deliberately crash the kernel",
        risk: Risk::Destructive,
    },
    Command {
        name: "fault-test",
        usage: "fault-test <scenario>",
        summary: "inject a real CPU or memory fault",
        risk: Risk::Destructive,
    },
];

pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

/// The suggestions that may be forwarded, and whether any of them needs the
/// user's confirmation.
#[derive(Debug, Default, PartialEq)]
pub struct Filtered {
    pub commands: Vec<String>,
    pub requires_confirmation: bool,
    /// Suggestions that were dropped, with the reason (for the log).
    pub dropped: Vec<(String, &'static str)>,
}

/// Keeps a suggestion only if it is a single Psydian command line: printable
/// ASCII, no shell syntax, a known command name, and not destructive.
pub fn filter_commands(suggested: &[String]) -> Filtered {
    let mut result = Filtered::default();
    for raw in suggested {
        let line = raw.trim();
        let reason = if line.is_empty() {
            Some("empty")
        } else if !line.bytes().all(|b| (0x20..0x7f).contains(&b)) {
            Some("not printable ASCII")
        } else if line.contains([';', '|', '&', '`', '$', '>', '<']) {
            Some("contains shell syntax")
        } else {
            let name = line.split_whitespace().next().unwrap_or("");
            match find(name) {
                None => Some("not a Psydian command"),
                Some(c) if c.risk == Risk::Destructive => Some("destructive command"),
                Some(c) => {
                    if c.risk > Risk::Safe {
                        result.requires_confirmation = true;
                    }
                    None
                }
            }
        };
        match reason {
            Some(why) => result.dropped.push((line.to_string(), why)),
            None if result.commands.iter().any(|c| c == line) => {}
            None => result.commands.push(line.to_string()),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn keeps_known_safe_commands() {
        let f = filter_commands(&strings(&["meminfo", " diag last ", "logs 5 error"]));
        assert_eq!(f.commands, ["meminfo", "diag last", "logs 5 error"]);
        assert!(!f.requires_confirmation);
        assert!(f.dropped.is_empty());
    }

    #[test]
    fn drops_dangerous_or_foreign_commands() {
        let f = filter_commands(&strings(&[
            "rm -rf /",
            "panic-test",
            "fault-test heap",
            "help; shutdown",
            "echo $(reboot)",
            "",
            "MEMINFO",
        ]));
        assert!(f.commands.is_empty());
        assert_eq!(f.dropped.len(), 7);
    }

    #[test]
    fn privileged_and_sensitive_commands_need_confirmation() {
        let f = filter_commands(&strings(&["shutdown"]));
        assert_eq!(f.commands, ["shutdown"]);
        assert!(f.requires_confirmation);
        assert!(filter_commands(&strings(&["ai tell me more"])).requires_confirmation);
    }

    #[test]
    fn duplicates_are_removed() {
        assert_eq!(
            filter_commands(&strings(&["meminfo", "meminfo"])).commands,
            ["meminfo"]
        );
    }

    #[test]
    fn table_matches_the_kernel_registry() {
        let names: Vec<_> = COMMANDS.iter().map(|c| c.name).collect();
        for name in [
            "help",
            "meminfo",
            "diag",
            "ai",
            "shutdown",
            "panic-test",
            "fault-test",
        ] {
            assert!(names.contains(&name), "{name}");
        }
        assert_eq!(find("shutdown").unwrap().risk, Risk::Privileged);
        assert_eq!(find("ai").unwrap().risk, Risk::Sensitive);
    }
}
