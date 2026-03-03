.PHONY: all build install clean test release run-mcp run-memgraph status help

# Directories
BIN_DIR := $(HOME)/.local/bin
CONFIG_DIR := $(HOME)/.cortex

# Binary
CORTEX_BIN := $(BIN_DIR)/cortex

# Default target
all: build

# Build debug binary
build:
	cargo build

# Build release binary
release:
	cargo build --release

# Run tests
test:
	cargo test --workspace

# Install binary to local bin
install: release
	@mkdir -p $(BIN_DIR)
	@cp target/release/cortex-cli $(CORTEX_BIN)
	@chmod +x $(CORTEX_BIN)
	@echo "Installed to $(CORTEX_BIN)"

# Uninstall binary
uninstall:
	@rm -f $(CORTEX_BIN)
	@echo "Removed $(CORTEX_BIN)"

# Clean build artifacts
clean:
	cargo clean

# Start MCP server
run-mcp:
	cargo run -p cortex-cli -- mcp start

# Start Memgraph with Docker
run-memgraph:
	@docker ps --format '{{.Names}}' | grep -q '^memgraph$$' && echo "Memgraph already running" || \
		docker run -d --name memgraph -p 7687:7687 -p 7444:7444 memgraph/memgraph:3.8.1 --also-log-to-stderr=true

# Stop Memgraph
stop-memgraph:
	@docker stop memgraph 2>/dev/null || echo "Memgraph not running"

# Show status
status:
	@echo "=== CodeCortex Status ==="
	@echo ""
	@echo "Binary:"
	@$(CORTEX_BIN) --version 2>/dev/null || echo "  Not installed"
	@echo ""
	@echo "Configuration:"
	@$(CORTEX_BIN) config show 2>/dev/null || echo "  Not configured"
	@echo ""
	@echo "Memgraph (Docker):"
	@docker ps --filter name=memgraph --format '  Status: {{.Status}}' 2>/dev/null || echo "  Not running"
	@echo ""
	@echo "MCP Tools: $$( $(CORTEX_BIN) mcp tools 2>/dev/null | wc -l | tr -d ' ' ) available"

# Format code
fmt:
	cargo fmt --all

# Run linter
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Run all checks (fmt, lint, test)
check: fmt lint test

# Development setup
setup: install run-memgraph
	@mkdir -p $(CONFIG_DIR)
	@if [ ! -f $(CONFIG_DIR)/config.json ]; then \
		echo '{"memgraph_uri":"bolt://localhost:7687","memgraph_user":"","memgraph_password":"","max_batch_size":1000}' > $(CONFIG_DIR)/config.json; \
		echo "Created $(CONFIG_DIR)/config.json"; \
	fi
	@echo ""
	@echo "Setup complete! Run 'cortex doctor' to verify."

# Help
help:
	@echo "CodeCortex Makefile"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  build        Build debug binary"
	@echo "  release      Build release binary"
	@echo "  test         Run tests"
	@echo "  install      Install binary to ~/.local/bin"
	@echo "  uninstall    Remove installed binary"
	@echo "  clean        Remove build artifacts"
	@echo "  run-mcp      Start MCP server"
	@echo "  run-memgraph Start Memgraph with Docker"
	@echo "  stop-memgraph Stop Memgraph container"
	@echo "  status       Show installation status"
	@echo "  fmt          Format code"
	@echo "  lint         Run clippy linter"
	@echo "  check        Run fmt, lint, and test"
	@echo "  setup        Full development setup"
