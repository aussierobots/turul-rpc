# turul-rpc

> Typed JSON-RPC 2.0 framework for Rust. Handlers return domain errors;
> the dispatcher owns the wire.

This is the **facade** crate of the `turul-rpc` family. It re-exports
[`turul-rpc-core`], [`turul-rpc-jsonrpc`], and [`turul-rpc-server`]
under a single import path. **Most consumers should depend on this
crate**; the split crates exist so you can pull in only the wire types
(no async runtime) when needed.

[`turul-rpc-core`]: https://crates.io/crates/turul-rpc-core
[`turul-rpc-jsonrpc`]: https://crates.io/crates/turul-rpc-jsonrpc
[`turul-rpc-server`]: https://crates.io/crates/turul-rpc-server

## Why

Most JSON-RPC crates either hand you raw envelopes or hide the wire
entirely. `turul-rpc` keeps a hard line between the two:

1. **Handlers return `Result<Value, YourError>`** — never `JsonRpcError`.
2. **Dispatcher converts `YourError → JsonRpcError`** via your
   `ToJsonRpcError` impl. One boundary, one direction.
3. **Transport-agnostic.** Bring your own HTTP / SSE / stdio / Lambda.
   The crate is pure dispatch and types.
4. **JSON-RPC 2.0 batch** is implemented and tested per spec (§6).

## Quick start

```rust,no_run
use turul_rpc::{JsonRpcDispatcher, JsonRpcHandler, RequestParams, SessionContext};
use turul_rpc::error::JsonRpcErrorObject;
use turul_rpc::r#async::ToJsonRpcError;
use async_trait::async_trait;
use serde_json::{json, Value};

#[derive(thiserror::Error, Debug)]
enum CalcError {
    #[error("bad params: {0}")]
    BadParams(&'static str),
}

impl ToJsonRpcError for CalcError {
    fn to_error_object(&self) -> JsonRpcErrorObject {
        match self {
            CalcError::BadParams(m) => JsonRpcErrorObject::invalid_params(m),
        }
    }
}

struct Add;

#[async_trait]
impl JsonRpcHandler for Add {
    type Error = CalcError;

    async fn handle(
        &self,
        _method: &str,
        params: Option<RequestParams>,
        _session: Option<SessionContext>,
    ) -> Result<Value, CalcError> {
        let m = params.ok_or(CalcError::BadParams("missing"))?.to_map();
        let a = m.get("a").and_then(Value::as_f64).ok_or(CalcError::BadParams("a"))?;
        let b = m.get("b").and_then(Value::as_f64).ok_or(CalcError::BadParams("b"))?;
        Ok(json!({ "sum": a + b }))
    }
}

# async fn run() {
let mut d: JsonRpcDispatcher<CalcError> = JsonRpcDispatcher::new();
d.register_method("add".into(), Add);
# }
```

## Runnable examples

Three examples ship with the crate:

```bash
cargo run -p turul-rpc --example simple_calculator      # stdin REPL
cargo run -p turul-rpc --example batch_dispatch         # §6 batch demo
cargo run -p turul-rpc --example in_process_round_trip  # client+server pattern
```

`in_process_round_trip` shows the calling pattern — id generation,
request construction, serialization, dispatch, response parsing, and
id correlation — using only the crate's existing types. It is the
template a future `turul-rpc-client` crate would build on; for v0.1
the pattern itself is enough.

## Compliance posture

JSON-RPC 2.0 with **one documented departure**: incoming requests with
`"id": null` are rejected as Invalid Request (`-32600`). The spec
permits null id but discourages it; this crate takes the strict line at
the type level. Server-emitted error responses correctly use `id: null`
for unparseable / unidentifiable requests as the spec requires. See
[ADR-002] for the v0.2 plan to add a permissive codec-level type for
callers who need to accept null-id requests.

[ADR-002]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/002-json-rpc-2-compliance.md

## Relationship to turul-mcp

[`turul-mcp-server`](https://crates.io/crates/turul-mcp-server) is built
on top of `turul-rpc`. If you want MCP semantics (tools, resources,
prompts, sessions, the Inspector flow), reach for `turul-mcp-server`
directly — it pulls `turul-rpc` in transitively.

For existing 0.3.x users of `turul-mcp-json-rpc-server`: **nothing to
do**. That crate ships as a thin re-export shim over `turul-rpc 0.1`
from turul-mcp-framework 0.3.39 onward. See [ADR-003].

[ADR-003]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/003-compatibility-with-turul-mcp-json-rpc-server.md

## Architecture decisions

- [ADR-001 — Crate boundaries](https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/001-crate-boundaries.md)
- [ADR-002 — JSON-RPC 2.0 compliance](https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/002-json-rpc-2-compliance.md)
- [ADR-003 — Compatibility with turul-mcp-json-rpc-server](https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/003-compatibility-with-turul-mcp-json-rpc-server.md)
- [ADR-004 — Non-goals for v0.1](https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/004-non-goals-for-v0-1.md)

## License

Dual-licensed under [MIT](https://github.com/aussierobots/turul-rpc/blob/main/LICENSE-MIT)
or [Apache-2.0](https://github.com/aussierobots/turul-rpc/blob/main/LICENSE-APACHE)
at your option. See the [workspace README] for details.

[workspace README]: https://github.com/aussierobots/turul-rpc#readme
