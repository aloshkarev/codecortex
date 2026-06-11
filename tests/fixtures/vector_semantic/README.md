# Vector semantic test fixture

Minimal Rust repo for PR MCP vector semantic oracles. Used with `CORTEX_TEST_EMBEDDER=1`.

## Anchor table

| NL query | Expected top hit | Must NOT rank first |
| --- | --- | --- |
| `validate user session token` | `src/auth.rs` / `validate_session_token` | `noise.rs` |
| `refund payment processing` | `src/payments.rs` / `process_refund` | `auth.rs` |
| `completely unrelated quantum physics` | (no anchor file in top-k) | `auth.rs`, `payments.rs` |

## Bootstrap (operator)

```bash
export CORTEX_TEST_EMBEDDER=1
export CORTEX_TEST_GRAPH=1
FIXTURE="$(pwd)/tests/fixtures/vector_semantic"
cortex index "$FIXTURE" --force
cortex vector-index "$FIXTURE"
make mcp-vector-semantic-pr FIXTURE="$FIXTURE"
```
