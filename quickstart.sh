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

set -e

REPO_URL="https://github.com/aloshkarev/codecortex"
INSTALL_DIR="${HOME}/.cortex"
REPO_DIR=""
NON_INTERACTIVE=false

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

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --non-interactive|-y)
            NON_INTERACTIVE=true
            shift
            ;;
        *)
            log_warning "Unknown argument: $1"
            shift
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

# Step 2: Check dependencies
log_info "Checking dependencies..."

# Check Rust
if ! command_exists cargo; then
    log_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "${HOME}/.cargo/env" 2>/dev/null || true
fi
log_success "Rust: $(rustc --version)"

# Check Docker (optional but recommended)
if command_exists docker; then
    if docker info &>/dev/null; then
        log_success "Docker: available and running"
    else
        log_warning "Docker: installed but not running"
    fi
else
    log_warning "Docker: not installed (required for Memgraph)"
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

SHELL_RC=""
case "${SHELL:-/bin/bash}" in
    */zsh)  SHELL_RC="${HOME}/.zshrc" ;;
    */bash) SHELL_RC="${HOME}/.bashrc" ;;
    *)      SHELL_RC="${HOME}/.profile" ;;
esac

if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    echo "export PATH=\"\${PATH}:${BIN_DIR}\"" >> "$SHELL_RC"
    log_info "Added ${BIN_DIR} to PATH in ${SHELL_RC}"
fi

log_success "Binary installed: ${BIN_DIR}/cortex"

# Step 5: Setup Memgraph (Docker)
CONTAINER_NAME="codecortex-memgraph"
MEMGRAPH_PORT=7687

log_info "Setting up Memgraph..."

if command_exists docker; then
    if ! docker info &>/dev/null; then
        log_warning "Docker not running. Please start Docker and run:"
        echo "  docker run -d --name ${CONTAINER_NAME} -p ${MEMGRAPH_PORT}:7687 memgraph/memgraph:3.8.1"
    else
        # Check for existing container FIRST (before any port checking)
        EXISTING_CONTAINER=""
        if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
            EXISTING_CONTAINER="${CONTAINER_NAME}"
        elif docker ps -a --format '{{.Names}}' | grep -q '^memgraph$'; then
            EXISTING_CONTAINER="memgraph"
        fi

        if [ -n "$EXISTING_CONTAINER" ]; then
            # Get port from existing container before starting
            MEMGRAPH_PORT=$(docker port "$EXISTING_CONTAINER" 7687 2>/dev/null | cut -d: -f2 || echo "7687")

            if ! docker ps --format '{{.Names}}' | grep -q "^${EXISTING_CONTAINER}$"; then
                docker start "$EXISTING_CONTAINER"
                sleep 2
            fi
            log_success "Memgraph container running on port ${MEMGRAPH_PORT}"
        else
            # No existing container - find available port BEFORE creating
            if check_port 7687; then
                log_info "Port 7687 is in use, finding available port..."
                MEMGRAPH_PORT=$(find_available_port 7688 10)
            fi

            docker run -d \
                --name "${CONTAINER_NAME}" \
                -p "${MEMGRAPH_PORT}:7687" \
                -v codecortex-memgraph:/var/lib/memgraph \
                memgraph/memgraph:3.8.1 \
                --also-log-to-stderr=true
            sleep 2
            log_success "Memgraph container started on port ${MEMGRAPH_PORT}"
        fi
    fi
else
    log_warning "Docker not found. Install Docker to use Memgraph, or connect to an existing instance."
    log_info "You will need to configure Memgraph manually."
fi

# Step 6: Create configuration
mkdir -p "${HOME}/.cortex"

# Determine vector store path
VECTOR_PATH="${HOME}/.cortex/vectors"

# Determine LLM settings based on available services
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

# Create TOML config
if [ ! -f "${HOME}/.cortex/config.toml" ]; then
    cat > "${HOME}/.cortex/config.toml" <<EOF
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
    log_success "Config created: ${HOME}/.cortex/config.toml"
else
    log_info "Config already exists: ${HOME}/.cortex/config.toml"
fi

# Step 7: Pull Ollama model if needed
if [ "$LLM_PROVIDER" = "ollama" ] && [ -n "$EMBEDDING_MODEL" ]; then
    if ! ollama list 2>/dev/null | grep -q "$EMBEDDING_MODEL"; then
        log_info "Pulling Ollama embedding model: ${EMBEDDING_MODEL}..."
        ollama pull "$EMBEDDING_MODEL" || log_warning "Failed to pull model - run manually: ollama pull ${EMBEDDING_MODEL}"
    fi
fi

# Step 8: Offer interactive setup
if [ "$NON_INTERACTIVE" = false ]; then
    echo ""
    read -p "$(echo -e ${CYAN}Run interactive setup wizard? [Y/n]: ${NC})" -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        cortex setup
    fi
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
if [ -f "${HOME}/.cortex/config.toml" ]; then
    log_success "Config: ${HOME}/.cortex/config.toml"
else
    log_warning "Config file not found"
fi

# Verify database container
if command_exists docker && docker ps --format '{{.Names}}' 2>/dev/null | grep -q "${CONTAINER_NAME}\|memgraph"; then
    log_success "Database: Container running"
else
    log_info "Database: Container not running (start with: docker start ${CONTAINER_NAME})"
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
echo "    Config:     ${HOME}/.cortex/config.toml"
echo "    Vectors:    ${VECTOR_PATH}"
echo "    Database:   memgraph://127.0.0.1:${MEMGRAPH_PORT}"
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
