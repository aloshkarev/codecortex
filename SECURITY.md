# Security Policy

> This document describes supported versions, the vulnerability reporting process, and the security model for CodeCortex — including MCP network transport hardening.

## Supported Versions

Security patches are applied to the latest minor release. Older versions may receive fixes on a best-effort basis.

| Version | Supported |
|---------|-----------|
| 1.0.x | Yes |

## Reporting a Vulnerability

**Do not open a public issue for security vulnerabilities.**

### How to report

1. **Email** the maintainer: [mailbox@aloshkarev.com](mailto:mailbox@aloshkarev.com)
2. **Include**:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to expect

- Acknowledgement of receipt within a reasonable timeframe
- Collaboration to understand and validate the report
- Progress updates as the fix is developed
- Credit in the security advisory (unless you prefer anonymity)

### Disclosure

We aim to fix critical vulnerabilities promptly and coordinate public disclosure with you. Disclosure typically occurs after a patch is available and deployed.

## MCP Transport Security Model

CodeCortex exposes an MCP server over stdio, HTTP-SSE, WebSocket, or all three simultaneously. Understanding the security boundaries is essential before deploying in shared or network-accessible environments.

### Default behavior (stdio)

The default transport is `stdio`. It binds to no network port and is only accessible to the local process that launches it. This is the recommended mode for single-user workstations.

```bash
cortex mcp start   # stdio — no network exposure
```

### Network transports

When using `--transport http-sse`, `--transport websocket`, or `--transport multi`:

- **Default bind is loopback only** (`127.0.0.1:3001`). Requests from remote hosts are rejected.
- **Non-loopback bind requires `--allow-remote`**. Never expose to `0.0.0.0` without this flag being an explicit operator decision.
- **Bearer token authentication** is available via `--token <value>` or `--token-env <ENV_NAME>`. All HTTP and WebSocket requests must include `Authorization: Bearer <token>`.
- **TLS is not terminated by CodeCortex**. Use a TLS-terminating reverse proxy (nginx, Caddy, Traefik) in front of CodeCortex for any network-accessible deployment.

```bash
# Localhost-only network server with bearer token
cortex mcp start \
  --transport http-sse \
  --listen 127.0.0.1:3001 \
  --token-env CORTEX_MCP_TOKEN

# Remote-accessible server (requires TLS proxy in front)
cortex mcp start \
  --transport multi \
  --listen 0.0.0.0:3001 \
  --allow-remote \
  --token-env CORTEX_MCP_TOKEN
```

### Production hardening checklist

- [ ] Use `stdio` transport when possible (single workstation, local AI client)
- [ ] If using network transports, bind to loopback (`127.0.0.1`) unless remote access is required
- [ ] Always set `--token-env` for HTTP and WebSocket transports
- [ ] Terminate TLS at a reverse proxy — never expose CodeCortex directly on a public port
- [ ] Restrict `--allow-remote` to known network segments with firewall rules
- [ ] Rotate bearer tokens periodically; use `--token-env` to avoid tokens in shell history

## General Security Considerations

- **Credentials**: Never commit secrets, tokens, or production credentials. Use environment variables or the `~/.cortex/config.toml` file (set file permissions to `600`).
- **Input validation**: All external input is validated and sanitized. Graph queries use parameterized Cypher to prevent injection.
- **Dependencies**: We use `cargo audit` and keep dependencies up to date. Run `cargo audit` locally to check for known vulnerabilities.
- **Config file permissions**: `~/.cortex/config.toml` may contain database credentials. Restrict to owner-read: `chmod 600 ~/.cortex/config.toml`.
