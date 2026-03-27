# Installation

> This guide covers building, installing, and configuring CodeCortex on macOS, Ubuntu/Debian, and other Linux distributions.

## Requirements

**Required:**
- Git
- A graph backend — Memgraph (default, recommended via Docker) or Neo4j

**Build toolchain (choose one):**
- [Nix](https://nixos.org/) with flakes enabled — provides a fully reproducible build environment
- Rust stable toolchain — use if Nix is unavailable

**Optional:**
- Docker — easiest way to run Memgraph
- AWS Neptune — for cloud-hosted graph backend (see [Neo4j backend](#neo4j-and-aws-neptune-backend))

## Build and install

### With Nix (recommended)

```bash
git clone https://github.com/aloshkarev/codecortex.git
cd codecortex
nix build .#cortex
```

For a guided install with dependency checks and retries:

```bash
./install.sh
```

For local development bootstrap:

```bash
./quickstart.sh
```

Binary location: `result/bin/cortex`

### With Cargo (without Nix)

```bash
git clone https://github.com/aloshkarev/codecortex.git
cd codecortex
cargo build --release -p cortex-cli
```

**macOS prerequisites:**
```bash
xcode-select --install
```

**Ubuntu/Debian prerequisites:**
```bash
sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev
```

**Fedora/RHEL prerequisites:**
```bash
sudo dnf install -y gcc openssl-devel pkgconf
```

Binary location: `target/release/cortex-cli`

### Install as `cortex`

```bash
mkdir -p ~/.local/bin

# Nix build
cp result/bin/cortex ~/.local/bin/cortex

# Cargo build
cp target/release/cortex-cli ~/.local/bin/cortex

chmod +x ~/.local/bin/cortex
```

Add to `PATH`:

```bash
# zsh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc

# bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc && source ~/.bashrc
```

## Start a graph backend

### Memgraph (default, recommended)

```bash
docker run -d --name codecortex-memgraph -p 7687:7687 memgraph/memgraph-mage:3.8.1
```

Verify it is running:

```bash
docker ps | grep memgraph
```

### Neo4j and AWS Neptune backend

To use Neo4j instead of Memgraph, start a Neo4j instance and set `backend_type = "neo4j"` in the config file:

```bash
docker run -d \
  --name codecortex-neo4j \
  -p 7687:7687 \
  -e NEO4J_AUTH=none \
  neo4j:5
```

For AWS Neptune, use the Neo4j driver with your Neptune Bolt endpoint:

```toml
memgraph_uri = "bolt://your.neptune.amazonaws.com:8182"
backend_type = "neo4j"
```

Note: Neptune requires IAM authentication or VPC access controls. TLS must be configured at the application level; see your Neptune cluster's connection string.

## Configure CodeCortex

```bash
mkdir -p ~/.cortex
cat > ~/.cortex/config.toml <<'CFG'
memgraph_uri = "memgraph://127.0.0.1:7687"
memgraph_user = ""
memgraph_password = ""
backend_type = "memgraph"
max_batch_size = 500
CFG
```

Restrict config file permissions if it contains credentials:

```bash
chmod 600 ~/.cortex/config.toml
```

### Optional: vector search configuration

To enable semantic search, add an LLM provider to the config:

```toml
[llm]
provider = "openai"
openai_api_key = "sk-..."           # or set OPENAI_API_KEY env var

# Alternative: local Ollama
# provider = "ollama"
# ollama_base_url = "http://127.0.0.1:11434"
# ollama_embedding_model = "nomic-embed-text"
```

## Verify installation

```bash
cortex --version
cortex doctor
cortex mcp tools
```

`cortex doctor` checks: binary is on PATH, backend is reachable, and config is valid.

## First run

```bash
# Index a repository
cortex index /path/to/repo --force

# Run a search
cortex find name main

# Analyze callers
cortex analyze callers authenticate

# Start MCP server
cortex mcp start
```

For a one-command bootstrap:

```bash
cortex doctor && cortex index /path/to/repo && cortex vector-index /path/to/repo && cortex mcp start
```

## MCP client setup

See [docs/INTEGRATION.md](INTEGRATION.md) for setup instructions for Cursor, Claude Code, Codex CLI, Gemini CLI, Zed, and Neovim.

## Update

```bash
git pull
nix build .#cortex
cp result/bin/cortex ~/.local/bin/cortex
```

## Uninstall

```bash
rm -f ~/.local/bin/cortex
rm -rf ~/.cortex
docker rm -f codecortex-memgraph
```

## Troubleshooting

### `cortex: command not found`

Confirm `~/.local/bin` is on `PATH`. Restart your shell or source your shell profile.

### Cannot connect to Memgraph

```bash
docker ps                          # confirm container is running
nc -z 127.0.0.1 7687 && echo ok   # confirm port is reachable
cortex doctor                      # full connectivity check
```

### Cannot connect to Neo4j

Verify the Neo4j container is healthy and `backend_type = "neo4j"` is set. Check that the Neo4j Bolt port (7687) is not blocked by a firewall.

### Build errors on macOS: missing C compiler

```bash
xcode-select --install
```

### Build errors on Ubuntu/Debian: missing build tools

```bash
sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev
```

### Vector search returns no results

Ensure you have indexed vectors (`cortex vector-index /path/to/repo`) and that `llm.provider` is set to `openai` or `ollama` with valid credentials. Run `cortex doctor` to verify backend connectivity.

### Stale results after re-indexing

Re-run `cortex index /path/to/repo --force` and restart the MCP server process. Outdated graph data is not automatically purged on incremental re-index.
