#!/usr/bin/env python3
"""Retrieval-quality eval harness: curated cases vs MCP tools or ripgrep baseline."""

from __future__ import annotations

import argparse
import json
import os
import re
import select
import subprocess
import sys
import time
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

try:
    import yaml  # type: ignore[import-untyped]
except ImportError:  # pragma: no cover
    yaml = None

ROOT = Path(__file__).resolve().parents[1]
FIXTURE_PATH = Path(
    os.environ.get("CORTEX_RETRIEVAL_FIXTURE", ROOT / "tests/retrieval/retrieval.yaml")
)
OUTPUT_PATH = Path(
    os.environ.get("CORTEX_RETRIEVAL_OUTPUT", ROOT / "target" / "retrieval-eval.json")
)
REPO = os.environ.get("CORTEX_RETRIEVAL_REPO", str(ROOT))
CORTEX_BIN = os.environ.get("CORTEX_BIN", "cortex")

STOP_WORDS = frozenset(
    {
        "a",
        "an",
        "the",
        "and",
        "or",
        "for",
        "to",
        "of",
        "in",
        "on",
        "by",
        "with",
        "that",
        "this",
        "from",
        "is",
        "are",
        "was",
        "were",
        "be",
        "all",
        "how",
        "what",
        "where",
        "when",
        "which",
        "who",
        "used",
        "using",
        "into",
        "plus",
        "versus",
        "raw",
    }
)

STRICT_FLOORS = {"exact": {"r_at_5": 0.5}}


@dataclass
class RetrievedRef:
    path: str
    symbol: str | None = None
    tokens: int = 0

    def key(self) -> str:
        if self.symbol:
            return f"{self.path}::{self.symbol}"
        return self.path


@dataclass
class CaseResult:
    case_id: str
    tier: str
    query: str
    pipeline: str
    gold: list[str]
    retrieved: list[str]
    hit_rank: int | None
    recall_at: dict[str, bool] = field(default_factory=dict)
    token_recall: dict[str, bool] = field(default_factory=dict)


def mcp_env() -> dict[str, str]:
    env = os.environ.copy()
    for key in ("CORTEX_TEST_EMBEDDER", "CORTEX_TEST_GRAPH"):
        if os.environ.get(key):
            env[key] = os.environ[key]
    return env


def _parse_scalar(raw: str) -> Any:
    text = raw.strip()
    if not text:
        return ""
    if text in ("true", "false"):
        return text == "true"
    if (text.startswith('"') and text.endswith('"')) or (
        text.startswith("'") and text.endswith("'")
    ):
        return text[1:-1]
    if re.fullmatch(r"-?\d+", text):
        return int(text)
    return text


def _indent_of(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def _collect_block(lines: list[str], start: int, indent: int) -> tuple[list[str], int]:
    block = [lines[start]]
    idx = start + 1
    while idx < len(lines):
        line = lines[idx]
        if not line.strip() or line.lstrip().startswith("#"):
            idx += 1
            continue
        if _indent_of(line) <= indent:
            break
        block.append(line)
        idx += 1
    return block, idx


def _parse_mapping_block(lines: list[str], base_indent: int) -> dict[str, Any]:
    mapping: dict[str, Any] = {}
    idx = 0
    while idx < len(lines):
        line = lines[idx]
        if not line.strip() or line.lstrip().startswith("#"):
            idx += 1
            continue
        if _indent_of(line) <= base_indent and idx > 0:
            break
        stripped = line.strip()
        if ":" not in stripped:
            idx += 1
            continue
        key, rest = stripped.split(":", 1)
        key = key.strip()
        rest = rest.strip()
        if rest:
            mapping[key] = _parse_scalar(rest)
            idx += 1
            continue
        child_lines, idx = _collect_block(lines, idx + 1, _indent_of(line))
        first_meaningful = next(
            (
                ln
                for ln in child_lines
                if ln.strip() and not ln.lstrip().startswith("#")
            ),
            "",
        )
        if first_meaningful.lstrip().startswith("- "):
            mapping[key] = _parse_list_block(child_lines, _indent_of(line))
        else:
            mapping[key] = _parse_mapping_block(child_lines, _indent_of(line))
    return mapping


def _parse_list_block(lines: list[str], base_indent: int) -> list[Any]:
    items: list[Any] = []
    idx = 0
    while idx < len(lines):
        line = lines[idx]
        if not line.strip() or line.lstrip().startswith("#"):
            idx += 1
            continue
        if _indent_of(line) <= base_indent and idx > 0:
            break
        stripped = line.lstrip()
        if not stripped.startswith("- "):
            idx += 1
            continue
        payload = stripped[2:].strip()
        scalar_item = (
            payload.startswith(('"', "'"))
            or "::" in payload
            or (":" not in payload)
        )
        if payload and not scalar_item and ":" in payload:
            key, rest = payload.split(":", 1)
            item: dict[str, Any] = {key.strip(): _parse_scalar(rest)}
            idx += 1
            while idx < len(lines):
                nxt = lines[idx]
                if not nxt.strip() or nxt.lstrip().startswith("#"):
                    idx += 1
                    continue
                if _indent_of(nxt) <= _indent_of(line):
                    break
                nstripped = nxt.strip()
                if nstripped.startswith("- "):
                    break
                if (
                    nstripped.startswith(('"', "'"))
                    or "::" in nstripped
                    or ":" not in nstripped
                ):
                    idx += 1
                    continue
                k, vrest = nstripped.split(":", 1)
                k = k.strip()
                vrest = vrest.strip()
                if vrest:
                    item[k] = _parse_scalar(vrest)
                    idx += 1
                    continue
                child_lines, idx = _collect_block(lines, idx + 1, _indent_of(nxt))
                if child_lines and child_lines[0].lstrip().startswith("- "):
                    item[k] = _parse_list_block(child_lines, _indent_of(nxt))
                else:
                    item[k] = _parse_mapping_block(child_lines, _indent_of(nxt))
            items.append(item)
            continue
        if payload == "":
            child_lines, idx = _collect_block(lines, idx + 1, _indent_of(line))
            items.append(_parse_mapping_block(child_lines, _indent_of(line)))
            continue
        items.append(_parse_scalar(payload))
        idx += 1
    return items


def _load_fixture_without_pyyaml(path: Path) -> dict[str, Any]:
    """Parse retrieval.yaml shape without external dependencies."""
    lines = path.read_text().splitlines()
    doc = _parse_mapping_block(lines, -1)
    if not isinstance(doc, dict):
        raise ValueError(f"invalid fixture root in {path}")
    return doc


def load_fixture(path: Path) -> dict[str, Any]:
    if yaml is not None:
        doc = yaml.safe_load(path.read_text())
    else:
        doc = _load_fixture_without_pyyaml(path)
    if not isinstance(doc, dict):
        raise ValueError(f"invalid fixture root in {path}")
    return doc


def resolve_anchors(value: Any, anchors: dict[str, str]) -> Any:
    if isinstance(value, str):
        out = value
        for key, repl in anchors.items():
            out = out.replace(f"${{anchors.{key}}}", repl)
        return out
    if isinstance(value, list):
        return [resolve_anchors(v, anchors) for v in value]
    if isinstance(value, dict):
        return {k: resolve_anchors(v, anchors) for k, v in value.items()}
    return value


def expand_repo(value: Any, repo: str) -> Any:
    if isinstance(value, str):
        return value.replace("${REPO}", repo)
    if isinstance(value, list):
        return [expand_repo(v, repo) for v in value]
    if isinstance(value, dict):
        return {k: expand_repo(v, repo) for k, v in value.items()}
    return value


def normalize_body(body: dict[str, Any]) -> dict[str, Any]:
    """Align MCP tool payloads with semantic-audit normalization."""
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
                            "name": inner.get("name") if isinstance(inner, dict) else None,
                            "path": inner.get("path") if isinstance(inner, dict) else None,
                        }
                    )
                data["results"] = flat
    return body


def get_at(body: dict[str, Any], path: str) -> Any:
    cur: Any = body
    for part in path.split("/"):
        if not part:
            continue
        if isinstance(cur, dict):
            cur = cur.get(part)
        elif isinstance(cur, list) and part.isdigit():
            idx = int(part)
            cur = cur[idx] if 0 <= idx < len(cur) else None
        else:
            return None
    return cur


def mcp_session_call(method: str, params: dict[str, Any], timeout: float = 120.0) -> dict[str, Any]:
    init = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "retrieval-eval", "version": "1.0.0"},
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


def bootstrap_project_context(repo: str) -> None:
    for tool, args in (
        ("add_project", {"path": repo, "track_branch": True}),
        ("set_current_project", {"path": repo}),
    ):
        mcp_session_call(
            "tools/call",
            {"name": tool, "arguments": args},
            timeout=60.0,
        )


def parse_label(label: str, repo: str) -> tuple[str, str | None]:
    label = expand_repo(label, repo).replace("\\", "/")
    repo_prefix = repo.rstrip("/") + "/"
    if label.startswith(repo_prefix):
        label = label[len(repo_prefix) :]
    label = label.lstrip("/")
    if "::" in label:
        path, symbol = label.rsplit("::", 1)
        return path, symbol
    return label, None


def normalize_path(path: str, repo: str) -> str:
    path = path.replace("\\", "/")
    repo_prefix = repo.rstrip("/") + "/"
    if path.startswith(repo_prefix):
        path = path[len(repo_prefix) :]
    return path.lstrip("/")


def estimate_tokens_for_path(repo: str, rel_path: str) -> int:
    full = Path(repo) / rel_path
    if not full.is_file():
        return 256
    try:
        text = full.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return 256
    return max(1, len(text) // 4)


def item_to_ref(item: dict[str, Any], repo: str) -> RetrievedRef | None:
    meta = item.get("metadata") if isinstance(item.get("metadata"), dict) else {}
    path = (
        item.get("path")
        or meta.get("path")
        or meta.get("file_path")
        or meta.get("relative_path")
    )
    symbol = item.get("name") or meta.get("name") or meta.get("symbol")
    if not path and isinstance(item.get("content"), str):
        m = re.search(r"([A-Za-z0-9_./-]+\.(?:rs|py|ts|tsx|js|go|java|md))", item["content"])
        if m:
            path = m.group(1)
    if not path:
        return None
    rel = normalize_path(str(path), repo)
    sym = str(symbol) if symbol else None
    return RetrievedRef(path=rel, symbol=sym, tokens=estimate_tokens_for_path(repo, rel))


def extract_refs_from_body(body: dict[str, Any], repo: str, limit: int = 20) -> list[RetrievedRef]:
    body = normalize_body(body)
    results = get_at(body, "data/results")
    if not isinstance(results, list):
        results = body.get("results")
    if not isinstance(results, list):
        return []
    refs: list[RetrievedRef] = []
    seen: set[str] = set()
    for item in results:
        if not isinstance(item, dict):
            continue
        node = item.get("n") if isinstance(item.get("n"), dict) else item
        ref = item_to_ref(node, repo)
        if ref is None:
            ref = item_to_ref(item, repo)
        if ref is None:
            continue
        key = ref.key()
        if key in seen:
            continue
        seen.add(key)
        refs.append(ref)
        if len(refs) >= limit:
            break
    return refs


def tokenize_query(query: str) -> list[str]:
    tokens = re.findall(r"[A-Za-z][A-Za-z0-9_]{2,}|[A-Za-z]{2,}", query)
    out: list[str] = []
    for tok in tokens:
        low = tok.lower()
        if low in STOP_WORDS:
            continue
        if tok not in out:
            out.append(tok)
    if not out:
        out = re.findall(r"\w+", query)
    return out[:12]


def rg_retrieve(query: str, repo: str, scope: str | None = None, limit: int = 20) -> list[RetrievedRef]:
    root = Path(repo)
    search_root = root / scope if scope else root
    if not search_root.exists():
        return []
    terms = tokenize_query(query)
    if not terms:
        return []
    scores: dict[str, float] = defaultdict(float)
    for term in terms:
        try:
            proc = subprocess.run(
                ["rg", "-l", "-i", "--no-heading", term, str(search_root)],
                capture_output=True,
                text=True,
                timeout=30,
            )
        except (FileNotFoundError, subprocess.TimeoutExpired):
            return []
        if proc.returncode not in (0, 1):
            continue
        for rank, line in enumerate(proc.stdout.splitlines()):
            if not line.strip():
                continue
            rel = normalize_path(line.strip(), repo)
            scores[rel] += 1.0 + 1.0 / (rank + 1)
    ranked = sorted(scores.items(), key=lambda kv: (-kv[1], kv[0]))
    refs: list[RetrievedRef] = []
    for rel, _ in ranked[:limit]:
        refs.append(
            RetrievedRef(
                path=rel,
                symbol=None,
                tokens=estimate_tokens_for_path(repo, rel),
            )
        )
    return refs


def label_matches_ref(label: str, ref: RetrievedRef, repo: str) -> bool:
    gold_path, gold_sym = parse_label(label, repo)
    if not path_matches(gold_path, ref.path):
        return False
    if gold_sym is None:
        return True
    if ref.symbol and ref.symbol == gold_sym:
        return True
    if ref.symbol and gold_sym in ref.symbol:
        return True
    # file-level ref can satisfy symbol label when path is exact
    return ref.symbol is None and gold_path.endswith(".rs")


def path_matches(gold_path: str, result_path: str) -> bool:
    gold_path = gold_path.replace("\\", "/").lstrip("/")
    result_path = result_path.replace("\\", "/").lstrip("/")
    if gold_path == result_path:
        return True
    return result_path.endswith(gold_path) or gold_path.endswith(result_path)


def first_hit_rank(refs: list[RetrievedRef], gold: list[str], repo: str) -> int | None:
    for idx, ref in enumerate(refs):
        for label in gold:
            if label_matches_ref(label, ref, repo):
                return idx + 1
    return None


def recall_at_k(refs: list[RetrievedRef], gold: list[str], repo: str, k: int) -> bool:
    for ref in refs[:k]:
        for label in gold:
            if label_matches_ref(label, ref, repo):
                return True
    return False


def token_budget_recall(
    refs: list[RetrievedRef], gold: list[str], repo: str, budget: int
) -> bool:
    spent = 0
    for ref in refs:
        spent += ref.tokens
        for label in gold:
            if label_matches_ref(label, ref, repo):
                return True
        if spent >= budget:
            break
    return False


def aggregate_metrics(results: list[CaseResult]) -> dict[str, Any]:
    def summarize(subset: list[CaseResult]) -> dict[str, Any]:
        if not subset:
            return {"cases": 0, "r_at_1": 0.0, "r_at_5": 0.0, "r_at_20": 0.0, "mrr": 0.0}
        n = len(subset)
        r1 = sum(1 for r in subset if r.recall_at.get("1")) / n
        r5 = sum(1 for r in subset if r.recall_at.get("5")) / n
        r20 = sum(1 for r in subset if r.recall_at.get("20")) / n
        mrr = sum((1.0 / r.hit_rank) if r.hit_rank else 0.0 for r in subset) / n
        out: dict[str, Any] = {
            "cases": n,
            "r_at_1": round(r1, 4),
            "r_at_5": round(r5, 4),
            "r_at_20": round(r20, 4),
            "mrr": round(mrr, 4),
        }
        if any(r.token_recall for r in subset):
            out["recall_at_2k"] = round(
                sum(1 for r in subset if r.token_recall.get("2k")) / n, 4
            )
            out["recall_at_10k"] = round(
                sum(1 for r in subset if r.token_recall.get("10k")) / n, 4
            )
        return out

    by_tier: dict[str, Any] = {}
    tiers = sorted({r.tier for r in results})
    for tier in tiers:
        by_tier[tier] = summarize([r for r in results if r.tier == tier])
    overall = summarize(results)
    token_eff = {
        "recall_at_2k": overall.pop("recall_at_2k", None),
        "recall_at_10k": overall.pop("recall_at_10k", None),
    }
    return {"overall": overall, "by_tier": by_tier, "token_efficiency": token_eff}


def mcp_retrieve(case: dict[str, Any], repo: str, limit: int = 20) -> list[RetrievedRef]:
    tool = case.get("tool", "vector_search_hybrid")
    args = dict(case.get("args") or {})
    args.setdefault("query", case["query"])
    if tool in ("vector_search_hybrid", "vector_search"):
        args.setdefault("repo_path", repo)
        args.setdefault("k", limit)
    msg = mcp_session_call(
        "tools/call",
        {"name": tool, "arguments": args},
        timeout=180.0 if tool.startswith("vector_") else 120.0,
    )
    body = parse_tool_body(msg) or {}
    if body.get("_rpc_error") or body.get("_is_error") or body.get("status") == "error":
        err = body.get("_rpc_error") or body.get("status") or "error"
        raise RuntimeError(f"{tool} failed: {err}")
    return extract_refs_from_body(body, repo, limit=limit)


def evaluate_case(
    case: dict[str, Any],
    repo: str,
    pipeline: str,
    token_mode: bool,
    limit: int = 20,
) -> CaseResult:
    if pipeline == "mcp":
        refs = mcp_retrieve(case, repo, limit=limit)
    else:
        scope = case.get("scope")
        refs = rg_retrieve(case["query"], repo, scope=scope, limit=limit)
    gold = [str(g) for g in case.get("gold") or []]
    hit = first_hit_rank(refs, gold, repo)
    recall = {
        "1": recall_at_k(refs, gold, repo, 1),
        "5": recall_at_k(refs, gold, repo, 5),
        "20": recall_at_k(refs, gold, repo, 20),
    }
    token_recall: dict[str, bool] = {}
    if token_mode:
        token_recall["2k"] = token_budget_recall(refs, gold, repo, 2000)
        token_recall["10k"] = token_budget_recall(refs, gold, repo, 10000)
    return CaseResult(
        case_id=str(case["id"]),
        tier=str(case.get("tier", "concept")),
        query=str(case["query"]),
        pipeline=pipeline,
        gold=gold,
        retrieved=[r.key() for r in refs],
        hit_rank=hit,
        recall_at=recall,
        token_recall=token_recall,
    )


def prepare_cases(doc: dict[str, Any], repo: str) -> list[dict[str, Any]]:
    anchors = doc.get("anchors") or {}
    if not isinstance(anchors, dict):
        anchors = {}
    raw_cases = doc.get("cases") or []
    prepared: list[dict[str, Any]] = []
    for case in raw_cases:
        if not isinstance(case, dict):
            continue
        resolved = resolve_anchors(case, {str(k): str(v) for k, v in anchors.items()})
        prepared.append(expand_repo(resolved, repo))
    return prepared


def check_strict(metrics: dict[str, Any]) -> list[str]:
    violations: list[str] = []
    by_tier = metrics.get("by_tier") or {}
    for tier, floors in STRICT_FLOORS.items():
        tier_metrics = by_tier.get(tier) or {}
        for metric, floor in floors.items():
            value = tier_metrics.get(metric)
            if value is None:
                violations.append(f"{tier}.{metric}: no value recorded")
            elif value < floor:
                violations.append(f"{tier}.{metric}={value} < floor {floor}")
    return violations


def main() -> int:
    parser = argparse.ArgumentParser(description="CodeCortex retrieval-quality eval")
    parser.add_argument("--fixture", default=str(FIXTURE_PATH), help="YAML fixture path")
    parser.add_argument("--repo", default=REPO, help="Repository under test")
    parser.add_argument("--output", default=str(OUTPUT_PATH), help="JSON report path")
    parser.add_argument(
        "--lexical-only",
        action="store_true",
        help="Ripgrep baseline only (no MCP); works offline",
    )
    parser.add_argument(
        "--pipeline",
        choices=("mcp", "rg", "auto"),
        default="auto",
        help="Retrieval pipeline (default: mcp unless --lexical-only)",
    )
    parser.add_argument(
        "--token-efficiency",
        action="store_true",
        help="Also compute recall@2k and recall@10k token budgets",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Exit non-zero if tier floors are violated (exact R@5 >= 0.5)",
    )
    parser.add_argument("--limit", type=int, default=20, help="Max results per query")
    parser.add_argument("--skip-bootstrap", action="store_true")
    args = parser.parse_args()

    fixture_path = Path(args.fixture)
    if not fixture_path.is_file():
        print(f"Missing fixture: {fixture_path}", file=sys.stderr)
        return 2

    doc = load_fixture(fixture_path)
    cases = prepare_cases(doc, args.repo)
    if not cases:
        print("No cases loaded", file=sys.stderr)
        return 2

    if args.lexical_only:
        pipeline = "rg"
    elif args.pipeline == "auto":
        pipeline = "mcp"
    else:
        pipeline = args.pipeline

    if pipeline == "mcp" and not args.skip_bootstrap:
        try:
            bootstrap_project_context(args.repo)
        except Exception as exc:
            print(f"bootstrap failed: {exc}", file=sys.stderr)
            print("falling back to ripgrep baseline (--lexical-only)", file=sys.stderr)
            pipeline = "rg"

    print(
        f"Retrieval eval: {len(cases)} cases pipeline={pipeline} repo={args.repo}",
        file=sys.stderr,
    )

    results: list[CaseResult] = []
    failures = 0
    for case in cases:
        try:
            result = evaluate_case(
                case,
                args.repo,
                pipeline=pipeline,
                token_mode=args.token_efficiency,
                limit=args.limit,
            )
        except Exception as exc:
            failures += 1
            result = CaseResult(
                case_id=str(case.get("id", "?")),
                tier=str(case.get("tier", "concept")),
                query=str(case.get("query", "")),
                pipeline=pipeline,
                gold=[str(g) for g in case.get("gold") or []],
                retrieved=[],
                hit_rank=None,
                recall_at={"1": False, "5": False, "20": False},
            )
            print(f"  {result.case_id}: ERROR — {exc}", file=sys.stderr)
        else:
            rank = result.hit_rank if result.hit_rank else "-"
            print(
                f"  {result.case_id} [{result.tier}]: "
                f"R@1={int(result.recall_at['1'])} R@5={int(result.recall_at['5'])} "
                f"MRR_rank={rank}",
                file=sys.stderr,
            )
        results.append(result)

    metrics = aggregate_metrics(results)
    report = {
        "version": 1,
        "fixture": str(fixture_path),
        "repo": args.repo,
        "pipeline": pipeline,
        "cases_run": len(cases),
        "case_errors": failures,
        "metrics": metrics,
        "strict_floors": STRICT_FLOORS,
        "cases": [
            {
                "id": r.case_id,
                "tier": r.tier,
                "query": r.query,
                "gold": r.gold,
                "hit_rank": r.hit_rank,
                "recall_at": r.recall_at,
                "token_recall": r.token_recall,
                "retrieved_top5": r.retrieved[:5],
            }
            for r in results
        ],
    }

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(report, indent=2) + "\n")
    print(f"\nWrote {out_path}", file=sys.stderr)
    print(
        f"Overall R@1={metrics['overall']['r_at_1']} "
        f"R@5={metrics['overall']['r_at_5']} "
        f"R@20={metrics['overall']['r_at_20']} "
        f"MRR={metrics['overall']['mrr']}",
        file=sys.stderr,
    )

    if args.strict:
        violations = check_strict(metrics)
        if violations:
            for v in violations:
                print(f"STRICT FAIL: {v}", file=sys.stderr)
            return 1

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
