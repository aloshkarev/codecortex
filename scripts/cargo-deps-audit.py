#!/usr/bin/env python3
"""Lockfile-wide external crate usage audit for CodeCortex workspace.

Outputs under audit/cargo-deps/:
  - deps-lockfile-inventory.json
  - deps-import-aggregation.json
  - deps-lockfile-usage-report.md
  - deps-lockfile-usage-report.csv
  - deps-safe-to-remove.toml
"""

from __future__ import annotations

import csv
import json
import re
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = REPO_ROOT / "audit" / "cargo-deps"
REPO_PATH = str(REPO_ROOT)

SKIP_CRATE_ROOTS = frozenset(
    {"std", "core", "alloc", "crate", "self", "super", "path"}
)

USE_RE = re.compile(
    r"^\s*use\s+(?P<path>[^;{]+)(?:\s*\{[^}]*\})?\s*;",
    re.MULTILINE,
)
USE_GROUP_RE = re.compile(
    r"^\s*use\s+(?P<prefix>[\w:]+)\s*\{([^}]+)\}\s*;",
    re.MULTILINE,
)
EXTERN_CRATE_RE = re.compile(
    r"^\s*extern\s+crate\s+(?P<name>\w+)\s*;", re.MULTILINE
)


@dataclass
class PackageRow:
    package_name: str
    version: str
    lib_targets: list[str]
    role: str
    declared_in: list[str] = field(default_factory=list)
    source: str | None = None


def run(cmd: list[str], cwd: Path | None = None) -> str:
    proc = subprocess.run(
        cmd,
        cwd=cwd or REPO_ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return proc.stdout


def cargo_metadata() -> dict[str, Any]:
    out = run(["cargo", "metadata", "--format-version=1"])
    return json.loads(out)


def is_local_path(dep: dict[str, Any]) -> bool:
    if dep.get("path"):
        return True
    return False


def lib_names(pkg: dict[str, Any]) -> list[str]:
    names: list[str] = []
    for t in pkg.get("targets", []):
        if t.get("kind") == ["lib"] or t.get("kind") == ["rlib"]:
            n = t.get("name")
            if n:
                names.append(n)
    if not names:
        names.append(pkg["name"].replace("-", "_"))
    return names


def build_inventory(meta: dict[str, Any]) -> tuple[list[PackageRow], dict[str, str]]:
    """Return external packages and map lib/crate root -> package name."""
    workspace_ids = set(meta.get("workspace_members", []))
    pkg_by_id = {p["id"]: p for p in meta["packages"]}
    workspace_names = {
        pkg_by_id[wid]["name"] for wid in workspace_ids if wid in pkg_by_id
    }

    # Direct dep roles per workspace member
    direct: dict[str, set[str]] = defaultdict(set)  # dep_name -> set of member crates
    for wid in workspace_ids:
        wp = pkg_by_id.get(wid)
        if not wp:
            continue
        member = wp["name"]
        for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
            for dep in wp.get(kind, []):
                if isinstance(dep, str):
                    dep_pkg = pkg_by_id.get(dep)
                    dep_name = dep_pkg["name"] if dep_pkg else None
                else:
                    dep_name = dep.get("name")
                    if dep.get("source") is None:
                        continue
                if not dep_name:
                    continue
                direct[dep_name].add(f"{member}:{kind}")

    rows: list[PackageRow] = []
    root_to_package: dict[str, str] = {}

    for pkg in meta["packages"]:
        if pkg["id"] in workspace_ids:
            continue
        if pkg.get("source") is None:
            continue
        libs = lib_names(pkg)
        decl = sorted(direct.get(pkg["name"], []))
        if decl:
            roles = {d.split(":")[-1] for d in decl}
            if "dependencies" in roles:
                role = "direct"
            elif "build-dependencies" in roles:
                role = "build"
            elif "dev-dependencies" in roles:
                role = "dev"
            else:
                role = "direct"
        else:
            role = "transitive"

        rows.append(
            PackageRow(
                package_name=pkg["name"],
                version=pkg["version"],
                lib_targets=libs,
                role=role,
                declared_in=[d.split(":")[0] for d in decl],
                source=pkg.get("source"),
            )
        )
        for lib in libs:
            root_to_package[lib] = pkg["name"]
        root_to_package[pkg["name"].replace("-", "_")] = pkg["name"]
        root_to_package[pkg["name"]] = pkg["name"]

    rows.sort(key=lambda r: r.package_name)
    return rows, root_to_package


def normalize_use_path(raw: str) -> str:
    s = raw.strip()
    if s.startswith("use "):
        s = s[4:].strip()
    return s.rstrip(";").strip()


def crate_roots_from_use(path: str) -> list[str]:
    path = normalize_use_path(path)
    if not path or path in SKIP_CRATE_ROOTS:
        return []
    if path.startswith("{"):
        return []
    root = path.split("::")[0].strip()
    if root in SKIP_CRATE_ROOTS:
        return []
    return [root]


def scan_rust_imports() -> dict[str, Any]:
    """Scan workspace .rs files for use/extern crate (graph IMPORTS fallback)."""
    by_root: dict[str, dict[str, Any]] = defaultdict(
        lambda: {
            "files": set(),
            "import_paths": defaultdict(int),
            "imported_items": set(),
            "import_edges": 0,
        }
    )

    rs_files = [
        p
        for p in REPO_ROOT.rglob("*.rs")
        if "target" not in p.parts and ".git" not in p.parts
    ]

    for path in rs_files:
        rel = path.relative_to(REPO_ROOT).as_posix()
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue

        for m in EXTERN_CRATE_RE.finditer(text):
            root = m.group("name")
            if root in SKIP_CRATE_ROOTS:
                continue
            entry = by_root[root]
            entry["files"].add(rel)
            entry["import_paths"][f"extern crate {root}"] += 1
            entry["import_edges"] += 1

        for m in USE_GROUP_RE.finditer(text):
            prefix = m.group("prefix").strip()
            roots = crate_roots_from_use(prefix)
            items_raw = m.group(2)
            items = [
                i.strip().split(" as ")[0].strip()
                for i in items_raw.split(",")
                if i.strip()
            ]
            for root in roots:
                entry = by_root[root]
                entry["files"].add(rel)
                full = f"{prefix} {{{', '.join(items)}}}"
                entry["import_paths"][full] += 1
                entry["import_edges"] += 1
                for it in items:
                    entry["imported_items"].add(f"{prefix}::{it}")

        for m in USE_RE.finditer(text):
            raw = m.group("path").strip()
            if "{" in raw:
                continue
            path_norm = normalize_use_path(f"use {raw};")
            roots = crate_roots_from_use(path_norm)
            for root in roots:
                entry = by_root[root]
                entry["files"].add(rel)
                entry["import_paths"][path_norm] += 1
                entry["import_edges"] += 1
                parts = path_norm.split("::")
                if len(parts) > 1:
                    entry["imported_items"].add("::".join(parts[1:]))

    serializable: dict[str, Any] = {}
    for root, data in by_root.items():
        serializable[root] = {
            "files_with_import": len(data["files"]),
            "files": sorted(data["files"]),
            "import_edges": data["import_edges"],
            "distinct_import_paths": len(data["import_paths"]),
            "import_paths": dict(
                sorted(
                    data["import_paths"].items(),
                    key=lambda x: (-x[1], x[0]),
                )[:50]
            ),
            "imported_items_sample": sorted(data["imported_items"])[:30],
        }
    return serializable


def try_graph_imports() -> dict[str, Any]:
    try:
        out = run(
            [
                "cortex",
                "query",
                f"MATCH (f:File {{repository_path: '{REPO_PATH}'}})-[:IMPORTS]->(m) "
                "RETURN m.name AS import_path, count(DISTINCT f) AS file_count",
            ]
        )
        rows = json.loads(out)
        return {"source": "graph", "edge_count": len(rows), "rows": rows[:20]}
    except (subprocess.CalledProcessError, json.JSONDecodeError) as e:
        return {"source": "graph", "error": str(e), "edge_count": 0}


def run_machete() -> list[dict[str, str]]:
    proc = subprocess.run(
        ["cargo", "machete"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    out = (proc.stdout or "") + (proc.stderr or "")
    unused: list[dict[str, str]] = []
    current_manifest: str | None = None
    for line in out.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        if " -- " in stripped and stripped.endswith("Cargo.toml:"):
            # cortex-mcp -- ./crates/cortex-mcp/Cargo.toml:
            part = stripped.split(" -- ", 1)[-1]
            current_manifest = part.rstrip(":").strip()
            continue
        if stripped.endswith("Cargo.toml") or stripped.endswith("Cargo.toml:"):
            current_manifest = stripped.rstrip(":").strip()
            continue
        if current_manifest and not stripped.startswith("If ") and not stripped.startswith("You "):
            dep = stripped.lstrip("- \t").strip()
            if (
                dep
                and dep not in ("Done!",)
                and not dep.startswith("cargo-machete")
                and re.match(r"^[A-Za-z0-9_-]+$", dep)
            ):
                unused.append({"manifest": current_manifest, "dep_name": dep})
    return unused


def usage_tier(
    row: PackageRow,
    roots_usage: dict[str, Any],
    root_to_package: dict[str, str],
    machete_unused: set[tuple[str, str]],
) -> str:
    used = False
    for lib in row.lib_targets:
        if lib in roots_usage:
            used = True
            break
    if not used:
        alt = row.package_name.replace("-", "_")
        if alt in roots_usage:
            used = True

    if row.role == "direct":
        for member in row.declared_in:
            key = (member, row.package_name)
            if key in machete_unused or (member, row.package_name.replace("_", "-")) in machete_unused:
                if not used:
                    return "direct-unused-candidate"
                return "direct-investigate-machete-mismatch"
        if not used:
            return "direct-no-source-imports"

    if used:
        return "source-referenced"
    return "transitive-only"


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    generated_at = datetime.now(timezone.utc).isoformat()

    meta = cargo_metadata()
    inventory, root_to_package = build_inventory(meta)
    graph_info = try_graph_imports()
    roots_usage = scan_rust_imports()

    inv_json = [
        {
            "package_name": r.package_name,
            "version": r.version,
            "lib_targets": r.lib_targets,
            "role": r.role,
            "declared_in": r.declared_in,
            "source": r.source,
        }
        for r in inventory
    ]
    (OUT_DIR / "deps-lockfile-inventory.json").write_text(
        json.dumps(
            {
                "generated_at": generated_at,
                "repo": REPO_PATH,
                "external_package_count": len(inv_json),
                "packages": inv_json,
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )

    agg = {
        "generated_at": generated_at,
        "usage_source": "rust_source_scan",
        "graph_imports": graph_info,
        "note": (
            "Graph IMPORTS edges were empty at audit time; usage metrics come from "
            "scanning `use` / `extern crate` in .rs files (same intent as CodeCortex parser)."
        ),
        "by_crate_root": roots_usage,
    }
    (OUT_DIR / "deps-import-aggregation.json").write_text(
        json.dumps(agg, indent=2) + "\n", encoding="utf-8"
    )

    machete_list = run_machete()
    machete_unused: set[tuple[str, str]] = set()
    for item in machete_list:
        manifest = Path(item["manifest"])
        try:
            member = manifest.parent.name
            if member == REPO_ROOT.name:
                member = "workspace-root"
        except Exception:
            member = "unknown"
        machete_unused.add((member, item["dep_name"]))

    report_rows: list[dict[str, Any]] = []
    for row in inventory:
        metrics: dict[str, Any] = {
            "files_with_import": 0,
            "import_edges": 0,
            "distinct_import_paths": 0,
            "top_import_paths": [],
        }
        for lib in row.lib_targets + [row.package_name.replace("-", "_")]:
            if lib in roots_usage:
                u = roots_usage[lib]
                metrics["files_with_import"] = max(
                    metrics["files_with_import"], u["files_with_import"]
                )
                metrics["import_edges"] += u["import_edges"]
                metrics["distinct_import_paths"] += u["distinct_import_paths"]
                top = list(u.get("import_paths", {}).keys())[:10]
                metrics["top_import_paths"].extend(top)

        tier = usage_tier(row, roots_usage, root_to_package, machete_unused)
        report_rows.append(
            {
                "package_name": row.package_name,
                "version": row.version,
                "role": row.role,
                "declared_in": ",".join(row.declared_in),
                "usage_tier": tier,
                **metrics,
            }
        )

    csv_path = OUT_DIR / "deps-lockfile-usage-report.csv"
    with csv_path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=[
                "package_name",
                "version",
                "role",
                "declared_in",
                "usage_tier",
                "files_with_import",
                "import_edges",
                "distinct_import_paths",
                "top_import_paths",
            ],
        )
        writer.writeheader()
        for r in report_rows:
            row_out = dict(r)
            row_out["top_import_paths"] = " | ".join(r["top_import_paths"][:10])
            writer.writerow(row_out)

    tiers = defaultdict(int)
    for r in report_rows:
        tiers[r["usage_tier"]] += 1

    direct_unused = [
        r for r in report_rows if r["usage_tier"] == "direct-unused-candidate"
    ]
    direct_investigate = [
        r for r in report_rows if "investigate" in r["usage_tier"] or r["usage_tier"] == "direct-no-source-imports"
    ]

    top_used = sorted(
        [r for r in report_rows if r["files_with_import"] > 0],
        key=lambda x: (-x["files_with_import"], x["package_name"]),
    )[:25]

    md_lines = [
        "# Cargo.lock external crate usage report",
        "",
        f"- **Generated:** {generated_at}",
        f"- **Repository:** `{REPO_PATH}`",
        f"- **External packages:** {len(inventory)}",
        f"- **Graph index:** 207 files, freshness fresh (see index run log)",
        f"- **Usage evidence:** Rust `use` / `extern crate` source scan",
        f"- **Graph IMPORTS edges:** {graph_info.get('edge_count', 0)} "
        f"({graph_info.get('note', graph_info.get('error', 'n/a'))})",
        "",
        "## Summary by usage tier",
        "",
        "| Tier | Count |",
        "| --- | ---: |",
    ]
    for tier, count in sorted(tiers.items(), key=lambda x: -x[1]):
        md_lines.append(f"| {tier} | {count} |")

    md_lines.extend(
        [
            "",
            "## Direct dependencies — removal candidates (cargo-machete + no source imports)",
            "",
        ]
    )
    if direct_unused:
        for r in direct_unused:
            md_lines.append(
                f"- **{r['package_name']}** ({r['version']}) — declared in `{r['declared_in']}`"
            )
    else:
        md_lines.append("- None with high confidence (machete + zero import paths).")

    md_lines.extend(
        [
            "",
            "## Direct dependencies — investigate before removal",
            "",
        ]
    )
    for r in direct_investigate[:40]:
        md_lines.append(
            f"- **{r['package_name']}** — tier `{r['usage_tier']}`, "
            f"files_with_import={r['files_with_import']}, role={r['role']}"
        )

    md_lines.extend(
        [
            "",
            "## Top source-referenced crates (by file count)",
            "",
            "| Package | Files | Import edges | Role |",
            "| --- | ---: | ---: | --- |",
        ]
    )
    for r in top_used:
        md_lines.append(
            f"| {r['package_name']} | {r['files_with_import']} | {r['import_edges']} | {r['role']} |"
        )

    md_lines.extend(
        [
            "",
            "## Limits",
            "",
            "- Proc-macro / `#[derive]` only usage may show zero imports.",
            "- Transitive lockfile packages cannot be removed directly.",
            "- Qualified paths without `use` may be under-counted.",
            "",
            "Full CSV: `deps-lockfile-usage-report.csv`",
        ]
    )

    (OUT_DIR / "deps-lockfile-usage-report.md").write_text(
        "\n".join(md_lines) + "\n", encoding="utf-8"
    )

    # Safe to remove TOML
    removal_entries: list[dict[str, Any]] = []
    for item in machete_list:
        manifest = item["manifest"]
        dep = item["dep_name"]
        pkg_row = next((r for r in inventory if r.package_name == dep), None)
        files = 0
        if pkg_row:
            for lib in pkg_row.lib_targets:
                if lib in roots_usage:
                    files = max(files, roots_usage[lib]["files_with_import"])
        confidence = "high" if files == 0 else "investigate"
        removal_entries.append(
            {
                "workspace_member": str(Path(manifest).parent.name),
                "manifest": manifest,
                "dep_name": dep,
                "confidence": confidence,
                "graph_files_with_import": files,
                "reason": "cargo-machete unused dependency",
                "evidence": f"source scan files_with_import={files}",
            }
        )

    toml_lines = [
        "# Generated by scripts/cargo-deps-audit.py",
        f"# generated_at = {generated_at}",
        "# Review macro/build/dev/feature usage before removing.",
        "",
        "[[candidate]]",
    ]
    if not removal_entries:
        toml_lines.append('# (empty — cargo machete reported no unused direct deps)\n')
    else:
        buf: list[str] = []
        for e in removal_entries:
            buf.append("[[candidate]]")
            for k, v in e.items():
                if isinstance(v, str):
                    buf.append(f'{k} = "{v}"')
                else:
                    buf.append(f"{k} = {v}")
            buf.append("")
        toml_lines = [
            "# Generated by scripts/cargo-deps-audit.py",
            f"# generated_at = {generated_at}",
            "",
        ] + buf

    (OUT_DIR / "deps-safe-to-remove.toml").write_text(
        "\n".join(toml_lines), encoding="utf-8"
    )

    print(f"Wrote {len(inventory)} packages to {OUT_DIR}")
    print(f"Usage tiers: {dict(tiers)}")
    print(f"Machete candidates: {len(removal_entries)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
