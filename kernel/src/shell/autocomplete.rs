//! Tab completion for command names.

/// Result of completing a prefix against a list of candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    /// Nothing starts with the prefix.
    NoMatch,
    /// Exactly one candidate matches.
    Unique(&'static str),
    /// Several candidates match. `common_prefix` is the longest text they all
    /// start with (at least as long as what the user typed).
    Ambiguous {
        common_prefix: &'static str,
        matches: usize,
    },
}

/// Completes `prefix` against `candidates`.
pub fn complete(
    prefix: &str,
    candidates: impl Iterator<Item = &'static str>,
) -> Completion {
    let mut matches = 0;
    let mut common: &'static str = "";

    for candidate in candidates.filter(|candidate| candidate.starts_with(prefix)) {
        common = if matches == 0 {
            candidate
        } else {
            common_prefix(common, candidate)
        };
        matches += 1;
    }

    match matches {
        0 => Completion::NoMatch,
        1 => Completion::Unique(common),
        _ => Completion::Ambiguous {
            common_prefix: common,
            matches,
        },
    }
}

/// The longest prefix of `a` that `b` also starts with.
fn common_prefix<'a>(a: &'a str, b: &str) -> &'a str {
    let length = a
        .char_indices()
        .zip(b.chars())
        .find(|((_, left), right)| left != right)
        .map(|((index, _), _)| index)
        .unwrap_or(a.len().min(b.len()));
    &a[..length]
}

#[cfg(test)]
const NAMES: [&str; 5] = ["help", "history", "info", "meminfo", "panic-test"];

#[test_case]
fn unique_match() {
    assert_eq!(complete("me", NAMES.into_iter()), Completion::Unique("meminfo"));
    assert_eq!(complete("info", NAMES.into_iter()), Completion::Unique("info"));
}

#[test_case]
fn no_match() {
    assert_eq!(complete("xyz", NAMES.into_iter()), Completion::NoMatch);
}

#[test_case]
fn ambiguous_match_reports_common_prefix() {
    assert_eq!(
        complete("h", NAMES.into_iter()),
        Completion::Ambiguous {
            common_prefix: "h",
            matches: 2
        }
    );
    let names = ["fault-test", "fault-trace", "help"];
    assert_eq!(
        complete("f", names.into_iter()),
        Completion::Ambiguous {
            common_prefix: "fault-t",
            matches: 2
        }
    );
}

#[test_case]
fn empty_prefix_matches_everything() {
    assert_eq!(
        complete("", NAMES.into_iter()),
        Completion::Ambiguous {
            common_prefix: "",
            matches: NAMES.len()
        }
    );
}
