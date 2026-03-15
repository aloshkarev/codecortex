# Installation

This guide covers a practical setup for CodeCortex on macOS and Ubuntu/Debian.

## Requirements

- Rust (stable)
- Git
- Memgraph (recommended via Docker)

Optional:

- Docker

## Build and install

```bash
git clone https://github.com/aloshkarev/codecortex.git
cd codecortex
cargo build --release -p cortex-cli
```

For a guided install with dependency checks and retries (protoc/OpenSSL/build tools), use:

```bash
./install.sh
```

For local development bootstrap with the same robustness checks:

```bash
./quickstart.sh
```

Binary:

- `target/release/cortex-cli`

Install as `cortex`:

```bash
mkdir -p ~/.local/bin
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

## Start Memgraph

```bash
docker run -d --name codecortex-memgraph -p 7687:7687 memgraph/memgraph-mage:3.8.1
```

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

### cannot connect to Memgraph

- check container is up: `docker ps`
- confirm port `7687` is reachable
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
cargo build --release -p cortex-cli
cp target/release/cortex-cli ~/.local/bin/cortex
```

## Uninstall

```bash
rm -f ~/.local/bin/cortex
rm -rf ~/.cortex
docker rm -f codecortex-memgraph
```
