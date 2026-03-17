pub fn tool_names() -> &'static [&'static str] {
    &[
        // Indexing tools
        "add_code_to_graph",
        "watch_directory",
        "list_watched_paths",
        "unwatch_directory",
        // Search and analysis tools
        "find_code",
        "analyze_code_relationships",
        "execute_cypher_query",
        "find_dead_code",
        "go_to_definition",
        "find_all_usages",
        "quick_info",
        "branch_structural_diff",
        "pr_review",
        "find_similar_across_projects",
        "find_shared_dependencies",
        "compare_api_surface",
        "calculate_cyclomatic_complexity",
        // Vector tools
        "vector_index_repository",
        "vector_index_file",
        "vector_search",
        "vector_search_hybrid",
        "search_across_projects",
        "vector_index_status",
        "vector_delete_repository",
        // Context and impact tools
        "get_context_capsule",
        "get_impact_graph",
        "search_logic_flow",
        "get_skeleton",
        "index_status",
        "workspace_setup",
        // LSP integration
        "submit_lsp_edges",
        // Memory tools
        "save_observation",
        "get_session_context",
        "search_memory",
        // Repository tools
        "list_indexed_repositories",
        "delete_repository",
        "get_repository_stats",
        // Job management
        "check_job_status",
        "list_jobs",
        // Bundle tools
        "load_bundle",
        "export_bundle",
        // Health tools
        "check_health",
        "diagnose",
        // Signature and test tools
        "get_signature",
        "find_tests",
        "explain_result",
        // Refactoring tools
        "analyze_refactoring",
        "find_patterns",
        // Project management tools
        "list_projects",
        "add_project",
        "remove_project",
        "set_current_project",
        "get_current_project",
        "list_branches",
        "refresh_project",
        "project_status",
        "project_sync",
        "project_branch_diff",
        "project_queue_status",
        "project_metrics",
    ]
}
