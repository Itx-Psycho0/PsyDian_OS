//! Request log (`ai-bridge.log`): one line per AI request with its id,
//! provider, prompt version, latency and outcome. API keys, prompts and
//! answers are never written (Docs/11-security-design.md SEC-011).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct Telemetry {
    file: Mutex<Option<File>>,
    path: Option<PathBuf>,
}

impl Telemetry {
    pub fn open(path: &Path) -> Self {
        let file = OpenOptions::new().create(true).append(true).open(path).ok();
        let path = file.as_ref().map(|_| path.to_path_buf());
        Self {
            file: Mutex::new(file),
            path,
        }
    }

    pub fn disabled() -> Self {
        Self {
            file: Mutex::new(None),
            path: None,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn event(&self, text: &str) {
        let line = format!("{} {}\n", timestamp(), one_line(text));
        if let Ok(mut file) = self.file.lock()
            && let Some(file) = file.as_mut()
        {
            let _ = file.write_all(line.as_bytes());
        }
    }

    pub fn request(&self, id: &str, provider: &str, latency: Duration, outcome: &str) {
        self.event(&format!(
            "request={id} provider=\"{provider}\" prompt={} latency_ms={} outcome={outcome}",
            crate::prompt::PROMPT_VERSION,
            latency.as_millis()
        ));
    }
}

fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("t={secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_single_lines() {
        let dir = std::env::temp_dir().join(format!("psydian-telemetry-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bridge.log");
        let t = Telemetry::open(&path);
        t.request("req-001", "mock", Duration::from_millis(12), "answered");
        t.event("two\nlines");
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.contains("request=req-001") && text.contains("latency_ms=12"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
