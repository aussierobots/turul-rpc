# turul-rpc

> Typed JSON-RPC 2.0 framework for Rust. Handlers return domain errors; the dispatcher owns the wire.

`turul-rpc` is the generic RPC core extracted from the
[turul-mcp-framework](https://github.com/aussierobots/turul-mcp-framework). It powers
`turul-mcp` but knows nothing about MCP, and is usable on its own.

## Crate layout

| Crate | Purpose |
|---|---|
| [`turul-rpc-core`](crates/turul-rpc-core)       | Pure JSON-RPC 2.0 wire types — request, response, notification, error. No async, no codec helpers. |
| [`turul-rpc-jsonrpc`](crates/turul-rpc-jsonrpc) | JSON-RPC 2.0 codec — parser, batch, error mapping per spec. Depends on `core`. |
| [`turul-rpc-server`](crates/turul-rpc-server)   | Async dispatcher, handler trait, session context, optional streaming. Depends on `core` + `jsonrpc`. |
| [`turul-rpc`](crates/turul-rpc)                 | Facade — single import path. `pub use` of the three crates above. |

Most consumers depend on `turul-rpc` (the facade). The split crates exist so you can pull in only the wire types (e.g. for a client) without dragging in the async runtime.

## Why

Most JSON-RPC crates either hand you raw envelopes or hide the wire entirely.
`turul-rpc` keeps a hard line between the two:

1. **Handlers return `Result<Value, YourError>`** — never `JsonRpcError`.
2. **Dispatcher converts `YourError → JsonRpcError`** via your `ToJsonRpcError` impl. One boundary, one direction.
3. **Transport-agnostic.** Bring your own HTTP/SSE/stdio/Lambda. The crate is pure dispatch and types.
4. **JSON-RPC 2.0 batch** is implemented and tested per spec (§6).

## Compliance posture

`turul-rpc 0.1` implements JSON-RPC 2.0 with **one documented departure**:
incoming requests with `"id": null` are rejected as `Invalid Request`
(`-32600`). The spec permits null ids but discourages them; this crate
takes the strict line at the type level (`RequestId = {String, Number}`).
Server-emitted error responses correctly use `id: null` for unparseable
or unidentifiable requests as the spec requires. See
[ADR-002](docs/adr/002-json-rpc-2-compliance.md) for full rationale and
the v0.2 plan to surface a permissive codec-level type for callers who
need to accept null-id requests.

## Quick start

```rust
use turul_rpc::{JsonRpcDispatcher, JsonRpcHandler, RequestParams, SessionContext};
use turul_rpc::error::JsonRpcErrorObject;
use turul_rpc::r#async::ToJsonRpcError;
use async_trait::async_trait;
use serde_json::{json, Value};

#[derive(thiserror::Error, Debug)]
enum CalcError {
    #[error("bad params: {0}")]
    BadParams(String),
}

impl ToJsonRpcError for CalcError {
    fn to_error_object(&self) -> JsonRpcErrorObject {
        match self {
            CalcError::BadParams(m) => JsonRpcErrorObject::invalid_params(m),
        }
    }
}

struct Calc;

#[async_trait]
impl JsonRpcHandler for Calc {
    type Error = CalcError;

    async fn handle(
        &self,
        method: &str,
        params: Option<RequestParams>,
        _session: Option<SessionContext>,
    ) -> Result<Value, CalcError> {
        match method {
            "add" => {
                let p = params.ok_or_else(|| CalcError::BadParams("missing".into()))?;
                let m = p.to_map();
                let a = m.get("a").and_then(|v| v.as_f64())
                    .ok_or_else(|| CalcError::BadParams("a".into()))?;
                let b = m.get("b").and_then(|v| v.as_f64())
                    .ok_or_else(|| CalcError::BadParams("b".into()))?;
                Ok(json!({ "result": a + b }))
            }
            _ => Err(CalcError::BadParams(format!("unknown method {method}"))),
        }
    }

    fn supported_methods(&self) -> Vec<String> { vec!["add".into()] }
}
```

## Relationship to turul-mcp

`turul-mcp-server` is built on top of `turul-rpc`. If you want MCP semantics
(tools, resources, prompts, sessions, the Inspector flow), reach for
[`turul-mcp-server`](https://crates.io/crates/turul-mcp-server) — it pulls
`turul-rpc` in transitively.

For existing 0.3.x users of `turul-mcp-json-rpc-server`: nothing to do. That
crate ships as a thin re-export shim over `turul-rpc` from
turul-mcp-framework 0.3.39 onward. See [ADR-003](docs/adr/003-compatibility-with-turul-mcp-json-rpc-server.md).

## Architecture decisions

See [`docs/adr/`](docs/adr/) for the four ADRs that govern this workspace:

- [ADR-001 — Crate boundaries](docs/adr/001-crate-boundaries.md)
- [ADR-002 — JSON-RPC 2.0 compliance](docs/adr/002-json-rpc-2-compliance.md)
- [ADR-003 — Compatibility with turul-mcp-json-rpc-server](docs/adr/003-compatibility-with-turul-mcp-json-rpc-server.md)
- [ADR-004 — Non-goals for v0.1](docs/adr/004-non-goals-for-v0-1.md)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
