#!/usr/bin/env python3
"""Refresh optional live MCP snapshots alongside TWAG golden oracles."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "crates/cortex-mcp/tests/fixtures/twag_goldens"
TWAG = Path(os.environ.get("CORTEX_TWAG_REPO", "/run/media/alex/artefacts/projects/work/twag"))
RD = TWAG / "third_party/tngf_cp/rdiameter/crates/rdiameter-core"
CORTEX_BIN = os.environ.get("CORTEX_BIN", "cortex")


def mcp_session(calls: list[tuple[str, dict]]) -> list[dict]:
    init = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "twag-golden-refresh", "version": "1"},
        },
    }
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
    results: list[dict] = []
    next_id = 2
    saw_init = False
    for tool, args in calls:
        req = {
            "jsonrpc": "2.0",
            "id": next_id,
            "method": "tools/call",
            "params": {"name": tool, "arguments": args},
        }
        next_id += 1
        if not saw_init:
            for line in proc.stdout:
                msg = json.loads(line)
                if msg.get("id") == 1:
                    saw_init = True
                    proc.stdin.write(
                        json.dumps(
                            {
                                "jsonrpc": "2.0",
                                "method": "notifications/initialized",
                                "params": {},
                            }
                        )
                        + "\n"
                    )
                    proc.stdin.write(json.dumps(req) + "\n")
                    proc.stdin.flush()
                    break
        else:
            proc.stdin.write(json.dumps(req) + "\n")
            proc.stdin.flush()
        for line in proc.stdout:
            msg = json.loads(line)
            if msg.get("id") == next_id - 1:
                text = msg["result"]["content"][0]["text"]
                results.append(json.loads(text))
                break
    proc.kill()
    proc.wait()
    return results


def main() -> int:
    if os.environ.get("CORTEX_TEST_TWAG") != "1":
        print("Set CORTEX_TEST_TWAG=1 to refresh live snapshots", file=sys.stderr)
        return 1
    manifest = json.loads((FIXTURES / "manifest.json").read_text())
    for case_id in manifest["cases"]:
        case = json.loads((FIXTURES / f"{case_id}.json").read_text())
        setup = case.get("setup", {})
        calls: list[tuple[str, dict]] = []
        if project := setup.get("set_current_project"):
            calls.append(("set_current_project", {"path": project}))
        calls.append((case["tool"], case["args"]))
        live = mcp_session(calls)[-1]
        out = FIXTURES / f"{case_id}.live.json"
        out.write_text(json.dumps(live, indent=2) + "\n")
        print("wrote", out.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
