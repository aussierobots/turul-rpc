//! Async JSON-RPC 2.0 dispatcher for `turul-rpc`. Handlers implement
//! [`JsonRpcHandler`] and return their own domain error; the
//! [`JsonRpcDispatcher`] maps it to a JSON-RPC error response through
//! [`ToJsonRpcError`] and handles single requests, notifications and batches.
//!
//! Depend on this crate when you want the dispatcher without the facade; most
//! applications use [`turul-rpc`], which re-exports it. There is no transport
//! here: pass request bytes to [`JsonRpcDispatcher::handle_batch`] and write
//! back the reply.
//!
//! # Example
//!
//! ```toml
//! [dependencies]
//! turul-rpc-server = "0.2"
//! turul-rpc-core = "0.2"
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
//! use turul_rpc_core::{error::JsonRpcErrorObject, RequestParams};
//! use turul_rpc_server::{JsonRpcDispatcher, JsonRpcHandler, SessionContext, ToJsonRpcError};
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
//! | `async` | yes | the dispatcher, handler trait and [`SessionContext`] |
//! | `streams` | no | the `streaming` module: a streaming dispatcher and handler (implies `async`) |
//!
//! # Where next
//!
//! - [`JsonRpcHandler`] documents notification handling and `supported_methods`.
//! - [`JsonRpcDispatcher::set_default_handler`] catches methods with no registered handler.
//! - [`turul-rpc`] has runnable examples:
//!   [calculator, batch dispatch, in-process round trip](https://github.com/aussierobots/turul-rpc/tree/main/crates/turul-rpc/examples).
//!
//! [`turul-rpc`]: https://docs.rs/turul-rpc

#[cfg(feature = "async")]
pub mod dispatcher;
#[cfg(feature = "async")]
pub mod handler;
#[cfg(feature = "async")]
pub mod session;

#[cfg(feature = "streams")]
pub mod streaming;

#[cfg(feature = "async")]
pub use dispatcher::JsonRpcDispatcher;
#[cfg(feature = "async")]
pub use handler::{FunctionHandler, JsonRpcHandler, ToJsonRpcError};
#[cfg(feature = "async")]
pub use session::SessionContext;
