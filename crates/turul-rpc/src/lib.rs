//! Typed JSON-RPC 2.0 for Rust: handlers return their own domain errors and the
//! dispatcher builds the wire responses. This is the facade crate; it
//! re-exports [`turul-rpc-core`] (wire types), [`turul-rpc-jsonrpc`] (parser and
//! batch) and [`turul-rpc-server`] (dispatcher and handler trait).
//!
//! Depend on this crate for a JSON-RPC server. Depend on `turul-rpc-core`
//! alone if you only need the wire types without an async runtime. This crate
//! provides no transport (HTTP, stdio, WebSocket): you read bytes and pass
//! them to [`JsonRpcDispatcher::handle_batch`](turul_rpc_server::JsonRpcDispatcher::handle_batch).
//!
//! # Example
//!
//! ```toml
//! [dependencies]
//! turul-rpc = "0.2"
//! async-trait = "0.1"
//! serde_json = "1"
//! thiserror = "2"
//! tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
//! ```
//!
//! ```rust
//! # #[cfg(feature = "async")]
//! # mod demo {
//! use async_trait::async_trait;
//! use serde_json::{Value, json};
//! use turul_rpc::error::JsonRpcErrorObject;
//! use turul_rpc::r#async::ToJsonRpcError;
//! use turul_rpc::{JsonRpcDispatcher, JsonRpcHandler, RequestParams, SessionContext};
//!
//! #[derive(thiserror::Error, Debug)]
//! #[error("unknown method: {0}")]
//! struct Unknown(String);
//!
//! impl ToJsonRpcError for Unknown {
//!     fn to_error_object(&self) -> JsonRpcErrorObject {
//!         JsonRpcErrorObject::method_not_found(&self.0)
//!     }
//! }
//!
//! struct Ping;
//!
//! #[async_trait]
//! impl JsonRpcHandler for Ping {
//!     type Error = Unknown;
//!     async fn handle(&self, method: &str, _: Option<RequestParams>, _: Option<SessionContext>)
//!         -> Result<Value, Unknown> {
//!         match method {
//!             "ping" => Ok(json!("pong")),
//!             other => Err(Unknown(other.into())),
//!         }
//!     }
//! }
//!
//! pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut dispatcher = JsonRpcDispatcher::<Unknown>::new();
//!     dispatcher.register_method("ping".into(), Ping);
//!     let body = r#"{"jsonrpc":"2.0","method":"ping","id":1}"#;
//!     let reply = dispatcher.handle_batch(body).await.ok_or("no reply")?;
//!     assert_eq!(reply, r#"{"jsonrpc":"2.0","id":1,"result":"pong"}"#);
//!     Ok(())
//! }
//! # }
//! # #[cfg(feature = "async")]
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> { demo::run().await }
//! # #[cfg(not(feature = "async"))]
//! # fn main() {}
//! ```
//!
//! # Features
//!
//! | Feature | Default | Enables |
//! |---|---|---|
//! | `async` | yes | `async` module and the dispatcher, handler and session re-exports |
//! | `streams` | no | streaming dispatcher and `JsonRpcFrame` (implies `async`) |
//!
//! # Where next
//!
//! - Runnable examples: [`turul-rpc/examples`](https://github.com/aussierobots/turul-rpc/tree/main/crates/turul-rpc/examples)
//!   (stdin calculator, batch dispatch, in-process round trip).
//! - Parsing without the dispatcher: [`turul-rpc-jsonrpc`](https://docs.rs/turul-rpc-jsonrpc).
//! - Dispatcher and handler API: [`turul-rpc-server`](https://docs.rs/turul-rpc-server).
//! - Wire types: [`turul-rpc-core`](https://docs.rs/turul-rpc-core).
//!
//! [`turul-rpc-core`]: https://docs.rs/turul-rpc-core
//! [`turul-rpc-jsonrpc`]: https://docs.rs/turul-rpc-jsonrpc
//! [`turul-rpc-server`]: https://docs.rs/turul-rpc-server

// Module re-exports — preserve the original `turul_mcp_json_rpc_server::<module>`
// paths so the compatibility shim can re-export this crate without API drift.
pub use turul_rpc_core::{error, error_codes, notification, request, response, types};
pub use turul_rpc_jsonrpc::dispatch;

/// Async dispatcher and handler trait. Mirrors the original `r#async`
/// module from `turul-mcp-json-rpc-server`.
#[cfg(feature = "async")]
pub mod r#async {
    pub use turul_rpc_server::{
        FunctionHandler, JsonRpcDispatcher, JsonRpcHandler, SessionContext, ToJsonRpcError,
    };

    #[cfg(feature = "streams")]
    pub use turul_rpc_server::streaming;
}

pub mod prelude;

// Root re-exports — match the original crate's `pub use` lines.

/// JSON-RPC 2.0 error types and standard error codes.
pub use error::{JsonRpcError, JsonRpcErrorCode};
/// JSON-RPC notification message structure for fire-and-forget communications.
pub use notification::JsonRpcNotification;
/// JSON-RPC request structure with method and parameters.
pub use request::{JsonRpcRequest, RequestParams};
/// JSON-RPC response types: the §5 Response object union (`JsonRpcResponse`)
/// and the success-only struct (`JsonRpcSuccessResponse`).
pub use response::{JsonRpcResponse, JsonRpcSuccessResponse, ResponseResult};
/// Core JSON-RPC types for version and request identification.
pub use types::{JsonRpcVersion, RequestId};

/// Incoming message union (request | notification) produced by the parser.
pub use turul_rpc_jsonrpc::JsonRpcMessage;

/// Bidirectional wire-message union (request | notification | response) — the
/// schema's `JSONRPCMessage`. Superset of [`JsonRpcMessage`]; for clients and
/// peers that read responses. Parse with [`parse_json_rpc_wire_message`].
pub use turul_rpc_jsonrpc::frame::{JsonRpcWireMessage, parse_json_rpc_wire_message};

#[cfg(feature = "async")]
pub use turul_rpc_server::{JsonRpcDispatcher, JsonRpcHandler, SessionContext};

#[cfg(feature = "streams")]
pub use turul_rpc_server::streaming::{
    JsonRpcFrame, StreamingJsonRpcDispatcher, StreamingJsonRpcHandler,
};

/// JSON-RPC 2.0 version constant.
pub const JSONRPC_VERSION: &str = "2.0";
