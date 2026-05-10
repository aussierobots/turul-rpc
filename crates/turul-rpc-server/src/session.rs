use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Minimal session context for JSON-RPC handlers.
///
/// Provides basic session information without imposing a transport.
/// Designed to be passed by value (it is `Clone`) into handlers.
///
/// ## Field semantics
///
/// - `session_id`: opaque session identifier; format is caller's choice.
/// - `metadata`: free-form key/value bag, persisted by the caller's
///   session storage if any.
/// - `broadcaster`: optional type-erased back-channel installed by the
///   transport. Stored as `Arc<dyn Any + Send + Sync>` so this crate does
///   not depend on any specific notification channel implementation.
///   Downstream code (e.g. an MCP server) downcasts to its own concrete
///   type.
/// - `timestamp`: Unix milliseconds when the context was created.
/// - `extensions`: request-scoped key/value bag for auth claims, middleware
///   data, etc. Populated by the transport layer; never persisted.
#[derive(Debug, Clone)]
pub struct SessionContext {
    /// Unique session identifier.
    pub session_id: String,
    /// Session metadata.
    pub metadata: HashMap<String, Value>,
    /// Optional type-erased broadcaster for session notifications.
    ///
    /// Stored as `Arc<dyn Any>` to avoid coupling this crate to any
    /// specific notification channel type. Transport layers downcast as
    /// needed.
    pub broadcaster: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// Session timestamp (Unix milliseconds).
    pub timestamp: u64,
    /// Request-scoped extensions (auth claims, middleware data).
    ///
    /// Populated by the transport layer; never persisted to session
    /// storage by `turul-rpc`.
    pub extensions: HashMap<String, Value>,
}
