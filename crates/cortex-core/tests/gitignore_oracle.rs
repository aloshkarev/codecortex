//! Optional `git check-ignore` oracle for `.cortexignore` parity with git semantics.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Returns true when `git check-ignore` is available on PATH.
pub fn git_check_ignore_available() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run `git check-ignore -q --no-index` for `relpath` inside `repo_root`.
pub fn git_check_ignore(repo_root: &Path, relpath: &str) -> std::io::Result<bool> {
    let mut child = Command::new("git")
        .current_dir(repo_root)
        .args(["check-ignore", "-q", "--no-index", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(relpath.as_bytes())?;
        stdin.write_all(b"\n")?;
    }

    let status = child.wait()?;
    Ok(status.success())
}

/// Initialize a git repo for oracle fixtures.
pub fn init_git_repo_with_cortexignore(repo_root: &Path) -> std::io::Result<()> {
    Command::new("git")
        .current_dir(repo_root)
        .args(["init", "-q"])
        .status()?;
    Ok(())
}

#[test]
#[ignore = "requires git binary; run with `cargo test gitignore_oracle -- --ignored`"]
fn git_oracle_matches_cortexignore_dir_pattern() {
    if !git_check_ignore_available() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join(".cortexignore"), "ignored/\n").unwrap();
    std::fs::create_dir_all(root.join("ignored")).unwrap();
    std::fs::write(root.join("ignored/skip.rs"), "fn s() {}").unwrap();
    std::fs::write(root.join("keep.rs"), "fn k() {}").unwrap();
    init_git_repo_with_cortexignore(root).unwrap();

    // Git does not read `.cortexignore` natively; mirror patterns in `.gitignore` for oracle.
    std::fs::write(root.join(".gitignore"), "ignored/\n").unwrap();

    assert!(git_check_ignore(root, "ignored/skip.rs").unwrap());
    assert!(!git_check_ignore(root, "keep.rs").unwrap());

    use cortex_core::{CortexIgnoreOptions, CortexIgnoreWalker};
    let walker = CortexIgnoreWalker::new(CortexIgnoreOptions {
        repo_root: root.to_path_buf(),
        scan_root: None,
        global_ignore_path: None,
        respect_gitignore: true,
        respect_cortexignore: true,
        include_hidden: false,
        policy_excludes: vec![],
        count_ignored_skips: false,
    });
    assert_eq!(
        walker.is_ignored(&root.join("ignored/skip.rs")),
        git_check_ignore(root, "ignored/skip.rs").unwrap()
    );
    assert_eq!(
        walker.is_ignored(&root.join("keep.rs")),
        git_check_ignore(root, "keep.rs").unwrap()
    );
}

#[test]
fn git_oracle_skipped_when_git_missing() {
    if git_check_ignore_available() {
        return;
    }
    let root = PathBuf::from("/tmp/nonexistent-cortex-git-oracle");
    assert!(!git_check_ignore(&root, "file.rs").unwrap_or(false));
}
