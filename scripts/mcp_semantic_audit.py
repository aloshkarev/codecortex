#!/usr/bin/env python3
"""Semantic MCP tool audit: ground-truth oracles vs live tools/call responses."""

from __future__ import annotations

import json
import os
import re
import select
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
ORACLES_PATH = Path(
    os.environ.get(
        "CORTEX_SEMANTIC_ORACLES",
        ROOT / "tests/mcp_semantic/oracles.json",
    )
)
REPO = os.environ.get("CORTEX_SEMANTIC_REPO", str(ROOT))
FIXTURE = os.environ.get(
    "CORTEX_SEMANTIC_FIXTURE",
    str(ROOT / "tests/fixtures/vector_semantic"),
)
PROFILE = os.environ.get("CORTEX_SEMANTIC_PROFILE", "pr")
CORTEX_BIN = os.environ.get("CORTEX_BIN", "cortex")
LEDGER_PATH = Path(
    os.environ.get(
        "CORTEX_SEMANTIC_LEDGER",
        ROOT / "target" / "mcp-semantic-ledger.json",
    )
)
FAILURES_DIR = Path(
    os.environ.get(
        "CORTEX_SEMANTIC_FAILURES",
        ROOT / "target" / "mcp-semantic-failures",
    )
)


def mcp_env() -> dict[str, str]:
    env = os.environ.copy()
    if os.environ.get("CORTEX_TEST_EMBEDDER"):
        env["CORTEX_TEST_EMBEDDER"] = os.environ["CORTEX_TEST_EMBEDDER"]
    if os.environ.get("CORTEX_TEST_GRAPH"):
        env["CORTEX_TEST_GRAPH"] = os.environ["CORTEX_TEST_GRAPH"]
    return env


def expand_args(obj: Any, repo: str, fixture: str | None = None) -> Any:
    if isinstance(obj, str):
        out = obj.replace("${REPO}", repo)
        if fixture is not None:
            out = out.replace("${FIXTURE}", fixture)
        return out
    if isinstance(obj, list):
        return [expand_args(x, repo, fixture) for x in obj]
    if isinstance(obj, dict):
        return {k: expand_args(v, repo, fixture) for k, v in obj.items()}
    return obj


def normalize_body(body: dict[str, Any]) -> dict[str, Any]:
    """Map legacy bare arrays and alternate field names into data/* for assertions."""
    if not isinstance(body, dict):
        return body
    if "_value" in body and isinstance(body["_value"], list):
        data = body.setdefault("data", {})
        if not isinstance(data, dict):
            data = {}
            body["data"] = data
        data.setdefault("results", body["_value"])
    data = body.get("data")
    if isinstance(data, dict):
        if "nodes" not in data and isinstance(data.get("direct_callers"), list):
            data["nodes"] = data["direct_callers"]
        if "items" not in data and isinstance(data.get("targets"), list):
            data["items"] = data["targets"]
        if "tools" not in data and isinstance(data.get("recommendations"), list):
            data["tools"] = data["recommendations"]
        results = data.get("results")
        if isinstance(results, list) and results:
            first = results[0]
            if isinstance(first, dict) and "result" in first and "metadata" not in first:
                flat = []
                for item in results:
                    if not isinstance(item, dict):
                        continue
                    inner = item.get("result") or {}
                    meta = inner.get("metadata") if isinstance(inner, dict) else None
                    flat.append(
                        {
                            "id": inner.get("id") if isinstance(inner, dict) else None,
                            "score": item.get("combined_score"),
                            "content": inner.get("content") if isinstance(inner, dict) else None,
                            "metadata": meta,
                            "graph_context": item.get("graph_context"),
                        }
                    )
                data["results"] = flat
    return body


def get_at(body: dict[str, Any], path: str) -> Any:
    cur: Any = body
    if not path:
        return cur
    for part in path.split("/"):
        if part == "":
            continue
        if isinstance(cur, dict):
            cur = cur.get(part)
        elif isinstance(cur, list) and part.isdigit():
            idx = int(part)
            cur = cur[idx] if 0 <= idx < len(cur) else None
        else:
            return None
    return cur


def field_text(item: dict[str, Any], field: str) -> str:
    """Resolve dotted field paths on a search result item."""
    cur: Any = item
    for part in field.split("/"):
        if isinstance(cur, dict):
            cur = cur.get(part)
        else:
            return ""
    if cur is None:
        return ""
    if isinstance(cur, str):
        return cur
    return str(cur)


def mcp_session_call(method: str, params: dict[str, Any], timeout: float = 120.0) -> dict[str, Any]:
    init = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "mcp-semantic-audit", "version": "1.0.0"},
        },
    }
    req = {"jsonrpc": "2.0", "id": 2, "method": method, "params": params}
    proc = subprocess.Popen(
        [CORTEX_BIN, "mcp", "start"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        env=mcp_env(),
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
                json.dumps(
                    {"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}
                )
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


def parse_tool_body(msg: dict[str, Any]) -> dict[str, Any] | None:
    if msg.get("error"):
        return {"_rpc_error": msg["error"]}
    result = msg.get("result")
    if not isinstance(result, dict):
        return {"_rpc_error": "missing result"}
    if result.get("isError"):
        return {"_is_error": True, "raw": result}
    content = result.get("content") or []
    if not content or not isinstance(content[0], dict):
        return {}
    text = content[0].get("text", "")
    if not text:
        return {}
    try:
        parsed = json.loads(text)
        return parsed if isinstance(parsed, dict) else {"_value": parsed}
    except json.JSONDecodeError:
        return {"raw": text[:500]}


def eval_assertion(body: dict[str, Any], assertion: dict[str, Any]) -> str | None:
    kind = assertion.get("type")
    if kind == "not_error":
        status = body.get("status")
        if status == "error" or body.get("_rpc_error") or body.get("_is_error"):
            return f"expected non-error, got status={status!r} rpc={body.get('_rpc_error')}"
        return None
    if kind == "status_in":
        status = body.get("status")
        allowed = assertion.get("values", [])
        if status not in allowed:
            return f"status {status!r} not in {allowed}"
        return None
    if kind == "exists":
        val = get_at(body, assertion["path"])
        if val is None:
            return f"missing path {assertion['path']}"
        return None
    if kind == "min_length":
        val = get_at(body, assertion["path"])
        if not isinstance(val, list):
            return f"path {assertion['path']}: expected array, got {type(val).__name__}"
        if len(val) < assertion.get("min", 1):
            return f"path {assertion['path']}: length {len(val)} < {assertion.get('min', 1)}"
        return None
    if kind == "max_length":
        val = get_at(body, assertion["path"])
        if val is None:
            return None
        if isinstance(val, list) and len(val) > assertion.get("max", 0):
            return f"path {assertion['path']}: length {len(val)} > {assertion.get('max', 0)}"
        return None
    if kind == "contains":
        val = get_at(body, assertion["path"])
        sub = assertion.get("substring", "")
        if not isinstance(val, str) or sub not in val:
            return f"path {assertion['path']}: {val!r} does not contain {sub!r}"
        return None
    if kind == "gte":
        val = get_at(body, assertion["path"])
        minimum = assertion.get("min", 0)
        try:
            num = float(val) if val is not None else -1
        except (TypeError, ValueError):
            return f"path {assertion['path']}: expected numeric, got {val!r}"
        if num < minimum:
            return f"path {assertion['path']}: {num} < {minimum}"
        return None
    if kind == "rank_contains":
        val = get_at(body, assertion["path"])
        if not isinstance(val, list) or not val:
            return f"path {assertion['path']}: expected non-empty results array"
        rank = assertion.get("max_rank", 0)
        if rank >= len(val):
            return f"path {assertion['path']}: rank {rank} out of range (len={len(val)})"
        item = val[rank]
        if not isinstance(item, dict):
            return f"path {assertion['path']}/{rank}: expected object"
        field = assertion.get("field", "metadata/path")
        sub = assertion.get("substring", "")
        hay = field_text(item, field)
        if not sub or sub not in hay:
            content = item.get("content", "")
            if isinstance(content, str) and sub in content:
                return None
            return (
                f"RANK_MISS: rank {rank} field {field!r}={hay!r} "
                f"does not contain {sub!r}"
            )
        return None
    if kind == "scores_descending":
        val = get_at(body, assertion["path"])
        if not isinstance(val, list) or len(val) < 2:
            return None
        scores: list[float] = []
        score_field = assertion.get("field", "score")
        for item in val:
            if isinstance(item, dict):
                raw = item.get(score_field)
                if raw is not None:
                    try:
                        scores.append(float(raw))
                    except (TypeError, ValueError):
                        pass
        for i in range(len(scores) - 1):
            if scores[i] < scores[i + 1]:
                return f"scores not descending at index {i}: {scores[i]} < {scores[i + 1]}"
        return None
    if kind == "negative_rank_absent":
        val = get_at(body, assertion["path"])
        sub = assertion.get("substring", "")
        top_k = assertion.get("top_k", 3)
        field = assertion.get("field", "metadata/path")
        if isinstance(val, list):
            for item in val[:top_k]:
                if not isinstance(item, dict):
                    continue
                hay = field_text(item, field)
                content = item.get("content", "")
                if sub and (sub in hay or (isinstance(content, str) and sub in content)):
                    return f"NEGATIVE_HIT: {sub!r} found in top-{top_k} at {hay!r}"
        return None
    if kind == "anchor_absent":
        val = get_at(body, assertion["path"])
        field = assertion.get("field", "name")
        needle = assertion.get("value", "")
        if isinstance(val, list):
            for item in val:
                if isinstance(item, dict) and item.get(field) == needle:
                    return f"anchor {needle!r} must not appear in {assertion['path']}"
        return None
    if kind == "one_of_tools":
        val = get_at(body, assertion["path"])
        tools = assertion.get("tools", [])
        if not isinstance(val, list):
            return f"path {assertion['path']}: expected tool list"
        names: set[str] = set()
        for item in val:
            if isinstance(item, str):
                names.add(item)
            elif isinstance(item, dict):
                names.add(str(item.get("name") or item.get("tool") or ""))
                meta = item.get("metadata")
                if isinstance(meta, dict):
                    names.add(str(meta.get("name") or ""))
        if not any(t in names for t in tools):
            return f"none of {tools} in suggestions {sorted(names)[:10]}"
        return None
    if kind == "cross_check_rg":
        val = get_at(body, assertion["path"])
        symbol = assertion.get("symbol", "")
        scope = assertion.get("scope", "crates/cortex-mcp")
        if isinstance(val, list) and len(val) == 0 and symbol:
            repo = assertion.get("_repo", "")
            root = Path(repo) if repo else Path(".")
            search_root = root / scope if scope else root
            if search_root.is_dir():
                proc = subprocess.run(
                    ["rg", "-l", re.escape(symbol), str(search_root)],
                    capture_output=True,
                    text=True,
                    timeout=30,
                )
                if proc.returncode == 0 and proc.stdout.strip():
                    return (
                        f"MCP empty at {assertion['path']} but rg found {symbol!r} under {scope}"
                    )
        return None
    return f"unknown assertion type {kind!r}"


def run_a2a_chain(repo: str) -> tuple[str | None, dict[str, str]]:
    """Spawn A2A session once; return task_id and tool -> status for chain oracles."""
    spawn_args = {
        "task": "mcp semantic audit A2A chain",
        "workflow": "consensus_review",
        "include_paths": ["crates/cortex-mcp"],
        "return_immediately": True,
    }
    spawn_msg = mcp_session_call(
        "tools/call",
        {"name": "cortex_a2a_spawn_session", "arguments": spawn_args},
        timeout=180.0,
    )
    body = normalize_body(parse_tool_body(spawn_msg) or {})
    if body.get("status") == "error" or body.get("_rpc_error"):
        return None, {"cortex_a2a_spawn_session": "BROKEN"}
    task_id = body.get("task_id") or get_at(body, "data/task_id")
    if not task_id:
        return None, {"cortex_a2a_spawn_session": "BROKEN"}
    results: dict[str, str] = {"cortex_a2a_spawn_session": "VERIFIED"}
    spawn_tools = body.get("suggested_next_tools") or get_at(body, "data/suggested_next_tools")
    if not spawn_tools:
        results["cortex_a2a_spawn_session"] = "BROKEN"
    chain_calls: list[tuple[str, dict[str, Any]]] = [
        ("cortex_a2a_get_task", {"task_id": task_id}),
        ("cortex_a2a_send_message", {"task_id": task_id, "message": "semantic audit ping"}),
        ("cortex_a2a_subscribe_task", {"task_id": task_id}),
        ("cortex_a2a_list_push_configs", {"task_id": task_id}),
    ]
    for tool, args in chain_calls:
        msg = mcp_session_call(
            "tools/call",
            {"name": tool, "arguments": args},
            timeout=60.0,
        )
        tb = normalize_body(parse_tool_body(msg) or {})
        if tb.get("status") == "error" or tb.get("_rpc_error"):
            results[tool] = "BROKEN"
        elif tool == "cortex_a2a_get_task":
            artifacts = tb.get("artifacts") or get_at(tb, "data/artifacts") or []
            has_protocol_meta = any(
                isinstance(a, dict)
                and (
                    (a.get("metadata") or {}).get("mcpToolId")
                    or (a.get("metadata") or {}).get("artifactKind")
                )
                for a in artifacts
            )
            has_legacy = any(
                isinstance(a, dict)
                and (
                    a.get("artifact_kind") in ("intelligence_pack", "tool_delegation", "workflow_result")
                    or a.get("artifactKind") in ("intelligence_pack", "tool_delegation", "workflow_result")
                    or a.get("mcp_tool_id")
                )
                for a in artifacts
            )
            task_meta = tb.get("metadata") or get_at(tb, "data/metadata") or {}
            has_task_meta = isinstance(task_meta, dict) and task_meta.get("extensionUri")
            results[tool] = (
                "VERIFIED"
                if (has_protocol_meta or has_legacy or has_task_meta)
                and isinstance(artifacts, list)
                else "BROKEN"
            )
        else:
            results[tool] = "VERIFIED"
    cancel_msg = mcp_session_call(
        "tools/call",
        {"name": "cortex_a2a_cancel_task", "arguments": {"task_id": task_id}},
        timeout=60.0,
    )
    tb = normalize_body(parse_tool_body(cancel_msg) or {})
    results["cortex_a2a_cancel_task"] = (
        "BROKEN" if tb.get("status") == "error" or tb.get("_rpc_error") else "VERIFIED"
    )
    return task_id, results


def run_oracle(
    oracle: dict[str, Any],
    repo: str,
    fixture: str | None = None,
    a2a_task_id: str | None = None,
) -> dict[str, Any]:
    tool = oracle["tool"]
    if oracle.get("skip") == "a2a_chain":
        if a2a_task_id is None:
            return {"tool": tool, "status": "SKIPPED", "reason": "a2a_chain"}
        args = expand_args(oracle.get("args") or {}, repo, fixture)
        if not args and tool == "cortex_a2a_get_task":
            args = {"task_id": a2a_task_id}
        elif "task_id" not in args:
            args = {**args, "task_id": a2a_task_id}
    else:
        args = expand_args(oracle.get("args") or {}, repo, fixture)
    timeout = 600.0 if tool in ("vector_index_repository", "add_code_to_graph") else 120.0
    t0 = time.time()
    try:
        msg = mcp_session_call(
            "tools/call",
            {"name": tool, "arguments": args},
            timeout=timeout,
        )
        body = normalize_body(parse_tool_body(msg) or {})
        failures: list[str] = []
        for assertion in oracle.get("assertions") or []:
            if assertion.get("type") == "cross_check_rg":
                assertion = {**assertion, "_repo": repo}
            err = eval_assertion(body, assertion)
            if err:
                failures.append(err)
        neg = oracle.get("negative_control")
        if neg and not failures:
            nargs = expand_args(neg.get("args") or {}, repo, fixture)
            nmsg = mcp_session_call(
                "tools/call",
                {"name": tool, "arguments": nargs},
                timeout=timeout,
            )
            nbody = normalize_body(parse_tool_body(nmsg) or {})
            for assertion in neg.get("assertions") or []:
                err = eval_assertion(nbody, assertion)
                if err:
                    failures.append(f"negative_control: {err}")
        status = "VERIFIED" if not failures else "BROKEN"
        entry: dict[str, Any] = {
            "tool": tool,
            "status": status,
            "duration_ms": int((time.time() - t0) * 1000),
            "failures": failures,
        }
        if failures:
            entry["body_sample"] = body
            FAILURES_DIR.mkdir(parents=True, exist_ok=True)
            (FAILURES_DIR / f"{tool}.json").write_text(
                json.dumps({"oracle": oracle, "body": body, "failures": failures}, indent=2)
                + "\n"
            )
        return entry
    except Exception as e:
        return {
            "tool": tool,
            "status": "BROKEN",
            "duration_ms": int((time.time() - t0) * 1000),
            "failures": [str(e)],
        }


def load_oracles(profile: str) -> list[dict[str, Any]]:
    doc = json.loads(ORACLES_PATH.read_text())
    out: list[dict[str, Any]] = []
    for o in doc.get("oracles", []):
        profiles = o.get("profile", ["pr", "nightly"])
        if profile in profiles:
            out.append(o)
    return out


def bootstrap_project_context(repo: str) -> None:
    """Register and select repo so project-scoped tools (find_code) succeed."""
    for tool, args in (
        ("add_project", {"path": repo, "track_branch": True}),
        ("set_current_project", {"path": repo}),
    ):
        mcp_session_call(
            "tools/call",
            {"name": tool, "arguments": args},
            timeout=60.0,
        )


def preflight_graph_fresh(repo: str) -> str | None:
    msg = mcp_session_call(
        "tools/call",
        {"name": "index_status", "arguments": {"repo_path": repo}},
        timeout=60.0,
    )
    body = parse_tool_body(msg) or {}
    graph = get_at(body, "data/freshness/graph") or get_at(body, "data/freshness/overall")
    if graph != "fresh":
        return f"FRESHNESS_BLOCK: graph freshness is {graph!r}, run cortex index --force on {repo}"
    return None


def preflight_vector_ready(repo: str, min_docs: int = 1) -> str | None:
    msg = mcp_session_call(
        "tools/call",
        {"name": "vector_index_status", "arguments": {"repo_path": repo}},
        timeout=60.0,
    )
    body = normalize_body(parse_tool_body(msg) or {})
    if body.get("status") == "error" or body.get("_rpc_error"):
        return f"VECTOR_NOT_READY: vector_index_status error for {repo}"
    docs = get_at(body, "data/repository_documents")
    if docs is None:
        docs = get_at(body, "data/total_documents")
    try:
        count = int(docs) if docs is not None else 0
    except (TypeError, ValueError):
        count = 0
    if count < min_docs:
        return (
            f"VECTOR_NOT_READY: repository_documents={count} < {min_docs} "
            f"(run cortex vector-index {repo})"
        )
    return None


def bootstrap_vector_fixture(fixture: str) -> None:
    env = mcp_env()
    env.setdefault("CORTEX_TEST_EMBEDDER", "1")
    print(f"Bootstrap fixture: index {fixture}", file=sys.stderr)
    subprocess.run(
        [CORTEX_BIN, "index", fixture, "--force"],
        check=True,
        env=env,
    )
    print(f"Bootstrap fixture: vector-index {fixture}", file=sys.stderr)
    subprocess.run(
        [CORTEX_BIN, "vector-index", fixture],
        check=True,
        env=env,
    )


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description="Semantic MCP tool audit")
    parser.add_argument(
        "--profile",
        default=PROFILE,
        choices=("pr", "nightly", "vector_pr"),
        help="Oracle profile to run",
    )
    parser.add_argument("--repo", default=REPO, help="Repository path")
    parser.add_argument("--fixture", default=FIXTURE, help="Vector fixture path")
    parser.add_argument("--skip-preflight", action="store_true")
    parser.add_argument(
        "--bootstrap-fixture",
        action="store_true",
        help="Index and vector-index fixture before vector_pr oracles",
    )
    parser.add_argument(
        "--a2a-chain",
        action="store_true",
        help="Run cortex_a2a_spawn_session once and exercise A2A oracles",
    )
    args = parser.parse_args()

    if not ORACLES_PATH.is_file():
        print(f"Missing oracles: {ORACLES_PATH}", file=sys.stderr)
        return 2

    audit_repo = args.fixture if args.profile == "vector_pr" else args.repo

    if args.bootstrap_fixture and args.profile == "vector_pr":
        bootstrap_vector_fixture(args.fixture)

    if not args.skip_preflight:
        if args.profile == "pr":
            block = preflight_graph_fresh(args.repo)
            if block:
                print(block, file=sys.stderr)
                return 2
            bootstrap_project_context(args.repo)
        if args.profile == "nightly":
            bootstrap_project_context(args.repo)
        if args.profile in ("vector_pr", "nightly"):
            block = preflight_graph_fresh(audit_repo)
            if block and args.profile == "vector_pr":
                print(block, file=sys.stderr)
                return 2
            if args.profile == "nightly" or args.profile == "vector_pr":
                vblock = preflight_vector_ready(audit_repo)
                if vblock and args.profile == "vector_pr":
                    print(vblock, file=sys.stderr)
                    return 2
                if vblock and args.profile == "nightly":
                    print(f"Warning: {vblock}", file=sys.stderr)

    fixture = args.fixture if args.profile == "vector_pr" else None
    oracles = load_oracles(args.profile)
    a2a_task_id: str | None = None
    if args.a2a_chain:
        print("Running A2A spawn chain (--a2a-chain)", file=sys.stderr)
        a2a_task_id, chain_status = run_a2a_chain(audit_repo)
        for tool, st in chain_status.items():
            print(f"  {tool}: {st} (a2a_chain)", file=sys.stderr)
        if a2a_task_id is None:
            print("A2A chain failed to obtain task_id", file=sys.stderr)
            return 2
    print(
        f"Semantic audit: {len(oracles)} oracles profile={args.profile} repo={audit_repo}",
        file=sys.stderr,
    )

    ledger: list[dict[str, Any]] = []
    broken = 0
    for oracle in oracles:
        entry = run_oracle(oracle, audit_repo, fixture, a2a_task_id=a2a_task_id)
        ledger.append(entry)
        st = entry["status"]
        if st == "BROKEN":
            broken += 1
        detail = ""
        if entry.get("failures"):
            detail = " — " + "; ".join(entry["failures"][:2])
        print(f"  {oracle['tool']}: {st}{detail}", file=sys.stderr)

    LEDGER_PATH.parent.mkdir(parents=True, exist_ok=True)
    LEDGER_PATH.write_text(
        json.dumps(
            {
                "profile": args.profile,
                "repo": audit_repo,
                "fixture": fixture,
                "oracles_run": len(oracles),
                "broken": broken,
                "tools": ledger,
            },
            indent=2,
        )
        + "\n"
    )
    print(f"\nWrote {LEDGER_PATH} ({broken} BROKEN)", file=sys.stderr)
    return 1 if broken else 0


if __name__ == "__main__":
    sys.exit(main())
