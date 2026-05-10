# turul-rpc-server

> Async JSON-RPC 2.0 dispatcher, handler trait, and session context for
> the [`turul-rpc`] framework.

[`turul-rpc`]: https://crates.io/crates/turul-rpc

Built on [`turul-rpc-core`] wire types and [`turul-rpc-jsonrpc`]
codec. **Most consumers should depend on the [`turul-rpc`] facade**,
which re-exports this crate's items.

[`turul-rpc-core`]: https://crates.io/crates/turul-rpc-core
[`turul-rpc-jsonrpc`]: https://crates.io/crates/turul-rpc-jsonrpc

## What's here

- `JsonRpcDispatcher<E>` — typed dispatcher. Routes incoming requests
  to registered handlers and converts handler-returned domain errors to
  JSON-RPC error responses via `ToJsonRpcError`. Includes
  `handle_batch()` for JSON-RPC 2.0 §6 batch dispatch.
- `JsonRpcHandler` async trait — your handler returns
  `Result<Value, YourError>`. The dispatcher owns wire-format conversion.
- `ToJsonRpcError` trait — implement on your domain error type to
  control how it maps to a `JsonRpcErrorObject`.
- `SessionContext` — opaque session info passed through to handlers
  (id, metadata, type-erased broadcaster, request-scoped extensions).
- Optional `streaming` module (feature `streams`): `JsonRpcFrame`,
  `StreamingJsonRpcHandler`, `StreamingJsonRpcDispatcher` for
  progressive responses.

## Quick start

```rust
use turul_rpc_server::{JsonRpcDispatcher, JsonRpcHandler, SessionContext, ToJsonRpcError};
use turul_rpc_core::error::JsonRpcErrorObject;
use turul_rpc_core::request::RequestParams;
use async_trait::async_trait;
use serde_json::{json, Value};

#[derive(thiserror::Error, Debug)]
#[error("oops: {0}")]
struct MyError(&'static str);

impl ToJsonRpcError for MyError {
    fn to_error_object(&self) -> JsonRpcErrorObject {
        JsonRpcErrorObject::invalid_params(&self.to_string())
    }
}

struct Echo;

#[async_trait]
impl JsonRpcHandler for Echo {
    type Error = MyError;
    async fn handle(
        &self,
        method: &str,
        _params: Option<RequestParams>,
        _session: Option<SessionContext>,
    ) -> Result<Value, MyError> {
        Ok(json!({ "echoed": method }))
    }
}

# async fn run() {
let mut d: JsonRpcDispatcher<MyError> = JsonRpcDispatcher::new();
d.register_method("echo".into(), Echo);

// Handle a batch body. Returns None for all-notifications, otherwise
// the response JSON string.
let _: Option<String> = d.handle_batch(r#"{"jsonrpc":"2.0","method":"echo","id":1}"#).await;
# }
```

## Features

| Feature | Default | Pulls in |
|---|---|---|
| `async` | yes | `async-trait`, `futures` — required for the dispatcher and handler trait |
| `streams` | no | `async` + the streaming dispatcher |

## License

Dual-licensed under [MIT](https://github.com/aussierobots/turul-rpc/blob/main/LICENSE-MIT)
or [Apache-2.0](https://github.com/aussierobots/turul-rpc/blob/main/LICENSE-APACHE)
at your option. See the [workspace README] for details.

[workspace README]: https://github.com/aussierobots/turul-rpc#readme
