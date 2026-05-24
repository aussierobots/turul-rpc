use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::JsonRpcError;
use crate::types::{JsonRpcVersion, RequestId};

/// Result data for a JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseResult {
    /// Success result with data
    Success(Value),
    /// Null result (for void methods)
    Null,
}

impl ResponseResult {
    pub fn success(value: Value) -> Self {
        ResponseResult::Success(value)
    }

    pub fn null() -> Self {
        ResponseResult::Null
    }

    pub fn is_null(&self) -> bool {
        matches!(self, ResponseResult::Null)
    }

    pub fn as_value(&self) -> Option<&Value> {
        match self {
            ResponseResult::Success(value) => Some(value),
            ResponseResult::Null => None,
        }
    }
}

impl From<Value> for ResponseResult {
    fn from(value: Value) -> Self {
        if value.is_null() {
            ResponseResult::Null
        } else {
            ResponseResult::Success(value)
        }
    }
}

impl From<()> for ResponseResult {
    fn from(_: ()) -> Self {
        ResponseResult::Null
    }
}

/// A successful JSON-RPC response object (JSON-RPC 2.0 §5, success case):
/// `{jsonrpc, id, result}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcSuccessResponse {
    #[serde(rename = "jsonrpc")]
    pub version: JsonRpcVersion,
    pub id: RequestId,
    pub result: ResponseResult,
}

impl JsonRpcSuccessResponse {
    pub fn new(id: RequestId, result: ResponseResult) -> Self {
        Self {
            version: JsonRpcVersion::V2_0,
            id,
            result,
        }
    }

    pub fn success(id: RequestId, result: Value) -> Self {
        Self::new(id, ResponseResult::Success(result))
    }

    pub fn null(id: RequestId) -> Self {
        Self::new(id, ResponseResult::Null)
    }
}

impl<T> From<(RequestId, T)> for JsonRpcSuccessResponse
where
    T: Into<ResponseResult>,
{
    fn from((id, result): (RequestId, T)) -> Self {
        Self::new(id, result.into())
    }
}

/// A JSON-RPC Response object (JSON-RPC 2.0 §5): success XOR error.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse {
    /// Successful response carrying a `result`.
    Success(JsonRpcSuccessResponse),
    /// Error response carrying an `error`.
    Error(JsonRpcError),
}

impl JsonRpcResponse {
    /// Build a success response.
    pub fn success(id: RequestId, result: ResponseResult) -> Self {
        Self::Success(JsonRpcSuccessResponse::new(id, result))
    }

    /// Build an error response.
    pub fn error(error: JsonRpcError) -> Self {
        Self::Error(error)
    }

    /// True if this is the error variant.
    pub fn is_error(&self) -> bool {
        matches!(self, JsonRpcResponse::Error(_))
    }

    /// The request id this response correlates to, if present.
    pub fn id(&self) -> Option<&RequestId> {
        match self {
            JsonRpcResponse::Success(r) => Some(&r.id),
            JsonRpcResponse::Error(e) => e.id.as_ref(),
        }
    }
}

impl From<JsonRpcSuccessResponse> for JsonRpcResponse {
    fn from(r: JsonRpcSuccessResponse) -> Self {
        Self::Success(r)
    }
}

impl From<JsonRpcError> for JsonRpcResponse {
    fn from(e: JsonRpcError) -> Self {
        Self::Error(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{JsonRpcError, JsonRpcErrorObject};
    use serde_json::{Value, from_str, json, to_string};

    #[test]
    fn success_response_serializes_with_result_no_error() {
        let r = JsonRpcSuccessResponse::success(RequestId::Number(1), json!({"ok": true}));
        let v: Value = serde_json::to_value(&r).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert!(v.get("result").is_some());
        assert!(v.get("error").is_none());
    }

    #[test]
    fn response_union_serializes_success_shape() {
        let r = JsonRpcResponse::success(
            RequestId::Number(1),
            ResponseResult::Success(json!({"ok": true})),
        );
        let v: Value = serde_json::to_value(&r).unwrap();
        assert!(v.get("result").is_some());
        assert!(v.get("error").is_none());
        assert!(!r.is_error());
    }

    #[test]
    fn response_union_serializes_error_shape() {
        let err = JsonRpcError::new(
            Some(RequestId::Number(7)),
            JsonRpcErrorObject {
                code: -32601,
                message: "Method not found".into(),
                data: None,
            },
        );
        let r = JsonRpcResponse::error(err);
        let v: Value = serde_json::to_value(&r).unwrap();
        assert!(v.get("error").is_some());
        assert!(v.get("result").is_none());
        assert!(r.is_error());
        assert_eq!(r.id(), Some(&RequestId::Number(7)));
    }

    #[test]
    fn response_union_deserializes_both_shapes() {
        let ok: JsonRpcResponse = from_str(r#"{"jsonrpc":"2.0","id":1,"result":{"a":1}}"#).unwrap();
        assert!(matches!(ok, JsonRpcResponse::Success(_)));
        let err: JsonRpcResponse =
            from_str(r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"x"}}"#).unwrap();
        assert!(matches!(err, JsonRpcResponse::Error(_)));
    }

    #[test]
    fn success_response_round_trip_and_null() {
        let r = JsonRpcSuccessResponse::null(RequestId::String("t".into()));
        let s = to_string(&r).unwrap();
        let parsed: JsonRpcSuccessResponse = from_str(&s).unwrap();
        assert_eq!(parsed.id, RequestId::String("t".into()));
        match parsed.result {
            ResponseResult::Success(ref val) if val.is_null() => {}
            ResponseResult::Null => {}
            _ => panic!("expected null-ish result, got {:?}", parsed.result),
        }
    }

    #[test]
    fn success_response_from_tuple() {
        let r: JsonRpcSuccessResponse = (RequestId::Number(1), json!({"x": true})).into();
        assert_eq!(r.id, RequestId::Number(1));
    }

    #[test]
    fn response_result_conversion() {
        assert!(matches!(
            ResponseResult::from(json!({"d": 42})),
            ResponseResult::Success(_)
        ));
        assert!(matches!(
            ResponseResult::from(json!(null)),
            ResponseResult::Null
        ));
        assert!(matches!(ResponseResult::from(()), ResponseResult::Null));
    }
}
