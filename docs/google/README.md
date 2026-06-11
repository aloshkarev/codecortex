# Vendored `google.api` / `google.protobuf` imports for `docs/a2a.proto`

Normative A2A proto: [`../a2a.proto`](../a2a.proto).

`protoc` needs these paths when codegen runs from `cortex-a2a` / `cortex-mcp` build scripts:

- `google/api/*` — copied from [googleapis](https://github.com/googleapis/googleapis) (HTTP annotations, field behavior, client).
- `google/protobuf/{empty,struct,timestamp}.proto` — vendored for `docs/a2a.proto`. `descriptor.proto` / `duration.proto` are **not** vendored (use the system `protoc` include path; upstream `main` breaks older compilers).

Refresh vendored files (uses `curl` with 15s connect / 120s total timeout):

```bash
./scripts/vendor-a2a-google-protos.sh
# or FORCE=1 ./scripts/vendor-a2a-google-protos.sh
```

At codegen time, `Struct`, `Timestamp`, and `Value` still map to `prost-types` via `extern_path` in each crate’s `build.rs`.
