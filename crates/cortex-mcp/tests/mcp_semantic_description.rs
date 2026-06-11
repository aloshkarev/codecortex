//! L1: oracle description_claim / keywords align with exported tool guidance and handler descriptions.

use cortex_mcp::{tool_cards, tool_guidance_for, tool_names};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct OracleDoc {
    oracles: Vec<OracleEntry>,
}

#[derive(Debug, Deserialize)]
struct OracleEntry {
    tool: String,
    description_claim: String,
    #[serde(default)]
    profile: Vec<String>,
    #[serde(default)]
    description_keywords: Vec<String>,
}

fn oracles_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/mcp_semantic/oracles.json")
}

/// Parse `#[tool(description = "...")]` blocks associated with `async fn <name>`.
fn handler_tool_descriptions() -> HashMap<String, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/handler.rs");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut pending_desc: Option<String> = None;
    let mut out = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find("description = \"") {
            let rest = &trimmed[idx + "description = \"".len()..];
            let desc = rest.split('"').next().unwrap_or("").to_string();
            if !desc.is_empty() {
                pending_desc = Some(desc);
            }
            continue;
        }
        let trimmed = line.trim();
        if let Some(fn_line) = trimmed.strip_prefix("async fn ") {
            if let Some(name) = fn_line.split('(').next() {
                if let Some(desc) = pending_desc.take() {
                    out.insert(name.to_string(), desc);
                }
            }
        }
    }
    out
}

fn tool_description_text(name: &str, handler: &HashMap<String, String>) -> String {
    let mut parts = Vec::new();
    if let Some(h) = handler.get(name) {
        parts.push(h.clone());
    }
    if let Some(card) = tool_cards().iter().find(|c| c.metadata.name == name) {
        parts.push(card.guidance.summary.clone());
        parts.extend(card.guidance.use_cases.iter().cloned());
    }
    let g = tool_guidance_for(name);
    parts.push(g.summary);
    parts.extend(g.use_cases);
    parts.join(" ").to_lowercase()
}

#[test]
fn tool_description_matches_oracle_claims() {
    let path = oracles_path();
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let doc: OracleDoc = serde_json::from_str(&text).expect("parse oracles.json");
    let handler = handler_tool_descriptions();

    let exported: HashMap<&str, ()> = tool_names().iter().map(|t| (*t, ())).collect();
    let mut missing_tools = Vec::new();
    let mut drift = Vec::new();

    for entry in &doc.oracles {
        if !exported.contains_key(entry.tool.as_str()) {
            missing_tools.push(entry.tool.clone());
            continue;
        }
        let desc = tool_description_text(&entry.tool, &handler);
        if desc.is_empty() {
            drift.push(format!("{}: no description text", entry.tool));
            continue;
        }
        let pr_oracle = entry.profile.iter().any(|p| p == "pr");
        if pr_oracle {
            let claim = entry.description_claim.to_lowercase();
            let claim_words: Vec<&str> = claim.split_whitespace().filter(|w| w.len() > 4).collect();
            let overlap = claim_words.iter().any(|w| desc.contains(w));
            if !overlap {
                drift.push(format!(
                    "{}: claim {:?} weakly overlaps description",
                    entry.tool, entry.description_claim
                ));
            }
        }
        if pr_oracle {
            for kw in &entry.description_keywords {
                let k = kw.to_lowercase();
                if !desc.contains(&k) {
                    drift.push(format!(
                        "{}: keyword {:?} not in tool description",
                        entry.tool, kw
                    ));
                }
            }
        }
    }

    assert!(
        missing_tools.is_empty(),
        "oracles for unknown tools: {missing_tools:?}"
    );
    assert!(
        drift.is_empty(),
        "DESCRIPTION_DRIFT (fix handler or oracle):\n{}",
        drift.join("\n")
    );
}

#[test]
fn oracles_cover_all_exported_tools() {
    let path = oracles_path();
    let text = std::fs::read_to_string(&path).expect("read oracles");
    let doc: OracleDoc = serde_json::from_str(&text).expect("parse");
    let covered: HashMap<&str, ()> = doc.oracles.iter().map(|o| (o.tool.as_str(), ())).collect();
    let missing: Vec<&str> = tool_names()
        .iter()
        .copied()
        .filter(|t| !covered.contains_key(t))
        .collect();
    assert!(missing.is_empty(), "missing oracles for tools: {missing:?}");
}
