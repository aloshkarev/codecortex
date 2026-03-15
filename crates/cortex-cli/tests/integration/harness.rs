use super::projects::RepoFixture;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct IntegrationContext {
    pub bin_path: PathBuf,
    pub home_dir: PathBuf,
    pub fixture_root: PathBuf,
}

impl IntegrationContext {
    pub fn from_env(bin_path: PathBuf) -> Option<Self> {
        if std::env::var("CORTEX_INTEGRATION_ENABLE").ok().as_deref() != Some("1") {
            return None;
        }
        let unique =
            std::env::var("CORTEX_INTEGRATION_RUN_ID").unwrap_or_else(|_| unique_test_id());
        let root = std::env::temp_dir().join(format!("codecortex-integration-{unique}"));
        let home_dir = root.join("home");
        let fixture_root = root.join("projects");
        if fs::create_dir_all(home_dir.as_path()).is_err() {
            return None;
        }
        if fs::create_dir_all(fixture_root.as_path()).is_err() {
            return None;
        }
        if write_default_config(home_dir.as_path()).is_err() {
            return None;
        }
        Some(Self {
            bin_path,
            home_dir,
            fixture_root,
        })
    }

    pub fn prepare_project_fixture(&self, fixture: RepoFixture) -> Result<PathBuf, String> {
        static CLONE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let lock = CLONE_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.lock().map_err(|e| e.to_string())?;

        let repo_dir = self.fixture_root.join(format!(
            "{}-{}",
            fixture.language.as_str(),
            fixture.commit_sha
        ));
        if repo_dir.exists() {
            return Ok(repo_dir);
        }
        let parent = repo_dir
            .parent()
            .ok_or_else(|| "failed to resolve fixture parent directory".to_string())?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;

        run_process(
            Command::new("git")
                .arg("clone")
                .arg("--filter=blob:none")
                .arg("--no-checkout")
                .arg(fixture.clone_url)
                .arg(repo_dir.as_os_str()),
        )?;
        run_process(
            Command::new("git")
                .current_dir(repo_dir.as_path())
                .arg("checkout")
                .arg(fixture.commit_sha),
        )?;
        Ok(repo_dir)
    }
}

pub fn run_cortex_json(
    ctx: &IntegrationContext,
    cwd: &Path,
    args: &[&str],
) -> Result<Value, String> {
    let mut cmd = Command::new(&ctx.bin_path);
    cmd.current_dir(cwd);
    cmd.arg("--format").arg("json");
    cmd.args(args);
    cmd.env("HOME", ctx.home_dir.as_os_str());
    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(output.stdout.as_slice()),
            String::from_utf8_lossy(output.stderr.as_slice())
        ));
    }
    serde_json::from_slice(output.stdout.as_slice()).map_err(|e| {
        format!(
            "non-json output for {:?}: {}\nstdout:\n{}",
            args,
            e,
            String::from_utf8_lossy(output.stdout.as_slice())
        )
    })
}

pub fn command_available(bin_name: &str) -> bool {
    Command::new(bin_name)
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn write_default_config(home_dir: &Path) -> Result<(), String> {
    let cortex_dir = home_dir.join(".cortex");
    fs::create_dir_all(cortex_dir.as_path()).map_err(|e| e.to_string())?;
    let config_path = cortex_dir.join("config.toml");

    if let Ok(home) = std::env::var("HOME") {
        let source = PathBuf::from(home).join(".cortex/config.toml");
        if source.exists() {
            let content = fs::read_to_string(source).map_err(|e| e.to_string())?;
            fs::write(config_path, content).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    let bolt_uri = std::env::var("CORTEX_TEST_BOLT_URI")
        .unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("CORTEX_TEST_BOLT_USER").unwrap_or_default();
    let password = std::env::var("CORTEX_TEST_BOLT_PASSWORD").unwrap_or_default();
    let backend = std::env::var("CORTEX_TEST_BACKEND").unwrap_or_else(|_| "memgraph".to_string());
    let cfg = format!(
        "memgraph_uri = \"{bolt_uri}\"\nmemgraph_user = \"{user}\"\nmemgraph_password = \"{password}\"\nbackend_type = \"{backend}\"\n"
    );
    fs::write(config_path, cfg).map_err(|e| e.to_string())
}

fn run_process(cmd: &mut Command) -> Result<(), String> {
    let rendered = render_command(cmd);
    let output = cmd.output().map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "command failed: {rendered}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(output.stdout.as_slice()),
        String::from_utf8_lossy(output.stderr.as_slice())
    ))
}

fn render_command(cmd: &Command) -> String {
    let mut parts = Vec::new();
    parts.push(cmd.get_program().to_string_lossy().to_string());
    for arg in cmd.get_args() {
        parts.push(arg.to_string_lossy().to_string());
    }
    parts.join(" ")
}

fn unique_test_id() -> String {
    let pid = std::process::id();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{pid}-{ts}")
}
