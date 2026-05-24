//! The bidirectional wire-message union.
//!
//! [`JsonRpcWireMessage`] is the JSON-RPC 2.0 / MCP schema `JSONRPCMessage`:
//! any single message a peer reads off the wire without knowing direction.
//! It is the superset of the inbound [`JsonRpcMessage`](crate::JsonRpcMessage)
//! (request | notification) that the server dispatcher consumes — it adds the
//! `Response` arm a client or bidirectional peer needs.
//!
//! This module is intentionally kept out of the `dispatch` path so the
//! `turul-mcp-json-rpc-server` (0.3.x) shim does not surface it. See ADR-003
//! and ADR-004.

use serde::Serialize;
use serde_json::Value;

use turul_rpc_core::error::JsonRpcError;
use turul_rpc_core::notification::JsonRpcNotification;
use turul_rpc_core::request::JsonRpcRequest;
use turul_rpc_core::response::JsonRpcResponse;

use crate::dispatch::{JsonRpcMessage, extract_id, parse_value_into_message};

/// Any single JSON-RPC 2.0 message read off the wire, in either direction.
///
/// The `Response` arm carries a [`JsonRpcResponse`], which is itself the §5
/// `Success | Error` union — so one variant covers both response kinds.
///
/// Serialization is untagged: a frame serializes to the bare message object,
/// not an externally-tagged wrapper. Parse incoming bytes with
/// [`parse_json_rpc_wire_message`]; it validates structure rather than relying
/// on `#[serde(untagged)]` deserialization, which would silently misclassify
/// a malformed-id request as a notification.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum JsonRpcWireMessage {
    /// A request: has a `method` and an `id`, expects a response.
    Request(JsonRpcRequest),
    /// A notification: has a `method`, no `id`, expects no response.
    Notification(JsonRpcNotification),
    /// A response to a previously-sent request (success or error).
    Response(JsonRpcResponse),
}

impl From<JsonRpcMessage> for JsonRpcWireMessage {
    fn from(message: JsonRpcMessage) -> Self {
        match message {
            JsonRpcMessage::Request(request) => JsonRpcWireMessage::Request(request),
            JsonRpcMessage::Notification(notification) => {
                JsonRpcWireMessage::Notification(notification)
            }
        }
    }
}

/// Parse a JSON string into a [`JsonRpcWireMessage`].
///
/// On failure returns `Err(JsonRpcError)` with the spec error code:
/// `-32700` (Parse error) for invalid JSON, `-32600` (Invalid Request) for a
/// non-object body, a wrong `jsonrpc` version, a fractional numeric id, or a
/// body that is neither a valid request/notification nor a response.
pub fn parse_json_rpc_wire_message(json_str: &str) -> Result<JsonRpcWireMessage, JsonRpcError> {
    let value: Value = serde_json::from_str(json_str).map_err(|_| JsonRpcError::parse_error())?;
    parse_wire_value(value)
}

fn parse_wire_value(value: Value) -> Result<JsonRpcWireMessage, JsonRpcError> {
    let Some(obj) = value.as_object() else {
        return Err(JsonRpcError::invalid_request(None));
    };

    // A `method` means request-or-notification. Delegate to the inbound
    // parser, which already validates the version and rejects fractional ids.
    if obj.contains_key("method") {
        return parse_value_into_message(value).map(Into::into);
    }

    // No `method` → a response. Validate the version, then parse the §5 union.
    match obj.get("jsonrpc") {
        Some(version) if version == "2.0" => {}
        _ => return Err(JsonRpcError::invalid_request(extract_id(obj))),
    }
    let id = extract_id(obj);
    serde_json::from_value::<JsonRpcResponse>(value)
        .map(JsonRpcWireMessage::Response)
        .map_err(|_| JsonRpcError::invalid_request(id))
}
