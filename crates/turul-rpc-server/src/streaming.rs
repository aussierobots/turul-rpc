//! Streaming JSON-RPC dispatcher.
//!
//! See ADR-004 — the `JsonRpcFrame::Progress` and `PartialResult` variants
//! emit MCP-flavored `_meta.progress` / `_meta.partial` keys for backward
//! compatibility with `turul-mcp-json-rpc-server 0.3.x`. A future v0.2 may
//! generalise this with a caller-supplied metadata builder.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;

use turul_rpc_core::error::JsonRpcErrorObject;
use turul_rpc_core::request::{JsonRpcRequest, RequestParams};
use turul_rpc_core::types::RequestId;

use crate::handler::{JsonRpcHandler, ToJsonRpcError};
use crate::session::SessionContext;

/// JSON-RPC frame for streaming responses.
/// Represents individual chunks in a progressive response stream.
#[derive(Debug, Clone)]
pub enum JsonRpcFrame {
    /// Progress update with optional token for cancellation.
    Progress {
        request_id: RequestId,
        progress: Value,
        progress_token: Option<String>,
    },
    /// Partial result chunk.
    PartialResult { request_id: RequestId, data: Value },
    /// Final result (ends the stream).
    FinalResult {
        request_id: RequestId,
        result: Value,
    },
    /// Error result (ends the stream).
    Error {
        request_id: RequestId,
        error: JsonRpcErrorObject,
    },
    /// Notification frame (does not end the stream).
    Notification {
        method: String,
        params: Option<Value>,
    },
}

impl JsonRpcFrame {
    /// Convert frame to JSON-RPC message format.
    pub fn to_json(&self) -> Value {
        match self {
            JsonRpcFrame::Progress {
                request_id,
                progress,
                progress_token,
            } => {
                let mut obj = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "_meta": { "progress": progress }
                });
                if let Some(token) = progress_token {
                    obj["_meta"]["progressToken"] = Value::String(token.clone());
                }
                obj
            }
            JsonRpcFrame::PartialResult { request_id, data } => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "_meta": { "partial": true },
                    "result": data
                })
            }
            JsonRpcFrame::FinalResult { request_id, result } => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": result
                })
            }
            JsonRpcFrame::Error { request_id, error } => {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {
                        "code": error.code,
                        "message": &error.message,
                        "data": &error.data
                    }
                })
            }
            JsonRpcFrame::Notification { method, params } => {
                let mut obj = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": method
                });
                if let Some(params) = params {
                    obj["params"] = params.clone();
                }
                obj
            }
        }
    }

    /// Check if this frame ends the stream.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JsonRpcFrame::FinalResult { .. } | JsonRpcFrame::Error { .. }
        )
    }
}

/// Trait for handlers that support streaming responses.
#[async_trait]
pub trait StreamingJsonRpcHandler: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn handle_streaming(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session_context: Option<SessionContext>,
        request_id: RequestId,
    ) -> Pin<Box<dyn Stream<Item = Result<JsonRpcFrame, Self::Error>> + Send>>;

    async fn handle_notification(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session_context: Option<SessionContext>,
    ) -> Result<(), Self::Error> {
        let _ = (method, params, session_context);
        Ok(())
    }

    fn supported_methods(&self) -> Vec<String> {
        vec![]
    }
}

/// Streaming JSON-RPC method dispatcher.
pub struct StreamingJsonRpcDispatcher<E>
where
    E: ToJsonRpcError,
{
    streaming_handlers: HashMap<String, Arc<dyn StreamingJsonRpcHandler<Error = E>>>,
    fallback_handlers: HashMap<String, Arc<dyn JsonRpcHandler<Error = E>>>,
    default_handler: Option<Arc<dyn JsonRpcHandler<Error = E>>>,
}

impl<E> StreamingJsonRpcDispatcher<E>
where
    E: ToJsonRpcError,
{
    pub fn new() -> Self {
        Self {
            streaming_handlers: HashMap::new(),
            fallback_handlers: HashMap::new(),
            default_handler: None,
        }
    }

    pub fn register_streaming_method<H>(&mut self, method: String, handler: H)
    where
        H: StreamingJsonRpcHandler<Error = E> + 'static,
    {
        self.streaming_handlers.insert(method, Arc::new(handler));
    }

    pub fn register_fallback_method<H>(&mut self, method: String, handler: H)
    where
        H: JsonRpcHandler<Error = E> + 'static,
    {
        self.fallback_handlers.insert(method, Arc::new(handler));
    }

    pub fn set_default_handler<H>(&mut self, handler: H)
    where
        H: JsonRpcHandler<Error = E> + 'static,
    {
        self.default_handler = Some(Arc::new(handler));
    }

    /// Process a JSON-RPC request with streaming support.
    pub async fn handle_request_streaming(
        &self,
        request: JsonRpcRequest,
        session_context: SessionContext,
    ) -> Pin<Box<dyn Stream<Item = JsonRpcFrame> + Send>> {
        if let Some(streaming_handler) = self.streaming_handlers.get(&request.method) {
            let request_id_clone = request.id.clone();
            let stream = streaming_handler
                .handle_streaming(
                    &request.method,
                    request.params,
                    Some(session_context),
                    request.id.clone(),
                )
                .await;

            return Box::pin(stream.map(move |result| match result {
                Ok(frame) => frame,
                Err(domain_error) => JsonRpcFrame::Error {
                    request_id: request_id_clone.clone(),
                    error: domain_error.to_error_object(),
                },
            }));
        }

        if let Some(fallback_handler) = self
            .fallback_handlers
            .get(&request.method)
            .or(self.default_handler.as_ref())
        {
            let method = request.method.clone();
            let params = request.params.clone();
            let request_id = request.id.clone();
            let handler = fallback_handler.clone();

            return Box::pin(futures::stream::once(async move {
                match handler.handle(&method, params, Some(session_context)).await {
                    Ok(result) => JsonRpcFrame::FinalResult { request_id, result },
                    Err(domain_error) => JsonRpcFrame::Error {
                        request_id,
                        error: domain_error.to_error_object(),
                    },
                }
            }));
        }

        let error = JsonRpcErrorObject {
            code: turul_rpc_core::error_codes::METHOD_NOT_FOUND,
            message: format!("Method '{}' not found", request.method),
            data: None,
        };

        Box::pin(futures::stream::once(async move {
            JsonRpcFrame::Error {
                request_id: request.id,
                error,
            }
        }))
    }

    pub async fn handle_notification(
        &self,
        notification: turul_rpc_core::notification::JsonRpcNotification,
    ) -> Result<(), E> {
        if let Some(streaming_handler) = self.streaming_handlers.get(&notification.method) {
            return streaming_handler
                .handle_notification(&notification.method, notification.params, None)
                .await;
        }

        if let Some(fallback_handler) = self
            .fallback_handlers
            .get(&notification.method)
            .or(self.default_handler.as_ref())
        {
            return fallback_handler
                .handle_notification(&notification.method, notification.params, None)
                .await;
        }

        Ok(())
    }
}

impl<E> Default for StreamingJsonRpcDispatcher<E>
where
    E: ToJsonRpcError,
{
    fn default() -> Self {
        Self::new()
    }
}
