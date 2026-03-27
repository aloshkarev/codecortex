# Getting Help

> This page describes how to get support for CodeCortex, including documentation links, community channels, and answers to common questions.

## Documentation

- [Install guide](docs/INSTALL.md) — build, install, start Memgraph, configure, verify
- [Integration guide](docs/INTEGRATION.md) — connect Cursor, Claude Code, Codex CLI, Gemini CLI, Zed, Neovim
- [Integration test matrix](docs/INTEGRATION_TEST_MATRIX.md) — per-language test runbook

## Community

- **Questions and discussions**: [GitHub Discussions](https://github.com/aloshkarev/codecortex/discussions)
- **Bug reports**: [Open an issue](https://github.com/aloshkarev/codecortex/issues/new/choose)
- **Security vulnerabilities**: See [SECURITY.md](SECURITY.md) for responsible disclosure — do not open a public issue

## Frequently Asked Questions

### Installation

**Q: `cortex: command not found` after install.**
Confirm `~/.local/bin` is on your `PATH`. Add it and reload your shell:
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

**Q: Build fails with C/C++ toolchain errors.**
On macOS: `xcode-select --install`
On Ubuntu/Debian: `sudo apt update && sudo apt install -y build-essential`

**Q: Can I install without Nix?**
Yes. Use `cargo build --release -p cortex-cli` and copy `target/release/cortex-cli` to `~/.local/bin/cortex`.

### Graph backend

**Q: `cortex doctor` fails with "cannot connect to Memgraph".**
Verify the container is running (`docker ps`) and port `7687` is reachable (`nc -z 127.0.0.1 7687`). Check `~/.cortex/config.toml` for the correct URI.

**Q: Can I use Neo4j instead of Memgraph?**
Yes. Set `backend_type = "neo4j"` and `memgraph_uri = "bolt://127.0.0.1:7687"` in `~/.cortex/config.toml`. See [docs/INSTALL.md](docs/INSTALL.md) for details.

**Q: Can I use AWS Neptune?**
Yes. Set `backend_type = "neo4j"` and provide your Neptune bolt endpoint. Authentication and TLS details depend on your Neptune configuration.

### Indexing

**Q: How do I re-index after large code changes?**
Run `cortex index /path/to/repo --force` to fully re-index, or `cortex index /path/to/repo --mode incremental-diff` to re-index only files changed since the last indexed git commit.

**Q: Indexing is slow on a large repository.**
Increase `max_batch_size` in `~/.cortex/config.toml` (default 500). Set `CORTEX_INDEXER_MAX_FILES=N` to limit scope during debugging. Use `cortex stats` to track progress.

**Q: How do I watch a repository for automatic re-indexing?**
Run `cortex watch /path/to/repo`. The watcher debounces file events and triggers incremental re-indexing automatically. Use `cortex jobs list` to monitor queued jobs.

### MCP server

**Q: My AI client shows no tools.**
Run `cortex mcp tools` to verify the server lists tools. Check the client config path and args. Ensure the `cortex` binary is on `PATH` where the client process runs.

**Q: Results are stale or empty after code changes.**
Re-index the repository (`cortex index /path/to/repo`) and restart the MCP process. For automatic updates, use `cortex watch`.

**Q: How do I enable memory / context capsule / impact graph tools?**
These are off by default. Use `--enable` args on `cortex mcp start` (preferred):
```bash
cortex mcp start --enable memory --enable context-capsule --enable impact-graph
```
Or set environment variables before starting (both sources are combined):
```bash
CORTEX_FLAG_MCP_MEMORY_READ_ENABLED=true cortex mcp start
```
See the [Feature flags section in README.md](README.md#feature-flags) for the full list of `--enable` values and env vars.

**Q: How do I expose the MCP server over the network?**
Use `--transport http-sse` or `--transport multi` with `--listen`, `--allow-remote`, and `--token-env`. Always set a bearer token and terminate TLS at a reverse proxy. See [SECURITY.md](SECURITY.md) for hardening guidance.

### Vector search

**Q: `cortex search` returns no results.**
Verify you have indexed vectors: `cortex vector-index /path/to/repo`. Check that `llm.provider` is set to `openai` or `ollama` in `~/.cortex/config.toml` and that the configured API key or Ollama instance is reachable.

**Q: Which embedding providers are supported?**
OpenAI (`text-embedding-3-small`, 1536 dimensions) and Ollama (`nomic-embed-text`, `bge-m3`). Configure via `llm.provider`, `llm.openai_api_key`, and `llm.ollama_base_url` in `~/.cortex/config.toml`.

### General

**Q: How do I check the current configuration?**
Run `cortex config show`.

**Q: How do I update CodeCortex?**
```bash
git pull && nix build .#cortex && cp result/bin/cortex ~/.local/bin/cortex
```

**Q: How do I completely uninstall?**
```bash
rm -f ~/.local/bin/cortex
rm -rf ~/.cortex
docker rm -f codecortex-memgraph
```
