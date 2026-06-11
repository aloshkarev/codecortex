//! Agent pack resolution and installation for Cursor / Claude Code workspaces.
//!
//! Copies or symlinks skills, subagents, hooks, rules, and MCP JSON from a packaged
//! layout (`plugin/codecortex`) into a target repository.

use serde::Serialize;
use serde_json::json;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const BACKUP_SUFFIX: &str = ".bak.codecortex";
const PACK_RELATIVE: &str = "plugin/codecortex";
const SHARE_RELATIVE: &str = "share/codecortex-agent-pack";

/// Result of installing the agent pack into a repository.
#[derive(Debug, Clone, Serialize)]
pub struct AgentPackInstallResult {
    pub agent_pack_root: String,
    pub installed: Vec<String>,
    pub skipped: Vec<String>,
    pub backed_up: Vec<String>,
    pub warnings: Vec<String>,
}

/// Options controlling agent pack installation.
#[derive(Debug, Clone)]
pub struct AgentPackInstallOptions {
    pub repo_path: PathBuf,
    pub pack_root: PathBuf,
    pub overwrite: bool,
    pub non_interactive: bool,
    pub install_skills: bool,
    pub install_agents: bool,
    pub install_hooks: bool,
    pub install_rules: bool,
    pub install_mcp: bool,
    pub install_cursor_mcp: bool,
    pub mcp_command: String,
}

impl AgentPackInstallOptions {
    pub fn for_repo(repo_path: impl AsRef<Path>, pack_root: impl AsRef<Path>) -> Self {
        Self {
            repo_path: repo_path.as_ref().to_path_buf(),
            pack_root: pack_root.as_ref().to_path_buf(),
            overwrite: false,
            non_interactive: false,
            install_skills: true,
            install_agents: true,
            install_hooks: true,
            install_rules: true,
            install_mcp: true,
            install_cursor_mcp: true,
            mcp_command: default_mcp_command(),
        }
    }
}

/// Resolve the agent pack root directory.
pub fn resolve_agent_pack(repo_path: &Path, override_root: Option<&Path>) -> Option<PathBuf> {
    if let Some(root) = override_root {
        let p = root.to_path_buf();
        if pack_layout_valid(&p) {
            return Some(p);
        }
        return None;
    }

    if let Ok(env) = std::env::var("CORTEX_AGENT_PACK") {
        let p = PathBuf::from(env);
        if pack_layout_valid(&p) {
            return Some(p);
        }
    }

    if let Some(found) = find_pack_walking_up(repo_path) {
        return Some(found);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let share = parent.join(SHARE_RELATIVE);
            if pack_layout_valid(&share) {
                return Some(share);
            }
        }
    }

    None
}

fn find_pack_walking_up(start: &Path) -> Option<PathBuf> {
    let mut cur = start.to_path_buf();
    loop {
        let candidate = cur.join(PACK_RELATIVE);
        if pack_layout_valid(&candidate) {
            return Some(candidate);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

fn pack_layout_valid(root: &Path) -> bool {
    root.join("skills").is_dir() && root.join("cursor").is_dir()
}

fn default_mcp_command() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "cortex".to_string())
}

fn find_git_root(path: &Path) -> Option<PathBuf> {
    let start = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut cur = start;
    loop {
        if cur.join(".git").exists() {
            return Some(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

fn same_git_repo(a: &Path, b: &Path) -> bool {
    match (find_git_root(a), find_git_root(b)) {
        (Some(ga), Some(gb)) => ga == gb,
        _ => false,
    }
}

fn should_replace(existing: &Path, overwrite: bool, non_interactive: bool) -> bool {
    if !existing.exists() {
        return true;
    }
    overwrite && non_interactive
}

fn backup_file(path: &Path, backed_up: &mut Vec<String>) -> io::Result<()> {
    if path.exists() {
        let backup = PathBuf::from(format!("{}{}", path.display(), BACKUP_SUFFIX));
        fs::copy(path, &backup)?;
        backed_up.push(backup.display().to_string());
    }
    Ok(())
}

fn install_entry(
    src: &Path,
    dest: &Path,
    use_symlink: bool,
    overwrite: bool,
    non_interactive: bool,
    installed: &mut Vec<String>,
    skipped: &mut Vec<String>,
    backed_up: &mut Vec<String>,
    _warnings: &mut Vec<String>,
) -> io::Result<()> {
    if dest.exists() {
        if should_replace(dest, overwrite, non_interactive) {
            if dest.is_dir() {
                fs::remove_dir_all(dest)?;
            } else {
                backup_file(dest, backed_up)?;
                fs::remove_file(dest)?;
            }
        } else {
            skipped.push(dest.display().to_string());
            return Ok(());
        }
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    if use_symlink {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(src, dest)?;
        }
        #[cfg(not(unix))]
        {
            if src.is_dir() {
                copy_dir_all(src, dest)?;
            } else {
                fs::copy(src, dest)?;
            }
        }
    } else if src.is_dir() {
        copy_dir_all(src, dest)?;
    } else {
        fs::copy(src, dest)?;
    }

    installed.push(dest.display().to_string());
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn install_skills_dir(
    opts: &AgentPackInstallOptions,
    use_symlink: bool,
    result: &mut AgentPackInstallResult,
) -> io::Result<()> {
    let skills_src = opts.pack_root.join("skills");
    if !skills_src.is_dir() {
        result
            .warnings
            .push(format!("missing skills dir: {}", skills_src.display()));
        return Ok(());
    }
    let skills_dest = opts.repo_path.join(".cursor/skills");
    fs::create_dir_all(&skills_dest)?;

    for entry in fs::read_dir(&skills_src)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let src = entry.path();
        let dest = skills_dest.join(name);
        if let Err(e) = install_entry(
            &src,
            &dest,
            use_symlink,
            opts.overwrite,
            opts.non_interactive,
            &mut result.installed,
            &mut result.skipped,
            &mut result.backed_up,
            &mut result.warnings,
        ) {
            result
                .warnings
                .push(format!("skill {}: {e}", src.display()));
        }
    }
    Ok(())
}

fn install_agents_dir(
    opts: &AgentPackInstallOptions,
    use_symlink: bool,
    result: &mut AgentPackInstallResult,
) -> io::Result<()> {
    let agents_src = opts.pack_root.join("agents");
    if !agents_src.is_dir() {
        result
            .warnings
            .push(format!("missing agents dir: {}", agents_src.display()));
        return Ok(());
    }
    let agents_dest = opts.repo_path.join(".cursor/agents");
    fs::create_dir_all(&agents_dest)?;

    for entry in fs::read_dir(&agents_src)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let dest = agents_dest.join(entry.file_name());
        if dest.exists() && !should_replace(&dest, opts.overwrite, opts.non_interactive) {
            result.skipped.push(dest.display().to_string());
            continue;
        }
        if dest.exists() {
            backup_file(&dest, &mut result.backed_up)?;
            fs::remove_file(&dest)?;
        }
        if use_symlink {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&path, &dest)?;
            }
            #[cfg(not(unix))]
            {
                fs::copy(&path, &dest)?;
            }
        } else {
            fs::copy(&path, &dest)?;
        }
        result.installed.push(dest.display().to_string());
    }
    Ok(())
}

fn install_hooks(
    opts: &AgentPackInstallOptions,
    result: &mut AgentPackInstallResult,
) -> io::Result<()> {
    let cursor_pack = opts.pack_root.join("cursor");
    let hooks_src = cursor_pack.join("hooks");
    let hooks_json_src = cursor_pack.join("hooks.json");
    if !hooks_src.is_dir() {
        result
            .warnings
            .push("missing cursor/hooks directory".to_string());
        return Ok(());
    }

    let hooks_dest = opts.repo_path.join(".cursor/hooks");
    fs::create_dir_all(&hooks_dest)?;

    for entry in fs::read_dir(&hooks_src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let dest = hooks_dest.join(entry.file_name());
            if dest.exists() && !should_replace(&dest, opts.overwrite, opts.non_interactive) {
                result.skipped.push(dest.display().to_string());
            } else {
                if dest.exists() {
                    backup_file(&dest, &mut result.backed_up)?;
                }
                fs::copy(&path, &dest)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&dest)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&dest, perms)?;
                }
                result.installed.push(dest.display().to_string());
            }
        }
    }

    if hooks_json_src.is_file() {
        let dest = opts.repo_path.join(".cursor/hooks.json");
        if dest.exists() {
            if should_replace(&dest, opts.overwrite, opts.non_interactive) {
                backup_file(&dest, &mut result.backed_up)?;
                fs::copy(&hooks_json_src, &dest)?;
                result.installed.push(dest.display().to_string());
            } else {
                result.skipped.push(dest.display().to_string());
            }
        } else {
            fs::copy(&hooks_json_src, &dest)?;
            result.installed.push(dest.display().to_string());
        }
    }
    Ok(())
}

fn install_rules(
    opts: &AgentPackInstallOptions,
    result: &mut AgentPackInstallResult,
) -> io::Result<()> {
    let rules_src = opts.pack_root.join("cursor/rules");
    if !rules_src.is_dir() {
        result
            .warnings
            .push("missing cursor/rules directory".to_string());
        return Ok(());
    }
    let rules_dest = opts.repo_path.join(".cursor/rules");
    fs::create_dir_all(&rules_dest)?;

    for entry in fs::read_dir(&rules_src)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let dest = rules_dest.join(entry.file_name());
        if dest.exists() && !should_replace(&dest, opts.overwrite, opts.non_interactive) {
            result.skipped.push(dest.display().to_string());
            continue;
        }
        if dest.exists() {
            backup_file(&dest, &mut result.backed_up)?;
        }
        fs::copy(&path, &dest)?;
        result.installed.push(dest.display().to_string());
    }
    Ok(())
}

fn mcp_config(repo_cwd: &str, command: &str) -> serde_json::Value {
    json!({
        "mcpServers": {
            "codecortex": {
                "command": command,
                "args": ["mcp", "start"],
                "cwd": repo_cwd
            }
        }
    })
}

fn write_mcp_file(
    path: &Path,
    repo_cwd: &str,
    command: &str,
    overwrite: bool,
    non_interactive: bool,
    installed: &mut Vec<String>,
    skipped: &mut Vec<String>,
    backed_up: &mut Vec<String>,
) -> io::Result<()> {
    if path.exists() {
        if should_replace(path, overwrite, non_interactive) {
            backup_file(path, backed_up)?;
        } else {
            skipped.push(path.display().to_string());
            return Ok(());
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(&mcp_config(repo_cwd, command))?;
    fs::write(path, body)?;
    installed.push(path.display().to_string());
    Ok(())
}

/// Install the agent pack into `opts.repo_path`.
pub fn install_agent_pack(opts: AgentPackInstallOptions) -> Result<AgentPackInstallResult, String> {
    if !pack_layout_valid(&opts.pack_root) {
        return Err(format!(
            "invalid agent pack at {}: expected skills/ and cursor/",
            opts.pack_root.display()
        ));
    }

    let repo = opts
        .repo_path
        .canonicalize()
        .map_err(|e| format!("repo_path: {e}"))?;
    let pack = opts
        .pack_root
        .canonicalize()
        .map_err(|e| format!("pack_root: {e}"))?;

    let use_symlink = same_git_repo(&repo, &pack);
    let repo_cwd = repo.display().to_string();

    let mut result = AgentPackInstallResult {
        agent_pack_root: pack.display().to_string(),
        installed: Vec::new(),
        skipped: Vec::new(),
        backed_up: Vec::new(),
        warnings: Vec::new(),
    };

    if opts.install_skills {
        install_skills_dir(
            &AgentPackInstallOptions {
                repo_path: repo.clone(),
                pack_root: pack.clone(),
                ..opts.clone()
            },
            use_symlink,
            &mut result,
        )
        .map_err(|e| e.to_string())?;
    }

    if opts.install_agents {
        install_agents_dir(
            &AgentPackInstallOptions {
                repo_path: repo.clone(),
                pack_root: pack.clone(),
                ..opts.clone()
            },
            use_symlink,
            &mut result,
        )
        .map_err(|e| e.to_string())?;
    }

    if opts.install_hooks {
        install_hooks(
            &AgentPackInstallOptions {
                repo_path: repo.clone(),
                pack_root: pack.clone(),
                ..opts.clone()
            },
            &mut result,
        )
        .map_err(|e| e.to_string())?;
    }

    if opts.install_rules {
        install_rules(
            &AgentPackInstallOptions {
                repo_path: repo.clone(),
                pack_root: pack.clone(),
                ..opts.clone()
            },
            &mut result,
        )
        .map_err(|e| e.to_string())?;
    }

    if opts.install_mcp {
        let mcp_path = repo.join("mcp.json");
        write_mcp_file(
            &mcp_path,
            &repo_cwd,
            &opts.mcp_command,
            opts.overwrite,
            opts.non_interactive,
            &mut result.installed,
            &mut result.skipped,
            &mut result.backed_up,
        )
        .map_err(|e| e.to_string())?;
    }

    if opts.install_cursor_mcp {
        let cursor_mcp = repo.join(".cursor/mcp.json");
        write_mcp_file(
            &cursor_mcp,
            &repo_cwd,
            &opts.mcp_command,
            opts.overwrite,
            opts.non_interactive,
            &mut result.installed,
            &mut result.skipped,
            &mut result.backed_up,
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn minimal_pack(root: &Path) {
        fs::create_dir_all(root.join("skills/demo")).unwrap();
        fs::write(root.join("skills/demo/SKILL.md"), "---\nname: demo\n---\n").unwrap();
        fs::create_dir_all(root.join("agents")).unwrap();
        fs::write(root.join("agents/codecortex-demo.md"), "# demo").unwrap();
        fs::create_dir_all(root.join("cursor/hooks")).unwrap();
        fs::write(
            root.join("cursor/hooks.json"),
            r#"{"version":1,"hooks":{}}"#,
        )
        .unwrap();
        fs::write(root.join("cursor/hooks/stub.sh"), "#!/bin/sh\n").unwrap();
        fs::create_dir_all(root.join("cursor/rules")).unwrap();
        fs::write(root.join("cursor/rules/codecortex-core.mdc"), "---\n---\n").unwrap();
    }

    #[test]
    fn install_creates_cursor_assets() {
        let pack_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();
        minimal_pack(pack_dir.path());

        let opts = AgentPackInstallOptions::for_repo(repo_dir.path(), pack_dir.path());
        let result = install_agent_pack(opts).expect("install");
        assert!(
            result
                .installed
                .iter()
                .any(|p| p.contains(".cursor/skills"))
        );
        assert!(repo_dir.path().join(".cursor/hooks.json").exists());
        assert!(repo_dir.path().join(".cursor/mcp.json").exists());
        assert!(repo_dir.path().join("mcp.json").exists());
    }

    #[test]
    fn install_skips_existing_without_overwrite() {
        let pack_dir = TempDir::new().unwrap();
        let repo_dir = TempDir::new().unwrap();
        minimal_pack(pack_dir.path());
        fs::create_dir_all(repo_dir.path().join(".cursor/rules")).unwrap();
        fs::write(
            repo_dir.path().join(".cursor/rules/codecortex-core.mdc"),
            "existing",
        )
        .unwrap();

        let opts = AgentPackInstallOptions::for_repo(repo_dir.path(), pack_dir.path());
        let result = install_agent_pack(opts).expect("install");
        assert!(result.skipped.iter().any(|p| p.contains("codecortex-core")));
        let content =
            fs::read_to_string(repo_dir.path().join(".cursor/rules/codecortex-core.mdc")).unwrap();
        assert_eq!(content, "existing");
    }

    #[test]
    fn resolve_finds_pack_via_env() {
        let pack_dir = TempDir::new().unwrap();
        minimal_pack(pack_dir.path());
        unsafe {
            std::env::set_var("CORTEX_AGENT_PACK", pack_dir.path());
        }
        let repo_dir = TempDir::new().unwrap();
        let resolved = resolve_agent_pack(repo_dir.path(), None);
        unsafe {
            std::env::remove_var("CORTEX_AGENT_PACK");
        }
        assert_eq!(resolved, Some(pack_dir.path().to_path_buf()));
    }
}
