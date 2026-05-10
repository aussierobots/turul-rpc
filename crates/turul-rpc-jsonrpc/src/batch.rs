//! JSON-RPC 2.0 §6 batch parsing.
//!
//! See [ADR-002] in the `turul-rpc` repository for the compliance contract.
//! This module is intentionally **not** re-exported by the
//! `turul-mcp-json-rpc-server 0.3.39` shim — new spec-additive APIs live
//! here so the shim's surface remains the v0.3.38 surface, per ADR-003.
//!
//! [ADR-002]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/002-json-rpc-2-compliance.md

use serde_json::Value;
use turul_rpc_core::error::JsonRpcError;

use crate::dispatch::{JsonRpcMessage, parse_value_into_message};

/// Outcome of parsing a request body that may be a batch or a single message.
///
/// JSON-RPC 2.0 §6 distinguishes three cases:
///
/// - **Single**: body is a JSON object → one message.
/// - **Batch**: body is a non-empty JSON array → multiple messages.
/// - **EmptyBatch**: body is `[]` → spec error, caller responds with a
///   single `Invalid Request` (`-32600`) with `id: null`.
#[derive(Debug)]
pub enum BatchOrSingle {
    Single(Result<JsonRpcMessage, JsonRpcError>),
    Batch(Vec<Result<JsonRpcMessage, JsonRpcError>>),
    EmptyBatch,
}

/// Parse a request body, distinguishing single message from batch.
///
/// Per JSON-RPC 2.0 §6:
/// - Empty array → caller should emit a single `Invalid Request` (`-32600`)
///   with `id: null`. This function returns [`BatchOrSingle::EmptyBatch`];
///   the dispatcher constructs the error response.
/// - Each batch member is parsed independently. Per-member parse failures
///   appear as `Err(JsonRpcError)` entries in the returned vec.
pub fn parse_json_rpc_batch(json_str: &str) -> BatchOrSingle {
    let value: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return BatchOrSingle::Single(Err(JsonRpcError::parse_error())),
    };

    match value {
        Value::Array(arr) if arr.is_empty() => BatchOrSingle::EmptyBatch,
        Value::Array(arr) => {
            let messages = arr.into_iter().map(parse_value_into_message).collect();
            BatchOrSingle::Batch(messages)
        }
        other => BatchOrSingle::Single(parse_value_into_message(other)),
    }
}
