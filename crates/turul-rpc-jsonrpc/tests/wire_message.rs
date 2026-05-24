//! Tests for the bidirectional wire-message union (`JsonRpcWireMessage`).
//!
//! This is the JSON-RPC 2.0 / MCP schema `JSONRPCMessage`: any single
//! message a peer reads off the wire without knowing direction — a request,
//! a notification, or a response. It is the superset of the inbound
//! `JsonRpcMessage` (request | notification) used by the server dispatcher.
//!
//! The parser must classify by validated structure, NOT by a naive
//! `#[serde(untagged)]` derive — untagged + serde's ignore-unknown-fields
//! default silently misclassifies a malformed-id request as a notification.

use turul_rpc_jsonrpc::frame::{JsonRpcWireMessage, parse_json_rpc_wire_message};

#[test]
fn parses_request() {
    let m = parse_json_rpc_wire_message(r#"{"jsonrpc":"2.0","id":1,"method":"foo"}"#).unwrap();
    assert!(matches!(m, JsonRpcWireMessage::Request(_)));
}

#[test]
fn parses_notification() {
    let m = parse_json_rpc_wire_message(r#"{"jsonrpc":"2.0","method":"ping"}"#).unwrap();
    assert!(matches!(m, JsonRpcWireMessage::Notification(_)));
}

#[test]
fn parses_success_response() {
    let m =
        parse_json_rpc_wire_message(r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#).unwrap();
    assert!(matches!(m, JsonRpcWireMessage::Response(_)));
}

#[test]
fn parses_error_response() {
    let m = parse_json_rpc_wire_message(
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"nope"}}"#,
    )
    .unwrap();
    assert!(matches!(m, JsonRpcWireMessage::Response(_)));
}

/// The key correctness guard: a request with a fractional id must be
/// rejected as Invalid Request — NOT silently reclassified as a
/// notification (which is what a `#[serde(untagged)]` derive would do,
/// since `JsonRpcNotification` ignores the unparseable `id` field).
#[test]
fn fractional_id_request_is_rejected_not_misclassified() {
    let err =
        parse_json_rpc_wire_message(r#"{"jsonrpc":"2.0","id":1.5,"method":"foo"}"#).unwrap_err();
    let v = serde_json::to_value(&err).unwrap();
    assert_eq!(v["error"]["code"], -32600);
}

#[test]
fn wrong_version_is_rejected() {
    assert!(parse_json_rpc_wire_message(r#"{"jsonrpc":"1.0","id":1,"method":"foo"}"#).is_err());
}

#[test]
fn non_object_body_is_rejected() {
    assert!(parse_json_rpc_wire_message("42").is_err());
}

#[test]
fn invalid_json_is_parse_error() {
    let err = parse_json_rpc_wire_message("{not json").unwrap_err();
    let v = serde_json::to_value(&err).unwrap();
    assert_eq!(v["error"]["code"], -32700);
}

#[test]
fn response_without_result_or_error_is_rejected() {
    assert!(parse_json_rpc_wire_message(r#"{"jsonrpc":"2.0","id":1}"#).is_err());
}

/// Serialization is untagged — a frame round-trips to the bare message
/// object, never an externally-tagged `{"Request": {...}}` wrapper.
#[test]
fn serializes_untagged_without_variant_tag() {
    let m = parse_json_rpc_wire_message(r#"{"jsonrpc":"2.0","id":1,"method":"foo"}"#).unwrap();
    let v = serde_json::to_value(&m).unwrap();
    assert_eq!(v["method"], "foo");
    assert_eq!(v["id"], 1);
    assert!(v.get("Request").is_none());
}

/// `id: null` is accepted per §4.2 (a request, not a notification).
#[test]
fn null_id_request_is_a_request() {
    let m = parse_json_rpc_wire_message(r#"{"jsonrpc":"2.0","id":null,"method":"foo"}"#).unwrap();
    assert!(matches!(m, JsonRpcWireMessage::Request(_)));
}
