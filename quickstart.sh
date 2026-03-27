#!/usr/bin/env bash
#
# CodeCortex Quickstart Script
# One-command setup for development and testing
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/aloshkarev/codecortex/main/quickstart.sh | bash
#
# For non-interactive setup (uses defaults):
#   curl -fsSL https://raw.githubusercontent.com/aloshkarev/codecortex/main/quickstart.sh | bash -s -- --non-interactive
#
# Memgraph via Docker is opt-in (matches install.sh):
#   ... | bash -s -- --memgraph
#   ... | bash -s -- --no-memgraph   # skip and suppress interactive prompt
#

set -euo pipefail

REPO_URL="https://github.com/aloshkarev/codecortex"
INSTALL_DIR="${HOME}/.cortex"
REPO_DIR=""
NON_INTERACTIVE=false
PREFER_NIX=true
INSTALL_MEMGRAPH=false   # opt-in: use --memgraph or answer yes at the prompt
INSTALL_DOCKER=false     # opt-in: parity with install.sh (reserved)

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

confirm() {
    if [ "$NON_INTERACTIVE" = true ]; then
        return 0
    fi
    local prompt="${1:-Continue?}"
    local default="${2:-y}"

    if [ "$default" = "y" ]; then
        prompt="$prompt [Y/n]"
    else
        prompt="$prompt [y/N]"
    fi

    read -rp "$prompt: " response
    response=${response:-$default}

    [[ "$response" =~ ^[Yy]$ ]]
}

run_with_retry() {
    local retries=${1:-3}
    shift
    local attempt=1
    local delay=2
    while true; do
        if "$@"; then
            return 0
        fi
        if [ "$attempt" -ge "$retries" ]; then
            return 1
        fi
        log_warning "Command failed (attempt ${attempt}/${retries}), retrying in ${delay}s: $*"
        sleep "$delay"
        attempt=$((attempt + 1))
        delay=$((delay * 2))
    done
}

detect_package_manager() {
    if command_exists brew; then
        echo "brew"
    elif command_exists apt-get; then
        echo "apt"
    elif command_exists dnf; then
        echo "dnf"
    elif command_exists yum; then
        echo "yum"
    else
        echo "unknown"
    fi
}

install_build_dependencies() {
    local pm
    pm=$(detect_package_manager)
    case "$pm" in
        brew)
            run_with_retry 3 brew update
            run_with_retry 3 brew install protobuf openssl@3 pkg-config cmake llvm
            ;;
        apt)
            run_with_retry 3 sudo apt-get update
            run_with_retry 3 sudo apt-get install -y \
                protobuf-compiler libssl-dev pkg-config build-essential cmake clang libclang-dev
            ;;
        dnf)
            run_with_retry 3 sudo dnf install -y \
                protobuf-compiler openssl-devel pkgconf-pkg-config gcc gcc-c++ make cmake clang clang-devel
            ;;
        yum)
            run_with_retry 3 sudo yum install -y \
                protobuf-compiler openssl-devel pkgconfig gcc gcc-c++ make cmake clang
            ;;
        *)
            return 1
            ;;
    esac
}

confirm_or_default_yes() {
    local prompt="$1"
    if [ "$NON_INTERACTIVE" = true ] || [ ! -t 0 ]; then
        return 0
    fi
    read -rp "$prompt [Y/n]: " response
    response=${response:-Y}
    [[ "$response" =~ ^[Yy]$ ]]
}

verify_build_prereqs() {
    local missing=()
    for dep in git curl tar cargo rustc protoc pkg-config cmake make; do
        if ! command_exists "$dep"; then
            missing+=("$dep")
        fi
    done
    if ! command_exists cc && ! command_exists gcc && ! command_exists clang; then
        missing+=("cc")
    fi
    if ! command_exists c++ && ! command_exists g++ && ! command_exists clang++; then
        missing+=("c++")
    fi
    if ! command_exists clang; then
        missing+=("clang")
    fi
    if command_exists pkg-config && ! pkg-config --exists openssl 2>/dev/null; then
        missing+=("openssl")
    fi

    if [ ${#missing[@]} -eq 0 ]; then
        log_success "Build preflight passed (toolchain + native deps)"
        return 0
    fi

    log_warning "Missing build dependencies: ${missing[*]}"
    local pm
    pm=$(detect_package_manager)
    if [ "$pm" = "unknown" ]; then
        log_error "Unsupported package manager; install missing deps manually and re-run."
        return 1
    fi

    if confirm_or_default_yes "Install missing build dependencies automatically?"; then
        install_build_dependencies
    else
        log_error "Cannot continue without required build dependencies"
        return 1
    fi

    # Re-verify
    verify_build_prereqs_final
}

verify_build_prereqs_final() {
    for dep in cargo rustc protoc pkg-config cmake make; do
        if ! command_exists "$dep"; then
            log_error "Missing dependency after attempted installation: ${dep}"
            return 1
        fi
    done
    if command_exists pkg-config && ! pkg-config --exists openssl 2>/dev/null; then
        log_error "OpenSSL development package not detected via pkg-config"
        return 1
    fi
    return 0
}

show_help() {
    echo "CodeCortex quickstart"
    echo ""
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --non-interactive, -y  Non-interactive (skips Memgraph unless --memgraph)"
    echo "  --no-nix               Do not use Nix even if installed"
    echo "  --memgraph             Install / start Memgraph via Docker (opt-in)"
    echo "  --no-memgraph          Skip Memgraph (suppresses interactive prompt)"
    echo "  --help, -h             Show this help"
    echo ""
    echo "By default this script does not start Memgraph. Use --memgraph or choose"
    echo "yes when prompted. Point memgraph_uri in ${INSTALL_DIR}/config.toml at an"
    echo "existing Memgraph or Neo4j server if you skip Docker setup."
    echo ""
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --non-interactive|-y)
            NON_INTERACTIVE=true
            shift
            ;;
        --no-nix)
            PREFER_NIX=false
            shift
            ;;
        --memgraph)
            INSTALL_MEMGRAPH=true
            shift
            ;;
        --no-memgraph)
            INSTALL_MEMGRAPH="skip"
            shift
            ;;
        --help|-h)
            show_help
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

get_os() {
    case "$(uname -s)" in
        Darwin*) echo "macos" ;;
        Linux*)  echo "linux" ;;
        *)       echo "unknown" ;;
    esac
}

check_port() {
    local port=$1
    if command_exists lsof; then
        lsof -i :$port &>/dev/null
    elif command_exists ss; then
        ss -ln | grep -q ":$port "
    elif command_exists netstat; then
        netstat -an | grep -q ":$port "
    else
        return 1  # Can't check, assume available
    fi
}

find_available_port() {
    local start_port=$1
    local max_attempts=${2:-10}

    for port in $(seq $start_port $((start_port + max_attempts - 1))); do
        if ! check_port $port; then
            echo $port
            return 0
        fi
    done
    echo $start_port
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
        run_with_retry 3 git pull --ff-only
    else
        log_info "Cloning fresh repository..."
        run_with_retry 3 git clone "$REPO_URL" "$REPO_DIR"
        cd "$REPO_DIR"
    fi
else
    REPO_DIR="$(pwd)"
    log_info "Running from existing repository: $REPO_DIR"
fi

# Step 2: Check dependencies
log_info "Checking dependencies..."

USE_NIX=false
if [ "$PREFER_NIX" = true ] && command_exists nix; then
    USE_NIX=true
    log_success "Nix found - will use flake build path"
fi

if [ "$USE_NIX" = false ]; then
    # Check Rust
    if ! command_exists cargo; then
        log_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "${HOME}/.cargo/env" 2>/dev/null || true
        export PATH="${HOME}/.cargo/bin:${PATH}"
    fi
    log_success "Rust: $(rustc --version)"
    verify_build_prereqs
fi

# Check Docker (informational only — Memgraph setup is opt-in)
if command_exists docker; then
    log_success "Docker found: $(docker --version)"
    if ! docker info &>/dev/null; then
        log_warning "Docker is installed but not running — start it before Memgraph via Docker"
    fi
else
    log_info "Docker not found — skipping Docker-based Memgraph setup"
    log_info "  Use --memgraph to install Memgraph, or point CodeCortex at your existing server"
    if [ "$INSTALL_MEMGRAPH" != "skip" ]; then
        INSTALL_MEMGRAPH=false
    fi
fi

# Check Ollama (optional)
if command_exists ollama; then
    if curl -s http://127.0.0.1:11434/api/tags &>/dev/null; then
        log_success "Ollama: available and running"
    else
        log_info "Ollama: installed but not running"
    fi
else
    log_info "Ollama: not installed (optional, for local embeddings)"
fi

# Step 3: Build
if [ "$USE_NIX" = true ]; then
    log_info "Building CodeCortex with Nix (this may take a few minutes)..."
    run_with_retry 2 nix build .#cortex
else
    log_info "Building CodeCortex (this may take a few minutes)..."
    if ! run_with_retry 2 cargo build --release --locked; then
        log_warning "Locked build failed; retrying without --locked"
        run_with_retry 2 cargo build --release
    fi
fi
log_success "Build complete"

# Step 4: Install binary
BIN_DIR="${HOME}/.local/bin"
mkdir -p "$BIN_DIR"

if [ "$USE_NIX" = true ]; then
    if [ ! -f result/bin/cortex ]; then
        log_error "Build completed but result/bin/cortex is missing"
        exit 1
    fi
    cp result/bin/cortex "${BIN_DIR}/cortex"
else
    if [ ! -f target/release/cortex-cli ]; then
        log_error "Build completed but target/release/cortex-cli is missing"
        exit 1
    fi
    cp target/release/cortex-cli "${BIN_DIR}/cortex"
fi
chmod +x "${BIN_DIR}/cortex"

if [[ "$(uname -s)" == "Darwin" ]]; then
    log_info "Applying macOS ad-hoc code signature to cortex binary..."
    codesign --force --sign - "${BIN_DIR}/cortex"
    codesign --verify --verbose=4 "${BIN_DIR}/cortex"
    PATH="${BIN_DIR}:${PATH}" cortex --version
fi

# Add to PATH
ORIGINAL_PATH="${PATH}"
export PATH="${BIN_DIR}:${PATH}"

SHELL_RC=""
case "${SHELL:-/bin/bash}" in
    */zsh)  SHELL_RC="${HOME}/.zshrc" ;;
    */bash) SHELL_RC="${HOME}/.bashrc" ;;
    *)      SHELL_RC="${HOME}/.profile" ;;
esac

if [[ ":$ORIGINAL_PATH:" != *":${BIN_DIR}:"* ]]; then
    echo "export PATH=\"\${PATH}:${BIN_DIR}\"" >> "$SHELL_RC"
    log_info "Added ${BIN_DIR} to PATH in ${SHELL_RC}"
fi

log_success "Binary installed: ${BIN_DIR}/cortex"

# Step 5: Setup Memgraph (Docker, opt-in — same flow as install.sh)
CONTAINER_NAME="codecortex-memgraph"
MEMGRAPH_PORT=7687

if [ "$INSTALL_MEMGRAPH" = "skip" ]; then
    log_info "Skipping Memgraph installation (--no-memgraph)"
elif [ "$INSTALL_MEMGRAPH" = false ]; then
    if [ "$NON_INTERACTIVE" = true ]; then
        log_info "Skipping Memgraph setup (use --memgraph to install, or configure manually)"
    else
        echo ""
        log_info "Graph backend setup (optional)"
        echo "  CodeCortex requires a Memgraph or Neo4j server."
        echo "  If you already have one running (local or remote), answer 'n' and"
        echo "  update memgraph_uri in ${INSTALL_DIR}/config.toml after quickstart."
        if ! confirm "Install Memgraph via Docker now?" "n"; then
            log_info "Skipping Memgraph setup — update ${INSTALL_DIR}/config.toml with your server URI"
        else
            INSTALL_MEMGRAPH=true
        fi
    fi
fi

if [ "$INSTALL_MEMGRAPH" = true ]; then
    log_info "Setting up Memgraph..."
    if ! command_exists docker; then
        log_warning "Docker not found. Install Docker for Memgraph, or connect to an existing instance."
        log_info "Update memgraph_uri in ${INSTALL_DIR}/config.toml with your server URI."
        INSTALL_MEMGRAPH=false
    elif ! docker info &>/dev/null; then
        log_warning "Docker not running. Start Docker, then run:"
        echo "  docker run -d --name ${CONTAINER_NAME} -p ${MEMGRAPH_PORT}:7687 memgraph/memgraph:3.8.1"
        INSTALL_MEMGRAPH=false
    else
        # Check for existing container FIRST (before any port checking)
        EXISTING_CONTAINER=""
        if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
            EXISTING_CONTAINER="${CONTAINER_NAME}"
        elif docker ps -a --format '{{.Names}}' | grep -q '^memgraph$'; then
            EXISTING_CONTAINER="memgraph"
        fi

        if [ -n "$EXISTING_CONTAINER" ]; then
            MEMGRAPH_PORT=$(docker port "$EXISTING_CONTAINER" 7687 2>/dev/null | cut -d: -f2 || echo "7687")

            if ! docker ps --format '{{.Names}}' | grep -q "^${EXISTING_CONTAINER}$"; then
                docker start "$EXISTING_CONTAINER"
                sleep 2
            fi
            log_success "Memgraph container running on port ${MEMGRAPH_PORT}"
        else
            if check_port 7687; then
                log_info "Port 7687 is in use, finding available port..."
                MEMGRAPH_PORT=$(find_available_port 7688 10)
            fi

            run_with_retry 3 docker pull memgraph/memgraph:3.8.1
            run_with_retry 2 docker run -d \
                --name "${CONTAINER_NAME}" \
                -p "${MEMGRAPH_PORT}:7687" \
                -v codecortex-memgraph:/var/lib/memgraph \
                memgraph/memgraph:3.8.1 \
                --also-log-to-stderr=true
            sleep 2
            log_success "Memgraph container started on port ${MEMGRAPH_PORT}"
        fi
    fi
fi

# Step 6: Create configuration
mkdir -p "${INSTALL_DIR}"

# Determine vector store path
VECTOR_PATH="${INSTALL_DIR}/vectors"
config_file="${INSTALL_DIR}/config.toml"

# Determine LLM settings based on available services (used for new config + summary)
if curl -s http://127.0.0.1:11434/api/tags &>/dev/null; then
    LLM_PROVIDER="ollama"
    EMBEDDING_MODEL="nomic-embed-text"
    log_info "Detected Ollama - will use for embeddings"
elif [ -n "$OPENAI_API_KEY" ]; then
    LLM_PROVIDER="openai"
    EMBEDDING_MODEL="text-embedding-3-small"
    log_info "Detected OPENAI_API_KEY - will use OpenAI for embeddings"
else
    LLM_PROVIDER="none"
    EMBEDDING_MODEL=""
    log_warning "No LLM provider detected - embeddings disabled"
fi

# Create TOML config (never overwrite an existing file — same as install.sh)
if [ -e "$config_file" ] || [ -L "$config_file" ]; then
    log_info "Configuration file already exists — leaving unchanged: ${config_file}"
else
    cat > "$config_file" <<EOF
# CodeCortex Configuration
# Generated by quickstart.sh

# Graph Database (Memgraph or Neo4j)
# Use memgraph://host for Memgraph, or neo4j://host for Neo4j
memgraph_uri = "memgraph://127.0.0.1:${MEMGRAPH_PORT}"
memgraph_user = ""
memgraph_password = ""
# Backend type: "memgraph" (default) or "neo4j"
# Can also be set via CORTEX_BACKEND_TYPE environment variable
backend_type = "memgraph"

# Vector Store Configuration
[vector]
store_type = "lancedb"
store_path = "${VECTOR_PATH}"
qdrant_uri = "http://127.0.0.1:6333"
qdrant_api_key = ""
embedding_dim = 1536

# LLM/Embedding Provider Configuration
[llm]
provider = "${LLM_PROVIDER}"
openai_api_key = ""
openai_embedding_model = "text-embedding-3-small"
ollama_base_url = "http://127.0.0.1:11434"
ollama_embedding_model = "${EMBEDDING_MODEL}"

# Indexer Settings
max_batch_size = 500
indexer_timeout_secs = 300
indexer_max_files = 0

# Analyzer Settings
analyzer_query_limit = 1000
analyzer_cache_ttl_secs = 300

# Watcher Settings
watcher_debounce_secs = 2
watcher_max_events = 128

# Connection Pool Settings
pool_max_connections = 10
pool_min_idle = 2
pool_connection_timeout_secs = 30

# Watched Paths (add repositories to watch)
watched_paths = []
EOF
    log_success "Configuration saved to ${config_file}"

    # Pull Ollama model if needed (only after creating a fresh config — matches install.sh)
    if [ "$LLM_PROVIDER" = "ollama" ] && [ -n "$EMBEDDING_MODEL" ]; then
        if ! ollama list 2>/dev/null | grep -q "$EMBEDDING_MODEL"; then
            log_info "Pulling Ollama embedding model: ${EMBEDDING_MODEL}..."
            ollama pull "$EMBEDDING_MODEL" || log_warning "Failed to pull model - run manually: ollama pull ${EMBEDDING_MODEL}"
        fi
    fi
fi

# Step 8: Offer interactive setup
if [ "$NON_INTERACTIVE" = false ] && [ -t 0 ]; then
    echo ""
    read -p "$(echo -e ${CYAN}Run interactive setup wizard? [Y/n]: ${NC})" -n 1 -r || true
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        cortex setup
    fi
elif [ "$NON_INTERACTIVE" = false ]; then
    log_info "Skipping interactive setup wizard (no TTY detected)"
fi

# Step 9: Verify
log_info "Verifying installation..."

# Verify binary
if cortex --version &>/dev/null; then
    VERSION=$(cortex --version 2>&1 | head -1)
    log_success "Binary: ${VERSION}"
else
    log_error "Binary verification failed"
fi

# Verify configuration
if [ -e "$config_file" ] || [ -L "$config_file" ]; then
    log_success "Config: ${config_file}"
else
    log_warning "Config file not found"
fi

# Verify database container
if [ "$INSTALL_MEMGRAPH" = true ]; then
    if command_exists docker && docker ps --format '{{.Names}}' 2>/dev/null | grep -q "${CONTAINER_NAME}\|memgraph"; then
        log_success "Database: Memgraph container running"
    else
        log_info "Database: Memgraph container not running (start with: docker start ${CONTAINER_NAME})"
    fi
else
    log_info "Graph backend: update memgraph_uri in ${config_file}, then run: cortex doctor"
fi

# Verify MCP tools
TOOL_COUNT=$(cortex mcp tools 2>/dev/null | wc -l | tr -d ' ')
if [ -n "$TOOL_COUNT" ] && [ "$TOOL_COUNT" -gt 0 ]; then
    log_success "MCP tools: ${TOOL_COUNT} available"
else
    log_info "MCP tools: Available after installation"
fi

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                    Quickstart Complete!                                ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "  ${YELLOW}Installation Summary:${NC}"
echo ""
echo "    Binary:     ${BIN_DIR}/cortex"
echo "    Config:     ${config_file}"
echo "    Vectors:    ${VECTOR_PATH}"
if [ "$INSTALL_MEMGRAPH" = true ]; then
    echo "    Database:   memgraph://127.0.0.1:${MEMGRAPH_PORT}"
else
    echo "    Graph:      set memgraph_uri in ${config_file}, then: cortex doctor"
fi
echo "    Embeddings: ${LLM_PROVIDER}"
echo ""
echo -e "  ${YELLOW}Next Steps:${NC}"
echo ""
echo "    1. Restart your shell or run: source ${SHELL_RC}"
echo "    2. Verify installation: cortex doctor"
echo "    3. Index a repository: cortex index /path/to/code"
echo "    4. Start MCP server: cortex mcp start"
echo ""
if [ "$LLM_PROVIDER" = "none" ]; then
    echo -e "  ${YELLOW}Note:${NC} No LLM provider configured."
    echo "  For local embeddings, install Ollama: https://ollama.ai"
    echo "  Then run: ollama pull nomic-embed-text"
    echo ""
fi
echo -e "  ${YELLOW}Documentation:${NC} ${REPO_URL}/blob/main/docs/INSTALL.md"
echo ""
