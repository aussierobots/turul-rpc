//! JSON-RPC 2.0 codec — parser, batch handling, response helpers.
//!
//! See [ADR-002] in the `turul-rpc` repository for the compliance contract.
//!
//! [ADR-002]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/002-json-rpc-2-compliance.md

use serde_json::Value;

use turul_rpc_core::error::JsonRpcError;
use turul_rpc_core::notification::JsonRpcNotification;
use turul_rpc_core::request::JsonRpcRequest;
use turul_rpc_core::response::JsonRpcResponse;
use turul_rpc_core::types::RequestId;

/// Enum representing different types of inbound JSON-RPC messages.
///
/// Note: this is the **incoming** message union (request | notification).
/// The **outgoing** message union (response | error) lives in
/// [`turul_rpc_core::response::JsonRpcMessage`]. Both names exist for
/// historical compatibility with `turul-mcp-json-rpc-server 0.3.x`; new
/// code should disambiguate via the qualified path.
#[derive(Debug, Clone)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

/// Result of parsing and processing a JSON-RPC message.
#[derive(Debug, Clone)]
pub enum JsonRpcMessageResult {
    /// A response to a request
    Response(JsonRpcResponse),
    /// An error response
    Error(JsonRpcError),
    /// No response needed (for notifications)
    NoResponse,
}

impl JsonRpcMessageResult {
    /// Convert to JSON string if there's a response to send.
    pub fn to_json_string(&self) -> Option<String> {
        match self {
            JsonRpcMessageResult::Response(response) => serde_json::to_string(response).ok(),
            JsonRpcMessageResult::Error(error) => serde_json::to_string(error).ok(),
            JsonRpcMessageResult::NoResponse => None,
        }
    }

    /// Check if this result represents an error.
    pub fn is_error(&self) -> bool {
        matches!(self, JsonRpcMessageResult::Error(_))
    }

    /// Check if this result needs a response.
    pub fn needs_response(&self) -> bool {
        !matches!(self, JsonRpcMessageResult::NoResponse)
    }
}

/// Parse a JSON string into a single JSON-RPC message.
///
/// On parse failure returns `Err(JsonRpcError)` with the appropriate
/// JSON-RPC 2.0 error code:
///
/// - Invalid JSON → `-32700` (Parse error), `id: null`
/// - Not an object → `-32600` (Invalid Request), `id: null`
/// - Wrong `jsonrpc` field → `-32600` (Invalid Request), `id: null`
/// - Object missing required fields → `-32600` (Invalid Request), echoing
///   the id when one is parseable
pub fn parse_json_rpc_message(json_str: &str) -> Result<JsonRpcMessage, JsonRpcError> {
    let value: Value = serde_json::from_str(json_str).map_err(|_| JsonRpcError::parse_error())?;
    parse_json_rpc_value(value)
}

/// Parse a `serde_json::Value` into a single JSON-RPC message.
///
/// Used by both [`parse_json_rpc_message`] and [`parse_json_rpc_batch`] so
/// that batch members do not pay a round-trip through string serialization.
fn parse_json_rpc_value(value: Value) -> Result<JsonRpcMessage, JsonRpcError> {
    if !value.is_object() {
        return Err(JsonRpcError::invalid_request(None));
    }

    let obj = value.as_object().unwrap();

    // Check JSON-RPC version
    match obj.get("jsonrpc") {
        Some(version) if version == "2.0" => {}
        _ => return Err(JsonRpcError::invalid_request(extract_id(obj))),
    }

    // Check if it has an ID (request) or not (notification)
    if obj.contains_key("id") {
        // Reject explicit null id at the parser per ADR-002 (strict posture).
        if matches!(obj.get("id"), Some(Value::Null)) {
            return Err(JsonRpcError::invalid_request(None));
        }
        // Reject fractional numeric ids (no `as_i64()`).
        if let Some(Value::Number(n)) = obj.get("id") {
            if n.as_i64().is_none() {
                return Err(JsonRpcError::invalid_request(None));
            }
        }
        serde_json::from_value::<JsonRpcRequest>(value.clone())
            .map(JsonRpcMessage::Request)
            .map_err(|_| JsonRpcError::invalid_request(extract_id(obj)))
    } else {
        serde_json::from_value::<JsonRpcNotification>(value)
            .map(JsonRpcMessage::Notification)
            .map_err(|_| JsonRpcError::invalid_request(None))
    }
}

fn extract_id(obj: &serde_json::Map<String, Value>) -> Option<RequestId> {
    obj.get("id").and_then(|v| match v {
        Value::String(s) => Some(RequestId::String(s.clone())),
        Value::Number(n) => n.as_i64().map(RequestId::Number),
        _ => None,
    })
}

/// Parse multiple JSON-RPC messages from a single JSON string.
///
/// **Compatibility shim** for the `turul-mcp-json-rpc-server 0.3.x` API.
/// New code should use [`crate::batch::parse_json_rpc_batch`] for proper
/// JSON-RPC 2.0 §6 batch semantics. This function returns a single-element
/// vec for non-array bodies and an empty vec for an empty array body —
/// it is **not** spec-correct on its own; the dispatcher must construct
/// the `Invalid Request` response for empty batches.
pub fn parse_json_rpc_messages(json_str: &str) -> Vec<Result<JsonRpcMessage, JsonRpcError>> {
    use crate::batch::{parse_json_rpc_batch, BatchOrSingle};
    match parse_json_rpc_batch(json_str) {
        BatchOrSingle::Single(r) => vec![r],
        BatchOrSingle::Batch(items) => items,
        BatchOrSingle::EmptyBatch => vec![Err(JsonRpcError::invalid_request(None))],
    }
}

/// Parse a single JSON-RPC value as a message. Used by both the single-message
/// parser and the batch parser to avoid round-trips through string serialization.
pub(crate) fn parse_value_into_message(
    value: Value,
) -> Result<JsonRpcMessage, JsonRpcError> {
    parse_json_rpc_value(value)
}

/// Create a simple success response.
pub fn create_success_response(id: RequestId, result: Value) -> JsonRpcMessageResult {
    JsonRpcMessageResult::Response(JsonRpcResponse::success(id, result))
}

/// Create a simple error response.
pub fn create_error_response(
    id: Option<RequestId>,
    code: i64,
    message: &str,
) -> JsonRpcMessageResult {
    let error_obj = turul_rpc_core::error::JsonRpcErrorObject {
        code,
        message: message.to_string(),
        data: None,
    };
    JsonRpcMessageResult::Error(JsonRpcError::new(id, error_obj))
}

/// Utility functions for working with JSON-RPC messages.
impl JsonRpcMessage {
    /// Get the method name.
    pub fn method(&self) -> &str {
        match self {
            JsonRpcMessage::Request(req) => &req.method,
            JsonRpcMessage::Notification(notif) => &notif.method,
        }
    }

    /// Check if this is a request (has ID).
    pub fn is_request(&self) -> bool {
        matches!(self, JsonRpcMessage::Request(_))
    }

    /// Check if this is a notification (no ID).
    pub fn is_notification(&self) -> bool {
        matches!(self, JsonRpcMessage::Notification(_))
    }

    /// Get the request ID if this is a request.
    pub fn request_id(&self) -> Option<&RequestId> {
        match self {
            JsonRpcMessage::Request(req) => Some(&req.id),
            JsonRpcMessage::Notification(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_valid_request() {
        let json = r#"{"jsonrpc": "2.0", "method": "test", "id": 1}"#;
        let message = parse_json_rpc_message(json).unwrap();

        assert!(message.is_request());
        assert_eq!(message.method(), "test");
        assert_eq!(message.request_id(), Some(&RequestId::Number(1)));
    }

    #[test]
    fn test_parse_valid_notification() {
        let json = r#"{"jsonrpc": "2.0", "method": "notify"}"#;
        let message = parse_json_rpc_message(json).unwrap();

        assert!(message.is_notification());
        assert_eq!(message.method(), "notify");
        assert_eq!(message.request_id(), None);
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = r#"{"jsonrpc": "2.0", "method": "test""#;
        let result = parse_json_rpc_message(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.error.code, -32700);
    }

    #[test]
    fn test_parse_invalid_version() {
        let json = r#"{"jsonrpc": "1.0", "method": "test", "id": 1}"#;
        let result = parse_json_rpc_message(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.error.code, -32600);
    }

    #[test]
    fn test_message_result_to_json() {
        let response = create_success_response(RequestId::Number(1), json!({"result": "success"}));

        let json_str = response.to_json_string().unwrap();
        assert!(json_str.contains("\"result\""));
        assert!(json_str.contains("\"jsonrpc\":\"2.0\""));
    }

    #[test]
    fn test_message_result_properties() {
        let success = create_success_response(RequestId::Number(1), json!({}));
        let error = create_error_response(Some(RequestId::Number(1)), -32601, "Not found");
        let no_response = JsonRpcMessageResult::NoResponse;

        assert!(!success.is_error());
        assert!(success.needs_response());

        assert!(error.is_error());
        assert!(error.needs_response());

        assert!(!no_response.is_error());
        assert!(!no_response.needs_response());
    }
}
