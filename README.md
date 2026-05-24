# turul-rpc

> Typed JSON-RPC 2.0 framework for Rust. Handlers return domain errors; the dispatcher owns the wire.

`turul-rpc` is a generic, transport-agnostic JSON-RPC 2.0 framework for Rust.
You get spec-conformant wire types and an async dispatcher; you bring the
transport. Handlers return your own error type and the dispatcher owns the
conversion to the wire.

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

## Core types

The wire model is a small set of enums you compose directly:

| Type | Shape | Role |
|---|---|---|
| `RequestId` | `String \| Number(i64) \| Null` | request/response id (§4.2) |
| `ResponseResult` | `Success(Value) \| Null` | the `result` payload of a success response |
| `JsonRpcResponse` | `Success(JsonRpcSuccessResponse) \| Error(JsonRpcError)` | the §5 Response object union |
| `JsonRpcMessage` | `Request \| Notification` | what a **server** reads off the wire |
| `JsonRpcWireMessage` | `Request \| Notification \| Response` | what a **client or peer** reads — the schema's `JSONRPCMessage` |

`JsonRpcMessage` is the inbound subset; `JsonRpcWireMessage` is the full
bidirectional union, its `Response` arm carrying the success-or-error
`JsonRpcResponse`. Parse incoming bytes with `parse_json_rpc_message` (server
side) or `parse_json_rpc_wire_message` (any peer that reads responses) — both
validate structure and reject malformed ids per spec rather than guessing via
untagged deserialization. `JsonRpcRequest` and `JsonRpcNotification` are the
matching outbound shapes.

## Compliance posture

`turul-rpc` implements JSON-RPC 2.0 id handling per §4.2: incoming
requests with `"id": null` are **accepted** (`RequestId = {String, Number,
Null}`); null ids are permitted by the spec though discouraged. The only
narrowing is that fractional numeric ids are rejected (the spec itself
SHOULD-NOTs them; only `i64` is representable). Server-emitted error
responses use `id: null` for unparseable or unidentifiable requests as
the spec requires. See
[ADR-002](docs/adr/002-json-rpc-2-compliance.md) for the full posture.

## Quick start

Define a handler — it returns your own error type, never a `JsonRpcError`:

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

Register it on a dispatcher and hand it a request — you get back a
`JsonRpcResponse` (the §5 `Success | Error` union):

```rust
use turul_rpc::{JsonRpcDispatcher, JsonRpcRequest, JsonRpcResponse, RequestId};
use serde_json::json;
use std::collections::HashMap;

async fn run() {
    let mut dispatcher: JsonRpcDispatcher<CalcError> = JsonRpcDispatcher::new();
    dispatcher.register_method("add".into(), Calc);

    let req = JsonRpcRequest::new_with_object_params(
        RequestId::Number(1),
        "add".into(),
        HashMap::from([("a".into(), json!(2)), ("b".into(), json!(3))]),
    );

    match dispatcher.handle_request(req).await {
        JsonRpcResponse::Success(ok) => println!("result: {:?}", ok.result.as_value()),
        JsonRpcResponse::Error(err)  => eprintln!("error {}: {}", err.error.code, err.error.message),
    }

    // Or feed a raw batch/single body straight in; returns the response
    // JSON string, or None for an all-notifications body.
    let _: Option<String> =
        dispatcher.handle_batch(r#"{"jsonrpc":"2.0","id":2,"method":"add","params":{"a":1,"b":1}}"#).await;
}
```

If you're below the dispatcher — parsing wire bytes or building responses
by hand — use the message helpers:

```rust
use turul_rpc::{JsonRpcWireMessage, parse_json_rpc_wire_message, RequestId};
use turul_rpc::dispatch::{create_success_response, create_error_response};
use serde_json::json;

// Classify whatever a peer sent: request, notification, or response.
match parse_json_rpc_wire_message(r#"{"jsonrpc":"2.0","id":1,"result":{"sum":5}}"#).unwrap() {
    JsonRpcWireMessage::Request(req)       => { /* dispatch it */ }
    JsonRpcWireMessage::Notification(note) => { /* fire-and-forget */ }
    JsonRpcWireMessage::Response(resp)     => { /* correlate to a request you sent */ }
}

// Build a ready-to-serialize result without a dispatcher.
let ok  = create_success_response(RequestId::Number(1), json!({ "sum": 5 }));
let err = create_error_response(Some(RequestId::Number(2)), -32601, "method not found");
let body: Option<String> = ok.to_json_string();
```

(Use `parse_json_rpc_message` instead of `parse_json_rpc_wire_message` when you
only ever receive requests/notifications — a server that never reads responses.)

## Ecosystem

`turul-rpc` is the JSON-RPC layer beneath
[`turul-mcp-server`](https://crates.io/crates/turul-mcp-server) — reach for that
crate if you want MCP semantics. The `turul-mcp-json-rpc-server` (0.3.x) shim
re-exports `turul-rpc`, so existing consumers keep working; new code should
depend on `turul-rpc` directly ([ADR-003](docs/adr/003-compatibility-with-turul-mcp-json-rpc-server.md)).

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
