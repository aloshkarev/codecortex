//! Shared helpers for intelligence outputs.

use std::path::PathBuf;

pub fn symbol_from_path(target_path: &str) -> String {
    PathBuf::from(target_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(target_path)
        .to_string()
}

pub fn detect_intent(query: &str) -> &'static str {
    let q = query.to_lowercase();
    if q.contains("debug") || q.contains("error") {
        "debug"
    } else if q.contains("refactor") {
        "refactor"
    } else if q.contains("test") {
        "test"
    } else if q.contains("review") {
        "review"
    } else {
        "explore"
    }
}

pub fn redact_secrets(input: &str) -> String {
    input
        .lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            if lower.contains("api_key")
                || lower.contains("apikey")
                || lower.contains("secret")
                || lower.contains("password")
                || lower.contains("token")
                || lower.contains("authorization:")
                || lower.contains("private_key")
            {
                "[REDACTED_SECRET_LINE]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
