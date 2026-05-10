//! Demonstrates JSON-RPC 2.0 §6 batch dispatch through `JsonRpcDispatcher`.
//!
//! Sends three batches and prints the response shape:
//!
//! 1. Mixed batch (request + notification + request) → array of two responses.
//! 2. All-notifications batch → no response (None).
//! 3. Empty batch `[]` → single Invalid Request (-32600) per spec.
//!
//! ```text
//! cargo run -p turul-rpc --example batch_dispatch
//! ```

use async_trait::async_trait;
use serde_json::{Value, json};
use turul_rpc::r#async::ToJsonRpcError;
use turul_rpc::error::JsonRpcErrorObject;
use turul_rpc::{JsonRpcDispatcher, JsonRpcHandler, RequestParams, SessionContext};

#[derive(thiserror::Error, Debug)]
#[error("echo error: {0}")]
struct EchoError(String);

impl ToJsonRpcError for EchoError {
    fn to_error_object(&self) -> JsonRpcErrorObject {
        JsonRpcErrorObject::internal_error(Some(self.0.clone()))
    }
}

struct Echo;

#[async_trait]
impl JsonRpcHandler for Echo {
    type Error = EchoError;

    async fn handle(
        &self,
        method: &str,
        params: Option<RequestParams>,
        _session: Option<SessionContext>,
    ) -> Result<Value, EchoError> {
        Ok(
            json!({ "echoed_method": method, "echoed_params": params.as_ref().map(|p| p.to_value()) }),
        )
    }

    async fn handle_notification(
        &self,
        method: &str,
        _params: Option<RequestParams>,
        _session: Option<SessionContext>,
    ) -> Result<(), EchoError> {
        eprintln!("[notification observed: {method}]");
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let mut dispatcher: JsonRpcDispatcher<EchoError> = JsonRpcDispatcher::new();
    dispatcher.register_method("echo".into(), Echo);

    let cases = [
        (
            "Mixed batch (request + notification + request)",
            r#"[
                {"jsonrpc":"2.0","method":"echo","params":{"a":1},"id":1},
                {"jsonrpc":"2.0","method":"echo","params":{"side_effect":true}},
                {"jsonrpc":"2.0","method":"echo","params":{"a":2},"id":2}
            ]"#,
        ),
        (
            "All-notifications batch",
            r#"[
                {"jsonrpc":"2.0","method":"echo"},
                {"jsonrpc":"2.0","method":"echo"}
            ]"#,
        ),
        ("Empty batch", r#"[]"#),
    ];

    for (label, body) in cases {
        println!("\n=== {label} ===");
        match dispatcher.handle_batch(body).await {
            Some(s) => println!("response: {s}"),
            None => println!("response: <no body>  (all-notifications batch)"),
        }
    }
}
