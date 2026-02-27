# cortex-cli

Command-line interface for CodeCortex code intelligence system.

## Overview

This crate provides the CLI for interacting with CodeCortex, supporting indexing, search, analysis, and more.

## Installation

```bash
cargo install cortex-cli
```

## Commands

### Repository Management

```bash
# Index a repository
cortex index /path/to/repo

# List indexed repositories
cortex list

# Delete a repository
cortex delete /path/to/repo

# Show statistics
cortex stats
```

### Code Search

```bash
# Find by name
cortex find name "UserRepository"

# Find by pattern (regex)
cortex find pattern "impl.*Handler"

# Find by type
cortex find type Function

# Search in source code
cortex find content "SELECT * FROM"
```

### Analysis

```bash
# Find callers of a function
cortex analyze callers "process_request"

# Find callees
cortex analyze callees "main"

# Find call chain between symbols
cortex analyze chain --from "main" --to "db_query"

# Show class hierarchy
cortex analyze hierarchy "BaseHandler"

# Find dead code
cortex analyze dead-code

# Show complexity analysis
cortex analyze complexity --top 20
```

### Context Capsule

```bash
# Get context capsule for a symbol
cortex capsule "auth_handler"

# Get impact graph
cortex impact "UserService"

# Analyze refactoring
cortex refactor "process_payment"
```

### Pattern Detection

```bash
# Find design patterns
cortex patterns

# Find tests for a symbol
cortex test "UserService"
```

### Diagnostics

```bash
# Run diagnostics
cortex diagnose

# Check health
cortex doctor
```

### Bundle Operations

```bash
# Export graph data
cortex bundle export output.ccx --repo /path/to/repo

# Import graph data
cortex bundle import input.ccx
```

### Memory Operations

```bash
# Save an observation
cortex memory save "Important note about authentication"

# Search memory
cortex memory search "auth"

# Get session context
cortex memory session
```

### Configuration

```bash
# Show current config
cortex config show

# Set a config value
cortex config set key value

# Reset to defaults
cortex config reset
```

### Interactive Mode

```bash
cortex interactive
> find name Handler
Found 3 matches:
  1. Struct Handler at handler.rs:307
  2. Function Handler::new at handler.rs:315
  3. Function Handler::handle at handler.rs:340
> analyze callers 1
Callers of Handler:
  - main::run (main.rs:45)
  - server::handle_request (server.rs:120)
> stats
Repository: codecortex
Files: 156
Functions: 892
Classes: 45
> help
> exit
```

### Shell Completion

```bash
# Generate completion for bash
cortex completion bash > ~/.local/share/bash-completion/completions/cortex

# Generate completion for zsh
cortex completion zsh > "${fpath[1]}/_cortex"

# Generate completion for fish
cortex completion fish > ~/.config/fish/completions/cortex.fish

# Generate completion for PowerShell
cortex completion powershell > cortex.ps1

# Generate completion for Elvish
cortex completion elvish > cortex.elv
```

## Output Formats

All commands support multiple output formats:

```bash
cortex find name Handler --format json    # JSON (default)
cortex find name Handler --format yaml    # YAML
cortex find name Handler --format table   # Table
cortex find name Handler --format csv     # CSV
```

## Global Options

```bash
--json              Output as compact JSON
-v, --verbose       Increase verbosity (-v, -vv, -vvv)
--help              Show help
--version           Show version
```

## MCP Mode

Start the MCP server for AI assistant integration:

```bash
# Start MCP server
cortex mcp start

# List available tools
cortex mcp tools
```

## Dependencies

- `clap` - CLI framework with derive macros
- `clap-complete` - Shell completion generation
- `dialoguer` - Interactive prompts
- `indicatif` - Progress bars
- `owo-colors` - Colored output
- `rustyline` - Interactive mode (REPL)
- `serde_yaml` - YAML output format
- `comfy-table` - Table output format

## Tests

Run tests with:
```bash
cargo test -p cortex-cli -- --test-threads=1
```
