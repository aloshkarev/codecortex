use cortex_mcp::tool_names;
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CoverageKind {
    HealthOrMetadata,
    RequiresRepoPath,
    RequiresIndexedGraph,
    RequiresVectorStore,
    ProjectManagement,
    WatchOrJobs,
    Memory,
    BundleOrLsp,
    AdvancedQuery,
}

#[derive(Debug, Clone, Copy)]
struct CoverageProfile {
    tool: &'static str,
    kind: CoverageKind,
}

const FULL_SURFACE_PROFILES: &[CoverageProfile] = &[
    CoverageProfile {
        tool: "add_code_to_graph",
        kind: CoverageKind::RequiresRepoPath,
    },
    CoverageProfile {
        tool: "list_indexed_repositories",
        kind: CoverageKind::HealthOrMetadata,
    },
    CoverageProfile {
        tool: "delete_repository",
        kind: CoverageKind::RequiresRepoPath,
    },
    CoverageProfile {
        tool: "get_repository_stats",
        kind: CoverageKind::HealthOrMetadata,
    },
    CoverageProfile {
        tool: "vector_index_repository",
        kind: CoverageKind::RequiresVectorStore,
    },
    CoverageProfile {
        tool: "vector_index_file",
        kind: CoverageKind::RequiresVectorStore,
    },
    CoverageProfile {
        tool: "vector_search",
        kind: CoverageKind::RequiresVectorStore,
    },
    CoverageProfile {
        tool: "vector_search_hybrid",
        kind: CoverageKind::RequiresVectorStore,
    },
    CoverageProfile {
        tool: "vector_index_status",
        kind: CoverageKind::RequiresVectorStore,
    },
    CoverageProfile {
        tool: "vector_delete_repository",
        kind: CoverageKind::RequiresVectorStore,
    },
    CoverageProfile {
        tool: "find_code",
        kind: CoverageKind::RequiresIndexedGraph,
    },
    CoverageProfile {
        tool: "get_skeleton",
        kind: CoverageKind::RequiresRepoPath,
    },
    CoverageProfile {
        tool: "get_signature",
        kind: CoverageKind::RequiresRepoPath,
    },
    CoverageProfile {
        tool: "analyze_code_relationships",
        kind: CoverageKind::RequiresIndexedGraph,
    },
    CoverageProfile {
        tool: "find_dead_code",
        kind: CoverageKind::RequiresIndexedGraph,
    },
    CoverageProfile {
        tool: "calculate_cyclomatic_complexity",
        kind: CoverageKind::RequiresIndexedGraph,
    },
    CoverageProfile {
        tool: "analyze_refactoring",
        kind: CoverageKind::RequiresRepoPath,
    },
    CoverageProfile {
        tool: "find_patterns",
        kind: CoverageKind::RequiresIndexedGraph,
    },
    CoverageProfile {
        tool: "find_tests",
        kind: CoverageKind::RequiresIndexedGraph,
    },
    CoverageProfile {
        tool: "get_context_capsule",
        kind: CoverageKind::RequiresIndexedGraph,
    },
    CoverageProfile {
        tool: "get_impact_graph",
        kind: CoverageKind::RequiresIndexedGraph,
    },
    CoverageProfile {
        tool: "search_logic_flow",
        kind: CoverageKind::RequiresIndexedGraph,
    },
    CoverageProfile {
        tool: "check_health",
        kind: CoverageKind::HealthOrMetadata,
    },
    CoverageProfile {
        tool: "index_status",
        kind: CoverageKind::HealthOrMetadata,
    },
    CoverageProfile {
        tool: "diagnose",
        kind: CoverageKind::HealthOrMetadata,
    },
    CoverageProfile {
        tool: "explain_result",
        kind: CoverageKind::HealthOrMetadata,
    },
    CoverageProfile {
        tool: "list_projects",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "add_project",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "remove_project",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "set_current_project",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "get_current_project",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "list_branches",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "refresh_project",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "project_status",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "project_sync",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "project_branch_diff",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "project_queue_status",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "project_metrics",
        kind: CoverageKind::ProjectManagement,
    },
    CoverageProfile {
        tool: "watch_directory",
        kind: CoverageKind::WatchOrJobs,
    },
    CoverageProfile {
        tool: "unwatch_directory",
        kind: CoverageKind::WatchOrJobs,
    },
    CoverageProfile {
        tool: "list_watched_paths",
        kind: CoverageKind::WatchOrJobs,
    },
    CoverageProfile {
        tool: "check_job_status",
        kind: CoverageKind::WatchOrJobs,
    },
    CoverageProfile {
        tool: "list_jobs",
        kind: CoverageKind::WatchOrJobs,
    },
    CoverageProfile {
        tool: "save_observation",
        kind: CoverageKind::Memory,
    },
    CoverageProfile {
        tool: "get_session_context",
        kind: CoverageKind::Memory,
    },
    CoverageProfile {
        tool: "search_memory",
        kind: CoverageKind::Memory,
    },
    CoverageProfile {
        tool: "load_bundle",
        kind: CoverageKind::BundleOrLsp,
    },
    CoverageProfile {
        tool: "export_bundle",
        kind: CoverageKind::BundleOrLsp,
    },
    CoverageProfile {
        tool: "submit_lsp_edges",
        kind: CoverageKind::BundleOrLsp,
    },
    CoverageProfile {
        tool: "workspace_setup",
        kind: CoverageKind::BundleOrLsp,
    },
    CoverageProfile {
        tool: "execute_cypher_query",
        kind: CoverageKind::AdvancedQuery,
    },
];

fn readme_tool_set() -> HashSet<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("README.md");
    let content = std::fs::read_to_string(root).expect("README.md");
    let mut tools = HashSet::new();
    let mut in_mcp_tool_section = false;
    for line in content.lines() {
        if line.trim_start().starts_with("### MCP Tool Coverage") {
            in_mcp_tool_section = true;
            continue;
        }
        if in_mcp_tool_section && line.trim_start().starts_with("## ") {
            break;
        }
        if !in_mcp_tool_section {
            continue;
        }
        if !line.contains("**") || !line.contains('`') {
            continue;
        }
        if !line.trim_start().starts_with("- **") {
            continue;
        }
        for segment in line.split('`').skip(1).step_by(2) {
            if segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                tools.insert(segment.to_string());
            }
        }
    }
    tools
}

#[test]
fn full_surface_profiles_cover_every_exported_tool() {
    let exported: HashSet<&str> = tool_names().iter().copied().collect();
    let profiled: HashSet<&str> = FULL_SURFACE_PROFILES.iter().map(|p| p.tool).collect();

    let missing_profiles: BTreeSet<&str> = exported.difference(&profiled).copied().collect();
    let stale_profiles: BTreeSet<&str> = profiled.difference(&exported).copied().collect();

    assert!(
        missing_profiles.is_empty(),
        "missing coverage profiles for tools: {:?}",
        missing_profiles
    );
    assert!(
        stale_profiles.is_empty(),
        "stale coverage profiles not in exported tool_names(): {:?}",
        stale_profiles
    );
}

#[test]
fn readme_mcp_tool_list_matches_exported_tools() {
    let exported: HashSet<String> = tool_names().iter().map(|t| (*t).to_string()).collect();
    let documented = readme_tool_set();

    let missing_in_readme: BTreeSet<String> = exported.difference(&documented).cloned().collect();
    let missing_in_code: BTreeSet<String> = documented.difference(&exported).cloned().collect();

    assert!(
        missing_in_readme.is_empty(),
        "README missing tools present in code: {:?}",
        missing_in_readme
    );
    assert!(
        missing_in_code.is_empty(),
        "README includes tools not present in code: {:?}",
        missing_in_code
    );
}

#[test]
fn coverage_profiles_include_all_categories() {
    let mut categories = BTreeSet::new();
    for profile in FULL_SURFACE_PROFILES {
        categories.insert(profile.kind);
    }
    assert_eq!(
        categories.len(),
        9,
        "all coverage categories should be represented"
    );
}
