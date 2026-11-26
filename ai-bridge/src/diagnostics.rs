//! Remembers the diagnostic records the kernel reported, so a later question
//! ("why did that happen?") can be answered with the facts.

use crate::protocol::Diagnostic;
use std::collections::VecDeque;

/// How many records are kept (oldest dropped first).
pub const CAPACITY: usize = 32;

#[derive(Default, Clone)]
pub struct DiagnosticStore {
    records: VecDeque<Diagnostic>,
}

impl DiagnosticStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, record: Diagnostic) {
        self.records.retain(|r| r.id != record.id);
        if self.records.len() == CAPACITY {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    pub fn get(&self, id: &str) -> Option<&Diagnostic> {
        self.records.iter().find(|r| r.id == id)
    }

    /// The newest records, newest first.
    pub fn recent(&self, count: usize) -> impl Iterator<Item = &Diagnostic> {
        self.records.iter().rev().take(count)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.records.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(n: usize) -> Diagnostic {
        Diagnostic {
            id: format!("diag-{n:03}"),
            severity: "error".into(),
            subsystem: "memory".into(),
            code: "MEM-003".into(),
            message: "Heap allocation failed".into(),
            uptime_ms: n as u64,
            context: vec![],
        }
    }

    #[test]
    fn keeps_the_newest_records() {
        let mut store = DiagnosticStore::new();
        for n in 1..=CAPACITY + 5 {
            store.add(record(n));
        }
        assert_eq!(store.len(), CAPACITY);
        assert!(store.get("diag-001").is_none());
        assert!(store.get(&format!("diag-{:03}", CAPACITY + 5)).is_some());
        assert_eq!(
            store.recent(1).next().unwrap().uptime_ms,
            (CAPACITY + 5) as u64
        );
    }

    #[test]
    fn a_resent_record_replaces_the_old_copy() {
        let mut store = DiagnosticStore::new();
        store.add(record(1));
        store.add(record(1));
        assert_eq!(store.len(), 1);
    }
}
