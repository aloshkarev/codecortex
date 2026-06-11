# Index profiles

CodeCortex selects indexer tuning via `indexing_profile` in `~/.cortex/config.toml` or `CORTEX_INDEX_PROFILE`.

| Profile | When to use | Key behavior |
| --- | --- | --- |
| **highspeed** (default) | Workstations with enough RAM; fastest full rebuilds | Larger parse batches, pipeline depth 1, write pool ∝ CPUs, **`index_force_delete_branch_before_parse: true`** on profile apply (wipe branch before parse on `force` + branch — avoids deferred node replay) |
| **conservative** | Low-RAM laptops | Smaller batches, no parse pipeline, write pool 2, deferred replay still available on `force` + branch |

## CLI overrides

```bash
# Explicit branch wipe before parse (same flag highspeed sets by default)
cortex index /path/to/repo --force --wipe-branch-first
```

## Gates and measurement

See [audit/index-perf/README.md](../../audit/index-perf/README.md) for phase gates (G1–G4), `bolt_multiplier`, and JSON reports under `audit/index-perf/`.

Post-backlog baseline: `review-backlog-complete-pure.json` (generated during backlog close-out).
