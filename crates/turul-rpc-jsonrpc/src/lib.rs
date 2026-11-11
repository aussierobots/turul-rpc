//! JSON-RPC 2.0 parsing for `turul-rpc`: turn request bytes into typed
//! messages, including [JSON-RPC 2.0 §6 batches][spec-batch], and build
//! response objects. It sits between the wire types in [`turul-rpc-core`] and
//! the dispatcher in [`turul-rpc-server`].
//!
//! Depend on this crate to write your own transport or dispatcher on top of
//! the parser. To route requests to handlers, use [`turul-rpc-server`] or the
//! [`turul-rpc`] facade instead.
//!
//! # Example
//!
//! ```toml
//! [dependencies]
//! turul-rpc-jsonrpc = "0.2"
//! ```
//!
//! ```rust
//! use turul_rpc_jsonrpc::{BatchOrSingle, parse_json_rpc_batch};
//!
//! let body = r#"[
//!     {"jsonrpc":"2.0","method":"add","params":[1,2],"id":1},
//!     {"jsonrpc":"2.0","method":"log"}
//! ]"#;
//!
//! match parse_json_rpc_batch(body) {
//!     BatchOrSingle::Batch(items) => assert_eq!(items.len(), 2),
//!     BatchOrSingle::Single(_) => unreachable!("body is an array"),
//!     // `[]` is invalid; the caller answers with -32600 and a null id.
//!     BatchOrSingle::EmptyBatch => unreachable!("array is not empty"),
//! }
//! ```
//!
//! Each batch member is parsed independently, so one malformed member is an
//! `Err` entry rather than a failure of the whole batch.
//!
//! # Where next
//!
//! - [`parse_json_rpc_message`] parses a single message; [`frame`] parses
//!   responses too ([`frame::parse_json_rpc_wire_message`]).
//! - [`turul-rpc-server`](https://docs.rs/turul-rpc-server) consumes the batch
//!   parser and assembles the response array.
//! - [`turul-rpc-core`](https://docs.rs/turul-rpc-core) defines the message types.
//!
//! [spec-batch]: https://www.jsonrpc.org/specification#batch
//! [`turul-rpc-core`]: https://docs.rs/turul-rpc-core
//! [`turul-rpc-server`]: https://docs.rs/turul-rpc-server
//! [`turul-rpc`]: https://docs.rs/turul-rpc

pub mod batch;
pub mod dispatch;
pub mod frame;

// Re-export the dispatch module's public items at the crate root for
// ergonomic access. The `batch` module is intentionally kept as a separate
// module path (not re-exported here) so that the
// `turul-mcp-json-rpc-server 0.3.39` shim does not surface the new batch
// APIs through the v0.3.38 dispatch path. See ADR-003.
pub use batch::{BatchOrSingle, parse_json_rpc_batch};
pub use dispatch::{
    JsonRpcMessage, JsonRpcMessageResult, create_error_response, create_success_response,
    parse_json_rpc_message, parse_json_rpc_messages,
};
