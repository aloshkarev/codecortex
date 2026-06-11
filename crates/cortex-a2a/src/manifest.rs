//! Role manifest registry: config.toml (Tier 1) + agent markdown (Tier 2).

use cortex_core::A2aConfig;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RoleManifest {
    pub role_name: String,
    pub subscriptions: Vec<String>,
    pub capabilities: Vec<String>,
    pub skills: Vec<String>,
    pub mcp_tools: Vec<String>,
    pub source: ManifestSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestSource {
    Config,
    Markdown,
    Merged,
}

#[derive(Debug, Clone, Default)]
pub struct RoleManifestRegistry {
    roles: HashMap<String, RoleManifest>,
    warnings: Vec<String>,
}

impl RoleManifestRegistry {
    pub fn load(config: &A2aConfig, repo_root: Option<&Path>) -> Self {
        let mut registry = Self::default();
        for (name, role_cfg) in &config.roles {
            registry.roles.insert(
                name.clone(),
                RoleManifest {
                    role_name: name.clone(),
                    subscriptions: role_cfg.subscriptions.clone(),
                    capabilities: role_cfg.capabilities.clone(),
                    skills: role_cfg.skills.clone(),
                    mcp_tools: role_cfg.mcp_tools.clone(),
                    source: ManifestSource::Config,
                },
            );
        }

        let paths: Vec<PathBuf> = if config.agent_manifest_paths.is_empty() {
            repo_root
                .map(A2aConfig::default_agent_manifest_paths)
                .unwrap_or_default()
        } else {
            config.agent_manifest_paths.clone()
        };

        for dir in paths {
            if let Err(e) = registry.ingest_markdown_dir(&dir) {
                registry
                    .warnings
                    .push(format!("manifest scan {}: {e}", dir.display()));
            }
        }

        registry
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn get(&self, role: &str) -> Option<&RoleManifest> {
        self.roles.get(role)
    }

    pub fn accepts_payload(&self, role: &str, payload_type: &str) -> bool {
        let Some(m) = self.roles.get(role) else {
            return true;
        };
        if m.subscriptions.is_empty() {
            return true;
        }
        m.subscriptions.iter().any(|s| s == payload_type)
            || m.capabilities.iter().any(|s| s == payload_type)
    }

    fn ingest_markdown_dir(&mut self, dir: &Path) -> std::io::Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if let Some(parsed) = parse_agent_markdown(&path) {
                self.merge_markdown(parsed);
            }
        }
        Ok(())
    }

    fn merge_markdown(&mut self, parsed: ParsedAgentMd) {
        let role_key = parsed.role_key;
        if let Some(existing) = self.roles.get_mut(&role_key) {
            if existing.subscriptions.is_empty() {
                existing.subscriptions = parsed.subscriptions;
            }
            if existing.capabilities.is_empty() {
                existing.capabilities = parsed.capabilities;
            }
            if existing.skills.is_empty() {
                existing.skills = parsed.skills;
            }
            if existing.mcp_tools.is_empty() {
                existing.mcp_tools = parsed.mcp_tools;
            }
            existing.source = ManifestSource::Merged;
        } else {
            self.roles.insert(
                role_key.clone(),
                RoleManifest {
                    role_name: role_key,
                    subscriptions: parsed.subscriptions,
                    capabilities: parsed.capabilities,
                    skills: parsed.skills,
                    mcp_tools: parsed.mcp_tools,
                    source: ManifestSource::Markdown,
                },
            );
        }
    }
}

struct ParsedAgentMd {
    role_key: String,
    subscriptions: Vec<String>,
    capabilities: Vec<String>,
    skills: Vec<String>,
    mcp_tools: Vec<String>,
}

fn parse_agent_markdown(path: &Path) -> Option<ParsedAgentMd> {
    let text = fs::read_to_string(path).ok()?;
    let stem = path.file_stem()?.to_str()?;
    let role_key = stem
        .strip_prefix("codecortex-")
        .unwrap_or(stem)
        .replace('-', "_");

    let subscriptions = extract_bullet_section(&text, "A2A subscriptions");
    let capabilities = extract_bullet_section(&text, "A2A capabilities");
    let skills = extract_backtick_tools(&text);
    let mcp_tools = extract_bullet_section(&text, "MCP tools");

    if subscriptions.is_empty()
        && capabilities.is_empty()
        && skills.is_empty()
        && mcp_tools.is_empty()
    {
        return None;
    }

    Some(ParsedAgentMd {
        role_key,
        subscriptions: subscriptions
            .into_iter()
            .filter_map(|s| extract_payload_type(&s))
            .collect(),
        capabilities: capabilities
            .into_iter()
            .filter_map(|s| extract_payload_type(&s))
            .collect(),
        skills,
        mcp_tools,
    })
}

fn extract_bullet_section(text: &str, heading: &str) -> Vec<String> {
    let needle = format!("## {heading}");
    let start = match text.find(&needle) {
        Some(i) => i + needle.len(),
        None => return Vec::new(),
    };
    let rest = &text[start..];
    let end = rest.find("\n## ").map(|i| i).unwrap_or(rest.len());
    rest[..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("- ").or_else(|| line.strip_prefix("* "))
        })
        .map(|s| s.to_string())
        .collect()
}

fn extract_payload_type(line: &str) -> Option<String> {
    if let Some(start) = line.find('`') {
        let rest = &line[start + 1..];
        if let Some(end) = rest.find('`') {
            let token = &rest[..end];
            if token.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                return Some(token.to_string());
            }
        }
    }
    for word in line.split_whitespace() {
        if word.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && !word.contains('.')
            && word.len() > 3
        {
            return Some(
                word.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string(),
            );
        }
    }
    None
}

fn extract_backtick_tools(text: &str) -> Vec<String> {
    let mut skills = Vec::new();
    for line in text.lines() {
        for part in line.split('`') {
            if part.contains('_')
                && part.chars().all(|c| c.is_ascii_lowercase() || c == '_')
                && part.len() > 4
            {
                if !skills.contains(&part.to_string()) {
                    skills.push(part.to_string());
                }
            }
        }
    }
    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_payload_from_subscription_line() {
        let line = "- `TaskDelegation` focused on analyze";
        assert_eq!(
            extract_payload_type(line).as_deref(),
            Some("TaskDelegation")
        );
    }
}
