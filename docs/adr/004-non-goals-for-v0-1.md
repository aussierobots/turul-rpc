# ADR-004: Non-goals for v0.1

**Status**: Accepted
**Date**: 2026-05-10

## Context

The first goal of `turul-rpc` is **functional replication**: extract the
generic JSON-RPC 2.0 layer that already exists inside
`turul-mcp-json-rpc-server`, restructure it into the four-crate layout from
ADR-001, ship it as v0.1.0, and preserve the type identity required by
ADR-003.

It is tempting to do more during the move. Each "while we're in there"
expansion is rejected here so the v0.1 surface stays small and the shim
diff stays empty.

## Decision

### What v0.1 explicitly DOES

- Implement JSON-RPC 2.0 **batch** processing in `turul-rpc-jsonrpc` and
  the dispatcher (ADR-002). The original crate's batch was an
  unimplemented stub with a misleading comment; shipping a "JSON-RPC 2.0
  framework" without batch would be false advertising. Tests in
  `turul-rpc-jsonrpc/tests/spec_conformance.rs` lock in the behaviour.
- Honour every public type/path the original published (ADR-003), via
  `pub use` chains. No newtype wrappers, no signature changes.

### What v0.1 explicitly does NOT do

| Excluded | Why |
|---|---|
| **MCP-specific types or methods** (`Tool`, `Resource`, `Prompt`, `Task`, MCP capability negotiation, MCP method strings, `notifications/*/list_changed`) | These belong in `turul-mcp-protocol` and `turul-mcp-server`. `turul-rpc` is the generic substrate, not the MCP runtime. |
| **HTTP / SSE / stdio / Lambda transports** | `turul-rpc` is transport-agnostic. Transport adapters belong in caller crates (or a future `turul-rpc-http` if real demand appears). |
| **A `turul-rpc-client` crate** | No client code exists in `turul-mcp-json-rpc-server` to extract. `turul-mcp-client` does not consume json-rpc-server types today (it builds requests via raw `serde_json::json!`). Building a generic typed RPC client is new design, not extraction. Defer to a future minor release if real demand materialises. |
| **A `turul-rpc-derive` crate** | All derives in `turul-mcp-derive` are MCP-domain (`McpTool`, `McpResource`, `McpPrompt`, `mcp_tool`). No JSON-RPC-generic codegen exists. Publishing an empty crate would be misleading. |
| **Stripping MCP-flavoured `_meta` keys from `JsonRpcFrame`** in `turul-rpc-server::streaming` | The variants (`Progress`, `PartialResult`) embed `_meta.progress`, `_meta.progressToken`, `_meta.partial` keys originally added for MCP's progress-notification convention. Removing them would break type identity for the shim. Cleanup deferred to v0.2 with an explicit migration. |
| **Resolving the `JsonRpcMessage` name collision** between `turul-rpc-core::response::JsonRpcMessage` (response union) and `turul-rpc-jsonrpc::dispatch::JsonRpcMessage` (incoming union) | Same type-identity reason. Both names existed in the original crate at different module paths; v0.1 preserves both. v0.2 may rename the incoming union to `IncomingMessage` with a deprecated alias. |
| **Generic-izing `SessionContext`** to remove the MCP-flavoured `broadcaster: Option<Arc<dyn Any + Send + Sync>>` and `extensions: HashMap<String, Value>` fields | The fields are structurally generic (`Arc<dyn Any>` and `HashMap` are stdlib types) and the original crate ships them this way. Breaking them out into a generic-parameterised type would change `SessionContext`'s signature and break downstream code that names it directly. v0.2 candidate. |
| **Hiding `JsonRpcDispatcher::{handlers, default_handler}`** (currently `pub` fields) | Original crate exposes them; downstream might pattern-match. Preserve in v0.1. v0.2 candidate. |
| **Adding new authentication / middleware / interceptor APIs** | Not in scope for "extract what exists". |
| **Removing dead code** (`JsonRpcTransportError`, `FunctionHandler`, `JsonRpcMessageResult`'s helper methods that have zero external consumers per workspace grep) | These are public items in the original crate. Dropping them = breaking change. v0.2 candidate (with an empty deprecation cycle in v0.1.1 if we ever publish one). |

### Decision logic, in one sentence

Anything that would change the `cargo public-api` diff for the
`turul-mcp-json-rpc-server 0.3.39` shim is out of scope for v0.1. The only
exception is **adding** the batch APIs, because those were a documented
spec gap in the original (per ADR-002).

## Consequences

**Positive**

- The shim diff stays small and reviewable.
- v0.1 ships fast.
- v0.2 has a clear backlog of "do these properly, with a real migration"
  items.

**Negative**

- v0.1 inherits some shape decisions that, in isolation, we would not
  make today (e.g. MCP-flavoured streaming frames in a generic crate).
  Documented and time-bounded.

## Revision log

- 2026-05-10: Initial proposal accepted.
- 2026-05-10: Revised — explicitly INCLUDE batch in v0.1 (was previously
  marked "deferred"). Compliance is a v0.1 success criterion per ADR-002,
  and shipping without batch would be false advertising.
