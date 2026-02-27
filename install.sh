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

set -e

# ═══════════════════════════════════════════════════════════════════════════════
# Configuration
# ═══════════════════════════════════════════════════════════════════════════════

CORTEX_VERSION="0.1.0"
CORTEX_BIN_NAME="cortex"
CORTEX_CONFIG_DIR="${HOME}/.cortex"
CORTEX_BIN_DIR="${HOME}/.local/bin"
CORTEX_DATA_DIR="${HOME}/.cortex/data"
MEMGRAPH_URI="bolt://localhost:7687"
MEMGRAPH_USER=""
MEMGRAPH_PASSWORD=""

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
INSTALL_MEMGRAPH=true

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

# ═══════════════════════════════════════════════════════════════════════════════
# Dependency Checking
# ═══════════════════════════════════════════════════════════════════════════════

check_dependencies() {
    log_step "Checking Dependencies"

    local missing_deps=()
    local os=$(get_os)

    # Required dependencies
    local required=("curl" "git")

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

    # Check for Rust (optional, for cargo install)
    if command_exists rustc && command_exists cargo; then
        log_success "Rust toolchain found: $(rustc --version)"
        INSTALL_METHOD="cargo"
    else
        log_info "Rust not found - will use binary installation"
        INSTALL_METHOD="binary"
    fi

    # Check for Docker (for Memgraph)
    if command_exists docker; then
        log_success "Docker found: $(docker --version)"
    else
        log_warning "Docker not found - Memgraph installation will be limited"
        if [ "$INSTALL_MEMGRAPH" = true ]; then
            if ! confirm "Continue without Docker? (Memgraph will not be available)" "n"; then
                exit 1
            fi
            INSTALL_MEMGRAPH=false
        fi
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

    # Build release binary
    cargo build --release

    # Create bin directory if it doesn't exist
    mkdir -p "$CORTEX_BIN_DIR"

    # Copy binary
    cp target/release/cortex-cli "${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
    chmod +x "${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"

    log_success "CodeCortex installed to ${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
}

install_cortex_binary() {
    log_step "Installing CodeCortex Binary"

    local os=$(get_os)
    local arch=$(get_arch)
    local binary_url="https://github.com/codecortex/codecortex/releases/download/v${CORTEX_VERSION}/cortex-${os}-${arch}.tar.gz"

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

    # Pull Memgraph image
    log_info "Pulling Memgraph Docker image..."
    docker pull memgraph/memgraph:2.19.0

    # Check if Memgraph container already exists
    if docker ps -a --format '{{.Names}}' | grep -q '^memgraph$'; then
        log_info "Memgraph container already exists"

        if docker ps --format '{{.Names}}' | grep -q '^memgraph$'; then
            log_success "Memgraph is already running"
        else
            log_info "Starting existing Memgraph container..."
            docker start memgraph
        fi
    else
        log_info "Creating Memgraph container..."
        docker run -d \
            --name memgraph \
            -p 7687:7687 \
            -p 7444:7444 \
            -v memgraph_data:/var/lib/memgraph \
            memgraph/memgraph:2.19.0 \
            --also-log-to-stderr=true
    fi

    # Wait for Memgraph to be ready
    log_info "Waiting for Memgraph to be ready..."
    sleep 5

    # Verify connection
    if docker exec memgraph mgm_client 1 &>/dev/null; then
        log_success "Memgraph is running on bolt://localhost:7687"
    else
        log_warning "Memgraph started but connection test inconclusive"
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
    if [ "$INSTALL_MEMGRAPH" = false ]; then
        log_info "Skipping Memgraph installation"
        return 0
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

    local config_file="${CORTEX_CONFIG_DIR}/config.json"

    if [ -f "$config_file" ]; then
        log_info "Configuration file already exists at ${config_file}"
        return 0
    fi

    # Get Memgraph credentials
    if [ "$NON_INTERACTIVE" = false ]; then
        read -rp "Memgraph URI [${MEMGRAPH_URI}]: " uri_input
        MEMGRAPH_URI="${uri_input:-$MEMGRAPH_URI}"

        read -rp "Memgraph username (leave empty if none): " MEMGRAPH_USER
        read -rsp "Memgraph password (leave empty if none): " MEMGRAPH_PASSWORD
        echo
    fi

    # Create config file
    cat > "$config_file" <<EOF
{
  "memgraph_uri": "${MEMGRAPH_URI}",
  "memgraph_user": "${MEMGRAPH_USER}",
  "memgraph_password": "${MEMGRAPH_PASSWORD}",
  "max_batch_size": 1000
}
EOF

    log_success "Configuration saved to ${config_file}"
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
    <false/>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>${CORTEX_BIN_DIR}:/usr/local/bin:/usr/bin:/bin</string>
    </dict>
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
After=network.target memgraph.service docker.service
Wants=memgraph.service

[Service]
Type=simple
User=${USER}
WorkingDirectory=${HOME}
ExecStart=${cortex_bin} mcp start
Restart=on-failure
RestartSec=10
Environment=PATH=${CORTEX_BIN_DIR}:/usr/local/bin:/usr/bin:/bin

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

    # Verify binary
    log_info "Testing CodeCortex binary..."
    if cortex --version &>/dev/null; then
        log_success "CodeCortex binary: OK"
    else
        log_error "CodeCortex binary: FAILED"
        return 1
    fi

    # Verify config
    log_info "Testing configuration..."
    if cortex config show &>/dev/null; then
        log_success "Configuration: OK"
    else
        log_warning "Configuration: Check manually with 'cortex config show'"
    fi

    # Verify Memgraph connection
    log_info "Testing Memgraph connection..."
    if cortex doctor &>/dev/null; then
        log_success "Memgraph connection: OK"
    else
        log_warning "Memgraph connection: Check if Memgraph is running"
    fi

    # Test MCP tools
    log_info "Listing MCP tools..."
    local tools=$(cortex mcp tools 2>/dev/null || echo "unavailable")
    if [ "$tools" != "unavailable" ]; then
        log_success "MCP tools available: $(echo "$tools" | wc -l | tr -d ' ') tools"
    else
        log_warning "MCP tools: Could not list tools"
    fi
}

print_summary() {
    local os=$(get_os)

    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                    CodeCortex Installation Complete!                   ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "  ${CYAN}Binary:${NC}     ${CORTEX_BIN_DIR}/${CORTEX_BIN_NAME}"
    echo -e "  ${CYAN}Config:${NC}    ${CORTEX_CONFIG_DIR}/config.json"
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
    echo -e "  ${YELLOW}Memgraph:${NC}"
    echo ""
    echo "    # Check Memgraph status (Docker)"
    echo "    docker ps | grep memgraph"
    echo ""
    echo "    # Start Memgraph if stopped"
    echo "    docker start memgraph"
    echo ""
    echo -e "${GREEN}Happy coding! 🚀${NC}"
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
    echo "  --no-memgraph        Skip Memgraph installation"
    echo "  --no-service         Skip service setup"
    echo "  --help               Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                           # Interactive installation"
    echo "  $0 --non-interactive         # Non-interactive with defaults"
    echo "  $0 --no-memgraph             # Skip Memgraph setup"
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
            --no-memgraph)
                INSTALL_MEMGRAPH=false
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
    if [ "$INSTALL_METHOD" = "cargo" ]; then
        install_cortex_cargo
    else
        install_cortex_binary
    fi

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
