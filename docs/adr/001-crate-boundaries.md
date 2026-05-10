# ADR-001: Crate boundaries

**Status**: Accepted
**Date**: 2026-05-10

## Context

`turul-rpc` is extracted from the `turul-mcp-json-rpc-server` crate of the
turul-mcp-framework. The original is a single 1,800-LoC crate that mixes
wire types, codec/parsing, and async dispatch. For v0.1, we need to choose
how many crates to publish and where the seams sit.

Two requirements push opposite ways:

- **Reuse without runtime tax.** A future `turul-rpc-client` (or any
  no-async consumer) should be able to depend on the wire types without
  pulling in `tokio`, `async-trait`, or `futures`.
- **Minimum viable surface.** Splitting too aggressively before any
  external consumer exists invites churn we cannot reverse without breaking
  semver.

The original crate's three responsibilities — types, codec, dispatch —
already align with three natural runtime profiles (zero-deps,
serde-only, async-runtime). The seams are pre-cut.

## Decision

Publish **four crates** from this workspace at v0.1.0:

1. **`turul-rpc-core`** — pure JSON-RPC 2.0 wire types.
   - Modules: `types`, `error`, `request`, `response`, `notification`,
     `error_codes`.
   - Public items: `RequestId`, `JsonRpcVersion`, `JsonRpcRequest`,
     `RequestParams`, `JsonRpcResponse`, `ResponseResult`, `JsonRpcMessage`
     (response union), `JsonRpcNotification`, `JsonRpcError`,
     `JsonRpcErrorObject`, `JsonRpcErrorCode`, `JsonRpcTransportError`,
     `JSONRPC_VERSION`.
   - Dependencies: `serde`, `serde_json`, `thiserror`. **No async, no
     codec helpers, no parser.**

2. **`turul-rpc-jsonrpc`** — JSON-RPC 2.0 codec.
   - Modules: `dispatch` (parser + helper constructors).
   - Public items: `JsonRpcMessage` (incoming union — `Request | Notification`),
     `JsonRpcMessageResult`, `parse_json_rpc_message`,
     `parse_json_rpc_messages`, `parse_json_rpc_batch`, `BatchOrSingle`,
     `create_success_response`, `create_error_response`.
   - Depends on `turul-rpc-core` only.
   - Implements **JSON-RPC 2.0 batch** parsing per spec.

3. **`turul-rpc-server`** — async dispatcher.
   - Modules: `handler`, `session`, `dispatcher`, `streaming` (feature
     `streams`).
   - Public items: `JsonRpcHandler`, `ToJsonRpcError`, `FunctionHandler`,
     `SessionContext`, `JsonRpcDispatcher<E>` (with `handle_batch` method),
     `JsonRpcFrame`, `StreamingJsonRpcHandler`, `StreamingJsonRpcDispatcher<E>`.
   - Depends on `turul-rpc-core`, `turul-rpc-jsonrpc`, `async-trait`,
     `futures` (optional via `streams` feature).

4. **`turul-rpc`** — facade re-export.
   - `pub use` of the three crates above under stable module paths
     (`turul_rpc::error::*`, `turul_rpc::request::*`, etc.) plus a
     `prelude` module mirroring the original `turul-mcp-json-rpc-server`
     shape.
   - Most consumers depend on this crate.

The two `JsonRpcMessage` types (response union in core, incoming union in
jsonrpc) live in different crates' namespaces and never collide at a single
import path. The historical name collision in the original
`turul-mcp-json-rpc-server` is preserved only at the shim's compatibility
surface; new code that imports `turul_rpc::*` reaches the response union
(matches the original re-export at `lib.rs:72`).

## Consequences

**Positive**

- Wire types are usable without an async runtime — opens the door to
  `turul-rpc-client` and embedded use cases without restructuring.
- Codec is testable in isolation (spec conformance tests live in
  `turul-rpc-jsonrpc/tests/`, no async fixtures required).
- The dispatcher crate carries the `tokio`/`async-trait`/`futures` weight
  alone; the other three are runtime-light.
- The facade gives downstream a single path that matches the original
  `turul-mcp-json-rpc-server` shape, satisfying ADR-003.

**Negative**

- Four crates to publish in dependency order on every release.
- Anyone depending on a sub-crate directly couples to the split — a future
  rearrangement would be a breaking change for them. Mitigated by directing
  most consumers to `turul-rpc` (the facade).

**Out of scope (do not publish in v0.1)**

- `turul-rpc-client` — no client code exists in `turul-mcp-json-rpc-server`
  to extract; building one would be new design, not extraction.
- `turul-rpc-derive` — no JSON-RPC-generic proc-macros exist; macros in
  `turul-mcp-derive` are entirely MCP-domain.

These names may be reserved on crates.io (placeholder publishes are a
deployment decision, not a code decision); they will be designed and
published in a future minor release if real demand materializes.

## Alternatives considered

1. **Single `turul-rpc` crate** matching the original layout. Rejected:
   couples wire types to the async runtime forever; closes the door to a
   no-async client.
2. **Five crates** (split codec from server further, publish empty client
   and derive crates). Rejected: empty crates mislead; codec is too small
   to justify a second wire-only crate.
3. **Two crates** (`turul-rpc-core` + `turul-rpc-server` with codec folded
   into core). Rejected: forces the parser to live in the wire-types
   crate, which then needs to know how parse errors are surfaced — a
   responsibility that belongs to the codec layer.

## Revision log

- 2026-05-10: Initial proposal accepted.
