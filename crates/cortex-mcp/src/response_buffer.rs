//! Ring buffer for large MCP tool responses with re-cutting helpers.

use serde::Serialize;
use std::collections::VecDeque;

pub const DEFAULT_CAPACITY: usize = 8;
pub const MIN_CAPTURE_BYTES: usize = 1024;

#[derive(Debug, Clone, Serialize)]
pub struct BufferEntrySummary {
    pub id: String,
    pub tool: String,
    pub byte_len: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BufferStats {
    pub capacity: usize,
    pub count: usize,
    pub total_bytes: usize,
    pub min_capture_bytes: usize,
    pub entries: Vec<BufferEntrySummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntryDetail {
    pub id: String,
    pub tool: String,
    pub byte_len: usize,
    pub line_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrepMatch {
    pub line_number: usize,
    pub line: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<String>,
}

#[derive(Debug, Clone)]
struct BufferedResponse {
    id: String,
    tool: String,
    text: String,
}

/// Fixed-capacity ring buffer of recent large tool responses.
#[derive(Debug)]
pub struct ResponseBuffer {
    capacity: usize,
    min_capture_bytes: usize,
    entries: VecDeque<BufferedResponse>,
    next_seq: usize,
}

impl Default for ResponseBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseBuffer {
    pub fn new() -> Self {
        Self {
            capacity: DEFAULT_CAPACITY,
            min_capture_bytes: MIN_CAPTURE_BYTES,
            entries: VecDeque::new(),
            next_seq: 0,
        }
    }

    #[cfg(test)]
    pub fn with_capacity(capacity: usize, min_capture_bytes: usize) -> Self {
        Self {
            capacity,
            min_capture_bytes,
            entries: VecDeque::new(),
            next_seq: 0,
        }
    }

    /// Store `text` when it meets the minimum size threshold. Returns the assigned handle.
    pub fn capture(&mut self, tool: &str, text: &str) -> Option<String> {
        if text.len() < self.min_capture_bytes {
            return None;
        }
        while self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        let id = format!("resp_{}", self.next_seq);
        self.next_seq += 1;
        self.entries.push_back(BufferedResponse {
            id: id.clone(),
            tool: tool.to_string(),
            text: text.to_string(),
        });
        Some(id)
    }

    pub fn latest_id(&self) -> Option<String> {
        self.entries.back().map(|e| e.id.clone())
    }

    fn resolve_id<'a>(&'a self, response_id: Option<&str>) -> Result<&'a BufferedResponse, String> {
        match response_id {
            Some(id) => self
                .get(id)
                .ok_or_else(|| format!("unknown response_id: {id}")),
            None => self
                .entries
                .back()
                .ok_or_else(|| "no buffered responses".to_string()),
        }
    }

    pub fn stats(&self) -> BufferStats {
        let entries = self
            .entries
            .iter()
            .map(|e| BufferEntrySummary {
                id: e.id.clone(),
                tool: e.tool.clone(),
                byte_len: e.text.len(),
            })
            .collect();
        let total_bytes = self.entries.iter().map(|e| e.text.len()).sum();
        BufferStats {
            capacity: self.capacity,
            count: self.entries.len(),
            total_bytes,
            min_capture_bytes: self.min_capture_bytes,
            entries,
        }
    }

    pub fn entry_detail(&self, response_id: Option<&str>) -> Result<EntryDetail, String> {
        let entry = self.resolve_id(response_id)?;
        Ok(EntryDetail {
            id: entry.id.clone(),
            tool: entry.tool.clone(),
            byte_len: entry.text.len(),
            line_count: entry.text.lines().count(),
        })
    }

    pub fn grep(
        &self,
        response_id: Option<&str>,
        pattern: &str,
        before: usize,
        after: usize,
    ) -> Result<Vec<GrepMatch>, String> {
        if pattern.is_empty() {
            return Err("pattern must not be empty".to_string());
        }
        let entry = self.resolve_id(response_id)?;
        let lines: Vec<&str> = entry.text.lines().collect();
        let mut matches = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            if !line.contains(pattern) {
                continue;
            }
            let start = idx.saturating_sub(before);
            let end = (idx + after + 1).min(lines.len());
            matches.push(GrepMatch {
                line_number: idx + 1,
                line: (*line).to_string(),
                before: lines[start..idx].iter().map(|s| (*s).to_string()).collect(),
                after: lines[idx + 1..end].iter().map(|s| (*s).to_string()).collect(),
            });
        }
        Ok(matches)
    }

    pub fn slice(
        &self,
        response_id: Option<&str>,
        from: usize,
        to: usize,
    ) -> Result<String, String> {
        let entry = self.resolve_id(response_id)?;
        let chars: Vec<char> = entry.text.chars().collect();
        if from > to {
            return Err(format!("from ({from}) must be <= to ({to})"));
        }
        if to > chars.len() {
            return Err(format!(
                "to ({to}) exceeds text length ({} chars)",
                chars.len()
            ));
        }
        Ok(chars[from..to].iter().collect())
    }

    pub fn peek(&self, response_id: Option<&str>, lines: usize) -> Result<String, String> {
        if lines == 0 {
            return Err("lines must be > 0".to_string());
        }
        let entry = self.resolve_id(response_id)?;
        Ok(entry
            .text
            .lines()
            .take(lines)
            .collect::<Vec<_>>()
            .join("\n"))
    }

    fn get(&self, id: &str) -> Option<&BufferedResponse> {
        self.entries.iter().find(|e| e.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_ignores_small_responses() {
        let mut buf = ResponseBuffer::with_capacity(4, 1024);
        assert!(buf.capture("find_code", &"x".repeat(512)).is_none());
        assert_eq!(buf.stats().count, 0);
    }

    #[test]
    fn capture_assigns_sequential_handles() {
        let mut buf = ResponseBuffer::with_capacity(4, 16);
        let a = buf.capture("find_code", &"a".repeat(32)).unwrap();
        let b = buf.capture("get_patch_context", &"b".repeat(32)).unwrap();
        assert_eq!(a, "resp_0");
        assert_eq!(b, "resp_1");
        assert_eq!(buf.stats().count, 2);
    }

    #[test]
    fn ring_evicts_oldest_entry() {
        let mut buf = ResponseBuffer::with_capacity(2, 8);
        let first = buf.capture("t1", &"1".repeat(16)).unwrap();
        let _second = buf.capture("t2", &"2".repeat(16)).unwrap();
        let third = buf.capture("t3", &"3".repeat(16)).unwrap();
        assert_eq!(buf.stats().count, 2);
        assert!(buf.get(&first).is_none());
        assert_eq!(third, "resp_2");
        assert_eq!(buf.latest_id().as_deref(), Some("resp_2"));
    }

    #[test]
    fn slice_returns_char_range() {
        let mut buf = ResponseBuffer::with_capacity(4, 8);
        let id = buf.capture("tool", "abcdefgh").unwrap();
        assert_eq!(buf.slice(Some(&id), 2, 5).unwrap(), "cde");
    }

    #[test]
    fn grep_returns_context_lines() {
        let mut buf = ResponseBuffer::with_capacity(4, 8);
        let text = "alpha\nbeta target line\ngamma\ndelta target again\n";
        let id = buf.capture("tool", text).unwrap();
        let hits = buf.grep(Some(&id), "target", 1, 1).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].line_number, 2);
        assert_eq!(hits[0].before, vec!["alpha"]);
        assert_eq!(hits[0].after, vec!["gamma"]);
        assert_eq!(hits[1].line_number, 4);
    }

    #[test]
    fn peek_limits_lines() {
        let mut buf = ResponseBuffer::with_capacity(4, 8);
        let id = buf.capture("tool", "one\ntwo\nthree\nfour\n").unwrap();
        assert_eq!(buf.peek(Some(&id), 2).unwrap(), "one\ntwo");
    }
}
