# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2026-05-25

### Added

- `JsonRpcWireMessage` — the bidirectional wire-message union
  (`Request | Notification | Response`), i.e. the JSON-RPC 2.0 / MCP schema
  `JSONRPCMessage`: any single message a peer reads off the wire without
  knowing direction. Superset of the inbound `JsonRpcMessage`
  (request | notification); the `Response` arm carries the §5
  `JsonRpcResponse` union (success or error). For clients and bidirectional
  peers that need to parse responses.
- `parse_json_rpc_wire_message` — validating parser for the above. Classifies
  by validated structure (version check, fractional-id rejection) rather than
  a naive `#[serde(untagged)]` derive, which would silently misclassify a
  malformed-id request as a notification. Lives in `turul-rpc-jsonrpc::frame`
  and is re-exported at the `turul-rpc` facade root; it is **not** surfaced
  through the `turul-mcp-json-rpc-server` shim. See ADR-003, ADR-004.

## [0.2.1] - 2026-05-24

### Added

- `From<i32>` and `From<u32>` for `RequestId`, removing the `_i64` suffix /
  explicit cast burden on consumers constructing numeric ids. Both widen
  losslessly into the existing `Number(i64)` variant. The wire-level id
  contract (accepted/rejected ids) is unchanged; see ADR-002.

## [0.2.0] - 2026-05-24

### Changed (breaking)

- `JsonRpcResponse` is now the JSON-RPC 2.0 §5 Response object **union**
  (`Success(JsonRpcSuccessResponse) | Error(JsonRpcError)`). The previous
  success-only struct is renamed `JsonRpcSuccessResponse`. `JsonRpcError`
  and `JsonRpcErrorObject` are unchanged; parser types `JsonRpcMessage`
  and `JsonRpcMessageResult` are unchanged.
- `RequestId` gains a `Null` variant. This is breaking for exhaustive
  `match` on `RequestId`.

### Fixed

- Incoming `"id": null` requests are now accepted per JSON-RPC 2.0 §4.2
  (previously rejected as Invalid Request — the ADR-002 strict-id
  departure). Fractional numeric ids remain rejected. See ADR-002.

### Migration

- `JsonRpcResponse { .. }` struct literals → `JsonRpcSuccessResponse { .. }`;
  the `JsonRpcResponse::success`/`null` struct constructors are now on
  `JsonRpcSuccessResponse`. The union `JsonRpcResponse` exposes
  `success(..)`, `error(..)`, `is_error()`, `id()` and variants
  `Success`/`Error`.
- Add a `RequestId::Null` arm to exhaustive matches on `RequestId`.

## [0.1.0] - 2026-05-10

### Added

- Initial release. Generic JSON-RPC 2.0 framework extracted from
  `turul-mcp-json-rpc-server` (turul-mcp-framework 0.3.x).
- Four crates published from this workspace:
  - `turul-rpc-core` — JSON-RPC 2.0 wire types (`JsonRpcRequest`,
    `JsonRpcResponse`, `JsonRpcNotification`, `JsonRpcMessage`, `JsonRpcError`,
    `JsonRpcErrorCode`, `JsonRpcErrorObject`, `RequestId`, `JsonRpcVersion`,
    `RequestParams`, `ResponseResult`, `error_codes::*`, `JSONRPC_VERSION`).
  - `turul-rpc-jsonrpc` — codec/parser including JSON-RPC 2.0 batch support.
  - `turul-rpc-server` — async `JsonRpcDispatcher<E>`, `JsonRpcHandler` trait,
    `ToJsonRpcError`, `SessionContext`, optional streaming dispatcher.
  - `turul-rpc` — facade re-exporting the three above under stable paths.
- JSON-RPC 2.0 batch processing (`JsonRpcDispatcher::handle_batch`).
- Spec conformance test suite covering parse errors, invalid request, method
  not found, invalid params, internal error, server error range, batch
  ordering/notification suppression, id type variants, notification
  no-response semantics.

### Compatibility

- `turul-mcp-json-rpc-server 0.3.39` (in turul-mcp-framework 0.3.39 and later
  0.3.x patches) re-exports this crate. All existing imports continue to
  resolve at the same nominal types. See [ADR-003].

[ADR-003]: docs/adr/003-compatibility-with-turul-mcp-json-rpc-server.md
