//! Minimal `turul-rpc` server demonstrating the handler-returns-domain-error
//! contract. Mirrors the README quick-start.
//!
//! ```text
//! cargo run -p turul-rpc --example simple_calculator
//! ```
//!
//! Then in another terminal:
//!
//! ```text
//! echo '{"jsonrpc":"2.0","method":"add","params":{"a":2,"b":3},"id":1}' \
//!   | cargo run -p turul-rpc --example simple_calculator -- --stdin
//! ```

use std::io::{self, BufRead, Write};

use async_trait::async_trait;
use serde_json::{json, Value};
use turul_rpc::error::JsonRpcErrorObject;
use turul_rpc::r#async::ToJsonRpcError;
use turul_rpc::{JsonRpcDispatcher, JsonRpcHandler, RequestParams, SessionContext};

#[derive(thiserror::Error, Debug)]
enum CalcError {
    #[error("missing parameter: {0}")]
    MissingParam(&'static str),
    #[error("unknown method: {0}")]
    UnknownMethod(String),
    #[error("division by zero")]
    DivisionByZero,
}

impl ToJsonRpcError for CalcError {
    fn to_error_object(&self) -> JsonRpcErrorObject {
        match self {
            CalcError::MissingParam(_) | CalcError::DivisionByZero => {
                JsonRpcErrorObject::invalid_params(&self.to_string())
            }
            CalcError::UnknownMethod(m) => JsonRpcErrorObject::method_not_found(m),
        }
    }
}

struct Calc;

#[async_trait]
impl JsonRpcHandler for Calc {
    type Error = CalcError;

    async fn handle(
        &self,
        method: &str,
        params: Option<RequestParams>,
        _session: Option<SessionContext>,
    ) -> Result<Value, CalcError> {
        let p = params.ok_or(CalcError::MissingParam("params"))?;
        let m = p.to_map();
        let a = m
            .get("a")
            .and_then(Value::as_f64)
            .ok_or(CalcError::MissingParam("a"))?;
        let b = m
            .get("b")
            .and_then(Value::as_f64)
            .ok_or(CalcError::MissingParam("b"))?;
        match method {
            "add" => Ok(json!({ "result": a + b })),
            "sub" => Ok(json!({ "result": a - b })),
            "mul" => Ok(json!({ "result": a * b })),
            "div" => {
                if b == 0.0 {
                    Err(CalcError::DivisionByZero)
                } else {
                    Ok(json!({ "result": a / b }))
                }
            }
            other => Err(CalcError::UnknownMethod(other.to_string())),
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["add".into(), "sub".into(), "mul".into(), "div".into()]
    }
}

fn main() -> io::Result<()> {
    let mut dispatcher: JsonRpcDispatcher<CalcError> = JsonRpcDispatcher::new();
    dispatcher.register_methods(
        vec!["add".into(), "sub".into(), "mul".into(), "div".into()],
        Calc,
    );

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    eprintln!("turul-rpc simple_calculator listening on stdin (one JSON request per line; Ctrl-D to exit)");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = runtime.block_on(dispatcher.handle_batch(&line));
        if let Some(s) = response {
            writeln!(out, "{s}")?;
            out.flush()?;
        }
    }
    Ok(())
}
