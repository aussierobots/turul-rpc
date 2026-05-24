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
id correlation. For peers that read messages off the wire,
`JsonRpcWireMessage` (the schema's `JSONRPCMessage` union) parses any
inbound request, notification, or response via
`parse_json_rpc_wire_message`. There is no dedicated client crate; these
pieces cover the in-process and bring-your-own-transport cases (see
[ADR-004]).

## Compliance posture

JSON-RPC 2.0, fully spec-conformant on id handling. Incoming requests
with `"id": null` are **accepted** per §4.2 (permitted, though the spec
discourages them); `RequestId` is `String | Number | Null`. The only
narrowing is that fractional numeric ids are rejected — only `i64` is
representable. Server-emitted error responses use `id: null` for
unparseable or unidentifiable requests, as the spec requires. Batch
(§6) is implemented and tested. See [ADR-002] for the full posture.

[ADR-002]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/002-json-rpc-2-compliance.md
[ADR-004]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/004-non-goals-for-v0-1.md

## Relationship to turul-mcp

[`turul-mcp-server`](https://crates.io/crates/turul-mcp-server) is built
on top of `turul-rpc`. If you want MCP semantics (tools, resources,
prompts, sessions, the Inspector flow), reach for `turul-mcp-server`
directly — it pulls `turul-rpc` in transitively.

`turul-mcp-json-rpc-server` (0.3.x) is a thin re-export shim over this
crate, so existing consumers resolve the same paths through it. New code
should depend on `turul-rpc` directly. See [ADR-003].

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
