//! JSON-RPC 2.0 wire types: request, notification, response, error object,
//! request id and the standard error codes. Pure data with `serde` support;
//! no async runtime, parser or transport.
//!
//! Depend on this crate to build or inspect JSON-RPC messages without pulling
//! in the dispatcher. To parse incoming bytes use [`turul-rpc-jsonrpc`]; to run
//! handlers use [`turul-rpc-server`]. Most applications want the [`turul-rpc`]
//! facade, which re-exports everything here.
//!
//! # Example
//!
//! ```toml
//! [dependencies]
//! turul-rpc-core = "0.2"
//! serde_json = "1"
//! ```
//!
//! ```rust
//! use serde_json::json;
//! use turul_rpc_core::{JsonRpcRequest, RequestParams};
//!
//! # fn main() -> Result<(), serde_json::Error> {
//! let params = RequestParams::Object([("a".to_string(), json!(2))].into());
//! let request = JsonRpcRequest::new(1.into(), "add".into(), Some(params));
//!
//! let wire = serde_json::to_string(&request)?;
//! let back: JsonRpcRequest = serde_json::from_str(&wire)?;
//! assert_eq!(back.method, "add");
//! # Ok(())
//! # }
//! ```
//!
//! # Where next
//!
//! - [`error_codes`] holds the standard `-32700`..`-32600` constants and the
//!   server-error range.
//! - [`JsonRpcResponse`] is the success-or-error response union.
//! - [`turul-rpc-jsonrpc`](https://docs.rs/turul-rpc-jsonrpc) parses bytes into these types.
//! - [`turul-rpc`](https://docs.rs/turul-rpc) is the facade most applications use.
//!
//! [`turul-rpc-jsonrpc`]: https://docs.rs/turul-rpc-jsonrpc
//! [`turul-rpc-server`]: https://docs.rs/turul-rpc-server
//! [`turul-rpc`]: https://docs.rs/turul-rpc

pub mod error;
pub mod notification;
pub mod request;
pub mod response;
pub mod types;

// Root re-exports matching the original `turul-mcp-json-rpc-server` shape.
pub use error::{JsonRpcError, JsonRpcErrorCode};
pub use notification::JsonRpcNotification;
pub use request::{JsonRpcRequest, RequestParams};
pub use response::{JsonRpcResponse, JsonRpcSuccessResponse, ResponseResult};
pub use types::{JsonRpcVersion, RequestId};

/// JSON-RPC 2.0 version constant.
pub const JSONRPC_VERSION: &str = "2.0";

/// Standard JSON-RPC 2.0 error codes.
pub mod error_codes {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL_ERROR: i64 = -32603;

    /// Inclusive lower bound of the JSON-RPC 2.0 server-defined error range.
    pub const SERVER_ERROR_START: i64 = -32099;
    /// Inclusive upper bound of the JSON-RPC 2.0 server-defined error range.
    pub const SERVER_ERROR_END: i64 = -32000;
}
