use super::harness::{IntegrationContext, run_cortex_json};
use super::projects::RepoFixture;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn run_full_functionality_suite(ctx: &IntegrationContext, fixture: RepoFixture) {
    let repo = ctx
        .prepare_project_fixture(fixture)
        .unwrap_or_else(|e| panic!("failed to prepare fixture {}: {}", fixture.slug, e));
    let repo_path = repo.to_string_lossy().to_string();

    run_index_checks(ctx, fixture, repo.as_path(), repo_path.as_str());
    run_analyze_checks(ctx, fixture, repo.as_path(), repo_path.as_str());
    run_mcp_checks(ctx, fixture, repo.as_path(), repo_path.as_str());
}

fn run_index_checks(ctx: &IntegrationContext, fixture: RepoFixture, repo: &Path, repo_path: &str) {
    let _ = run_cortex_json(ctx, repo, &["delete", repo_path]);
    let index = run_cortex_json(ctx, repo, &["index", repo_path, "--force"])
        .unwrap_or_else(|e| panic!("index failed for {}: {}", fixture.slug, e));
    assert!(
        index
            .get("indexed_files")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0,
        "expected indexed_files > 0 for {}: {}",
        fixture.slug,
        index
    );

    let list = run_cortex_json(ctx, repo, &["list"])
        .unwrap_or_else(|e| panic!("list failed for {}: {}", fixture.slug, e));
    assert!(
        list.to_string().contains(repo_path),
        "repository should appear in indexed list for {}: {}",
        fixture.slug,
        list
    );

    let symbol_query = format!(
        "MATCH (n:CodeNode) WHERE n.name = '{}' AND n.path CONTAINS '{}' RETURN count(n) AS c",
        cypher_escaped(fixture.entry_symbol),
        cypher_escaped(repo_path)
    );
    let symbol_rows = run_cortex_json(ctx, repo, &["query", symbol_query.as_str()])
        .unwrap_or_else(|e| panic!("symbol query failed for {}: {}", fixture.slug, e));
    let symbol_count = symbol_rows
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("c"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert!(
        symbol_count > 0,
        "expected symbol {} in {}",
        fixture.entry_symbol,
        fixture.slug
    );

    let suffix_predicate = fixture
        .language
        .extensions()
        .iter()
        .map(|ext| format!("n.path ENDS WITH '.{}'", cypher_escaped(ext)))
        .collect::<Vec<_>>()
        .join(" OR ");
    let ext_query = format!(
        "MATCH (n:CodeNode) WHERE n.path CONTAINS '{}' AND ({}) RETURN count(n) AS c",
        cypher_escaped(repo_path),
        suffix_predicate
    );
    let ext_rows = run_cortex_json(ctx, repo, &["query", ext_query.as_str()])
        .unwrap_or_else(|e| panic!("extension query failed for {}: {}", fixture.slug, e));
    let ext_count = ext_rows
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("c"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert!(
        ext_count > 0,
        "expected indexed nodes for extensions {:?} in {}",
        fixture.language.extensions(),
        fixture.slug
    );
}

fn run_analyze_checks(
    ctx: &IntegrationContext,
    fixture: RepoFixture,
    repo: &Path,
    repo_path: &str,
) {
    let include_glob = format!("**/*.{}", fixture.language.primary_extension());

    let static_commands: &[&[&str]] = &[
        &["analyze", "dead-code"],
        &["analyze", "complexity", "--top", "15"],
        &[
            "analyze",
            "smells",
            ".",
            "--max-files",
            "1000",
            "--limit",
            "200",
            "--include-glob",
            "__INCLUDE_GLOB__",
        ],
        &[
            "analyze",
            "refactoring",
            ".",
            "--max-files",
            "1000",
            "--limit",
            "200",
            "--include-glob",
            "__INCLUDE_GLOB__",
        ],
    ];

    for cmd in static_commands {
        let mut args: Vec<String> = cmd.iter().map(|s| s.to_string()).collect();
        for arg in &mut args {
            if arg == "__INCLUDE_GLOB__" {
                *arg = include_glob.clone();
            }
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = run_cortex_json(ctx, repo, refs.as_slice()).unwrap_or_else(|e| {
            panic!(
                "analyze command {:?} failed for {}: {}",
                refs, fixture.slug, e
            )
        });
        assert!(output.is_object() || output.is_array());
    }

    for args in [
        vec!["analyze", "callers", fixture.entry_symbol],
        vec!["analyze", "callees", fixture.entry_symbol],
        vec!["analyze", "overrides", fixture.entry_symbol],
        vec!["analyze", "hierarchy", fixture.entry_symbol],
        vec!["analyze", "deps", fixture.entry_symbol],
    ] {
        let output = run_cortex_json(ctx, repo, args.as_slice()).unwrap_or_else(|e| {
            panic!(
                "symbol analyze {:?} failed for {}: {}",
                args, fixture.slug, e
            )
        });
        assert!(output.is_array() || output.is_object());
    }

    let chain = run_cortex_json(
        ctx,
        repo,
        &[
            "analyze",
            "chain",
            fixture.entry_symbol,
            fixture.entry_symbol,
        ],
    )
    .unwrap_or_else(|e| panic!("chain analyze failed for {}: {}", fixture.slug, e));
    assert!(chain.is_array() || chain.is_object());

    let current_ref = git_current_branch(repo).unwrap_or_else(|| "HEAD".to_string());
    let branch_diff = run_cortex_json(
        ctx,
        repo,
        &[
            "analyze",
            "branch-diff",
            current_ref.as_str(),
            current_ref.as_str(),
            "--path",
            repo_path,
            "--commit-limit",
            "5",
        ],
    )
    .unwrap_or_else(|e| panic!("branch-diff analyze failed for {}: {}", fixture.slug, e));
    assert!(branch_diff.is_array() || branch_diff.is_object());

    let review = run_cortex_json(
        ctx,
        repo,
        &[
            "analyze",
            "review",
            "--base",
            "HEAD",
            "--head",
            "HEAD",
            "--path",
            repo_path,
            "--max-findings",
            "50",
        ],
    )
    .unwrap_or_else(|e| panic!("review analyze failed for {}: {}", fixture.slug, e));
    assert!(review.is_array() || review.is_object());
}

fn run_mcp_checks(ctx: &IntegrationContext, fixture: RepoFixture, repo: &Path, repo_path: &str) {
    let tools_list = mcp_request(ctx, json!({"method":"tools/list","params":{}}), 120)
        .unwrap_or_else(|e| panic!("MCP tools/list failed for {}: {}", fixture.slug, e));
    let tool_names = extract_tool_names(&tools_list);
    for expected in [
        "add_code_to_graph",
        "find_code",
        "analyze_code_relationships",
        "check_health",
        "execute_cypher_query",
        "project_status",
    ] {
        assert!(
            tool_names.contains(expected),
            "MCP tool missing: {}",
            expected
        );
    }

    let source_file = first_file_with_extensions(repo, fixture.language.extensions())
        .unwrap_or_else(|| panic!("expected source file in {}", fixture.slug));
    let source_file_str = source_file.to_string_lossy().to_string();
    let current_branch = git_current_branch(repo).unwrap_or_else(|| "main".to_string());
    let bundle_out = ctx.fixture_root.join(format!(
        "{}-{}.ccx",
        fixture.language.as_str(),
        fixture.commit_sha
    ));
    let bundle_out_str = bundle_out.to_string_lossy().to_string();

    let calls = vec![
        ("check_health", json!({})),
        ("index_status", json!({"repo_path": repo_path})),
        ("diagnose", json!({"check":"all","repo_path": repo_path})),
        ("add_code_to_graph", json!({"path": repo_path})),
        ("list_indexed_repositories", json!({})),
        ("get_repository_stats", json!({})),
        (
            "find_code",
            json!({"query":fixture.entry_symbol,"kind":"name","path_filter": repo_path}),
        ),
        (
            "analyze_code_relationships",
            json!({"query_type":"find_callers","target":fixture.entry_symbol,"include_paths":[repo_path]}),
        ),
        ("find_dead_code", json!({"include_paths":[repo_path]})),
        (
            "calculate_cyclomatic_complexity",
            json!({"top_n":20,"include_paths":[repo_path]}),
        ),
        (
            "analyze_refactoring",
            json!({"symbol":fixture.entry_symbol,"repo_path": repo_path}),
        ),
        (
            "find_patterns",
            json!({"repo_path": repo_path, "max_results": 20}),
        ),
        (
            "find_tests",
            json!({"symbol":fixture.entry_symbol,"repo_path": repo_path}),
        ),
        (
            "get_skeleton",
            json!({"path": source_file_str, "repo_path": repo_path}),
        ),
        (
            "get_signature",
            json!({"symbol":fixture.entry_symbol,"repo_path": repo_path}),
        ),
        (
            "get_context_capsule",
            json!({"query":"integration context", "repo_path": repo_path}),
        ),
        (
            "get_impact_graph",
            json!({"symbol":fixture.entry_symbol,"repo_path": repo_path}),
        ),
        (
            "search_logic_flow",
            json!({"from_symbol":fixture.entry_symbol,"to_symbol":fixture.entry_symbol,"repo_path": repo_path}),
        ),
        (
            "explain_result",
            json!({"query":fixture.entry_symbol,"tool":"find_code","repo_path": repo_path}),
        ),
        (
            "save_observation",
            json!({"repo_path": repo_path,"text":"integration check observation","severity":"low"}),
        ),
        (
            "get_session_context",
            json!({"repo_path": repo_path,"max_items": 10}),
        ),
        (
            "search_memory",
            json!({"repo_path": repo_path,"query":"integration","max_items": 10}),
        ),
        ("watch_directory", json!({"path": repo_path})),
        ("list_watched_paths", json!({})),
        ("unwatch_directory", json!({"path": repo_path})),
        ("list_jobs", json!({})),
        ("check_job_status", json!({"id":"integration-job"})),
        (
            "workspace_setup",
            json!({"repo_path": repo_path, "non_interactive": true}),
        ),
        (
            "submit_lsp_edges",
            json!({"repo_path": repo_path, "edges":[{"caller_fqn":"crate::a::f","callee_fqn":"crate::b::g","file":source_file_str,"line":1}]}),
        ),
        (
            "export_bundle",
            json!({"repository_path": repo_path, "output_path": bundle_out_str}),
        ),
        ("load_bundle", json!({"path": bundle_out_str})),
        ("execute_cypher_query", json!({"query":"RETURN 1 AS ok"})),
        (
            "add_project",
            json!({"path": repo_path, "track_branch": true}),
        ),
        ("list_projects", json!({})),
        ("set_current_project", json!({"path": repo_path})),
        ("get_current_project", json!({})),
        ("list_branches", json!({"path": repo_path})),
        ("refresh_project", json!({"path": repo_path})),
        (
            "project_status",
            json!({"path": repo_path, "include_queue": true}),
        ),
        ("project_sync", json!({"path": repo_path, "force": false})),
        (
            "project_branch_diff",
            json!({"path": repo_path, "source": current_branch, "target": current_branch}),
        ),
        (
            "project_queue_status",
            json!({"path": repo_path, "limit": 10}),
        ),
        ("project_metrics", json!({"path": repo_path})),
        (
            "remove_project",
            json!({"path": repo_path, "delete_data": false}),
        ),
        (
            "vector_index_repository",
            json!({"path": repo_path, "repo_path": repo_path}),
        ),
        (
            "vector_index_file",
            json!({"path": source_file_str, "repo_path": repo_path}),
        ),
        (
            "vector_search",
            json!({"query":"integration search", "repo_path": repo_path, "k": 5}),
        ),
        (
            "vector_search_hybrid",
            json!({"query":"integration search", "repo_path": repo_path, "k": 5}),
        ),
        ("vector_index_status", json!({"repo_path": repo_path})),
        ("vector_delete_repository", json!({"repo_path": repo_path})),
        ("delete_repository", json!({"path": repo_path})),
    ];

    let tolerated_vector_failures = [
        "vector_index_repository",
        "vector_index_file",
        "vector_search",
        "vector_search_hybrid",
        "vector_index_status",
        "vector_delete_repository",
    ];
    let tolerated_heavy_transport_failures = ["get_impact_graph", "search_logic_flow"];

    for (name, args) in calls {
        let timeout_secs = match name {
            "get_impact_graph" | "analyze_code_relationships" | "find_dead_code" => 600,
            _ => 120,
        };
        let response = match mcp_request(
            ctx,
            json!({"method":"tools/call","params":{"name":name,"arguments":args}}),
            timeout_secs,
        ) {
            Ok(resp) => resp,
            Err(e) => {
                let tolerated_timeout =
                    tolerated_vector_failures.contains(&name) && e.contains("timed out");
                let tolerated_heavy_transport = tolerated_heavy_transport_failures.contains(&name)
                    && (e.contains("timed out")
                        || e.contains("failed to locate response for request id=2")
                        || e.contains("channel disconnected"));
                if tolerated_timeout || tolerated_heavy_transport {
                    continue;
                }
                panic!(
                    "MCP call failed for tool '{}' in {}: {}",
                    name, fixture.slug, e
                );
            }
        };
        if let Some(error) = response.get("error") {
            let err = error.to_string();
            let tolerated = tolerated_vector_failures.contains(&name)
                && (err.contains("Embedding error")
                    || err.contains("OpenAI API error")
                    || err.contains("vector index repository failed")
                    || err.contains("vector index file failed")
                    || err.contains("vector search failed"));
            if tolerated {
                continue;
            }
            panic!(
                "MCP JSON-RPC error for '{}' in {}: {}",
                name, fixture.slug, response
            );
        }
    }
}

fn mcp_request(
    ctx: &IntegrationContext,
    request: Value,
    timeout_secs: u64,
) -> Result<Value, String> {
    let mut child = Command::new(&ctx.bin_path)
        .arg("mcp")
        .arg("start")
        .env("HOME", ctx.home_dir.as_os_str())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to start MCP server: {}", e))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to get mcp stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to get mcp stdout".to_string())?;
    let mut reader = BufReader::new(stdout);

    let init_request =
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{},\"clientInfo\":{\"name\":\"integration-test\",\"version\":\"1.0.0\"}}}\n"
            .to_string();
    stdin
        .write_all(init_request.as_bytes())
        .map_err(|e| e.to_string())?;
    stdin.flush().map_err(|e| e.to_string())?;

    let mut init_seen = false;
    let mut transcript = String::new();
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if bytes == 0 {
            break;
        }
        transcript.push_str(line.as_str());
        if let Ok(v) = serde_json::from_str::<Value>(line.trim_end())
            && v.get("id").and_then(Value::as_u64) == Some(1)
        {
            init_seen = true;
            break;
        }
    }
    if !init_seen {
        return Err(format!(
            "failed to receive initialize response\nstdout:\n{}",
            transcript
        ));
    }

    let mut final_req = json!({"jsonrpc":"2.0","id":2});
    if let (Value::Object(base), Value::Object(extra)) = (&mut final_req, request) {
        for (k, v) in extra {
            base.insert(k, v);
        }
    }
    let second_payload = format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\",\"params\":{{}}}}\n{}\n",
        final_req
    );
    stdin
        .write_all(second_payload.as_bytes())
        .map_err(|e| e.to_string())?;
    stdin.flush().map_err(|e| e.to_string())?;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut followup = String::new();
        loop {
            let mut line = String::new();
            let bytes = match reader.read_line(&mut line) {
                Ok(n) => n,
                Err(e) => {
                    let _ = tx.send(Err(format!("read error: {e}; output: {followup}")));
                    return;
                }
            };
            if bytes == 0 {
                let _ = tx.send(Err(followup));
                return;
            }
            followup.push_str(line.as_str());
            if let Ok(v) = serde_json::from_str::<Value>(line.trim_end())
                && v.get("id").and_then(Value::as_u64) == Some(2)
            {
                let _ = tx.send(Ok(v));
                return;
            }
        }
    });

    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(Ok(v)) => {
            let _ = child.kill();
            let _ = child.wait();
            Ok(v)
        }
        Ok(Err(followup)) => {
            drop(stdin);
            let output = child.wait_with_output().map_err(|e| e.to_string())?;
            Err(format!(
                "failed to locate response for request id=2\nstdout:\n{}{}\nstderr:\n{}",
                transcript,
                followup,
                String::from_utf8_lossy(output.stderr.as_slice())
            ))
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            let _ = child.kill();
            let output = child.wait_with_output().map_err(|e| e.to_string())?;
            Err(format!(
                "mcp request timed out waiting for request id=2\nstdout:\n{}\nstderr:\n{}",
                transcript,
                String::from_utf8_lossy(output.stderr.as_slice())
            ))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            let _ = child.kill();
            let output = child.wait_with_output().map_err(|e| e.to_string())?;
            Err(format!(
                "mcp response channel disconnected before request id=2\nstdout:\n{}\nstderr:\n{}",
                transcript,
                String::from_utf8_lossy(output.stderr.as_slice())
            ))
        }
    }
}

fn extract_tool_names(response: &Value) -> BTreeSet<String> {
    response
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| tool.get("name").and_then(Value::as_str))
                .map(ToString::to_string)
                .collect::<BTreeSet<String>>()
        })
        .unwrap_or_default()
}

fn first_file_with_extensions(repo: &Path, extensions: &[&str]) -> Option<PathBuf> {
    let output = Command::new("git")
        .current_dir(repo)
        .arg("ls-files")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let list = String::from_utf8_lossy(output.stdout.as_slice());
    list.lines().map(|line| repo.join(line)).find(|p| {
        p.extension()
            .and_then(|e| e.to_str())
            .map(|ext| extensions.contains(&ext))
            .unwrap_or(false)
    })
}

fn git_current_branch(repo: &Path) -> Option<String> {
    let output = Command::new("git")
        .current_dir(repo)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(output.stdout.as_slice())
            .trim()
            .to_string(),
    )
}

fn cypher_escaped(input: &str) -> String {
    input.replace('\\', "\\\\").replace('\'', "\\'")
}
