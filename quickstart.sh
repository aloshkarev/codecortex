#!/usr/bin/env bash
#
# CodeCortex Quickstart Script
# One-command setup for development and testing
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/aloshkarev/codecortex/main/quickstart.sh | bash
#

set -e

REPO_URL="https://github.com/aloshkarev/codecortex"
INSTALL_DIR="${HOME}/.cortex"
REPO_DIR=""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERR]${NC} $1"; }

command_exists() { command -v "$1" &>/dev/null; }

get_os() {
    case "$(uname -s)" in
        Darwin*) echo "macos" ;;
        Linux*)  echo "linux" ;;
        *)       echo "unknown" ;;
    esac
}

echo ""
echo -e "${CYAN}╔════════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║                     CodeCortex Quickstart                              ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Step 1: Clone repository if needed
if [ ! -f "Cargo.toml" ]; then
    log_info "Cloning CodeCortex repository..."
    REPO_DIR="${INSTALL_DIR}/repo"
    mkdir -p "$INSTALL_DIR"

    if [ -d "$REPO_DIR" ]; then
        log_info "Updating existing repository..."
        cd "$REPO_DIR"
        git pull
    else
        log_info "Cloning fresh repository..."
        git clone "$REPO_URL" "$REPO_DIR"
        cd "$REPO_DIR"
    fi
else
    REPO_DIR="$(pwd)"
    log_info "Running from existing repository: $REPO_DIR"
fi

# Step 2: Check Rust
if ! command_exists cargo; then
    log_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "${HOME}/.cargo/env" 2>/dev/null || true
fi

log_success "Rust: $(rustc --version)"

# Step 3: Build
log_info "Building CodeCortex (this may take a few minutes)..."
cargo build --release
log_success "Build complete"

# Step 4: Install binary
BIN_DIR="${HOME}/.local/bin"
mkdir -p "$BIN_DIR"

cp target/release/cortex-cli "${BIN_DIR}/cortex"
chmod +x "${BIN_DIR}/cortex"

# Add to PATH
export PATH="${BIN_DIR}:${PATH}"

if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    SHELL_RC=""
    case "${SHELL:-/bin/bash}" in
        */zsh)  SHELL_RC="${HOME}/.zshrc" ;;
        */bash) SHELL_RC="${HOME}/.bashrc" ;;
        *)      SHELL_RC="${HOME}/.profile" ;;
    esac
    echo "export PATH=\"\${PATH}:${BIN_DIR}\"" >> "$SHELL_RC"
    log_info "Added ${BIN_DIR} to PATH in ${SHELL_RC}"
fi

log_success "Binary installed: ${BIN_DIR}/cortex"

# Step 5: Setup Memgraph (Docker)
log_info "Setting up Memgraph..."

if command_exists docker; then
    if ! docker info &>/dev/null; then
        log_warning "Docker not running. Please start Docker and run:"
        echo "  docker run -d --name memgraph -p 7687:7687 memgraph/memgraph:2.19.0"
    else
        if docker ps -a --format '{{.Names}}' | grep -q '^memgraph$'; then
            if ! docker ps --format '{{.Names}}' | grep -q '^memgraph$'; then
                docker start memgraph
            fi
            log_success "Memgraph container running"
        else
            docker run -d --name memgraph -p 7687:7687 memgraph/memgraph:2.19.0
            log_success "Memgraph container started"
        fi
    fi
else
    log_warning "Docker not found. Install Docker to use Memgraph."
fi

# Step 6: Create config
mkdir -p "${HOME}/.cortex"

if [ ! -f "${HOME}/.cortex/config.json" ]; then
    cat > "${HOME}/.cortex/config.json" <<EOF
{
  "memgraph_uri": "bolt://localhost:7687",
  "memgraph_user": "",
  "memgraph_password": "",
  "max_batch_size": 1000
}
EOF
    log_success "Config created: ${HOME}/.cortex/config.json"
fi

# Step 7: Verify
log_info "Verifying installation..."

if cortex --version &>/dev/null; then
    log_success "CodeCortex: $(cortex --version 2>&1 | head -1)"
fi

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                    Quickstart Complete!                                ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "  ${YELLOW}Next Steps:${NC}"
echo ""
echo "    1. Restart your shell or run: source ${SHELL_RC}"
echo "    2. Verify installation: cortex doctor"
echo "    3. Index a repository: cortex index /path/to/code"
echo "    4. Start MCP server: cortex mcp start"
echo ""
echo -e "  ${YELLOW}Documentation:${NC} ${REPO_URL}/blob/main/docs/INSTALL.md"
echo ""
