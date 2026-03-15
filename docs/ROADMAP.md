# CodeCortex Roadmap

This roadmap tracks the next delivery steps after the current production baseline.

## Current baseline

Available now:

- 14-language parsing and indexing (Rust, Python, Go, TypeScript, JavaScript, C, C++, Java, PHP, Ruby, Kotlin, Swift, JSON, Shell)
- graph-backed code analysis
- vector indexing/search integration
- MCP tool surface for assistant workflows across stdio, HTTP+SSE, and WebSocket transports
- one-by-one real integration tests across pinned OSS fixtures

## Near-term priorities

## 1) Reliability and performance

- stabilize heavy analysis operations on large repositories
- reduce long-tail query latency for impact and relationship traversals
- improve timeout and retry controls across CLI and MCP paths
- extend failure artifacts in integration CI for faster triage

## 2) Analysis quality

- continue improving smell/refactoring signal quality
- reduce false positives in language-specific heuristics
- improve branch-diff and review outputs for large changesets
- add explicit expected-shape contracts for more analysis outputs

## 3) MCP operability

- tighten tool-level diagnostics and error categories
- improve high-cost tool behavior under constrained environments
- keep tool-surface drift guards strict in CI
- improve metadata and schema hints for better agent tool selection

## 4) Project workflows

- better project lifecycle ergonomics (`project` subcommands)
- queue and sync visibility improvements
- cleaner branch-scoped indexing routines

## 5) Vector/search workflows

- improve vector indexing behavior for large files
- better fallback behavior when embedding providers are unavailable
- stronger hybrid scoring controls

## Release framing

- v1.1: reliability, analysis-quality improvements, MCP operability
- v1.2: deeper project workflows and vector search hardening
- v1.3: enterprise-grade policy/observability/security extensions

## How roadmap items are accepted

A roadmap item is complete when:

- implementation merged
- regression coverage added where needed
- integration tests pass in one-by-one real mode
- docs updated (`README`, crate docs, and relevant runbooks)
