//! JSON-RPC 2.0 specification conformance tests.
//!
//! Each test maps to a numbered section of <https://www.jsonrpc.org/specification>.
//! See ADR-002 in the `turul-rpc` repository for the full compliance contract.

use serde_json::{json, Value};
use turul_rpc_core::error_codes::*;
use turul_rpc_core::types::RequestId;
use turul_rpc_jsonrpc::{
    parse_json_rpc_batch, parse_json_rpc_message, BatchOrSingle, JsonRpcMessage,
};

// -----------------------------------------------------------------------------
// §4.1 — `jsonrpc` field strictness
// -----------------------------------------------------------------------------

#[test]
fn rejects_jsonrpc_1_0() {
    let r = parse_json_rpc_message(r#"{"jsonrpc":"1.0","method":"x","id":1}"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
}

#[test]
fn rejects_jsonrpc_as_number() {
    let r = parse_json_rpc_message(r#"{"jsonrpc":2.0,"method":"x","id":1}"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
}

#[test]
fn rejects_missing_jsonrpc_field() {
    let r = parse_json_rpc_message(r#"{"method":"x","id":1}"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
}

// -----------------------------------------------------------------------------
// §4.2 — id rules
// -----------------------------------------------------------------------------

#[test]
fn accepts_string_id() {
    let m = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x","id":"abc"}"#).unwrap();
    assert!(m.is_request());
    assert_eq!(
        m.request_id(),
        Some(&RequestId::String("abc".to_string()))
    );
}

#[test]
fn accepts_number_id() {
    let m = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x","id":42}"#).unwrap();
    assert_eq!(m.request_id(), Some(&RequestId::Number(42)));
}

#[test]
fn accepts_zero_id() {
    let m = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x","id":0}"#).unwrap();
    assert_eq!(m.request_id(), Some(&RequestId::Number(0)));
}

#[test]
fn rejects_null_id_per_strict_posture() {
    // ADR-002: turul-rpc rejects null id at the parser. JSON-RPC 2.0 permits
    // it but discourages it; we are strict.
    let r = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x","id":null}"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
}

#[test]
fn rejects_fractional_id() {
    let r = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x","id":1.5}"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
}

// -----------------------------------------------------------------------------
// §4.4 — Notification (no id) does not produce a response
// -----------------------------------------------------------------------------

#[test]
fn accepts_notification_no_id() {
    let m = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x"}"#).unwrap();
    assert!(m.is_notification());
    assert_eq!(m.request_id(), None);
}

// -----------------------------------------------------------------------------
// §5 / §5.1 — Error responses
// -----------------------------------------------------------------------------

#[test]
fn parse_error_returns_minus_32700() {
    let r = parse_json_rpc_message(r#"{garbage}"#).unwrap_err();
    assert_eq!(r.error.code, PARSE_ERROR);
}

#[test]
fn parse_error_has_null_id() {
    let r = parse_json_rpc_message(r#"{garbage}"#).unwrap_err();
    assert!(r.id.is_none());
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v.get("id"), Some(&Value::Null));
}

#[test]
fn empty_body_is_parse_error() {
    let r = parse_json_rpc_message("").unwrap_err();
    assert_eq!(r.error.code, PARSE_ERROR);
}

#[test]
fn truncated_json_is_parse_error() {
    let r = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x""#).unwrap_err();
    assert_eq!(r.error.code, PARSE_ERROR);
}

#[test]
fn primitive_body_is_invalid_request() {
    let r = parse_json_rpc_message(r#"42"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
}

#[test]
fn invalid_request_echoes_id_when_parseable() {
    // Object missing `method` but has parseable id 7.
    let r = parse_json_rpc_message(r#"{"jsonrpc":"2.0","id":7}"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
    assert_eq!(r.id, Some(RequestId::Number(7)));
}

// -----------------------------------------------------------------------------
// §5.1 — Server error range
// -----------------------------------------------------------------------------

#[test]
fn server_error_range_constants() {
    assert_eq!(SERVER_ERROR_START, -32099);
    assert_eq!(SERVER_ERROR_END, -32000);
    assert!(SERVER_ERROR_START < SERVER_ERROR_END);
}

#[test]
#[should_panic(expected = "Server error code must be in range")]
fn server_error_panics_outside_range_low() {
    use turul_rpc_core::error::JsonRpcErrorObject;
    let _ = JsonRpcErrorObject::server_error(-32100, "oops", None);
}

#[test]
#[should_panic(expected = "Server error code must be in range")]
fn server_error_panics_outside_range_high() {
    use turul_rpc_core::error::JsonRpcErrorObject;
    let _ = JsonRpcErrorObject::server_error(-31999, "oops", None);
}

#[test]
fn server_error_accepts_range_endpoints() {
    use turul_rpc_core::error::JsonRpcErrorObject;
    let lo = JsonRpcErrorObject::server_error(-32099, "lo", None);
    let hi = JsonRpcErrorObject::server_error(-32000, "hi", None);
    assert_eq!(lo.code, -32099);
    assert_eq!(hi.code, -32000);
}

// -----------------------------------------------------------------------------
// §6 — Batch
// -----------------------------------------------------------------------------

#[test]
fn empty_batch_yields_empty_batch_marker() {
    match parse_json_rpc_batch("[]") {
        BatchOrSingle::EmptyBatch => {}
        other => panic!("expected EmptyBatch, got {other:?}"),
    }
}

#[test]
fn single_object_yields_single() {
    let body = r#"{"jsonrpc":"2.0","method":"x","id":1}"#;
    match parse_json_rpc_batch(body) {
        BatchOrSingle::Single(Ok(JsonRpcMessage::Request(_))) => {}
        other => panic!("expected Single Request, got {other:?}"),
    }
}

#[test]
fn batch_with_two_requests() {
    let body = r#"[
        {"jsonrpc":"2.0","method":"a","id":1},
        {"jsonrpc":"2.0","method":"b","id":2}
    ]"#;
    let batch = match parse_json_rpc_batch(body) {
        BatchOrSingle::Batch(b) => b,
        other => panic!("expected Batch, got {other:?}"),
    };
    assert_eq!(batch.len(), 2);
    assert!(batch[0].is_ok());
    assert!(batch[1].is_ok());
    assert_eq!(batch[0].as_ref().unwrap().method(), "a");
    assert_eq!(batch[1].as_ref().unwrap().method(), "b");
}

#[test]
fn batch_with_mixed_request_and_notification() {
    let body = r#"[
        {"jsonrpc":"2.0","method":"req","id":1},
        {"jsonrpc":"2.0","method":"notif"}
    ]"#;
    let batch = match parse_json_rpc_batch(body) {
        BatchOrSingle::Batch(b) => b,
        other => panic!("expected Batch, got {other:?}"),
    };
    assert_eq!(batch.len(), 2);
    assert!(batch[0].as_ref().unwrap().is_request());
    assert!(batch[1].as_ref().unwrap().is_notification());
}

#[test]
fn batch_with_one_invalid_member_processes_others() {
    let body = r#"[
        {"jsonrpc":"2.0","method":"good","id":1},
        {"jsonrpc":"1.0","method":"bad","id":2},
        {"jsonrpc":"2.0","method":"alsoGood","id":3}
    ]"#;
    let batch = match parse_json_rpc_batch(body) {
        BatchOrSingle::Batch(b) => b,
        other => panic!("expected Batch, got {other:?}"),
    };
    assert_eq!(batch.len(), 3);
    assert!(batch[0].is_ok());
    assert!(batch[1].is_err());
    assert_eq!(batch[1].as_ref().unwrap_err().error.code, INVALID_REQUEST);
    assert!(batch[2].is_ok());
}

#[test]
fn batch_with_all_invalid_members() {
    let body = r#"[1, 2, 3]"#;
    let batch = match parse_json_rpc_batch(body) {
        BatchOrSingle::Batch(b) => b,
        other => panic!("expected Batch, got {other:?}"),
    };
    assert_eq!(batch.len(), 3);
    assert!(batch.iter().all(|r| r.is_err()));
    assert!(batch
        .iter()
        .all(|r| r.as_ref().unwrap_err().error.code == INVALID_REQUEST));
}

#[test]
fn batch_unparseable_outer_array_is_parse_error() {
    let body = r#"[{"jsonrpc":"2.0","method":"x","id":1}"#; // missing closing bracket
    match parse_json_rpc_batch(body) {
        BatchOrSingle::Single(Err(e)) => assert_eq!(e.error.code, PARSE_ERROR),
        other => panic!("expected Single(parse error), got {other:?}"),
    }
}

// -----------------------------------------------------------------------------
// Round-trip tests for response shape
// -----------------------------------------------------------------------------

#[test]
fn parse_error_response_serializes_with_null_id() {
    use turul_rpc_core::error::JsonRpcError;
    let e = JsonRpcError::parse_error();
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v["jsonrpc"], "2.0");
    assert_eq!(v["id"], Value::Null);
    assert_eq!(v["error"]["code"], json!(-32700));
    assert_eq!(v["error"]["message"], "Parse error");
}

#[test]
fn method_not_found_echoes_string_id() {
    use turul_rpc_core::error::JsonRpcError;
    let e = JsonRpcError::method_not_found(RequestId::String("abc".into()), "missing");
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v["id"], "abc");
    assert_eq!(v["error"]["code"], json!(-32601));
}

#[test]
fn method_not_found_echoes_number_id() {
    use turul_rpc_core::error::JsonRpcError;
    let e = JsonRpcError::method_not_found(RequestId::Number(7), "missing");
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v["id"], 7);
}
