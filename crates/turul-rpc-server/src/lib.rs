//! # turul-rpc-server
//!
//! Async JSON-RPC 2.0 dispatcher and handler trait for `turul-rpc`. The
//! `JsonRpcDispatcher<E>` routes incoming requests to registered handlers
//! and converts handler-returned domain errors to JSON-RPC error responses
//! via the [`ToJsonRpcError`] trait.
//!
//! Most consumers should depend on the [`turul-rpc`] facade instead of
//! this crate directly.
//!
//! [`turul-rpc`]: https://crates.io/crates/turul-rpc
//! [`ToJsonRpcError`]: handler::ToJsonRpcError

#[cfg(feature = "async")]
pub mod handler;
#[cfg(feature = "async")]
pub mod session;
#[cfg(feature = "async")]
pub mod dispatcher;

#[cfg(feature = "streams")]
pub mod streaming;

#[cfg(feature = "async")]
pub use dispatcher::JsonRpcDispatcher;
#[cfg(feature = "async")]
pub use handler::{FunctionHandler, JsonRpcHandler, ToJsonRpcError};
#[cfg(feature = "async")]
pub use session::SessionContext;
