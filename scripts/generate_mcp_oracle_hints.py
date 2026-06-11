#!/usr/bin/env python3
"""Suggest oracle anchors from a live MCP session (find_code / relationship probes)."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from mcp_semantic_audit import expand_args, mcp_session_call, normalize_body, parse_tool_body  # noqa: E402


def probe(tool: str, args: dict[str, object], repo: str) -> dict | None:
    msg = mcp_session_call(
        "tools/call",
        {"name": tool, "arguments": expand_args(args, repo)},
        timeout=120.0,
    )
    body = normalize_body(parse_tool_body(msg) or {})
    if body.get("_rpc_error") or body.get("status") == "error":
        return None
    return body


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate MCP oracle anchor hints")
    parser.add_argument("--repo", default=os.environ.get("CORTEX_SEMANTIC_REPO", str(ROOT)))
    parser.add_argument(
        "--cortex-bin",
        default=os.environ.get("CORTEX_BIN", str(ROOT / "target/release/cortex-cli")),
    )
    args = parser.parse_args()
    os.environ["CORTEX_BIN"] = args.cortex_bin

    repo = args.repo
    hints: dict[str, object] = {"repo": repo, "symbols": {}}

    find = probe("find_code", {"query": "tool_names", "kind": "name"}, repo)
    if find:
        results = (find.get("data") or {}).get("results") or find.get("_value") or []
        if isinstance(results, list) and results:
            hints["symbols"]["tool_names"] = results[0]

    rel = probe(
        "analyze_code_relationships",
        {
            "query_type": "find_complexity",
            "target": "tool_names",
            "include_paths": ["crates/cortex-mcp"],
        },
        repo,
    )
    if rel:
        hints["relationship_sample"] = (rel.get("data") or {}).get("results", [])

    flow = probe(
        "search_logic_flow",
        {"from_symbol": "tool_names", "to_symbol": "tool_names", "repo_path": repo},
        repo,
    )
    if flow:
        hints["logic_flow_self_reference"] = (flow.get("data") or {}).get("paths", [])

    out = ROOT / "target/mcp-oracle-hints.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(hints, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
