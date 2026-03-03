# CodeCortex Installation Guide

Complete installation guide for CodeCortex on **macOS** and **Ubuntu/Debian Linux**.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Installation](#quick-installation)
3. [Manual Installation](#manual-installation)
4. [Memgraph Setup](#memgraph-setup)
5. [MCP Service Configuration](#mcp-service-configuration)
6. [IDE Integration](#ide-integration)
7. [Verification](#verification)
8. [Troubleshooting](#troubleshooting)
9. [Uninstallation](#uninstallation)

---

## Prerequisites

### Required

| Dependency | macOS | Ubuntu/Debian |
|------------|-------|---------------|
| **Rust** (1.70+) | `brew install rust` or [rustup](https://rustup.rs) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Git** | `brew install git` | `sudo apt install git` |
| **curl** | (pre-installed) | `sudo apt install curl` |

### Optional

| Dependency | Purpose | macOS | Ubuntu/Debian |
|------------|---------|-------|---------------|
| **Docker** | Memgraph (recommended) | [Docker Desktop](https://docs.docker.com/desktop/install/mac-install/) | `sudo apt install docker.io` |
| **Memgraph** | Graph database | Docker or `brew install memgraph` | Docker or native package |

---

## Quick Installation

### One-Line Install

```bash
# Clone and install in one command
curl -fsSL https://raw.githubusercontent.com/codecortex/codecortex/main/quickstart.sh | bash
```

### Interactive Install

```bash
# Clone the repository
git clone https://github.com/codecortex/codecortex.git
cd codecortex

# Run the installer
./install.sh
```

### Non-Interactive Install

```bash
# Skip all prompts, use defaults
./install.sh --non-interactive

# Skip Memgraph setup
./install.sh --non-interactive --no-memgraph

# Skip service setup
./install.sh --non-interactive --no-service
```

---

## Manual Installation

### Step 1: Install Rust (if not installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
```

### Step 2: Clone and Build

```bash
git clone https://github.com/codecortex/codecortex.git
cd codecortex

# Build release binary
cargo build --release
```

### Step 3: Install Binary

```bash
# Create bin directory
mkdir -p ~/.local/bin

# Copy binary
cp target/release/cortex-cli ~/.local/bin/cortex
chmod +x ~/.local/bin/cortex
```

### Step 4: Add to PATH

Add to your shell configuration:

```bash
# For zsh (default on macOS)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# For bash (common on Linux)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Step 5: Create Configuration

```bash
mkdir -p ~/.cortex

cat > ~/.cortex/config.json << 'EOF'
{
  "memgraph_uri": "bolt://localhost:7687",
  "memgraph_user": "",
  "memgraph_password": "",
  "max_batch_size": 1000
}
EOF
```

---

## Memgraph Setup

### Option A: Docker (Recommended)

**macOS & Linux:**

```bash
# Start Memgraph container
docker run -d \
  --name memgraph \
  -p 7687:7687 \
  -p 7444:7444 \
  -v memgraph_data:/var/lib/memgraph \
  memgraph/memgraph:3.8.1 \
  --also-log-to-stderr=true

# Verify it's running
docker ps | grep memgraph
```

**Using Docker Compose:**

```bash
# From the codecortex repository
docker-compose up -d memgraph
```

### Option B: Native Installation (macOS)

```bash
# Install via Homebrew
brew install memgraph

# Start Memgraph
brew services start memgraph

# Or run directly
/usr/local/opt/memgraph/bin/memgraph
```

### Option C: Native Installation (Ubuntu/Debian)

```bash
# Add Memgraph repository
curl -L https://download.memgraph.com/memgraph-keyring.gpg | sudo gpg --dearmor -o /usr/share/keyrings/memgraph-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/memgraph-keyring.gpg] https://download.memgraph.com/debian stable main" | sudo tee /etc/apt/sources.list.d/memgraph.list

# Install
sudo apt update
sudo apt install memgraph

# Start service
sudo systemctl start memgraph
sudo systemctl enable memgraph
```

### Verify Memgraph Connection

```bash
# Using cortex doctor
cortex doctor

# Using docker
docker exec -it memgraph mgm_client 1
```

---

## MCP Service Configuration

### macOS (launchd)

#### Automatic Setup

The installer creates a launchd service automatically. Manual setup:

```bash
# Create LaunchAgents directory
mkdir -p ~/Library/LaunchAgents

# Create plist file
cat > ~/Library/LaunchAgents/com.codecortex.mcp.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.codecortex.mcp</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/YOUR_USERNAME/.local/bin/cortex</string>
        <string>mcp</string>
        <string>start</string>
    </array>
    <key>WorkingDirectory</key>
    <string>/Users/YOUR_USERNAME</string>
    <key>StandardOutPath</key>
    <string>/Users/YOUR_USERNAME/.cortex/logs/mcp.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/YOUR_USERNAME/.cortex/logs/mcp.log</string>
    <key>RunAtLoad</key>
    <false/>
    <key>KeepAlive</key>
    <false/>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/Users/YOUR_USERNAME/.local/bin:/usr/local/bin:/usr/bin:/bin</string>
    </dict>
</dict>
</plist>
EOF

# Create log directory
mkdir -p ~/.cortex/logs

# Load the service
launchctl load ~/Library/LaunchAgents/com.codecortex.mcp.plist
```

#### Service Management (macOS)

```bash
# Start MCP service
launchctl start com.codecortex.mcp

# Stop MCP service
launchctl stop com.codecortex.mcp

# View logs
tail -f ~/.cortex/logs/mcp.log

# Unload service
launchctl unload ~/Library/LaunchAgents/com.codecortex.mcp.plist
```

### Ubuntu/Debian (systemd)

#### Automatic Setup

The installer creates a systemd service automatically. Manual setup:

```bash
# Create service file
sudo tee /etc/systemd/system/cortex-mcp.service << 'EOF'
[Unit]
Description=CodeCortex MCP Server
After=network.target memgraph.service docker.service
Wants=memgraph.service

[Service]
Type=simple
User=YOUR_USERNAME
WorkingDirectory=/home/YOUR_USERNAME
ExecStart=/home/YOUR_USERNAME/.local/bin/cortex mcp start
Restart=on-failure
RestartSec=10
Environment=PATH=/home/YOUR_USERNAME/.local/bin:/usr/local/bin:/usr/bin:/bin

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd
sudo systemctl daemon-reload

# Enable and start
sudo systemctl enable --now cortex-mcp
```

#### Service Management (Linux)

```bash
# Start MCP service
sudo systemctl start cortex-mcp

# Stop MCP service
sudo systemctl stop cortex-mcp

# Restart MCP service
sudo systemctl restart cortex-mcp

# Check status
sudo systemctl status cortex-mcp

# View logs
sudo journalctl -u cortex-mcp -f

# Disable autostart
sudo systemctl disable cortex-mcp
```

---

## IDE Integration

### Cursor

Create or edit `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "codecortex": {
      "command": "/home/YOUR_USERNAME/.local/bin/cortex",
      "args": ["mcp", "start"],
      "cwd": "/home/YOUR_USERNAME"
    }
  }
}
```

### VS Code

Create or edit `~/.vscode/mcp.json`:

```json
{
  "mcpServers": {
    "codecortex": {
      "command": "/home/YOUR_USERNAME/.local/bin/cortex",
      "args": ["mcp", "start"],
      "cwd": "/home/YOUR_USERNAME"
    }
  }
}
```

### Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `~/.config/claude/config.json` (Linux):

```json
{
  "mcpServers": {
    "codecortex": {
      "command": "/home/YOUR_USERNAME/.local/bin/cortex",
      "args": ["mcp", "start"],
      "cwd": "/home/YOUR_USERNAME"
    }
  }
}
```

### Project-Specific Configuration

Create `mcp.json` in your project root:

```json
{
  "mcpServers": {
    "codecortex": {
      "command": "cortex",
      "args": ["mcp", "start"],
      "cwd": "."
    }
  }
}
```

---

## Verification

### Check Installation

```bash
# Version
cortex --version

# Configuration
cortex config show

# System check
cortex doctor

# List MCP tools
cortex mcp tools
```

### Test Indexing

```bash
# Index a repository
cortex index /path/to/your/code

# Search for symbols
cortex find name authenticate

# Analyze code
cortex analyze callers my_function
```

### Test MCP Server

```bash
# Start MCP server interactively
cortex mcp start

# In another terminal, test a tool
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | cortex mcp start
```

---

## Troubleshooting

### Common Issues

#### "command not found: cortex"

**Solution:** Add to PATH or use full path.

```bash
# Add to PATH
export PATH="$HOME/.local/bin:$PATH"

# Or use full path
~/.local/bin/cortex --version
```

#### "Memgraph connection refused"

**Solution:** Ensure Memgraph is running.

```bash
# Check Docker container
docker ps | grep memgraph

# Start if stopped
docker start memgraph

# Or start fresh
docker run -d --name memgraph -p 7687:7687 memgraph/memgraph:3.8.1
```

#### "Permission denied" on Linux

**Solution:** Check file permissions.

```bash
# Make binary executable
chmod +x ~/.local/bin/cortex

# Check Docker permissions
sudo usermod -aG docker $USER
# Log out and back in for changes to take effect
```

#### Build fails with tree-sitter errors

**Solution:** Ensure you have a C compiler.

```bash
# macOS
xcode-select --install

# Ubuntu/Debian
sudo apt install build-essential
```

#### MCP server won't start

**Solution:** Check logs and configuration.

```bash
# Check logs (macOS)
tail -f ~/.cortex/logs/mcp.log

# Check logs (Linux)
sudo journalctl -u cortex-mcp -n 50

# Verify config
cortex config show

# Run interactively for debugging
RUST_LOG=debug cortex mcp start
```

### Reset Installation

```bash
# Clear all caches
cortex debug cache --clear

# Reset configuration
rm ~/.cortex/config.json
cortex config reset

# Full reset (WARNING: deletes all data)
rm -rf ~/.cortex
```

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug cortex mcp start

# Enable trace logging
RUST_LOG=trace cortex mcp start

# Debug specific module
RUST_LOG=cortex_mcp=debug cortex mcp start
```

---

## Uninstallation

### Remove Binary

```bash
rm ~/.local/bin/cortex
```

### Remove Configuration

```bash
rm -rf ~/.cortex
```

### Remove from PATH

Edit your shell configuration (`~/.zshrc`, `~/.bashrc`, or `~/.profile`) and remove:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Stop and Remove Services

**macOS:**

```bash
launchctl unload ~/Library/LaunchAgents/com.codecortex.mcp.plist
rm ~/Library/LaunchAgents/com.codecortex.mcp.plist
```

**Linux:**

```bash
sudo systemctl stop cortex-mcp
sudo systemctl disable cortex-mcp
sudo rm /etc/systemd/system/cortex-mcp.service
sudo systemctl daemon-reload
```

### Remove Memgraph

**Docker:**

```bash
docker stop memgraph
docker rm memgraph
docker volume rm memgraph_data
```

**Native (macOS):**

```bash
brew services stop memgraph
brew uninstall memgraph
```

**Native (Linux):**

```bash
sudo systemctl stop memgraph
sudo apt remove memgraph
```

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `CORTEX_CONFIG_PATH` | Custom config file path | `~/.cortex/config.json` |
| `CORTEX_SKELETON_CACHE_PATH` | Skeleton cache directory | `~/.cortex/skeletons.db` |
| `RUST_LOG` | Logging level | `info` |
| `RUST_BACKTRACE` | Show backtraces on panic | `0` |

---

## Platform-Specific Notes

### macOS

- **Apple Silicon (M1/M2/M3):** Native builds work without issues
- **Intel Mac:** Use standard installation
- **Docker Desktop:** Required for Memgraph via Docker

### Ubuntu/Debian

- **Ubuntu 22.04+ / Debian 12+:** Fully supported
- **Older versions:** May need newer Rust toolchain
- **systemd:** Required for service management

### WSL2 (Windows)

- Use the Linux installation instructions
- Docker Desktop for Windows with WSL2 backend required for Memgraph
- File watching may have performance limitations

---

## Support

- **Documentation:** [https://github.com/codecortex/codecortex](https://github.com/codecortex/codecortex)
- **Issues:** [https://github.com/codecortex/codecortex/issues](https://github.com/codecortex/codecortex/issues)
- **Discussions:** [https://github.com/codecortex/codecortex/discussions](https://github.com/codecortex/codecortex/discussions)
