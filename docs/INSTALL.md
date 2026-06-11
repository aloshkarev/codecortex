# Installation

This guide covers a practical FalkorDB-first setup for CodeCortex on macOS and Ubuntu/Debian.

## Requirements

- Nix (flakes enabled, preferred) or Rust stable (Cargo fallback)
- Git
- Docker (recommended for FalkorDB)

Optional:

- Ollama or OpenAI API key for vector embeddings

## Build and install

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

Binary:

- Nix build output: `result/bin/cortex`
- Cargo fallback output: `target/release/cortex-cli`

Install as `cortex`:

```bash
mkdir -p ~/.local/bin
cp result/bin/cortex ~/.local/bin/cortex
chmod +x ~/.local/bin/cortex
```

Cargo fallback:

```bash
cargo build --release -p cortex-cli
cp target/release/cortex-cli ~/.local/bin/cortex
chmod +x ~/.local/bin/cortex
```

Add to `PATH`:

```bash
# zsh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

## Start FalkorDB

Docker (recommended):

```bash
docker run -d --name codecortex-falkordb -p 6379:6379 falkordb/falkordb:latest
```

Or use the repo compose file:

```bash
docker compose up -d falkordb
```

## Configure CodeCortex

```bash
mkdir -p ~/.cortex
cat > ~/.cortex/config.toml <<'CFG'
backend_type = "falkordb"
falkordb_uri = "falkor://127.0.0.1:6379"
falkordb_graph = "codecortex"
falkordb_password = ""
max_batch_size = 4096
CFG
```

See [FALKORDB.md](FALKORDB.md) for tuning (`falkordb_write_pool_size`, `falkordb_unwind_batch_max`, indexing profiles).

## Verify

```bash
cortex --version
cortex doctor
cortex mcp tools
```

## First run

```bash
cortex index /path/to/repo --force
cortex find name main
cortex analyze callers authenticate
```

## Integration clients

MCP client setup examples:

- `docs/INTEGRATION.md`

## Troubleshooting

### `cortex: command not found`

- confirm `~/.local/bin` is on `PATH`
- restart shell or source shell profile

### cannot connect to FalkorDB

- check container is up: `docker ps`
- confirm port `6379` is reachable
- run `cortex doctor`

### parser/toolchain build errors

```bash
# macOS
xcode-select --install

# Ubuntu/Debian
sudo apt update && sudo apt install -y build-essential
```

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
docker rm -f codecortex-falkordb
```
