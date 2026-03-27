#!/usr/bin/env bash
#
# CodeCortex Installation Script
# Supports: macOS (Intel/Apple Silicon) and Ubuntu/Debian Linux
#
# Usage:
#   ./install.sh                    # Interactive installation
#   ./install.sh --non-interactive  # Non-interactive mode
#   ./install.sh --help             # Show help
#

set -euo pipefail

# ═══════════════════════════════════════════════════════════════════════════════
# Configuration
# ═══════════════════════════════════════════════════════════════════════════════

CORTEX_VERSION="1.0.1"
CORTEX_BIN_NAME="cortex"
REPO_URL="https://github.com/aloshkarev/codecortex"
CORTEX_CONFIG_DIR="${HOME}/.cortex"
CORTEX_BIN_DIR="${HOME}/.local/bin"
CORTEX_DATA_DIR="${HOME}/.cortex/data"
MEMGRAPH_URI="memgraph://localhost:7687"
MEMGRAPH_USER="memgraph"
MEMGRAPH_PASSWORD="memgraph"
CONTAINER_NAME="codecortex-memgraph"
VECTOR_PATH="${HOME}/.cortex/vectors"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Flags
NON_INTERACTIVE=false
INSTALL_METHOD=""
START_SERVICES=true
INSTALL_MEMGRAPH=false   # opt-in: use --memgraph or answer yes at the prompt
INSTALL_DOCKER=false     # opt-in: use --docker to also install Docker
PREFER_NIX=true

# ═══════════════════════════════════════════════════════════════════════════════
# Utility Functions
# ═══════════════════════════════════════════════════════════════════════════════

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "\n${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
}

command_exists() {
    command -v "$1" &> /dev/null
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

get_os() {
    case "$(uname -s)" in
        Darwin*)    echo "macos" ;;
        Linux*)     echo "linux" ;;
        *)          echo "unknown" ;;
    esac
}

get_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   echo "x86_64" ;;
        arm64|aarch64)  echo "aarch64" ;;
        *)              echo "unknown" ;;
    esac
}

is_ubuntu() {
    [ -f /etc/os-release ] && grep -qi "ubuntu\|debian" /etc/os-release
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

print_dependency_help() {
    local dep="$1"
    local pm
    pm=$(detect_package_manager)
    case "$pm" in
        brew)
            case "$dep" in
                protoc) echo "brew install protobuf" ;;
                openssl) echo "brew install openssl@3 pkg-config" ;;
                clang|libclang) echo "brew install llvm" ;;
                gcc|cc|c++) echo "xcode-select --install" ;;
                pkg-config) echo "brew install pkg-config" ;;
                cmake) echo "brew install cmake" ;;
                make) echo "xcode-select --install" ;;
                *) echo "brew install ${dep}" ;;
            esac
            ;;
        apt)
            case "$dep" in
                protoc) echo "sudo apt-get install -y protobuf-compiler" ;;
                openssl) echo "sudo apt-get install -y libssl-dev pkg-config" ;;
                clang|libclang) echo "sudo apt-get install -y clang libclang-dev" ;;
                gcc|cc|c++) echo "sudo apt-get install -y build-essential" ;;
                pkg-config) echo "sudo apt-get install -y pkg-config" ;;
                cmake) echo "sudo apt-get install -y cmake" ;;
                make) echo "sudo apt-get install -y build-essential" ;;
                *) echo "sudo apt-get install -y ${dep}" ;;
            esac
            ;;
        dnf|yum)
            case "$dep" in
                protoc) echo "sudo ${pm} install -y protobuf-compiler" ;;
                openssl) echo "sudo ${pm} install -y openssl-devel pkgconf-pkg-config" ;;
                clang|libclang) echo "sudo ${pm} install -y clang clang-devel" ;;
                gcc|cc|c++) echo "sudo ${pm} install -y gcc gcc-c++ make" ;;
                pkg-config) echo "sudo ${pm} install -y pkgconf-pkg-config" ;;
                cmake) echo "sudo ${pm} install -y cmake" ;;
                make) echo "sudo ${pm} install -y make" ;;
                *) echo "sudo ${pm} install -y ${dep}" ;;
            esac
            ;;
        *)
            echo "Install '${dep}' using your system package manager"
            ;;
    esac
}

# ═══════════════════════════════════════════════════════════════════════════════
# Dependency Checking
# ═══════════════════════════════════════════════════════════════════════════════

check_dependencies() {
    log_step "Checking Dependencies"

    local missing_deps=()
    local os=$(get_os)

    # Required baseline dependencies
    local required=("curl" "git" "tar")

    for dep in "${required[@]}"; do
        if ! command_exists "$dep"; then
            missing_deps+=("$dep")
        fi
    done

    if [ ${#missing_deps[@]} -gt 0 ]; then
        log_warning "Missing required dependencies: ${missing_deps[*]}"

        if confirm "Install missing dependencies?"; then
            install_dependencies "${missing_deps[@]}"
        else
            log_error "Cannot continue without required dependencies"
            exit 1
        fi
    fi

    # Prefer Nix when available unless explicitly disabled.
    if [ "$PREFER_NIX" = true ] && command_exists nix; then
        INSTALL_METHOD="nix"
        log_success "Nix found - will use Nix build path"
    else
        # Check for Rust (optional, for cargo install)
        if command_exists rustc && command_exists cargo; then
            log_success "Rust toolchain found: $(rustc --version)"
            INSTALL_METHOD="cargo"
        else
            log_info "Rust not found - will use binary installation"
            INSTALL_METHOD="binary"
        fi
    fi

    # Check for Docker (informational only — Memgraph setup is opt-in)
    if command_exists docker; then
        log_success "Docker found: $(docker --version)"
    else
        log_info "Docker not found — skipping Docker-based Memgraph setup"
        log_info "  Use --memgraph to install Memgraph, or point CodeCortex at your existing server"
        # Don't abort; user may have a dedicated/remote Memgraph server
        INSTALL_MEMGRAPH=false
    fi

    # Build-time preflight dependencies (source build path)
    if [ "$INSTALL_METHOD" = "nix" ]; then
        log_info "Skipping Cargo native build dependency preflight (Nix selected)"
        return 0
    fi

    local build_missing=()
    for dep in pkg-config cmake make protoc; do
        if ! command_exists "$dep"; then
            build_missing+=("$dep")
        fi
    done
    if ! command_exists cc && ! command_exists gcc && ! command_exists clang; then
        build_missing+=("cc")
    fi
    if ! command_exists c++ && ! command_exists g++ && ! command_exists clang++; then
        build_missing+=("c++")
    fi
    if ! command_exists clang; then
        build_missing+=("clang")
    fi
    if command_exists pkg-config && ! pkg-config --exists openssl 2>/dev/null; then
        build_missing+=("openssl")
    fi

    if [ ${#build_missing[@]} -gt 0 ]; then
        log_warning "Missing build dependencies: ${build_missing[*]}"
        log_info "These are required before running cargo build."
        for dep in "${build_missing[@]}"; do
            echo "  - ${dep}: $(print_dependency_help "$dep")"
        done

        if confirm "Try to install missing build dependencies now?"; then
            install_build_dependencies "${build_missing[@]}"
        else
            log_error "Cannot proceed without required build dependencies"
            exit 1
        fi
    else
        log_success "Build dependency preflight passed"
    fi
}

install_dependencies() {
    local deps=("$@")
    local os=$(get_os)

    log_info "Installing dependencies: ${deps[*]}"

    case "$os" in
        macos)
            if command_exists brew; then
                brew install "${deps[@]}"
            else
                log_error "Homebrew not found. Please install Homebrew first:"
                echo "  /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
                exit 1
            fi
            ;;
        linux)
            if command_exists apt-get; then
                sudo apt-get update
                sudo apt-get install -y "${deps[@]}"
            elif command_exists dnf; then
                sudo dnf install -y "${deps[@]}"
            elif command_exists yum; then
                sudo yum install -y "${deps[@]}"
            else
                log_error "Unsupported package manager. Please install: ${deps[*]}"
                exit 1
            fi
            ;;
        *)
            log_error "Unsupported OS for automatic dependency installation"
            exit 1
            ;;
    esac

    log_success "Dependencies installed"
}

install_build_dependencies() {
    local os
    os=$(get_os)
    local pm
    pm=$(detect_package_manager)

    log_info "Installing build dependencies for ${os}/${pm}..."
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
            log_error "Unsupported package manager for automatic build dependency installation"
            return 1
            ;;
    esac

    # Re-verify critical deps
    local verify_failed=false
    for dep in protoc pkg-config cmake make; do
        if ! command_exists "$dep"; then
            log_error "Still missing dependency after installation: ${dep}"
            verify_failed=true
        fi
    done
    if command_exists pkg-config && ! pkg-config --exists openssl 2>/dev/null; then
        log_error "OpenSSL development package not detected via pkg-config"
        verify_failed=true
    fi
    if [ "$verify_failed" = true ]; then
        return 1
    fi
    log_success "Build dependencies installed and verified"
}

# ═══════════════════════════════════════════════════════════════════════════════
# Rust Installation
# ═══════════════════════════════════════════════════════════════════════════════

install_rust() {
    log_step "Installing Rust"

    if command_exists rustc; then
        log_success "Rust already installed: $(rustc --version)"
        return 0
    fi

    log_info "Installing Rust via rustup..."

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

    # Source the environment
    source "${HOME}/.cargo/env" 2>/dev/null || true
    export PATH="${HOME}/.cargo/bin:${PATH}"

    if command_exists rustc; then
        log_success "Rust installed: $(rustc --version)"
        INSTALL_METHOD="cargo"
    else
        log_error "Failed to install Rust"
        return 1
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# CodeCortex Installation
# ═══════════════════════════════════════════════════════════════════════════════

install_cortex_cargo() {
    log_step "Installing CodeCortex via Cargo"

    local repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    log_info "Building CodeCortex from source..."
    cd "$repo_dir"

    # Build release binary after a strict dependency preflight
    if ! run_with_retry 2 cargo build --release --locked; then
        log_warning "Locked build failed; retrying without --locked"
        run_with_retry 2 cargo build --release
    fi

    # Create bin directory if it doesn't exist
    mkdir -p "$CORTEX_BIN_DIR"

    # Copy binary
    if [ ! -f target/release/cortex-cli ]; then
        log_error "Build finished but target/release/cortex-cli not found"
        return 1
    fi
    cp target/release/cortex-cli "${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
    chmod +x "${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"

    if [[ "$(uname -s)" == "Darwin" ]]; then
        log_info "Applying macOS ad-hoc code signature to cortex binary..."
        codesign --force --sign - "${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
        codesign --verify --verbose=4 "${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
        PATH="${CORTEX_BIN_DIR}:${PATH}" cortex --version
    fi

    log_success "CodeCortex installed to ${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
}

install_cortex_nix() {
    log_step "Installing CodeCortex via Nix"

    local repo_dir
    repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    log_info "Building CodeCortex from flake output..."
    cd "$repo_dir"
    run_with_retry 2 nix build .#cortex

    mkdir -p "$CORTEX_BIN_DIR"
    if [ ! -f result/bin/cortex ]; then
        log_error "Nix build finished but result/bin/cortex not found"
        return 1
    fi
    cp result/bin/cortex "${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
    chmod +x "${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"

    if [[ "$(uname -s)" == "Darwin" ]]; then
        log_info "Applying macOS ad-hoc code signature to cortex binary..."
        codesign --force --sign - "${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
        codesign --verify --verbose=4 "${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
        PATH="${CORTEX_BIN_DIR}:${PATH}" cortex --version
    fi

    log_success "CodeCortex installed via Nix to ${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
}

install_cortex_binary() {
    log_step "Installing CodeCortex Binary"

    local os=$(get_os)
    local arch=$(get_arch)
    local binary_url="${REPO_URL}/releases/download/v${CORTEX_VERSION}/cortex-${os}-${arch}.tar.gz"

    # For now, build from source since releases aren't available
    log_warning "Pre-built binaries not yet available"
    log_info "Installing Rust and building from source..."

    install_rust
    install_cortex_cargo
}

verify_installation() {
    log_step "Verifying Installation"

    # Add to PATH if not already there
    if [[ ":$PATH:" != *":${CORTEX_BIN_DIR}:"* ]]; then
        log_info "Adding ${CORTEX_BIN_DIR} to PATH..."

        local shell_rc=""
        case "${SHELL:-/bin/bash}" in
            */zsh)  shell_rc="${HOME}/.zshrc" ;;
            */bash) shell_rc="${HOME}/.bashrc" ;;
            *)      shell_rc="${HOME}/.profile" ;;
        esac

        echo "" >> "$shell_rc"
        echo "# CodeCortex" >> "$shell_rc"
        echo "export PATH=\"\${PATH}:${CORTEX_BIN_DIR}\"" >> "$shell_rc"

        log_info "Added to ${shell_rc}. Run 'source ${shell_rc}' or restart your shell."
    fi

    # Verify binary
    export PATH="${CORTEX_BIN_DIR}:${PATH}"

    if command_exists cortex; then
        local version=$(cortex --version 2>/dev/null || echo "unknown")
        log_success "CodeCortex installed: ${version}"
    else
        log_error "Installation verification failed"
        return 1
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# Memgraph Installation
# ═══════════════════════════════════════════════════════════════════════════════

# Check if a port is available
check_port_available() {
    local port=$1
    if command_exists lsof; then
        ! lsof -i :$port &>/dev/null
    elif command_exists ss; then
        ! ss -ln | grep -q ":$port "
    elif command_exists netstat; then
        ! netstat -an | grep -q ":$port "
    else
        return 0  # Assume available if we can't check
    fi
}

# Find an available port starting from the given one
find_available_port() {
    local start_port=$1
    local max_attempts=${2:-10}

    for port in $(seq $start_port $((start_port + max_attempts - 1))); do
        if check_port_available $port; then
            echo $port
            return 0
        fi
    done
    echo $start_port  # Fallback to start port
}

install_memgraph_docker() {
    log_step "Setting up Memgraph with Docker"

    if ! command_exists docker; then
        log_error "Docker not found. Please install Docker first."
        return 1
    fi

    # Check if Docker daemon is running
    if ! docker info &>/dev/null; then
        log_warning "Docker daemon not running. Attempting to start..."

        local os=$(get_os)
        case "$os" in
            macos)
                open -a Docker 2>/dev/null || {
                    log_error "Please start Docker Desktop manually"
                    return 1
                }
                log_info "Waiting for Docker to start..."
                sleep 15
                ;;
            linux)
                sudo systemctl start docker || sudo service docker start || {
                    log_error "Failed to start Docker"
                    return 1
                }
                ;;
        esac
    fi

    # Check for existing container FIRST (before any port checking)
    local existing_container=""
    if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        existing_container="${CONTAINER_NAME}"
    elif docker ps -a --format '{{.Names}}' | grep -q '^memgraph$'; then
        existing_container="memgraph"
    fi

    if [ -n "$existing_container" ]; then
        log_info "Found existing Memgraph container (${existing_container})"

        # Get port from existing container before starting it
        local port
        port=$(docker port "$existing_container" 7687 2>/dev/null | cut -d: -f2 || echo "7687")
        MEMGRAPH_URI="memgraph://localhost:${port}"

        if docker ps --format '{{.Names}}' | grep -q "^${existing_container}$"; then
            log_success "Memgraph is already running on port ${port}"
        else
            log_info "Starting existing Memgraph container..."
            docker start "$existing_container"
            sleep 3
            log_success "Memgraph started on port ${port}"
        fi
    else
        # No existing container - find available port BEFORE pulling image
        local port=7687

        if ! check_port_available 7687; then
            log_info "Port 7687 is in use, finding available port..."
            port=$(find_available_port 7688 10)
            log_info "Will use port ${port}"
        fi

        MEMGRAPH_URI="memgraph://localhost:${port}"

        # Now pull and create container
        log_info "Pulling Memgraph Docker image..."
        run_with_retry 3 docker pull memgraph/memgraph-mage:3.8.1

        log_info "Creating Memgraph container on port ${port}..."
        run_with_retry 2 docker run -d \
            --name "${CONTAINER_NAME}" \
            -p "${port}:7687" \
            -v codecortex-memgraph:/var/lib/memgraph \
            memgraph/memgraph-mage:3.8.1 \
            --also-log-to-stderr=true

        sleep 3
        log_success "Memgraph started on port ${port}"
    fi
}

install_memgraph_native() {
    log_step "Installing Memgraph Natively"

    local os=$(get_os)

    case "$os" in
        macos)
            if command_exists brew; then
                log_info "Installing Memgraph via Homebrew..."
                brew install memgraph
                log_success "Memgraph installed. Start with: brew services start memgraph"
            else
                log_error "Homebrew required for native Memgraph installation on macOS"
                return 1
            fi
            ;;
        linux)
            if is_ubuntu; then
                log_info "Installing Memgraph on Ubuntu..."

                # Add Memgraph repository
                curl -L https://download.memgraph.com/memgraph-keyring.gpg | sudo gpg --dearmor -o /usr/share/keyrings/memgraph-keyring.gpg
                echo "deb [signed-by=/usr/share/keyrings/memgraph-keyring.gpg] https://download.memgraph.com/debian stable main" | sudo tee /etc/apt/sources.list.d/memgraph.list

                sudo apt-get update
                sudo apt-get install -y memgraph

                log_success "Memgraph installed. Start with: sudo systemctl start memgraph"
            else
                log_warning "Native installation only supported on Ubuntu/Debian. Use Docker instead."
                return 1
            fi
            ;;
        *)
            log_error "Unsupported OS for native Memgraph installation"
            return 1
            ;;
    esac
}

setup_memgraph() {
    # If user explicitly skipped with --no-memgraph, honour it silently.
    if [ "$INSTALL_MEMGRAPH" = "skip" ]; then
        log_info "Skipping Memgraph installation (--no-memgraph)"
        return 0
    fi

    # If not explicitly requested, ask in interactive mode.
    if [ "$INSTALL_MEMGRAPH" = false ]; then
        if [ "$NON_INTERACTIVE" = true ]; then
            log_info "Skipping Memgraph setup (use --memgraph to install, or configure manually)"
            return 0
        fi
        echo ""
        log_info "Graph backend setup (optional)"
        echo "  CodeCortex requires a Memgraph or Neo4j server."
        echo "  If you already have one running (local or remote), answer 'n' and"
        echo "  update memgraph_uri in ~/.cortex/config.toml after install."
        if ! confirm "Install Memgraph via Docker now?" "n"; then
            log_info "Skipping Memgraph setup — update ~/.cortex/config.toml with your server URI"
            return 0
        fi
        INSTALL_MEMGRAPH=true
    fi

    log_step "Setting up Memgraph"

    local use_docker=true

    if ! command_exists docker; then
        if confirm "Docker not found. Install Memgraph natively instead?"; then
            use_docker=false
        else
            log_warning "Skipping Memgraph installation"
            INSTALL_MEMGRAPH=false
            return 0
        fi
    fi

    if [ "$use_docker" = true ]; then
        install_memgraph_docker
    else
        install_memgraph_native
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# Configuration
# ═══════════════════════════════════════════════════════════════════════════════

setup_config() {
    log_step "Configuring CodeCortex"

    # Create config directory
    mkdir -p "$CORTEX_CONFIG_DIR"
    mkdir -p "$CORTEX_DATA_DIR"
    mkdir -p "$VECTOR_PATH"

    local config_file="${CORTEX_CONFIG_DIR}/config.toml"

    # Never overwrite an existing config (regular file, symlink, etc.).
    # Use -L so a dangling symlink is still treated as "exists" and we do not clobber.
    if [ -e "$config_file" ] || [ -L "$config_file" ]; then
        log_info "Configuration file already exists — leaving unchanged: ${config_file}"
        return 0
    fi

    # Get Memgraph credentials
    if [ "$NON_INTERACTIVE" = false ]; then
        read -rp "Memgraph URI [${MEMGRAPH_URI}]: " uri_input
        MEMGRAPH_URI="${uri_input:-$MEMGRAPH_URI}"

        read -rp "Memgraph username [${MEMGRAPH_USER}]: " user_input
        MEMGRAPH_USER="${user_input:-$MEMGRAPH_USER}"

        read -rsp "Memgraph password [${MEMGRAPH_PASSWORD}]: " pass_input
        echo
        MEMGRAPH_PASSWORD="${pass_input:-$MEMGRAPH_PASSWORD}"
    fi

    # Detect LLM provider
    local llm_provider="none"
    local embedding_model=""

    if command_exists ollama && curl -s http://127.0.0.1:11434/api/tags &>/dev/null; then
        llm_provider="ollama"
        embedding_model="nomic-embed-text"
        log_info "Detected Ollama - will use for embeddings"
    elif [ -n "$OPENAI_API_KEY" ]; then
        llm_provider="openai"
        embedding_model="text-embedding-3-small"
        log_info "Detected OPENAI_API_KEY - will use OpenAI for embeddings"
    else
        log_warning "No LLM provider detected - embeddings disabled"
    fi

    # Create TOML config file
    cat > "$config_file" <<EOF
# CodeCortex Configuration
# Generated by install.sh

# Graph Database (Memgraph or Neo4j)
memgraph_uri = "${MEMGRAPH_URI}"
memgraph_user = "${MEMGRAPH_USER}"
memgraph_password = "${MEMGRAPH_PASSWORD}"
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
provider = "${llm_provider}"
openai_api_key = ""
openai_embedding_model = "text-embedding-3-small"
ollama_base_url = "http://127.0.0.1:11434"
ollama_embedding_model = "${embedding_model}"

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

# Watched Paths
watched_paths = []
EOF

    log_success "Configuration saved to ${config_file}"

    # Pull Ollama model if needed
    if [ "$llm_provider" = "ollama" ] && [ -n "$embedding_model" ]; then
        if ! ollama list 2>/dev/null | grep -q "$embedding_model"; then
            log_info "Pulling Ollama embedding model: ${embedding_model}..."
            ollama pull "$embedding_model" || log_warning "Failed to pull model - run manually: ollama pull ${embedding_model}"
        fi
    fi
}

setup_mcp_config() {
    log_step "Setting up MCP Configuration"

    local mcp_config=""

    # Detect IDE/editor
    local editors=()
    [ -d "${HOME}/.cursor" ] && editors+=("cursor")
    [ -d "${HOME}/.vscode" ] && editors+=("vscode")
    [ -f "${HOME}/.claude/settings.json" ] && editors+=("claude")

    if [ ${#editors[@]} -eq 0 ]; then
        log_info "No supported editors found. Creating mcp.json in current directory..."
        mcp_config="mcp.json"
    else
        log_info "Found editors: ${editors[*]}"

        if [ "$NON_INTERACTIVE" = true ] || confirm "Create MCP config for all detected editors?"; then
            for editor in "${editors[@]}"; do
                case "$editor" in
                    cursor)
                        create_mcp_config "${HOME}/.cursor/mcp.json"
                        ;;
                    vscode)
                        create_mcp_config "${HOME}/.vscode/mcp.json"
                        ;;
                    claude)
                        create_mcp_config "${HOME}/.claude/settings.json" "claude"
                        ;;
                esac
            done
            return 0
        fi
    fi

    create_mcp_config "$mcp_config"
}

create_mcp_config() {
    local config_path="$1"
    local format="${2:-mcp}"

    local cortex_bin="${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
    local cwd="$(pwd)"

    mkdir -p "$(dirname "$config_path")"

    if [ "$format" = "claude" ]; then
        # Claude Desktop format
        cat > "$config_path" <<EOF
{
  "mcpServers": {
    "codecortex": {
      "command": "${cortex_bin}",
      "args": ["mcp", "start"],
      "cwd": "${cwd}"
    }
  }
}
EOF
    else
        # Standard MCP format
        cat > "$config_path" <<EOF
{
  "mcpServers": {
    "codecortex": {
      "command": "${cortex_bin}",
      "args": ["mcp", "start"],
      "cwd": "${cwd}"
    }
  }
}
EOF
    fi

    log_success "MCP config created at ${config_path}"
}

# ═══════════════════════════════════════════════════════════════════════════════
# Service Setup
# ═══════════════════════════════════════════════════════════════════════════════

setup_service() {
    local os=$(get_os)

    log_step "Setting up MCP Service"

    if [ "$START_SERVICES" = false ]; then
        log_info "Skipping service setup"
        return 0
    fi

    case "$os" in
        macos)
            setup_launchd_service
            ;;
        linux)
            setup_systemd_service
            ;;
        *)
            log_warning "Service setup not supported on this OS"
            ;;
    esac
}

setup_launchd_service() {
    log_info "Setting up launchd service for macOS..."

    local plist_path="${HOME}/Library/LaunchAgents/com.codecortex.mcp.plist"
    local cortex_bin="${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
    local log_dir="${CORTEX_CONFIG_DIR}/logs"

    mkdir -p "$log_dir"

    cat > "$plist_path" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.codecortex.mcp</string>
    <key>ProgramArguments</key>
    <array>
        <string>${cortex_bin}</string>
        <string>mcp</string>
        <string>start</string>
    </array>
    <key>WorkingDirectory</key>
    <string>${HOME}</string>
    <key>StandardOutPath</key>
    <string>${log_dir}/mcp.log</string>
    <key>StandardErrorPath</key>
    <string>${log_dir}/mcp.log</string>
    <key>RunAtLoad</key>
    <false/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
        <key>Crashed</key>
        <true/>
    </dict>
    <key>ThrottleInterval</key>
    <integer>10</integer>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>${CORTEX_BIN_DIR}:/usr/local/bin:/usr/bin:/bin</string>
        <key>RUST_LOG</key>
        <string>info</string>
    </dict>
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>LegacyTimers</key>
    <true/>
</dict>
</plist>
EOF

    log_success "launchd plist created at ${plist_path}"

    if confirm "Load the MCP service now?"; then
        launchctl load "$plist_path" 2>/dev/null || true
        log_success "MCP service loaded. Start with: launchctl start com.codecortex.mcp"
    else
        log_info "Load manually with: launchctl load ${plist_path}"
    fi
}

setup_systemd_service() {
    log_info "Setting up systemd service for Linux..."

    local service_path="/etc/systemd/system/cortex-mcp.service"
    local cortex_bin="${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"

    if [ ! -w /etc/systemd/system ]; then
        log_warning "Root access required for systemd service setup"

        if ! confirm "Continue with sudo?"; then
            log_info "Skipping systemd service setup"
            log_info "You can set it up manually later"
            return 0
        fi
    fi

    cat | sudo tee "$service_path" > /dev/null <<EOF
[Unit]
Description=CodeCortex MCP Server
Documentation=${REPO_URL}
After=network.target network-online.target memgraph.service docker.service
Wants=network-online.target memgraph.service

[Service]
Type=simple
User=${USER}
Group=${USER}
WorkingDirectory=${HOME}
ExecStart=${cortex_bin} mcp start
Restart=on-failure
RestartSec=10
StartLimitIntervalSec=60
StartLimitBurst=3
Environment=PATH=${CORTEX_BIN_DIR}:/usr/local/bin:/usr/bin:/bin
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1
StandardOutput=journal
StandardError=journal
SyslogIdentifier=cortex-mcp
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload
    log_success "systemd service created at ${service_path}"

    if confirm "Enable and start the MCP service now?"; then
        sudo systemctl enable cortex-mcp
        sudo systemctl start cortex-mcp
        log_success "MCP service started"
    else
        log_info "Enable manually with: sudo systemctl enable --now cortex-mcp"
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# Final Verification
# ═══════════════════════════════════════════════════════════════════════════════

run_verification() {
    log_step "Running Verification"

    export PATH="${CORTEX_BIN_DIR}:${PATH}"

    local all_ok=true

    # Verify binary
    log_info "Testing CodeCortex binary..."
    if cortex --version &>/dev/null; then
        local version=$(cortex --version 2>&1 | head -1)
        log_success "Binary: ${version}"
    else
        log_error "Binary: FAILED"
        all_ok=false
    fi

    # Verify config
    log_info "Testing configuration..."
    if cortex config show &>/dev/null; then
        log_success "Configuration: Valid"
    else
        log_warning "Configuration: Check manually with 'cortex config show'"
    fi

    # Verify database connection (quick check)
    log_info "Testing database connection..."
    if docker ps --format '{{.Names}}' 2>/dev/null | grep -q "${CONTAINER_NAME}\|memgraph"; then
        log_success "Database: Container running"
    else
        log_warning "Database: Container not running"
        log_info "  Start with: docker start ${CONTAINER_NAME}"
    fi

    # Verify vector store path
    log_info "Testing vector store..."
    if [ -d "${VECTOR_PATH}" ]; then
        log_success "Vector store: Directory exists"
    else
        log_info "Vector store: Will be created on first use"
    fi

    # Verify LLM provider
    log_info "Testing LLM provider..."
    if command_exists ollama && curl -s http://127.0.0.1:11434/api/tags &>/dev/null; then
        log_success "LLM: Ollama running"
    elif [ -n "$OPENAI_API_KEY" ]; then
        log_success "LLM: OpenAI API key configured"
    else
        log_info "LLM: Not configured (optional for basic usage)"
    fi

    # Test MCP tools
    log_info "Listing MCP tools..."
    local tools=$(cortex mcp tools 2>/dev/null || echo "")
    if [ -n "$tools" ]; then
        local tool_count=$(echo "$tools" | wc -l | tr -d ' ')
        log_success "MCP tools: ${tool_count} available"
    else
        log_warning "MCP tools: Could not list"
    fi

    echo ""
    if [ "$all_ok" = true ]; then
        log_success "Verification complete!"
    else
        log_warning "Verification complete with some issues"
    fi
    echo ""
    log_info "Run 'cortex doctor' for detailed health check"
}

print_summary() {
    local os=$(get_os)

    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                    CodeCortex Installation Complete!                   ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "  ${CYAN}Binary:${NC}     ${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
    echo -e "  ${CYAN}Config:${NC}    ${CORTEX_CONFIG_DIR}/config.toml"
    echo -e "  ${CYAN}Vectors:${NC}   ${VECTOR_PATH}"
    echo -e "  ${CYAN}MCP Logs:${NC}  ${CORTEX_CONFIG_DIR}/logs/"
    echo ""
    echo -e "  ${YELLOW}Quick Start:${NC}"
    echo ""
    echo "    # Verify installation"
    echo "    cortex --version"
    echo "    cortex doctor"
    echo ""
    echo "    # Index a repository"
    echo "    cortex index /path/to/your/code"
    echo ""
    echo "    # Start MCP server"
    echo "    cortex mcp start"
    echo ""

    case "$os" in
        macos)
            echo -e "  ${YELLOW}Service Management (macOS):${NC}"
            echo ""
            echo "    # Start MCP service"
            echo "    launchctl start com.codecortex.mcp"
            echo ""
            echo "    # Stop MCP service"
            echo "    launchctl stop com.codecortex.mcp"
            echo ""
            echo "    # View logs"
            echo "    tail -f ${CORTEX_CONFIG_DIR}/logs/mcp.log"
            ;;
        linux)
            echo -e "  ${YELLOW}Service Management (Linux):${NC}"
            echo ""
            echo "    # Start MCP service"
            echo "    sudo systemctl start cortex-mcp"
            echo ""
            echo "    # Stop MCP service"
            echo "    sudo systemctl stop cortex-mcp"
            echo ""
            echo "    # View logs"
            echo "    sudo journalctl -u cortex-mcp -f"
            ;;
    esac

    echo ""
    if [ "$INSTALL_MEMGRAPH" = true ]; then
        echo -e "  ${YELLOW}Memgraph:${NC}"
        echo ""
        echo "    # Check Memgraph status (Docker)"
        echo "    docker ps | grep ${CONTAINER_NAME}"
        echo ""
        echo "    # Start Memgraph if stopped"
        echo "    docker start ${CONTAINER_NAME}"
        echo ""
    else
        echo -e "  ${YELLOW}Graph backend:${NC}"
        echo ""
        echo "    Update memgraph_uri in ~/.cortex/config.toml to point at your server."
        echo "    Then run: cortex doctor"
        echo ""
    fi

    if [ "$NON_INTERACTIVE" = true ]; then
        echo -e "  ${YELLOW}Note:${NC} Run 'cortex setup' for interactive configuration"
        echo ""
    fi

    echo -e "${GREEN}Happy coding!${NC}"
    echo ""
}

# ═══════════════════════════════════════════════════════════════════════════════
# Help
# ═══════════════════════════════════════════════════════════════════════════════

show_help() {
    echo "CodeCortex Installation Script v${CORTEX_VERSION}"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --non-interactive    Run without prompts (use defaults)"
    echo "  --no-nix             Do not use Nix even if installed"
    echo "  --memgraph           Install and start Memgraph via Docker (opt-in, default: off)"
    echo "  --no-memgraph        Explicitly skip Memgraph setup (suppresses interactive prompt)"
    echo "  --no-service         Skip launchd/systemd service setup"
    echo "  --help               Show this help message"
    echo ""
    echo "Memgraph / Docker behaviour:"
    echo "  By default the installer does NOT install Docker or Memgraph."
    echo "  If you have an existing Memgraph or Neo4j server (local or remote),"
    echo "  skip this step and update memgraph_uri in ~/.cortex/config.toml."
    echo "  Use --memgraph to have the installer pull and start a Memgraph container."
    echo ""
    echo "Examples:"
    echo "  $0                           # Interactive installation (asks about Memgraph)"
    echo "  $0 --non-interactive         # Non-interactive, skips Memgraph (configure manually)"
    echo "  $0 --memgraph                # Also install Memgraph via Docker"
    echo "  $0 --no-memgraph             # Suppress the Memgraph prompt entirely"
    echo "  $0 --no-nix                  # Force Cargo/binary installer path"
    echo ""
}

# ═══════════════════════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════════════════════

main() {
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
                # Explicitly requested: install Memgraph via Docker
                INSTALL_MEMGRAPH=true
                shift
                ;;
            --no-memgraph)
                # Explicitly skipped: suppress the interactive prompt too
                INSTALL_MEMGRAPH="skip"
                shift
                ;;
            --no-service)
                START_SERVICES=false
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

    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║              CodeCortex Installer v${CORTEX_VERSION}                          ║${NC}"
    echo -e "${CYAN}║                    $(get_os) ($(get_arch))                              ${NC}   ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Check dependencies
    check_dependencies

    # Install Rust if needed
    if [ "$INSTALL_METHOD" = "binary" ] && ! command_exists cargo; then
        if confirm "Install Rust for building from source?"; then
            install_rust
        else
            log_error "Cannot proceed without Rust or pre-built binaries"
            exit 1
        fi
    fi

    # Install CodeCortex
    case "$INSTALL_METHOD" in
        nix)
            install_cortex_nix
            ;;
        cargo)
            install_cortex_cargo
            ;;
        *)
            install_cortex_binary
            ;;
    esac

    # Verify installation
    verify_installation

    # Setup Memgraph
    setup_memgraph

    # Setup configuration
    setup_config

    # Setup MCP configuration
    setup_mcp_config

    # Setup service
    setup_service

    # Run verification
    run_verification

    # Print summary
    print_summary
}

main "$@"
