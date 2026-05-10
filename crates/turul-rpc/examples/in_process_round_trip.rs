//! In-process JSON-RPC 2.0 round trip — no HTTP, no async runtime concerns.
//!
//! Demonstrates the "calling story" using only `turul-rpc-core` wire types
//! and the `JsonRpcDispatcher` directly. This is the pattern any future
//! `turul-rpc-client` would build on:
//!
//!   1. Generate a request id.
//!   2. Construct a `JsonRpcRequest` (or batch).
//!   3. Serialize to JSON.
//!   4. Hand the JSON to the server (here: in-process; in production: an
//!      HTTP/SSE/stdio transport of your choice).
//!   5. Parse the JSON response back into a `JsonRpcMessage` (response
//!      union) or `JsonRpcError`.
//!   6. Correlate the response id back to the originating request id.
//!
//! ```text
//! cargo run -p turul-rpc --example in_process_round_trip
//! ```

use std::sync::atomic::{AtomicI64, Ordering};

use async_trait::async_trait;
use serde_json::{Value, json};
use turul_rpc::r#async::ToJsonRpcError;
use turul_rpc::dispatch::{JsonRpcMessage as IncomingMessage, parse_json_rpc_message};
use turul_rpc::error::JsonRpcErrorObject;
use turul_rpc::{
    JsonRpcDispatcher, JsonRpcHandler, JsonRpcMessage, JsonRpcRequest, RequestId, RequestParams,
    SessionContext,
};

// ---------------------------------------------------------------------------
// 1. A trivial server: an "add" method that returns the sum of two numbers.
// ---------------------------------------------------------------------------

#[derive(thiserror::Error, Debug)]
enum CalcError {
    #[error("invalid params: {0}")]
    Invalid(&'static str),
}

impl ToJsonRpcError for CalcError {
    fn to_error_object(&self) -> JsonRpcErrorObject {
        JsonRpcErrorObject::invalid_params(&self.to_string())
    }
}

struct Add;

#[async_trait]
impl JsonRpcHandler for Add {
    type Error = CalcError;

    async fn handle(
        &self,
        _method: &str,
        params: Option<RequestParams>,
        _session: Option<SessionContext>,
    ) -> Result<Value, CalcError> {
        let m = params.ok_or(CalcError::Invalid("missing params"))?.to_map();
        let a = m
            .get("a")
            .and_then(Value::as_f64)
            .ok_or(CalcError::Invalid("a"))?;
        let b = m
            .get("b")
            .and_then(Value::as_f64)
            .ok_or(CalcError::Invalid("b"))?;
        Ok(json!({ "sum": a + b }))
    }
}

// ---------------------------------------------------------------------------
// 2. A monotonic id generator. Any future turul-rpc-client would expose a
//    `RequestIdGenerator` trait; for now this shows the smallest workable
//    pattern. UUID v7 would be another reasonable choice.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct MonotonicIds {
    next: AtomicI64,
}

impl MonotonicIds {
    fn next(&self) -> RequestId {
        RequestId::Number(self.next.fetch_add(1, Ordering::Relaxed))
    }
}

// ---------------------------------------------------------------------------
// 3. An in-process "transport" that takes a JSON string and returns the
//    response JSON string. In production, this is HTTP/SSE/stdio. The
//    function signature deliberately hides whether the implementation is
//    local or remote — that's the point of JSON-RPC over a transport.
// ---------------------------------------------------------------------------

async fn transport(dispatcher: &JsonRpcDispatcher<CalcError>, body: &str) -> Option<String> {
    dispatcher.handle_batch(body).await
}

// ---------------------------------------------------------------------------
// 4. A typed call helper showing the full round-trip:
//    construct → serialize → send → parse → correlate.
// ---------------------------------------------------------------------------

async fn call_add(
    dispatcher: &JsonRpcDispatcher<CalcError>,
    ids: &MonotonicIds,
    a: f64,
    b: f64,
) -> Result<f64, String> {
    // (1) generate id
    let id = ids.next();

    // (2) construct typed request
    let mut params = std::collections::HashMap::new();
    params.insert("a".to_string(), json!(a));
    params.insert("b".to_string(), json!(b));
    let req = JsonRpcRequest::new_with_object_params(id.clone(), "add".to_string(), params);

    // (3) serialize
    let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;
    println!("→ request:  {body}");

    // (4) send
    let response_body = transport(dispatcher, &body)
        .await
        .ok_or_else(|| "no response (notification?)".to_string())?;
    println!("← response: {response_body}");

    // (5) parse — JsonRpcMessage is the outbound union (Response | Error)
    let response: JsonRpcMessage =
        serde_json::from_str(&response_body).map_err(|e| e.to_string())?;

    // (6) correlate id
    if response.id() != Some(&id) {
        return Err(format!(
            "id mismatch: sent {:?}, received {:?}",
            id,
            response.id()
        ));
    }

    match response {
        JsonRpcMessage::Response(r) => {
            let v = r.result.as_value().ok_or("null result")?;
            v.get("sum")
                .and_then(Value::as_f64)
                .ok_or_else(|| "missing sum".to_string())
        }
        JsonRpcMessage::Error(e) => Err(format!("rpc error {}: {}", e.error.code, e.error.message)),
    }
}

// ---------------------------------------------------------------------------
// 5. parse_json_rpc_message demo: parsing an incoming wire body without a
//    dispatcher (e.g. a client receiving a notification it must dispatch
//    itself).
// ---------------------------------------------------------------------------

fn parse_demo() {
    let wire = r#"{"jsonrpc":"2.0","method":"server_event","params":{"value":42}}"#;
    let parsed = parse_json_rpc_message(wire).expect("valid notification");
    match parsed {
        IncomingMessage::Request(_) => unreachable!(),
        IncomingMessage::Notification(n) => {
            println!(
                "\nparsed notification: method={} params={:?}",
                n.method, n.params
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 6. main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let mut dispatcher: JsonRpcDispatcher<CalcError> = JsonRpcDispatcher::new();
    dispatcher.register_method("add".into(), Add);

    let ids = MonotonicIds::default();

    println!("=== single round-trip ===");
    let result = call_add(&dispatcher, &ids, 2.0, 3.0).await;
    println!("result: {result:?}");

    println!("\n=== second call (id increments) ===");
    let result = call_add(&dispatcher, &ids, 10.5, 0.5).await;
    println!("result: {result:?}");

    parse_demo();
}
