//! # turul-rpc-jsonrpc
//!
//! JSON-RPC 2.0 codec and parser for `turul-rpc`. Provides incoming-message
//! parsing, batch processing, and response helper constructors.
//!
//! Most consumers should depend on the [`turul-rpc`] facade rather than this
//! crate directly.
//!
//! [`turul-rpc`]: https://crates.io/crates/turul-rpc
//!
//! ## JSON-RPC 2.0 batch support
//!
//! Per [JSON-RPC 2.0 §6][spec-batch], a request body may be an array of
//! request objects. This crate implements batch parsing via
//! [`parse_json_rpc_batch`] and surfaces the result as [`BatchOrSingle`].
//! The dispatcher in `turul-rpc-server` consumes this and runs the
//! per-member handlers, suppressing notification responses per spec.
//!
//! [spec-batch]: https://www.jsonrpc.org/specification#batch

pub mod batch;
pub mod dispatch;

// Re-export the dispatch module's public items at the crate root for
// ergonomic access. The `batch` module is intentionally kept as a separate
// module path (not re-exported here) so that the
// `turul-mcp-json-rpc-server 0.3.39` shim does not surface the new batch
// APIs through the v0.3.38 dispatch path. See ADR-003.
pub use dispatch::{
    parse_json_rpc_message, parse_json_rpc_messages, JsonRpcMessage, JsonRpcMessageResult,
    create_error_response, create_success_response,
};
pub use batch::{parse_json_rpc_batch, BatchOrSingle};
