//! # turul-rpc
//!
//! Typed JSON-RPC 2.0 framework. Facade crate re-exporting the
//! [`turul-rpc-core`], [`turul-rpc-jsonrpc`], and [`turul-rpc-server`]
//! crates under stable paths that mirror the original
//! `turul-mcp-json-rpc-server 0.3.x` layout.
//!
//! Most consumers should depend on this crate. Use the split crates
//! directly only when you need to avoid pulling in the async runtime
//! (depend on `turul-rpc-core` only) or want to write a transport on top
//! of the codec without the dispatcher (depend on `turul-rpc-jsonrpc`).
//!
//! [`turul-rpc-core`]: https://crates.io/crates/turul-rpc-core
//! [`turul-rpc-jsonrpc`]: https://crates.io/crates/turul-rpc-jsonrpc
//! [`turul-rpc-server`]: https://crates.io/crates/turul-rpc-server

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
/// JSON-RPC response types including success and error variants.
pub use response::{JsonRpcMessage, JsonRpcResponse, ResponseResult};
/// Core JSON-RPC types for version and request identification.
pub use types::{JsonRpcVersion, RequestId};

#[cfg(feature = "async")]
pub use turul_rpc_server::{JsonRpcDispatcher, JsonRpcHandler, SessionContext};

#[cfg(feature = "streams")]
pub use turul_rpc_server::streaming::{
    JsonRpcFrame, StreamingJsonRpcDispatcher, StreamingJsonRpcHandler,
};

/// JSON-RPC 2.0 version constant.
pub const JSONRPC_VERSION: &str = "2.0";
