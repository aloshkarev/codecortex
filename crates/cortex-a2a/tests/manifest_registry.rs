//! Agent manifest registry: config precedence over markdown.

use cortex_a2a::RoleManifestRegistry;
use cortex_core::A2aConfig;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn config_overrides_markdown_subscriptions() {
    let dir = TempDir::new().unwrap();
    let agents = dir.path().join("agents");
    std::fs::create_dir_all(&agents).unwrap();
    let mut f = std::fs::File::create(agents.join("codecortex-analyzer.md")).unwrap();
    writeln!(
        f,
        r#"
## A2A subscriptions (incoming)
- `GraphMutationSignal` only from markdown
"#
    )
    .unwrap();

    let mut config = A2aConfig::default();
    config.agent_manifest_paths = vec![agents];
    config.roles.insert(
        "analyzer".to_string(),
        cortex_core::A2aRoleConfig {
            subscriptions: vec!["CodeInsight".to_string()],
            ..Default::default()
        },
    );

    let reg = RoleManifestRegistry::load(&config, None);
    let m = reg.get("analyzer").expect("analyzer manifest");
    assert!(m.subscriptions.contains(&"CodeInsight".to_string()));
    assert!(!m.subscriptions.contains(&"GraphMutationSignal".to_string()));
}
