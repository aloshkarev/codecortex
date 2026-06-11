# cortex-benches

Performance benchmarks for CodeCortex using Criterion.

## Overview

This crate contains benchmarks for critical performance paths in CodeCortex:

| Benchmark | Description | Target |
| --------- | ----------- | ------ |
| `capsule_benchmark` | Context capsule building | `cortex-mcp` |
| `impact_benchmark` | Impact graph construction | `cortex-mcp` |
| `cache_benchmark` | L1/L2 cache operations | `cortex-mcp` |
| `tfidf_benchmark` | TF-IDF scoring | `cortex-indexer` |
| `parse_batch_benchmark` | Parse fixture tree batch | `cortex-parser` |
| `pipeline_stage_benchmark` | Pipeline construct / empty run | `cortex-pipeline` |
| `watcher_perf_benchmark` | Bounded event queue saturation | `cortex-watcher` |
| `vector_smoke_benchmark` | Lance store open (temp dir) | `cortex-vector` |
| `graph_bulk_benchmark` | Synthetic node build + writer chunk split | `cortex-core` |
| `hybrid_search_benchmark` | Hybrid rerank with mock store/embedder | `cortex-vector` |

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench capsule_benchmark

# Generate HTML reports
cargo bench -- --save-baseline new
```

## Benchmark Details

### Capsule Benchmark

Measures performance of context capsule building:

- Small corpus (50 items)
- Medium corpus (200 items)
- Large corpus (1000 items)

### Impact Benchmark

Measures impact graph construction:

- Shallow call graphs (2 levels)
- Deep call graphs (10 levels)
- Wide call graphs (100 callers)

### Cache Benchmark

Measures cache hierarchy operations:

- L1 in-memory cache get/put
- L2 disk-based cache operations
- Cache hit/miss scenarios

### TF-IDF Benchmark

Measures text scoring operations:

- Document tokenization
- TF-IDF computation
- Corpus building

## Output

Benchmarks generate:

- Console output with statistics
- HTML reports in `target/criterion/`
- Comparison with previous runs

## Requirements

- Rust 1.70+ (edition 2024)
- `criterion` 0.8 with HTML reports feature
