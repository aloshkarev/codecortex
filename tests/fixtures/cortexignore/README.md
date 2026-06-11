# cortexignore fixture

Minimal tree for manual or CI checks that graph and vector indexing honor `.cortexignore`.

Layout:

- `.cortexignore` — ignores `generated/` and `skip.rs`
- `keep.rs` — should be indexed
- `skip.rs` — should be ignored
- `generated/auto.rs` — should be ignored
- `pkg/.cortexignore` — ignores `*.tmp`
- `pkg/src/keep.rs` — should be indexed when scanning `pkg/`
- `pkg/build/out.tmp` — should be ignored

Verify:

```bash
cargo test -p cortex-core ignore::
cargo test -p cortex-core --test cortexignore_hierarchical
cargo test -p cortex-indexer test_collect_source_files_respects_cortexignore
cargo test -p cortex-mcp collect_indexable_code_files_honors_cortexignore
cargo test -p cortex-mcp --test cortexignore_parity
make cortexignore-git-oracle
```
