use serde::{Deserialize, Serialize};
use std::fmt;

/// A uniquely identifying ID for a JSON-RPC request: a String, a Number,
/// or Null.
///
/// JSON-RPC 2.0 §4.2 permits Null ids for client→server requests but
/// discourages them; `turul-rpc` accepts Null. Fractional numeric ids are
/// still rejected (only `i64` is representable). See ADR-002 in the
/// `turul-rpc` repository for the compliance posture.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
    Null,
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestId::String(s) => write!(f, "{}", s),
            RequestId::Number(n) => write!(f, "{}", n),
            RequestId::Null => write!(f, "null"),
        }
    }
}

impl From<i64> for RequestId {
    fn from(n: i64) -> Self {
        RequestId::Number(n)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        RequestId::String(s.to_string())
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId::String(s)
    }
}

impl RequestId {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            RequestId::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            RequestId::Number(n) => Some(*n),
            _ => None,
        }
    }
}

/// JSON-RPC version
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum JsonRpcVersion {
    #[default]
    V2_0,
}

impl JsonRpcVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            JsonRpcVersion::V2_0 => "2.0",
        }
    }
}

impl fmt::Display for JsonRpcVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for JsonRpcVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for JsonRpcVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "2.0" => Ok(JsonRpcVersion::V2_0),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid JSON-RPC version: {}",
                s
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_serialization() {
        let id_str = RequestId::String("test".to_string());
        let id_num = RequestId::Number(42);

        assert_eq!(serde_json::to_string(&id_str).unwrap(), r#""test""#);
        assert_eq!(serde_json::to_string(&id_num).unwrap(), "42");
    }

    #[test]
    fn null_id_round_trips() {
        let id = RequestId::Null;
        assert_eq!(serde_json::to_string(&id).unwrap(), "null");
        let back: RequestId = serde_json::from_str("null").unwrap();
        assert_eq!(back, RequestId::Null);
    }

    #[test]
    fn string_and_number_ids_still_win_over_null() {
        // Ordering guard: concrete values must not deserialize as Null.
        let s: RequestId = serde_json::from_str(r#""abc""#).unwrap();
        assert_eq!(s, RequestId::String("abc".into()));
        let n: RequestId = serde_json::from_str("7").unwrap();
        assert_eq!(n, RequestId::Number(7));
    }

    #[test]
    fn test_json_rpc_version() {
        let version = JsonRpcVersion::V2_0;
        assert_eq!(version.as_str(), "2.0");
        assert_eq!(serde_json::to_string(&version).unwrap(), r#""2.0""#);
    }
}
