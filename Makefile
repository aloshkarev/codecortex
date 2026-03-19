.PHONY: all build install clean test release run-mcp run-memgraph status help mcp-bootstrap mcp-smoke measure-init measure-session-start measure-session-end measure-report measure-mcp-capture measure-bootstrap fmt lint check nix-build nix-check

# Directories
BIN_DIR := $(HOME)/.local/bin
CONFIG_DIR := $(HOME)/.cortex

# Binary
CORTEX_BIN := $(BIN_DIR)/cortex
NIX_BIN := $(shell command -v nix 2>/dev/null)

# Default target
all: build

# Build debug binary
build:
	@if [ -n "$(NIX_BIN)" ]; then \
		nix build .#cortex; \
	else \
		cargo build; \
	fi

# Build release binary
release:
	@if [ -n "$(NIX_BIN)" ]; then \
		nix build .#cortex; \
	else \
		cargo build --release; \
	fi

# Run tests
test:
	@if [ -n "$(NIX_BIN)" ]; then \
		nix flake check --print-build-logs; \
	else \
		cargo test --workspace; \
	fi

# Install binary to local bin
install: release
	@mkdir -p $(BIN_DIR)
	@if [ -n "$(NIX_BIN)" ]; then \
		cp result/bin/cortex $(CORTEX_BIN); \
	else \
		cp target/release/cortex-cli $(CORTEX_BIN); \
	fi
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

# Bootstrap index + vector + MCP server for a repo
mcp-bootstrap:
	@if [ -z "$(REPO)" ]; then \
		echo "Usage: make mcp-bootstrap REPO=/path/to/repo"; \
		exit 1; \
	fi
	@./scripts/bootstrap-codecortex-mcp.sh "$(REPO)"

# Quick local MCP readiness check
mcp-smoke:
	@echo "Checking CLI health..."
	@$(CORTEX_BIN) doctor >/dev/null || (echo "doctor failed" && exit 1)
	@echo "Checking tools list..."
	@$(CORTEX_BIN) mcp tools >/dev/null || (echo "mcp tools failed" && exit 1)
	@echo "MCP smoke check passed"

# Initialize measurement kit database
measure-init:
	@python3 scripts/measurement/codecortex_measure.py $(if $(DB),--db "$(DB)",) init

# Start measurement session (MODE=baseline|cortex [SESSION=<id>])
measure-session-start:
	@if [ -z "$(MODE)" ]; then \
		echo "Usage: make measure-session-start MODE=baseline|cortex [SESSION=<id>]"; \
		exit 1; \
	fi
	@python3 scripts/measurement/codecortex_measure.py $(if $(DB),--db "$(DB)",) session-start \
		--mode "$(MODE)" \
		--repo-path "$(PWD)" \
		$(if $(SESSION),--session-id "$(SESSION)",)

# End measurement session (SESSION=<id>)
measure-session-end:
	@if [ -z "$(SESSION)" ]; then \
		echo "Usage: make measure-session-end SESSION=<id>"; \
		exit 1; \
	fi
	@python3 scripts/measurement/codecortex_measure.py $(if $(DB),--db "$(DB)",) session-end --session-id "$(SESSION)"

# Report measurement KPIs
measure-report:
	@python3 scripts/measurement/codecortex_measure.py $(if $(DB),--db "$(DB)",) report

# Start MCP server with capture logging (SESSION=<id>)
measure-mcp-capture:
	@if [ -z "$(SESSION)" ]; then \
		echo "Usage: make measure-mcp-capture SESSION=<id>"; \
		exit 1; \
	fi
	@./scripts/measurement/start_mcp_capture.sh "$(SESSION)"

# Bootstrap measurement flow in one command (MODE=baseline|cortex)
measure-bootstrap:
	@MODE_VAL="$(if $(MODE),$(MODE),cortex)"; \
	if [ "$$MODE_VAL" != "baseline" ] && [ "$$MODE_VAL" != "cortex" ]; then \
		echo "Usage: make measure-bootstrap [MODE=baseline|cortex] [DB=/path/to.db]"; \
		exit 1; \
	fi; \
	python3 scripts/measurement/codecortex_measure.py $(if $(DB),--db "$(DB)",) init >/dev/null; \
	SESSION_ID=$$(python3 scripts/measurement/codecortex_measure.py $(if $(DB),--db "$(DB)",) session-start --mode "$$MODE_VAL" --repo-path "$(PWD)" --assistant cursor); \
	echo "Measurement bootstrap complete"; \
	echo "  mode: $$MODE_VAL"; \
	echo "  session_id: $$SESSION_ID"; \
	echo ""; \
	echo "Next steps:"; \
	echo "  make measure-mcp-capture SESSION=$$SESSION_ID$(if $(DB), DB=$(DB),)"; \
	echo "  python3 scripts/measurement/codecortex_measure.py $(if $(DB),--db \"$(DB)\",) task-log --session-id $$SESSION_ID --task-key TASK-001 --category bugfix --minutes 20 --success true --rework false"; \
	echo "  python3 scripts/measurement/codecortex_measure.py $(if $(DB),--db \"$(DB)\",) tokens-import --session-id $$SESSION_ID --csv-path ./token-usage.csv --provider cursor"; \
	echo "  make measure-session-end SESSION=$$SESSION_ID$(if $(DB), DB=$(DB),)"; \
	echo "  make measure-report$(if $(DB), DB=$(DB),)"

# Start Memgraph with Docker
run-memgraph:
	@docker ps --format '{{.Names}}' | grep -q '^memgraph$$' && echo "Memgraph already running" || \
		docker run -d --name memgraph -p 7687:7687 -p 7444:7444 memgraph/memgraph-mage:3.8.1 --also-log-to-stderr=true

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
	@if [ -n "$(NIX_BIN)" ]; then \
		nix develop -c cargo fmt --all; \
	else \
		cargo fmt --all; \
	fi

# Run linter
lint:
	@if [ -n "$(NIX_BIN)" ]; then \
		nix develop -c cargo clippy --all-targets --all-features -- -D warnings; \
	else \
		cargo clippy --all-targets --all-features -- -D warnings; \
	fi

# Run all checks (fmt, lint, test)
check: fmt lint test

# Explicit Nix build/check helpers
nix-build:
	nix build .#cortex

nix-check:
	nix flake check --print-build-logs

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
	@echo "  mcp-bootstrap Bootstrap index/vector/MCP (REPO=/path)"
	@echo "  mcp-smoke    Run quick MCP readiness checks"
	@echo "  measure-init Initialize measurement DB"
	@echo "  measure-session-start Start baseline/cortex measurement session"
	@echo "  measure-session-end Close measurement session (SESSION=<id>)"
	@echo "  measure-mcp-capture Start MCP with log capture (SESSION=<id>)"
	@echo "  measure-report Show token/time/quality KPI report"
	@echo "  measure-bootstrap One-shot measurement session bootstrap"
	@echo "  run-memgraph Start Memgraph with Docker"
	@echo "  stop-memgraph Stop Memgraph container"
	@echo "  status       Show installation status"
	@echo "  fmt          Format code"
	@echo "  lint         Run clippy linter"
	@echo "  check        Run fmt, lint, and test"
	@echo "  setup        Full development setup"
