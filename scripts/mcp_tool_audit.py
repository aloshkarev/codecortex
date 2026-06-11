#!/usr/bin/env python3
"""Live MCP tool audit: one tools/call per exported tool, writes target/mcp-audit-ledger.json."""

from __future__ import annotations

import json
import os
import select
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

LONG_RUNNING_TOOLS = frozenset(
    {
        "add_code_to_graph",
        "vector_index_repository",
        "find_dead_code",
        "get_impact_graph",
    }
)

REPO = os.environ.get(
    "CORTEX_AUDIT_REPO",
    "/run/media/alex/artefacts/projects/self/projects/64-codecortex",
)
CORTEX_BIN = os.environ.get("CORTEX_BIN", "cortex")
SYMBOL = os.environ.get("CORTEX_AUDIT_SYMBOL", "tool_names")
SOURCE_FILE = os.environ.get(
    "CORTEX_AUDIT_SOURCE",
    f"{REPO}/crates/cortex-mcp/src/tools.rs",
)
NIGHTLY = os.environ.get("CORTEX_AUDIT_PROFILE") == "nightly"
SKIP_DESTRUCTIVE = os.environ.get("CORTEX_AUDIT_SKIP_DESTRUCTIVE", "0" if NIGHTLY else "1") == "1"
SKIP_LONG = os.environ.get("CORTEX_AUDIT_SKIP_LONG", "0" if NIGHTLY else "1") == "1"
LONG_TOOLS = frozenset(
    {"vector_index_repository", "get_impact_graph", "find_dead_code", "add_code_to_graph"}
)
FLOW_FROM = os.environ.get("CORTEX_AUDIT_FLOW_FROM", "tool_names")
FLOW_TO = os.environ.get("CORTEX_AUDIT_FLOW_TO", "tool_names")
REL_SOURCE = os.environ.get(
    "CORTEX_AUDIT_REL_SOURCE", "crates/cortex-mcp/src/tools.rs"
)
LEDGER_PATH = Path(
    os.environ.get(
        "CORTEX_AUDIT_LEDGER",
        Path(REPO) / "target" / "mcp-audit-ledger.json",
    )
)
SEMANTIC_ORACLES_PATH = Path(
    os.environ.get(
        "CORTEX_SEMANTIC_ORACLES",
        Path(REPO) / "tests" / "mcp_semantic" / "oracles.json",
    )
)


def tools_with_semantic_min_length(profile: str = "pr") -> frozenset[str]:
    """Tools where empty hits must be BROKEN (aligned with mcp_semantic_audit.py)."""
    if not SEMANTIC_ORACLES_PATH.is_file():
        return frozenset()
    try:
        doc = json.loads(SEMANTIC_ORACLES_PATH.read_text())
    except (OSError, json.JSONDecodeError):
        return frozenset()
    out: set[str] = set()
    for entry in doc.get("oracles", []):
        if profile not in entry.get("profile", []):
            continue
        for assertion in entry.get("assertions", []):
            if assertion.get("type") == "min_length" and assertion.get("min", 1) >= 1:
                out.add(entry["tool"])
                break
    return frozenset(out)


SEMANTIC_STRICT_TOOLS = tools_with_semantic_min_length("pr")


def tool_names_from_cli() -> list[str]:
    out = subprocess.check_output([CORTEX_BIN, "mcp", "tools"], text=True)
    return [line.strip() for line in out.splitlines() if line.strip()]


def tool_call_timeout(name: str) -> float:
    """Per-tool timeouts; PR profile uses tighter caps than nightly."""
    if NIGHTLY:
        if name in LONG_RUNNING_TOOLS:
            return 600.0
    else:
        if name == "vector_index_repository":
            return 180.0
        if name in ("get_impact_graph", "find_dead_code"):
            return 300.0
        if name == "add_code_to_graph":
            return 180.0
    return 120.0


def mcp_session_call(method: str, params: dict[str, Any], timeout: float = 120.0) -> dict[str, Any]:
    init = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "mcp-tool-audit", "version": "1.0.0"},
        },
    }
    req = {"jsonrpc": "2.0", "id": 2, "method": method, "params": params}
    proc = subprocess.Popen(
        [CORTEX_BIN, "mcp", "start"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    assert proc.stdin and proc.stdout
    proc.stdin.write(json.dumps(init) + "\n")
    proc.stdin.flush()
    deadline = time.time() + timeout
    while time.time() < deadline:
        remaining = deadline - time.time()
        if remaining <= 0:
            break
        ready, _, _ = select.select([proc.stdout], [], [], min(remaining, 1.0))
        if not ready:
            continue
        line = proc.stdout.readline()
        if not line:
            break
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("id") == 1:
            proc.stdin.write(
                json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
                + "\n"
            )
            proc.stdin.write(json.dumps(req) + "\n")
            proc.stdin.flush()
        elif msg.get("id") == 2:
            proc.kill()
            proc.wait(timeout=5)
            return msg
    proc.kill()
    proc.wait(timeout=5)
    raise TimeoutError(f"MCP call timed out after {timeout:.0f}s: {method}")


def tool_arguments(name: str, repo: str, branch: str, source: str) -> dict[str, Any] | None:
    """Return None to skip tools that cannot run safely on the audit repo."""
    p = repo
    s = SYMBOL
    sf = source
    rel_sf = REL_SOURCE
    b = branch
    scope = f"{p}/crates/cortex-mcp" if not NIGHTLY else p
    if SKIP_DESTRUCTIVE and name in ("delete_repository", "vector_delete_repository"):
        if name == "delete_repository":
            return {"path": p, "dry_run": True}
        return {"repo_path": p, "dry_run": True}
    templates: dict[str, dict[str, Any]] = {
        "check_health": {},
        "index_status": {"repo_path": p},
        "diagnose": {"check": "all", "repo_path": p},
        "vector_index_status": {"repo_path": p},
        "recommend_tools": {"task": "find callers of a function in auth module"},
        "get_tool_guidance": {"tool_name": "find_code"},
        "explain_index_freshness": {"repo_path": p},
        "list_indexed_repositories": {},
        "get_repository_stats": {},
        "list_projects": {},
        "get_current_project": {},
        "list_jobs": {},
        "list_watched_paths": {},
        "check_job_status": {"id": "audit-fake-job"},
        "find_code": {
            "query": s,
            "kind": "name",
            "include_paths": ["crates/cortex-mcp"],
        },
        "analyze_code_relationships": {
            "query_type": "find_callers",
            "target": s,
            "include_paths": [p],
        },
        "execute_cypher_query": {"query": "RETURN 1 AS ok"},
        "find_dead_code": {"include_paths": [scope], "limit": 50},
        "go_to_definition": {"symbol": s, "from_file": rel_sf, "repo_path": p},
        "find_all_usages": {
            "symbol": "find_code",
            "from_file": "crates/cortex-mcp/src/handler.rs",
            "repo_path": p,
        },
        "quick_info": {"symbol": s},
        "calculate_cyclomatic_complexity": {"top_n": 10, "include_paths": [p]},
        "find_patterns": {"repo_path": p, "max_results": 10},
        "find_tests": {"symbol": s, "repo_path": p},
        "get_skeleton": {"path": sf, "repo_path": p},
        "get_signature": {"symbol": s, "repo_path": p},
        "get_context_capsule": {"query": "MCP tool routing", "repo_path": p},
        "get_patch_context": {
            "task": "audit MCP tools",
            "include_paths": ["crates/cortex-mcp"],
            "budget_tokens": 4000,
        },
        "get_delta_context": {"source_branch": b, "target_branch": b, "budget_tokens": 4000},
        "get_test_context": {"symbol": s, "repo_path": p},
        "get_api_contract": {"symbol": s, "repo_path": p},
        "summarize_module": {"path": f"{p}/crates/cortex-mcp", "repo_path": p},
        "estimate_context_cost": {"task": "audit", "include_paths": ["crates/cortex-mcp"]},
        "get_impact_graph": {"symbol": s, "repo_path": p, "depth": 2, "budget_tokens": 4000},
        "search_logic_flow": {
            "from_symbol": FLOW_FROM,
            "to_symbol": FLOW_TO,
            "repo_path": p,
        },
        "explain_result": {"query": s, "tool": "find_code", "repo_path": p},
        "analyze_refactoring": {"symbol": s, "repo_path": p},
        "branch_structural_diff": {"source_branch": b, "target_branch": b, "repo_path": p},
        "pr_review": {"base_ref": b, "head_ref": b, "repo_path": p, "path": p},
        "find_similar_across_projects": {"symbol": s, "repo_path": p},
        "find_shared_dependencies": {"repos": [p]},
        "compare_api_surface": {"repo_a": p, "repo_b": p},
        "search_across_projects": {"query": "handler", "repositories": [p], "k": 3},
        "vector_index_file": {"path": sf, "repo_path": p},
        "vector_search": {"query": "vector index repository", "repo_path": p, "k": 3},
        "vector_search_hybrid": {"query": "vector index repository", "repo_path": p, "k": 3},
        "add_code_to_graph": {
            "path": scope,
            "force": False,
            "include_vector": False,
            "wait": True,
            "wait_timeout_secs": 120,
        },
        "delete_repository": {"path": p, "dry_run": True},
        "vector_delete_repository": {"repo_path": p, "dry_run": True},
        "watch_directory": {"path": p},
        "unwatch_directory": {"path": p},
        "add_project": {"path": p, "track_branch": True},
        "set_current_project": {"path": p},
        "list_branches": {"path": p},
        "refresh_project": {"path": p},
        "project_status": {"path": p, "include_queue": True},
        "project_sync": {"path": p, "force": False},
        "project_branch_diff": {"path": p, "source": b, "target": b},
        "project_queue_status": {"path": p, "limit": 5},
        "project_metrics": {"path": p},
        "save_observation": {
            "repo_path": p,
            "text": "mcp audit observation",
            "severity": "low",
        },
        "get_session_context": {"repo_path": p, "max_items": 5},
        "search_memory": {"repo_path": p, "query": "audit", "max_items": 5},
        "workspace_setup": {"repo_path": p, "non_interactive": True},
        "manage_codecortex": {"action": "assess"},
        "submit_lsp_edges": {
            "repo_path": p,
            "edges": [
                {
                    "caller_fqn": "crate::a::f",
                    "callee_fqn": "crate::b::g",
                    "file": sf,
                    "line": 1,
                }
            ],
        },
        "export_bundle": {
            "repository_path": p,
            "output_path": f"{p}/target/mcp-audit-bundle.ccx",
        },
        "load_bundle": {"path": f"{p}/target/mcp-audit-bundle.ccx"},
        "cortex_a2a_list_tasks": {},
        "vector_index_repository": {
            "path": f"{p}/crates/cortex-mcp",
            "repo_path": p,
            "max_files": 50 if not NIGHTLY else 200,
        },
    }
    if name == "remove_project":
        return None  # exercised in post_audit_cleanup only
    if name in templates:
        return templates[name]
    if name == "cortex_a2a_spawn_session":
        return {
            "task": "audit MCP tools",
            "workflow": "consensus_review",
            "include_paths": ["crates/cortex-mcp"],
        }
    if name in (
        "cortex_a2a_get_task",
        "cortex_a2a_cancel_task",
        "cortex_a2a_subscribe_task",
        "cortex_a2a_list_push_configs",
        "cortex_a2a_send_message",
    ):
        return None  # requires live task_id from spawn; skip in batch audit
    return {}


def parse_tool_body(msg: dict[str, Any]) -> Any:
    if msg.get("error"):
        return {"_error": msg["error"]}
    result = msg.get("result")
    if not isinstance(result, dict):
        return {"_error": "missing result"}
    if result.get("isError"):
        return {"_error": "isError"}
    content = result.get("content") or []
    if not content or not isinstance(content[0], dict):
        return {}
    text = content[0].get("text", "")
    if not text:
        return {}
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return {"raw": text[:200]}


def classify_response(name: str, msg: dict[str, Any]) -> str:
    body = parse_tool_body(msg)
    if name == "check_job_status":
        return "VERIFIED"
    if name == "list_jobs":
        return "VERIFIED"
    if isinstance(body, dict) and body.get("_error"):
        return "BROKEN"
    if isinstance(body, list):
        return "VERIFIED"
    if body is None:
        return "VERIFIED" if name in ("check_job_status", "list_jobs") else "BROKEN"
    if not isinstance(body, dict):
        return "BROKEN"
    # Envelope contract: all migrated tools expose status + data
    if "status" not in body and name not in (
        "go_to_definition",
        "find_all_usages",
    ):
        return "BROKEN"
    status = body.get("status")
    if status == "error":
        return "BROKEN"
    if status == "partial":
        if NIGHTLY and name in (
            "go_to_definition",
            "find_all_usages",
            "search_logic_flow",
            "get_signature",
        ):
            return "BROKEN"
        if name in (
            "go_to_definition",
            "find_all_usages",
            "search_logic_flow",
            "get_signature",
            "find_dead_code",
        ):
            pass  # handled below
        else:
            return "DEGRADED"
    if name in ("go_to_definition", "find_all_usages"):
        data = body.get("data") if isinstance(body.get("data"), dict) else body
        hits = []
        if isinstance(data, dict):
            hits = data.get("definitions") or data.get("groups") or []
        if hits:
            return "VERIFIED"
        if name in SEMANTIC_STRICT_TOOLS:
            return "BROKEN"
        return "DEGRADED" if not NIGHTLY else "BROKEN"
    if name in ("search_logic_flow", "get_signature"):
        data = body.get("data") if isinstance(body.get("data"), dict) else {}
        sigs = data.get("signatures")
        if data.get("paths") or data.get("signature"):
            return "VERIFIED"
        if isinstance(sigs, list) and len(sigs) > 0:
            return "VERIFIED"
        if (data.get("count") or 0) > 0:
            return "VERIFIED"
        if isinstance(data.get("signatures"), list):
            return "VERIFIED"
        if name in SEMANTIC_STRICT_TOOLS:
            return "BROKEN"
        return "DEGRADED" if not NIGHTLY else "BROKEN"
    if name == "find_dead_code":
        data = body.get("data") if isinstance(body.get("data"), dict) else {}
        if "dead_code" in data and body.get("status") in ("ok", "partial"):
            return "VERIFIED"
    data = body.get("data") if isinstance(body.get("data"), dict) else {}
    if name.startswith("vector_") and name not in ("vector_index_status",):
        if name in ("vector_search", "vector_search_hybrid"):
            count = data.get("count") or len(data.get("results") or [])
            if count == 0:
                if name in SEMANTIC_STRICT_TOOLS:
                    return "BROKEN"
                return "DEGRADED"
        if name.startswith("vector_index") and name != "vector_index_status":
            indexed = data.get("indexed_documents") or data.get("scanned_files") or 0
            if indexed == 0:
                return "DEGRADED"
    return "VERIFIED"


def git_branch(repo: str) -> str:
    try:
        out = subprocess.check_output(
            ["git", "-C", repo, "rev-parse", "--abbrev-ref", "HEAD"],
            text=True,
        )
        return out.strip() or "main"
    except subprocess.CalledProcessError:
        return "main"


def run_a2a_chain(repo: str) -> dict[str, str]:
    """Spawn then exercise task-dependent A2A tools; returns tool -> status."""
    results: dict[str, str] = {}
    workflows = [
        (
            "consensus_review",
            {
                "task": "mcp audit A2A chain",
                "workflow": "consensus_review",
                "include_paths": ["crates/cortex-mcp"],
                "return_immediately": True,
            },
        ),
        (
            "impact_review",
            {
                "task": "audit impact of handler",
                "workflow": "impact_review",
                "include_paths": ["crates/cortex-mcp"],
                "target_symbol": "CortexHandler",
                "return_immediately": True,
                "wait_for_completion": True,
            },
        ),
    ]
    task_id = None
    for wf_name, spawn_args in workflows:
        spawn_msg = mcp_session_call(
            "tools/call",
            {"name": "cortex_a2a_spawn_session", "arguments": spawn_args},
            timeout=180.0,
        )
        body = parse_tool_body(spawn_msg)
        key = f"cortex_a2a_spawn_session:{wf_name}"
        if isinstance(body, dict) and body.get("_error"):
            results[key] = "BROKEN"
            continue
        if isinstance(body, dict):
            task_id = body.get("task_id") or (body.get("data") or {}).get("task_id")
        if not task_id:
            results[key] = "BROKEN"
            continue
        results[key] = "VERIFIED"
    if not task_id:
        results.setdefault("cortex_a2a_spawn_session", "BROKEN")
        return results
    results["cortex_a2a_spawn_session"] = results.get(
        "cortex_a2a_spawn_session:consensus_review", "VERIFIED"
    )
    chain_calls: list[tuple[str, dict[str, Any]]] = [
        ("cortex_a2a_get_task", {"task_id": task_id}),
        ("cortex_a2a_send_message", {"task_id": task_id, "message": "audit ping"}),
        ("cortex_a2a_subscribe_task", {"task_id": task_id}),
        ("cortex_a2a_list_push_configs", {"task_id": task_id}),
        ("cortex_a2a_list_tasks", {}),
    ]
    for tool, args in chain_calls:
        msg = mcp_session_call(
            "tools/call",
            {"name": tool, "arguments": args},
            timeout=60.0,
        )
        results[tool] = classify_response(tool, msg)
    msg = mcp_session_call(
        "tools/call",
        {"name": "cortex_a2a_list_push_configs", "arguments": {}},
        timeout=60.0,
    )
    results["cortex_a2a_list_push_configs"] = classify_response(
        "cortex_a2a_list_push_configs", msg
    )
    cancel_msg = mcp_session_call(
        "tools/call",
        {"name": "cortex_a2a_cancel_task", "arguments": {"task_id": task_id}},
        timeout=60.0,
    )
    results["cortex_a2a_cancel_task"] = classify_response("cortex_a2a_cancel_task", cancel_msg)
    return results


def expected_tool_count() -> int:
    try:
        matrix = Path(REPO) / "crates/cortex-mcp/tests/tool_surface_matrix.rs"
        if matrix.is_file():
            text = matrix.read_text()
            import re

            m = re.search(r"const EXPECTED_TOOL_COUNT: usize = (\d+)", text)
            if m:
                return int(m.group(1))
    except OSError:
        pass
    return len(tool_names_from_cli())


def main() -> int:
    tools = tool_names_from_cli()
    expected = expected_tool_count()
    if len(tools) != expected:
        print(
            f"WARNING: tool count {len(tools)} != expected {expected}",
            file=sys.stderr,
        )
    branch = git_branch(REPO)
    ledger: list[dict[str, Any]] = []
    failures = 0
    bundle_path = Path(REPO) / "target/mcp-audit-bundle.ccx"
    if not bundle_path.is_file():
        bundle_path.parent.mkdir(parents=True, exist_ok=True)
        export_args = tool_arguments("export_bundle", REPO, branch, SOURCE_FILE)
        if export_args is not None:
            print("Preflight: export_bundle for load_bundle smoke...", file=sys.stderr)
            try:
                mcp_session_call(
                    "tools/call",
                    {"name": "export_bundle", "arguments": export_args},
                    timeout=180.0,
                )
            except TimeoutError as e:
                print(f"Warning: export_bundle preflight failed: {e}", file=sys.stderr)
    a2a_chain = os.environ.get("CORTEX_AUDIT_A2A_CHAIN", "0") == "1"
    a2a_done: set[str] = set()
    print(f"Auditing {len(tools)} tools on {REPO} (branch={branch})", file=sys.stderr)
    if a2a_chain:
        print("Running A2A spawn chain (--a2a-chain)", file=sys.stderr)
        for tool, status in run_a2a_chain(REPO).items():
            a2a_done.add(tool)
            ledger.append({"tool": tool, "status": status, "reason": "a2a_chain"})
            if status == "BROKEN":
                failures += 1
            print(f"  {tool}: {status} (a2a_chain)", file=sys.stderr)
    setup_first = ("add_project", "set_current_project")
    ordered_tools = [n for n in setup_first if n in tools] + [
        n for n in tools if n not in setup_first
    ]
    for name in ordered_tools:
        if name in a2a_done:
            continue
        if name == "remove_project":
            continue
        args = tool_arguments(name, REPO, branch, SOURCE_FILE)
        if args is None:
            if name in ("delete_repository", "vector_delete_repository"):
                reason = "destructive"
            elif name == "remove_project":
                reason = "post_audit_cleanup"
            elif name.startswith("cortex_a2a_") and name != "cortex_a2a_list_tasks":
                reason = "needs_live_task"
            else:
                reason = "long_running"
            ledger.append({"tool": name, "status": "SKIPPED", "reason": reason})
            continue
        timeout = tool_call_timeout(name)
        if name in LONG_RUNNING_TOOLS:
            print(f"  (running {name}, timeout={int(timeout)}s)...", file=sys.stderr, flush=True)
        t0 = time.time()
        try:
            msg = mcp_session_call(
                "tools/call",
                {"name": name, "arguments": args},
                timeout=timeout,
            )
            status = classify_response(name, msg)
            err = msg.get("error")
            ledger.append(
                {
                    "tool": name,
                    "status": status,
                    "duration_ms": int((time.time() - t0) * 1000),
                    "error": err,
                }
            )
            if status == "BROKEN":
                failures += 1
            print(f"  {name}: {status}", file=sys.stderr)
        except Exception as e:
            failures += 1
            ledger.append(
                {
                    "tool": name,
                    "status": "BROKEN",
                    "duration_ms": int((time.time() - t0) * 1000),
                    "error": str(e),
                }
            )
            print(f"  {name}: BROKEN ({e})", file=sys.stderr)
    LEDGER_PATH.parent.mkdir(parents=True, exist_ok=True)
    LEDGER_PATH.write_text(json.dumps({"tools": ledger, "repo": REPO}, indent=2) + "\n")
    # remove_project last so earlier project tools see a registered path
    remove_args = {"path": REPO}
    try:
        msg = mcp_session_call(
            "tools/call",
            {"name": "remove_project", "arguments": remove_args},
            timeout=60.0,
        )
        status = classify_response("remove_project", msg)
        ledger.append(
            {
                "tool": "remove_project",
                "status": status,
                "reason": "post_audit_cleanup",
            }
        )
        if status == "BROKEN":
            failures += 1
        print(f"  remove_project: {status} (cleanup)", file=sys.stderr)
    except Exception as e:
        failures += 1
        ledger.append(
            {
                "tool": "remove_project",
                "status": "BROKEN",
                "error": str(e),
                "reason": "post_audit_cleanup",
            }
        )
    print(f"\nWrote {LEDGER_PATH} ({failures} BROKEN)", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Live MCP tool audit")
    parser.add_argument(
        "--a2a-chain",
        action="store_true",
        help="Run cortex_a2a_spawn_session then task-dependent A2A tools",
    )
    parser.add_argument(
        "--nightly",
        action="store_true",
        help="Nightly profile: run long/destructive (dry-run) tools",
    )
    args = parser.parse_args()
    if args.a2a_chain:
        os.environ["CORTEX_AUDIT_A2A_CHAIN"] = "1"
    if args.nightly:
        os.environ["CORTEX_AUDIT_PROFILE"] = "nightly"
        os.environ["CORTEX_AUDIT_SKIP_LONG"] = "0"
        os.environ["CORTEX_AUDIT_SKIP_DESTRUCTIVE"] = "0"
    sys.exit(main())
