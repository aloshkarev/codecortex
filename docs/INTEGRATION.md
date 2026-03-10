# CodeCortex MCP Integrations

This guide provides practical one-line and production-grade integration paths for:

- Cursor
- Neovim
- Zed
- Claude Code
- Codex CLI
- Gemini CLI

It assumes `cortex` is installed and available in `PATH`.

## Recommended Runtime Model

Use one shared local runtime for all clients:

1. index graph data (`cortex index`)
2. index vector data (`cortex vector-index`)
3. expose MCP server (`cortex mcp start`)

This avoids per-client indexing drift and keeps tool results consistent.

## Universal One-Line Bootstrap

Replace `<repo>` with your repository path:

```bash
cortex doctor && cortex index "<repo>" && cortex vector-index "<repo>" && cortex mcp start
```

## Cursor

### Cursor one-line integration

```bash
mkdir -p ~/.cursor && printf '%s\n' '{' '  "mcpServers": {' '    "codecortex": {' '      "command": "cortex",' '      "args": ["mcp", "start"]' '    }' '  }' '}' > ~/.cursor/mcp.json
```

### Cursor production setup

- keep `~/.cursor/mcp.json` under dotfiles management
- add env vars as needed in the server block
- keep indexing in a separate terminal (watch/daemon), not on every prompt

## Neovim

### Neovim one-line integration

```bash
cortex mcp start
```

Then connect your MCP-capable Neovim AI plugin/client to stdio command `cortex mcp start`.

### Neovim production setup

- add a Neovim command wrapper (for example `:CortexMcpStart`) that runs `cortex mcp start`
- use one shared config per project for repository path and indexing policy

## Zed

### Zed one-line integration

Add this server in Zed settings (`context_servers`):

```json
{
  "context_servers": {
    "codecortex": {
      "source": "custom",
      "command": "cortex",
      "args": ["mcp", "start"],
      "env": {}
    }
  }
}
```

### Zed production setup

- store team-default server profile in docs
- validate server status in Agent Panel after startup

## Claude Code

### Claude Code one-line integration

```bash
claude mcp add cortex -- cortex mcp start
```

### Claude Code production setup

- use project scope for shared repositories
- limit tool surface where needed for safety and focus

## Codex CLI

### Codex CLI one-line integration

```bash
codex mcp add cortex -- cortex mcp start
```

### Codex CLI production setup

- configure `.codex/config.toml` for enabled/disabled tools and timeouts
- verify active servers via `codex mcp list`

## Gemini CLI

### Gemini CLI one-line integration

```bash
mkdir -p ~/.gemini && printf '%s\n' '{' '  "mcpServers": {' '    "codecortex": {' '      "command": "cortex",' '      "args": ["mcp", "start"],' '      "env": {}' '    }' '  }' '}' > ~/.gemini/settings.json
```

### Gemini CLI production setup

- prefer project-level `.gemini/settings.json` for team repos
- use allowlist/exclude MCP controls if required by policy

## How AI Agents Use MCP and Query the Database

Once the MCP server is configured (see Cursor/Claude sections above), the **agent discovers all CodeCortex tools automatically** via the MCP protocol. You do not call tools from the prompt directly; the model decides when to call them based on your question.

### Flow: Prompt → Tool Call → Database → Answer

1. **You ask in natural language** (e.g. in Cursor or Claude):  
   *“What calls `authenticate_user`?”* or *“Find code related to token refresh in auth.”*

2. **The agent picks a tool** from the list it received at startup (e.g. `get_impact_graph`, `find_code`, `vector_search`).

3. **The agent invokes the tool with parameters** (e.g. `symbol: "authenticate_user"`, `path: "src/auth"`).  
   The MCP server receives the request and runs the corresponding CodeCortex logic.

4. **CodeCortex queries the databases**:
   - **Graph (Memgraph)**: for structure—callers, callees, dependencies, impact graph, dead code. Tools like `get_impact_graph`, `find_code`, `analyze_code_relationships`, `execute_cypher_query` read from the graph.
   - **Vector store**: for semantic search over indexed code. Tools like `vector_search`, `vector_search_hybrid` run embedding + similarity search and return relevant snippets.

5. **The server returns JSON** (symbols, file paths, snippets, graph nodes/edges, explanations) to the agent.

6. **The agent uses that data in its reply**—citing files, summarizing call chains, or suggesting changes.

So **the prompt does not “extract” data itself**; it triggers the model to call the right tools, and those tools run the queries that read from the indexed graph and vector store.

### Tools That Read From the Database

| Goal | Example tools | Data source |
|------|----------------|-------------|
| Who calls / what does this symbol impact? | `get_impact_graph`, `analyze_code_relationships` | Graph |
| Find by name or pattern | `find_code` | Graph |
| Semantic “code like this” search | `vector_search`, `vector_search_hybrid` | Vector store (+ graph for hybrid) |
| Bounded context for a symbol | `get_context_capsule` | Graph + optional vectors |
| Dead code, complexity, refactors | `find_dead_code`, `calculate_cyclomatic_complexity`, `analyze_refactoring` | Graph |
| Raw graph query | `execute_cypher_query` | Graph |
| Session/memory | `get_session_context`, `search_memory` | Memory store |

Index the repo first so the databases are populated: `cortex index <repo>` and `cortex vector-index <repo>`. Then the agent’s tool calls will return useful results.

## Clarifying Tool Purpose for the AI Agent

With a working MCP service, the client (Cursor, Claude, etc.) receives tool metadata on `tools/list`. The **model uses that metadata** to decide which tool to call and with which arguments. Clarify purpose by making tool names, descriptions, and parameter schemas explicit.

### What the agent receives (MCP protocol)

For each tool the server sends:

- **name** — Unique identifier (e.g. `get_impact_graph`, `vector_search`). The agent uses this to invoke the tool.
- **description** — Free-text explanation of what the tool does, when to use it, and optionally what it returns. This is the main signal for tool selection.
- **inputSchema** — JSON Schema of parameters: types, required fields, and **per-property descriptions**. Good parameter descriptions reduce wrong or empty arguments.

Optional: **title** — Short display name (if the server sends it). Some runtimes show this in the UI.

CodeCortex sets the tool description via `#[tool(description = "...")]` and the input schema from the request structs (with `schemars::JsonSchema`). **Doc comments on request struct fields** (`/// ...`) become property descriptions in the schema, so the agent sees what each parameter is for.

### Recommendations (aligned with MCP best practices)

1. **Tool descriptions**
   - Write for someone who has not read the code: say **what** the tool does and **when** to use it (e.g. “Use when the user asks who calls X or what X affects”).
   - Mention what the tool returns in one line if it helps (e.g. “Returns callers, callees, and dependency graph”).
   - Avoid internal jargon; prefer terms the user would use (“call graph”, “dead code”, “semantic search”).

2. **Tool naming**
   - Use **snake_case** and **action verbs** (e.g. `get_impact_graph`, `vector_search`, `find_dead_code`). Names should disambiguate tools without wasting context.

3. **Parameter schemas**
   - Add **doc comments** (or `#[schemars(description = "...")]`) on every request struct field so the schema sent to the client includes parameter descriptions. Include allowed values when relevant (e.g. “One of: name | pattern | type | content”).

4. **Design around use cases**
   - One tool per user goal (e.g. “what calls this?” → `get_impact_graph`) rather than one per low-level API. That reduces wrong tool choice and failed calls.

5. **Errors and safety**
   - Return clear, human-readable error messages in tool results. The agent (and user) should understand why a call failed (e.g. “Repository not indexed”, “Symbol not found”).

References: [MCP specification – Tools](https://modelcontextprotocol.io/docs/concepts/tools), [MCP best practices](https://modelcontextprotocol.info/docs/best-practices/). CodeCortex tool descriptions and request struct docs are in `crates/cortex-mcp/src/handler.rs`; improving them there improves agent behavior for all MCP clients.

## Efficient Usage Patterns

1. Keep index fresh in background:
   - `cortex watch <repo>` or daemon-based flow
2. Use two-stage retrieval:
   - graph tools for structure/dependencies
   - vector tools for semantic expansion
   - direct MCP vector tools when available:
     - `vector_index_repository`, `vector_index_file`
     - `vector_search`, `vector_search_hybrid`
     - `vector_index_status`, `vector_delete_repository`
3. Use task profiles:
   - debug, refactor, review, onboarding
4. Control noise and cost:
   - branch-scoped indexing
   - include/exclude policy
   - depth limits for graph traversals

## Validation Matrix

For each platform, validate these 5 scenarios:

1. MCP server connection is active
2. `tools/list` returns CodeCortex tools
3. symbol lookup returns expected entity
4. impact/call-chain query works on indexed repo
5. diagnostics/health tool returns healthy status

## Runbook

### Start sequence

```bash
cortex doctor
cortex index "<repo>"
cortex vector-index "<repo>"
cortex mcp start
```

### Update sequence

```bash
cortex index "<repo>"
cortex vector-index "<repo>"
```

### Troubleshooting

- server not visible: verify client config path and command
- no results: verify indexing completed for target repo
- stale results: re-run index and vector-index, then restart MCP server
- backend failure: run `cortex doctor` and inspect Memgraph/vector store status
