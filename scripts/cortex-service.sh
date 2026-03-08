#!/usr/bin/env bash
#
# CodeCortex Service Management Script
# Unified service control for macOS and Linux
#
# Usage:
#   ./scripts/cortex-service.sh [start|stop|restart|status|logs|install|uninstall]
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
BIN_DIR="${HOME}/.local/bin"
CORTEX_BIN="${BIN_DIR}/cortex"
CONFIG_DIR="${HOME}/.cortex"
LOG_DIR="${CONFIG_DIR}/logs"
REPO_URL="https://github.com/aloshkarev/codecortex"

# Detect OS
get_os() {
    case "$(uname -s)" in
        Darwin*) echo "macos" ;;
        Linux*)  echo "linux" ;;
        *)       echo "unknown" ;;
    esac
}

OS=$(get_os)

# Service names
SERVICE_NAME="cortex-mcp"
LAUNCHD_LABEL="com.codecortex.mcp"
PLIST_PATH="${HOME}/Library/LaunchAgents/${LAUNCHD_LABEL}.plist"
SYSTEMD_SERVICE="/etc/systemd/system/${SERVICE_NAME}.service"

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_error() { echo -e "${RED}[ERR]${NC} $1"; }

# ═══════════════════════════════════════════════════════════════════════════════
# macOS Functions
# ═══════════════════════════════════════════════════════════════════════════════

macos_start() {
    if [ -f "$PLIST_PATH" ]; then
        launchctl load "$PLIST_PATH" 2>/dev/null || true
        launchctl start "$LAUNCHD_LABEL"
        log_success "MCP service started"
    else
        log_error "Service not installed. Run: $0 install"
    fi
}

macos_stop() {
    launchctl stop "$LAUNCHD_LABEL" 2>/dev/null || true
    launchctl unload "$PLIST_PATH" 2>/dev/null || true
    log_success "MCP service stopped"
}

macos_restart() {
    macos_stop
    sleep 1
    macos_start
}

macos_status() {
    if launchctl list "$LAUNCHD_LABEL" &>/dev/null; then
        echo -e "Status: ${GREEN}Running${NC}"
        local pid=$(launchctl list "$LAUNCHD_LABEL" 2>/dev/null | grep -o '"PID"[0-9]*' | grep -o '[0-9]*')
        [ -n "$pid" ] && echo "PID: $pid"
    else
        echo -e "Status: ${YELLOW}Stopped${NC}"
    fi
}

macos_logs() {
    local log_file="${LOG_DIR}/mcp.log"
    if [ -f "$log_file" ]; then
        tail -f "$log_file"
    else
        log_error "Log file not found: $log_file"
    fi
}

macos_install() {
    mkdir -p "$(dirname "$PLIST_PATH")"
    mkdir -p "$LOG_DIR"

    cat > "$PLIST_PATH" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${LAUNCHD_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${CORTEX_BIN}</string>
        <string>mcp</string>
        <string>start</string>
    </array>
    <key>WorkingDirectory</key>
    <string>${HOME}</string>
    <key>StandardOutPath</key>
    <string>${LOG_DIR}/mcp.log</string>
    <key>StandardErrorPath</key>
    <string>${LOG_DIR}/mcp.log</string>
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
        <string>${BIN_DIR}:/usr/local/bin:/usr/bin:/bin</string>
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

    log_success "Service installed: $PLIST_PATH"
    log_info "Start with: $0 start"
}

macos_uninstall() {
    macos_stop
    rm -f "$PLIST_PATH"
    log_success "Service uninstalled"
}

# ═══════════════════════════════════════════════════════════════════════════════
# Linux Functions
# ═══════════════════════════════════════════════════════════════════════════════

linux_start() {
    if [ -f "$SYSTEMD_SERVICE" ]; then
        sudo systemctl start "$SERVICE_NAME"
        log_success "MCP service started"
    else
        log_error "Service not installed. Run: $0 install"
    fi
}

linux_stop() {
    sudo systemctl stop "$SERVICE_NAME" 2>/dev/null || true
    log_success "MCP service stopped"
}

linux_restart() {
    sudo systemctl restart "$SERVICE_NAME"
    log_success "MCP service restarted"
}

linux_status() {
    sudo systemctl status "$SERVICE_NAME" --no-pager 2>/dev/null || echo -e "Status: ${YELLOW}Not installed${NC}"
}

linux_logs() {
    sudo journalctl -u "$SERVICE_NAME" -f
}

linux_install() {
    if [ ! -w /etc/systemd/system ]; then
        log_error "Root access required for systemd service installation"
        exit 1
    fi

    cat | sudo tee "$SYSTEMD_SERVICE" > /dev/null <<EOF
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
ExecStart=${CORTEX_BIN} mcp start
Restart=on-failure
RestartSec=10
StartLimitIntervalSec=60
StartLimitBurst=3
Environment=PATH=${BIN_DIR}:/usr/local/bin:/usr/bin:/bin
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
    log_success "Service installed: $SYSTEMD_SERVICE"
    log_info "Enable with: sudo systemctl enable $SERVICE_NAME"
    log_info "Start with: $0 start"
}

linux_uninstall() {
    sudo systemctl stop "$SERVICE_NAME" 2>/dev/null || true
    sudo systemctl disable "$SERVICE_NAME" 2>/dev/null || true
    sudo rm -f "$SYSTEMD_SERVICE"
    sudo systemctl daemon-reload
    log_success "Service uninstalled"
}

# ═══════════════════════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════════════════════

show_help() {
    echo "CodeCortex Service Management"
    echo ""
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  start       Start the MCP service"
    echo "  stop        Stop the MCP service"
    echo "  restart     Restart the MCP service"
    echo "  status      Show service status"
    echo "  logs        Follow service logs"
    echo "  install     Install the service"
    echo "  uninstall   Uninstall the service"
    echo ""
    echo "Platform: $OS"
}

# Check if cortex binary exists
check_binary() {
    if [ ! -f "$CORTEX_BIN" ]; then
        log_error "CodeCortex binary not found: $CORTEX_BIN"
        log_info "Install with: make install"
        exit 1
    fi
}

case "${1:-}" in
    start)
        check_binary
        case "$OS" in
            macos) macos_start ;;
            linux) linux_start ;;
            *)     log_error "Unsupported OS: $OS" ;;
        esac
        ;;
    stop)
        case "$OS" in
            macos) macos_stop ;;
            linux) linux_stop ;;
            *)     log_error "Unsupported OS: $OS" ;;
        esac
        ;;
    restart)
        check_binary
        case "$OS" in
            macos) macos_restart ;;
            linux) linux_restart ;;
            *)     log_error "Unsupported OS: $OS" ;;
        esac
        ;;
    status)
        case "$OS" in
            macos) macos_status ;;
            linux) linux_status ;;
            *)     log_error "Unsupported OS: $OS" ;;
        esac
        ;;
    logs)
        case "$OS" in
            macos) macos_logs ;;
            linux) linux_logs ;;
            *)     log_error "Unsupported OS: $OS" ;;
        esac
        ;;
    install)
        check_binary
        case "$OS" in
            macos) macos_install ;;
            linux) linux_install ;;
            *)     log_error "Unsupported OS: $OS" ;;
        esac
        ;;
    uninstall)
        case "$OS" in
            macos) macos_uninstall ;;
            linux) linux_uninstall ;;
            *)     log_error "Unsupported OS: $OS" ;;
        esac
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        show_help
        exit 1
        ;;
esac
