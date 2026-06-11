# Cargo.lock external crate usage report

- **Generated:** 2026-05-30T13:08:53.802498+00:00
- **Repository:** `/run/media/alex/artefacts/projects/self/projects/64-codecortex`
- **External packages:** 828
- **Graph index:** 207 files, freshness fresh (see index run log)
- **Usage evidence:** Rust `use` / `extern crate` source scan
- **Graph IMPORTS edges:** 0 (n/a)

## Summary by usage tier

| Tier | Count |
| --- | ---: |
| transitive-only | 752 |
| source-referenced | 44 |
| direct-no-source-imports | 26 |
| direct-unused-candidate | 4 |
| direct-investigate-machete-mismatch | 2 |

## Direct dependencies — removal candidates (cargo-machete + no source imports)

- **dotenvy** (0.15.7) — declared in `cortex-cli`
- **lru** (0.12.5) — declared in `cortex-mcp`
- **lru** (0.16.4) — declared in `cortex-mcp`
- **tokio-stream** (0.1.18) — declared in `cortex-mcp`

## Direct dependencies — investigate before removal

- **anyhow** — tier `direct-investigate-machete-mismatch`, files_with_import=4, role=direct
- **blake3** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **chrono** — tier `direct-investigate-machete-mismatch`, files_with_import=5, role=direct
- **dirs** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **dirs** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **glob** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **jsonschema** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **redis** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **rmp-serde** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **serde_yaml** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **sled** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **testcontainers** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **toml** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-bash** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-c** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-cpp** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-go** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-java** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-javascript** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-json** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-kotlin-ng** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-php** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-python** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-ruby** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-rust** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-swift** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **tree-sitter-typescript** — tier `direct-no-source-imports`, files_with_import=0, role=direct
- **walkdir** — tier `direct-no-source-imports`, files_with_import=0, role=direct

## Top source-referenced crates (by file count)

| Package | Files | Import edges | Role |
| --- | ---: | ---: | --- |
| serde | 52 | 200 | direct |
| serde_json | 20 | 58 | direct |
| tree-sitter | 16 | 74 | direct |
| tokio | 14 | 32 | direct |
| tempfile | 13 | 26 | direct |
| tracing | 13 | 46 | direct |
| async-trait | 6 | 12 | direct |
| rsmgclient | 6 | 14 | direct |
| chrono | 5 | 18 | direct |
| criterion | 5 | 20 | direct |
| anyhow | 4 | 10 | direct |
| rmcp | 4 | 22 | direct |
| dashmap | 3 | 6 | direct |
| owo-colors | 3 | 6 | direct |
| sha2 | 3 | 12 | direct |
| futures-util | 2 | 12 | direct |
| grafeo | 2 | 8 | direct |
| rayon | 2 | 8 | direct |
| rusqlite | 2 | 8 | direct |
| tokio-tungstenite | 2 | 8 | direct |
| arrow-array | 1 | 8 | direct |
| arrow-schema | 1 | 4 | direct |
| axum | 1 | 26 | direct |
| clap | 1 | 66 | direct |
| clap_complete | 1 | 4 | direct |

## Limits

- Proc-macro / `#[derive]` only usage may show zero imports.
- Transitive lockfile packages cannot be removed directly.
- Qualified paths without `use` may be under-counted.

Full CSV: `deps-lockfile-usage-report.csv`
