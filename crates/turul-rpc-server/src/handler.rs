use async_trait::async_trait;
use serde_json::Value;

use turul_rpc_core::error::JsonRpcErrorObject;
use turul_rpc_core::request::RequestParams;

use crate::session::SessionContext;

/// Trait for handling JSON-RPC method calls.
///
/// Handlers return their own domain error type. The dispatcher converts
/// it to a JSON-RPC error response via [`ToJsonRpcError`].
#[async_trait]
pub trait JsonRpcHandler: Send + Sync {
    /// The error type returned by this handler.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Handle a JSON-RPC method call with optional session context.
    /// Returns domain errors only — the dispatcher converts them to
    /// JSON-RPC errors.
    async fn handle(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session_context: Option<SessionContext>,
    ) -> Result<Value, Self::Error>;

    /// Handle a JSON-RPC notification with optional session context.
    /// Default implementation does nothing.
    async fn handle_notification(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session_context: Option<SessionContext>,
    ) -> Result<(), Self::Error> {
        let _ = (method, params, session_context);
        Ok(())
    }

    /// List supported methods (optional — used for introspection).
    fn supported_methods(&self) -> Vec<String> {
        vec![]
    }
}

/// A simple function-based handler.
pub struct FunctionHandler<F, N, E>
where
    E: std::error::Error + Send + Sync + 'static,
    F: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<Value, E>>
        + Send
        + Sync,
    N: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<(), E>>
        + Send
        + Sync,
{
    handler_fn: F,
    notification_fn: Option<N>,
    methods: Vec<String>,
}

impl<F, N, E> FunctionHandler<F, N, E>
where
    E: std::error::Error + Send + Sync + 'static,
    F: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<Value, E>>
        + Send
        + Sync,
    N: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<(), E>>
        + Send
        + Sync,
{
    pub fn new(handler_fn: F) -> Self {
        Self {
            handler_fn,
            notification_fn: None,
            methods: vec![],
        }
    }

    pub fn with_notification_handler(mut self, notification_fn: N) -> Self {
        self.notification_fn = Some(notification_fn);
        self
    }

    pub fn with_methods(mut self, methods: Vec<String>) -> Self {
        self.methods = methods;
        self
    }
}

#[async_trait]
impl<F, N, E> JsonRpcHandler for FunctionHandler<F, N, E>
where
    E: std::error::Error + Send + Sync + 'static,
    F: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<Value, E>>
        + Send
        + Sync,
    N: Fn(
            &str,
            Option<RequestParams>,
            Option<SessionContext>,
        ) -> futures::future::BoxFuture<'static, Result<(), E>>
        + Send
        + Sync,
{
    type Error = E;

    async fn handle(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session_context: Option<SessionContext>,
    ) -> Result<Value, Self::Error> {
        (self.handler_fn)(method, params, session_context).await
    }

    async fn handle_notification(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session_context: Option<SessionContext>,
    ) -> Result<(), Self::Error> {
        if let Some(ref notification_fn) = self.notification_fn {
            (notification_fn)(method, params, session_context).await
        } else {
            Ok(())
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        self.methods.clone()
    }
}

/// Trait for errors that can be converted to JSON-RPC error objects.
///
/// Implement this on your domain error type so the dispatcher can map
/// handler failures to spec-compliant JSON-RPC error responses.
pub trait ToJsonRpcError: std::error::Error + Send + Sync + 'static {
    /// Convert this error to a JSON-RPC error object.
    fn to_error_object(&self) -> JsonRpcErrorObject;
}
