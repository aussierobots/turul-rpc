//! # turul-rpc-core
//!
//! Pure JSON-RPC 2.0 wire types. No async, no codec helpers, no parser.
//! Use [`turul-rpc-jsonrpc`] for parsing/batch and [`turul-rpc-server`] for
//! the dispatcher and handler trait. Most consumers should depend on the
//! [`turul-rpc`] facade instead of these crates directly.
//!
//! [`turul-rpc-jsonrpc`]: https://crates.io/crates/turul-rpc-jsonrpc
//! [`turul-rpc-server`]: https://crates.io/crates/turul-rpc-server
//! [`turul-rpc`]: https://crates.io/crates/turul-rpc

pub mod error;
pub mod notification;
pub mod request;
pub mod response;
pub mod types;

// Root re-exports matching the original `turul-mcp-json-rpc-server` shape.
pub use error::{JsonRpcError, JsonRpcErrorCode};
pub use notification::JsonRpcNotification;
pub use request::{JsonRpcRequest, RequestParams};
pub use response::{JsonRpcMessage, JsonRpcResponse, ResponseResult};
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
