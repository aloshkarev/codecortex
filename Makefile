.PHONY: all build install clean test release run-mcp run-falkordb stop-falkordb status help mcp-bootstrap mcp-smoke mcp-semantic-audit mcp-semantic-pr mcp-vector-semantic-pr retrieval-eval retrieval-eval-strict perf-regression mcp-audit-all cortexignore-git-oracle measure-init measure-session-start measure-session-end measure-report measure-mcp-capture measure-bootstrap fmt lint check nix-build nix-check

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
agent-pack:
	@./plugin/codecortex/scripts/sync-from-docs.sh

mcp-bootstrap:
	@if [ -z "$(REPO)" ]; then \
		echo "Usage: make mcp-bootstrap REPO=/path/to/repo"; \
		exit 1; \
	fi
	@./scripts/bootstrap-codecortex-mcp.sh "$(REPO)"

# Live MCP tool audit (77 tools); requires graph + Ollama; skips destructive tools by default
CORTEX_AUDIT_BIN ?= $(CURDIR)/target/release/cortex-cli
mcp-tool-audit:
	@if [ -z "$(REPO)" ]; then \
		echo "Usage: make mcp-tool-audit REPO=/path/to/repo"; \
		exit 1; \
	fi
	@CORTEX_TEST_EMBEDDER=1 CORTEX_TEST_GRAPH=1 CORTEX_AUDIT_REPO="$(REPO)" CORTEX_BIN="$(CORTEX_AUDIT_BIN)" python3 scripts/mcp_tool_audit.py

# Optional A2A chain audit (spawn + task-dependent tools)
mcp-tool-audit-a2a:
	@if [ -z "$(REPO)" ]; then \
		echo "Usage: make mcp-tool-audit-a2a REPO=/path/to/repo"; \
		exit 1; \
	fi
	@CORTEX_AUDIT_REPO="$(REPO)" CORTEX_BIN="$(CORTEX_AUDIT_BIN)" python3 scripts/mcp_tool_audit.py --a2a-chain

# Semantic MCP audit (oracles.json); PROFILE=pr (~21 tools) or nightly (77)
CORTEX_SEMANTIC_PROFILE ?= pr
mcp-semantic-audit:
	@if [ -z "$(REPO)" ]; then \
		echo "Usage: make mcp-semantic-audit REPO=/path/to/repo [PROFILE=pr|nightly]"; \
		exit 1; \
	fi
	@CORTEX_SEMANTIC_REPO="$(REPO)" CORTEX_SEMANTIC_PROFILE="$(CORTEX_SEMANTIC_PROFILE)" CORTEX_BIN="$(CORTEX_AUDIT_BIN)" python3 scripts/mcp_semantic_audit.py --profile "$(CORTEX_SEMANTIC_PROFILE)" --repo "$(REPO)"

mcp-semantic-pr:
	@$(MAKE) mcp-semantic-audit REPO="$(REPO)" PROFILE=pr CORTEX_AUDIT_BIN="$(CORTEX_AUDIT_BIN)"

# Vector semantic PR gate (fixture + HashEmbedder)
CORTEX_SEMANTIC_FIXTURE ?= $(CURDIR)/tests/fixtures/vector_semantic
mcp-vector-semantic-pr:
	@CORTEX_TEST_EMBEDDER=1 CORTEX_TEST_GRAPH=1 \
		CORTEX_SEMANTIC_FIXTURE="$(CORTEX_SEMANTIC_FIXTURE)" \
		CORTEX_BIN="$(CORTEX_AUDIT_BIN)" \
		python3 scripts/mcp_semantic_audit.py \
		--profile vector_pr \
		--fixture "$(CORTEX_SEMANTIC_FIXTURE)" \
		--bootstrap-fixture

# Retrieval-quality eval (curated cases in tests/retrieval/retrieval.yaml)
CORTEX_RETRIEVAL_REPO ?= $(CURDIR)
retrieval-eval:
	@CORTEX_RETRIEVAL_REPO="$(CORTEX_RETRIEVAL_REPO)" CORTEX_BIN="$(CORTEX_AUDIT_BIN)" \
		python3 scripts/retrieval_eval.py --repo "$(CORTEX_RETRIEVAL_REPO)" --token-efficiency

retrieval-eval-strict:
	@CORTEX_TEST_GRAPH=1 CORTEX_TEST_EMBEDDER=1 \
		CORTEX_RETRIEVAL_REPO="$(CORTEX_RETRIEVAL_REPO)" CORTEX_BIN="$(CORTEX_AUDIT_BIN)" \
		python3 scripts/retrieval_eval.py \
		--repo "$(CORTEX_RETRIEVAL_REPO)" \
		--token-efficiency \
		--strict

# Performance regression gate (Criterion scenarios + budget checker)
perf-regression:
	@cargo bench -p cortex-benches --bench performance_scenarios -- --sample-size 10
	@python3 scripts/measurement/check_perf_regression.py --strict

# Full PR audit gate: smoke + graph semantic + vector semantic
mcp-audit-all:
	@if [ -z "$(REPO)" ]; then \
		echo "Usage: make mcp-audit-all REPO=/path/to/repo"; \
		exit 1; \
	fi
	@$(MAKE) mcp-tool-audit REPO="$(REPO)" CORTEX_AUDIT_BIN="$(CORTEX_AUDIT_BIN)"
	@$(MAKE) mcp-semantic-pr REPO="$(REPO)" CORTEX_AUDIT_BIN="$(CORTEX_AUDIT_BIN)"
	@CORTEX_TEST_EMBEDDER=1 $(MAKE) mcp-vector-semantic-pr CORTEX_AUDIT_BIN="$(CORTEX_AUDIT_BIN)"

# Local/CI: git check-ignore oracle for cortexignore hierarchical tests
cortexignore-git-oracle:
	cargo test -p cortex-core gitignore_oracle -- --ignored

# Nightly: 77/77 on disposable fixture (long + destructive dry-run + A2A chain)
mcp-nightly-audit:
	@chmod +x scripts/nightly-mcp-audit.sh
	@CORTEX_BIN="$(CORTEX_AUDIT_BIN)" ./scripts/nightly-mcp-audit.sh

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

# Start FalkorDB with Docker
run-falkordb:
	@docker ps --format '{{.Names}}' | grep -q '^codecortex-falkordb$$' && echo "FalkorDB already running" || \
		docker run -d --name codecortex-falkordb -p 6379:6379 falkordb/falkordb:latest

# Stop FalkorDB
stop-falkordb:
	@docker stop codecortex-falkordb 2>/dev/null || echo "FalkorDB not running"

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
	@echo "FalkorDB (Docker):"
	@docker ps --filter name=codecortex-falkordb --format '  Status: {{.Status}}' 2>/dev/null || echo "  Not running"
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
setup: install run-falkordb
	@mkdir -p $(CONFIG_DIR)
	@if [ ! -f $(CONFIG_DIR)/config.toml ]; then \
		printf '%s\n' \
			'backend_type = "falkordb"' \
			'falkordb_uri = "falkor://127.0.0.1:6379"' \
			'falkordb_graph = "codecortex"' \
			'falkordb_password = ""' \
			'max_batch_size = 4096' \
			> $(CONFIG_DIR)/config.toml; \
		echo "Created $(CONFIG_DIR)/config.toml"; \
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
	@echo "  run-falkordb   Start FalkorDB with Docker"
	@echo "  stop-falkordb  Stop FalkorDB container"
	@echo "  status       Show installation status"
	@echo "  fmt          Format code"
	@echo "  lint         Run clippy linter"
	@echo "  check        Run fmt, lint, and test"
	@echo "  setup        Full development setup"
