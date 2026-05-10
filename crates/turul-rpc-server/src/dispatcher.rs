use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use turul_rpc_core::error::JsonRpcError;
use turul_rpc_core::notification::JsonRpcNotification;
use turul_rpc_core::request::JsonRpcRequest;
use turul_rpc_core::response::{JsonRpcMessage, ResponseResult};
use turul_rpc_jsonrpc::batch::{BatchOrSingle, parse_json_rpc_batch};
use turul_rpc_jsonrpc::{JsonRpcMessage as IncomingMessage, JsonRpcMessageResult};

use crate::handler::{JsonRpcHandler, ToJsonRpcError};
use crate::session::SessionContext;

/// JSON-RPC method dispatcher with a typed domain-error type.
pub struct JsonRpcDispatcher<E>
where
    E: ToJsonRpcError,
{
    pub handlers: HashMap<String, Arc<dyn JsonRpcHandler<Error = E>>>,
    pub default_handler: Option<Arc<dyn JsonRpcHandler<Error = E>>>,
}

impl<E> JsonRpcDispatcher<E>
where
    E: ToJsonRpcError,
{
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            default_handler: None,
        }
    }

    /// Register a handler for a specific method.
    pub fn register_method<H>(&mut self, method: String, handler: H)
    where
        H: JsonRpcHandler<Error = E> + 'static,
    {
        self.handlers.insert(method, Arc::new(handler));
    }

    /// Register a handler for multiple methods.
    pub fn register_methods<H>(&mut self, methods: Vec<String>, handler: H)
    where
        H: JsonRpcHandler<Error = E> + 'static,
    {
        let handler_arc = Arc::new(handler);
        for method in methods {
            self.handlers.insert(method, handler_arc.clone());
        }
    }

    /// Set a default handler for unregistered methods.
    pub fn set_default_handler<H>(&mut self, handler: H)
    where
        H: JsonRpcHandler<Error = E> + 'static,
    {
        self.default_handler = Some(Arc::new(handler));
    }

    /// Process a JSON-RPC request with session context and return a response.
    pub async fn handle_request_with_context(
        &self,
        request: JsonRpcRequest,
        session_context: SessionContext,
    ) -> JsonRpcMessage {
        let handler = self
            .handlers
            .get(&request.method)
            .or(self.default_handler.as_ref());

        match handler {
            Some(handler) => {
                match handler
                    .handle(&request.method, request.params, Some(session_context))
                    .await
                {
                    Ok(result) => {
                        JsonRpcMessage::success(request.id, ResponseResult::Success(result))
                    }
                    Err(domain_error) => {
                        let error_object = domain_error.to_error_object();
                        let rpc_error = JsonRpcError::new(Some(request.id.clone()), error_object);
                        JsonRpcMessage::error(rpc_error)
                    }
                }
            }
            None => {
                let error = JsonRpcError::method_not_found(request.id.clone(), &request.method);
                JsonRpcMessage::error(error)
            }
        }
    }

    /// Process a JSON-RPC request and return a response (no session context).
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcMessage {
        let handler = self
            .handlers
            .get(&request.method)
            .or(self.default_handler.as_ref());

        match handler {
            Some(handler) => match handler.handle(&request.method, request.params, None).await {
                Ok(result) => JsonRpcMessage::success(request.id, ResponseResult::Success(result)),
                Err(domain_error) => {
                    let error_object = domain_error.to_error_object();
                    let rpc_error = JsonRpcError::new(Some(request.id.clone()), error_object);
                    JsonRpcMessage::error(rpc_error)
                }
            },
            None => {
                let error = JsonRpcError::method_not_found(request.id.clone(), &request.method);
                JsonRpcMessage::error(error)
            }
        }
    }

    /// Process a JSON-RPC notification.
    pub async fn handle_notification(&self, notification: JsonRpcNotification) -> Result<(), E> {
        let handler = self
            .handlers
            .get(&notification.method)
            .or(self.default_handler.as_ref());

        match handler {
            Some(handler) => {
                handler
                    .handle_notification(&notification.method, notification.params, None)
                    .await
            }
            None => Ok(()),
        }
    }

    /// Process a JSON-RPC notification with session context.
    pub async fn handle_notification_with_context(
        &self,
        notification: JsonRpcNotification,
        session_context: Option<SessionContext>,
    ) -> Result<(), E> {
        let handler = self
            .handlers
            .get(&notification.method)
            .or(self.default_handler.as_ref());

        match handler {
            Some(handler) => {
                handler
                    .handle_notification(&notification.method, notification.params, session_context)
                    .await
            }
            None => Ok(()),
        }
    }

    /// Dispatch a request body that may be a single message or a batch.
    ///
    /// Per JSON-RPC 2.0 §6:
    /// - Single object → returns the JSON response string.
    /// - Batch (non-empty array) → returns the JSON array of responses,
    ///   omitting notifications. Returns `None` if the batch consisted
    ///   entirely of notifications.
    /// - Empty batch (`[]`) → returns a single `Invalid Request` (`-32600`)
    ///   with `id: null`.
    /// - Parse error → returns a single `Parse error` (`-32700`) with
    ///   `id: null`.
    pub async fn handle_batch(&self, body: &str) -> Option<String> {
        match parse_json_rpc_batch(body) {
            BatchOrSingle::EmptyBatch => {
                let err = JsonRpcError::invalid_request(None);
                Some(serde_json::to_string(&err).unwrap_or_default())
            }
            BatchOrSingle::Single(parsed) => {
                let result = self.dispatch_one(parsed).await;
                result.to_json_string()
            }
            BatchOrSingle::Batch(items) => {
                let mut responses: Vec<Value> = Vec::with_capacity(items.len());
                for parsed in items {
                    let r = self.dispatch_one(parsed).await;
                    let v = match r {
                        JsonRpcMessageResult::Response(resp) => serde_json::to_value(&resp).ok(),
                        JsonRpcMessageResult::Error(err) => serde_json::to_value(&err).ok(),
                        JsonRpcMessageResult::NoResponse => None,
                    };
                    if let Some(v) = v {
                        responses.push(v);
                    }
                }
                if responses.is_empty() {
                    None
                } else {
                    serde_json::to_string(&responses).ok()
                }
            }
        }
    }

    async fn dispatch_one(
        &self,
        parsed: Result<IncomingMessage, JsonRpcError>,
    ) -> JsonRpcMessageResult {
        match parsed {
            Err(e) => JsonRpcMessageResult::Error(e),
            Ok(IncomingMessage::Request(req)) => match self.handle_request(req).await {
                JsonRpcMessage::Response(r) => JsonRpcMessageResult::Response(r),
                JsonRpcMessage::Error(e) => JsonRpcMessageResult::Error(e),
            },
            Ok(IncomingMessage::Notification(notif)) => {
                let _ = self.handle_notification(notif).await;
                JsonRpcMessageResult::NoResponse
            }
        }
    }

    /// Get all registered method names.
    pub fn registered_methods(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
}

impl<E> Default for JsonRpcDispatcher<E>
where
    E: ToJsonRpcError,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use turul_rpc_core::error::JsonRpcErrorObject;
    use turul_rpc_core::request::RequestParams;
    use turul_rpc_core::types::RequestId;

    #[derive(thiserror::Error, Debug)]
    enum TestError {
        #[error("Test error: {0}")]
        TestError(String),
        #[error("Unknown method: {0}")]
        UnknownMethod(String),
    }

    impl ToJsonRpcError for TestError {
        fn to_error_object(&self) -> JsonRpcErrorObject {
            match self {
                TestError::TestError(msg) => JsonRpcErrorObject::internal_error(Some(msg.clone())),
                TestError::UnknownMethod(method) => JsonRpcErrorObject::method_not_found(method),
            }
        }
    }

    struct TestHandler;

    #[async_trait]
    impl JsonRpcHandler for TestHandler {
        type Error = TestError;

        async fn handle(
            &self,
            method: &str,
            _params: Option<RequestParams>,
            _session_context: Option<SessionContext>,
        ) -> Result<Value, Self::Error> {
            match method {
                "add" => Ok(json!({"result": "addition"})),
                "error" => Err(TestError::TestError("test error".to_string())),
                _ => Err(TestError::UnknownMethod(method.to_string())),
            }
        }

        fn supported_methods(&self) -> Vec<String> {
            vec!["add".to_string(), "error".to_string()]
        }
    }

    #[tokio::test]
    async fn test_dispatcher_success() {
        let mut dispatcher: JsonRpcDispatcher<TestError> = JsonRpcDispatcher::new();
        dispatcher.register_method("add".to_string(), TestHandler);

        let request = JsonRpcRequest::new_no_params(RequestId::Number(1), "add".to_string());

        let response = dispatcher.handle_request(request).await;
        assert_eq!(response.id(), Some(&RequestId::Number(1)));
        assert!(!response.is_error());
    }

    #[tokio::test]
    async fn test_dispatcher_method_not_found() {
        let dispatcher: JsonRpcDispatcher<TestError> = JsonRpcDispatcher::new();

        let request = JsonRpcRequest::new_no_params(RequestId::Number(1), "unknown".to_string());

        let response = dispatcher.handle_request(request).await;
        assert_eq!(response.id(), Some(&RequestId::Number(1)));
        assert!(response.is_error());
    }

    #[tokio::test]
    async fn test_function_handler() {
        let handler = TestHandler;
        let result = handler.handle("add", None, None).await.unwrap();
        assert_eq!(result["result"], "addition");
    }

    // -- Batch tests (ADR-002 compliance) --

    #[tokio::test]
    async fn test_batch_empty_returns_single_invalid_request() {
        let dispatcher: JsonRpcDispatcher<TestError> = JsonRpcDispatcher::new();
        let body = "[]";
        let response = dispatcher.handle_batch(body).await.unwrap();
        let v: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(v["error"]["code"], -32600);
        assert_eq!(v["id"], Value::Null);
    }

    #[tokio::test]
    async fn test_batch_two_requests_returns_array() {
        let mut dispatcher: JsonRpcDispatcher<TestError> = JsonRpcDispatcher::new();
        dispatcher.register_method("add".to_string(), TestHandler);

        let body = r#"[
            {"jsonrpc":"2.0","method":"add","id":1},
            {"jsonrpc":"2.0","method":"add","id":2}
        ]"#;
        let response = dispatcher.handle_batch(body).await.unwrap();
        let v: Value = serde_json::from_str(&response).unwrap();
        let arr = v.as_array().expect("response should be a JSON array");
        assert_eq!(arr.len(), 2);
        // ids preserved
        let ids: Vec<&Value> = arr.iter().map(|e| &e["id"]).collect();
        assert!(ids.contains(&&json!(1)));
        assert!(ids.contains(&&json!(2)));
    }

    #[tokio::test]
    async fn test_batch_mixed_omits_notification_responses() {
        let mut dispatcher: JsonRpcDispatcher<TestError> = JsonRpcDispatcher::new();
        dispatcher.register_method("add".to_string(), TestHandler);

        let body = r#"[
            {"jsonrpc":"2.0","method":"add","id":1},
            {"jsonrpc":"2.0","method":"add"}
        ]"#;
        let response = dispatcher.handle_batch(body).await.unwrap();
        let v: Value = serde_json::from_str(&response).unwrap();
        let arr = v.as_array().expect("response should be a JSON array");
        assert_eq!(arr.len(), 1, "notification must not produce response entry");
        assert_eq!(arr[0]["id"], 1);
    }

    #[tokio::test]
    async fn test_batch_all_notifications_returns_no_response() {
        let mut dispatcher: JsonRpcDispatcher<TestError> = JsonRpcDispatcher::new();
        dispatcher.register_method("add".to_string(), TestHandler);

        let body = r#"[
            {"jsonrpc":"2.0","method":"add"},
            {"jsonrpc":"2.0","method":"add"}
        ]"#;
        assert!(dispatcher.handle_batch(body).await.is_none());
    }

    #[tokio::test]
    async fn test_batch_one_invalid_member_others_succeed() {
        let mut dispatcher: JsonRpcDispatcher<TestError> = JsonRpcDispatcher::new();
        dispatcher.register_method("add".to_string(), TestHandler);

        let body = r#"[
            {"jsonrpc":"2.0","method":"add","id":1},
            {"jsonrpc":"1.0","method":"bad","id":2},
            {"jsonrpc":"2.0","method":"add","id":3}
        ]"#;
        let response = dispatcher.handle_batch(body).await.unwrap();
        let v: Value = serde_json::from_str(&response).unwrap();
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        // The invalid one comes back as an INVALID_REQUEST error.
        assert!(arr.iter().any(|e| e["error"]["code"] == -32600));
    }

    #[tokio::test]
    async fn test_batch_parse_error_returns_single_parse_error() {
        let dispatcher: JsonRpcDispatcher<TestError> = JsonRpcDispatcher::new();
        let body = r#"["#;
        let response = dispatcher.handle_batch(body).await.unwrap();
        let v: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(v["error"]["code"], -32700);
        assert_eq!(v["id"], Value::Null);
    }

    #[tokio::test]
    async fn test_batch_single_object_dispatches_normally() {
        let mut dispatcher: JsonRpcDispatcher<TestError> = JsonRpcDispatcher::new();
        dispatcher.register_method("add".to_string(), TestHandler);

        let body = r#"{"jsonrpc":"2.0","method":"add","id":42}"#;
        let response = dispatcher.handle_batch(body).await.unwrap();
        let v: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(v["id"], 42);
        assert!(!v.as_object().unwrap().contains_key("error"));
    }
}
