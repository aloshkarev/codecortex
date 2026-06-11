#!/usr/bin/env python3
"""Compare Criterion benchmark estimates against perf_budget.json (--strict exits non-zero)."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BUDGET = ROOT / "crates/cortex-benches/perf_budget.json"
DEFAULT_CRITERION = ROOT / "target/criterion"


def load_budgets(path: Path) -> dict[str, int]:
    data = json.loads(path.read_text())
    return {str(k): int(v) for k, v in data.get("budgets", {}).items()}


def criterion_key(bench_dir: Path) -> str:
    """Map target/criterion/<group>/<id>/ to budget key."""
    parts = bench_dir.parts
    try:
        idx = parts.index("criterion")
    except ValueError:
        return bench_dir.name
    tail = parts[idx + 1 :]
    if len(tail) >= 2:
        return f"{tail[0]}/{tail[1]}"
    if len(tail) == 1:
        return tail[0]
    return bench_dir.name


def read_p95_ns(estimates_path: Path) -> int | None:
    if not estimates_path.is_file():
        return None
    data = json.loads(estimates_path.read_text())
    median = data.get("median", {})
    point = median.get("point_estimate")
    if point is None:
        return None
    return int(point)


def collect_measurements(criterion_root: Path) -> dict[str, int]:
    out: dict[str, int] = {}
    if not criterion_root.is_dir():
        return out
    for estimates in criterion_root.rglob("estimates.json"):
        bench_dir = estimates.parent
        key = criterion_key(bench_dir)
        p95_path = bench_dir / "sample.json"
        p95 = read_p95_ns(estimates)
        if p95 is not None:
            out[key] = p95
            continue
        if p95_path.is_file():
            sample = json.loads(p95_path.read_text())
            times = sample.get("times", [])
            if times:
                sorted_times = sorted(times)
                idx = int(len(sorted_times) * 0.95) - 1
                idx = max(0, min(idx, len(sorted_times) - 1))
                out[key] = int(sorted_times[idx])
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description="Enforce Criterion perf budgets")
    parser.add_argument(
        "--budget",
        type=Path,
        default=Path(os.environ.get("CORTEX_PERF_BUDGET", DEFAULT_BUDGET)),
    )
    parser.add_argument(
        "--criterion-dir",
        type=Path,
        default=Path(os.environ.get("CORTEX_CRITERION_DIR", DEFAULT_CRITERION)),
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Exit non-zero when any budget is exceeded or missing measurement",
    )
    parser.add_argument("--json", action="store_true", help="Emit machine-readable report")
    args = parser.parse_args()

    if not args.budget.is_file():
        print(f"perf budget file missing: {args.budget}", file=sys.stderr)
        return 1 if args.strict else 0

    budgets = load_budgets(args.budget)
    measurements = collect_measurements(args.criterion_dir)

    violations: list[dict[str, object]] = []
    missing: list[str] = []
    rows: list[dict[str, object]] = []

    for key, budget_ns in budgets.items():
        measured = measurements.get(key)
        if measured is None:
            alt = key.split("/")[-1]
            measured = measurements.get(alt)
        if measured is None:
            missing.append(key)
            rows.append({"key": key, "budget_ns": budget_ns, "measured_ns": None, "ok": False})
            continue
        ok = measured <= budget_ns
        row = {
            "key": key,
            "budget_ns": budget_ns,
            "measured_ns": measured,
            "ok": ok,
            "ratio": round(measured / budget_ns, 3) if budget_ns else None,
        }
        rows.append(row)
        if not ok:
            violations.append(row)

    report = {
        "budget_file": str(args.budget),
        "criterion_dir": str(args.criterion_dir),
        "rows": rows,
        "violations": violations,
        "missing": missing,
    }

    if args.json:
        print(json.dumps(report, indent=2))
    else:
        print("Performance budget check")
        print("=" * 60)
        for row in rows:
            status = "OK" if row["ok"] else "FAIL"
            measured = row["measured_ns"]
            measured_s = f"{measured:,}ns" if measured is not None else "MISSING"
            print(
                f"  [{status}] {row['key']}: {measured_s} "
                f"(budget {row['budget_ns']:,}ns)"
            )
        if missing:
            print(f"\nMissing measurements ({len(missing)}): {', '.join(missing)}")
        if violations:
            print(f"\nBudget violations: {len(violations)}")

    if args.strict and (violations or missing):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
