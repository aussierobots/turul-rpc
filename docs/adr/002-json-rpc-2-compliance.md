# ADR-002: JSON-RPC 2.0 compliance contract

**Status**: Accepted
**Date**: 2026-05-10

## Context

`turul-rpc` advertises JSON-RPC 2.0 support. "Support" without an explicit
contract drifts: the original `turul-mcp-json-rpc-server` ships a
`parse_json_rpc_messages` function (plural) whose comment claims "JSON-RPC
2.0 removed batch support" — a documented misreading of the spec that
silently strips batch processing.

This ADR pins the v0.1 compliance surface. Each rule below is enforced by a
test in `turul-rpc-jsonrpc/tests/spec_conformance.rs`.

## Decision

### Batch processing — IMPLEMENTED in v0.1

The JSON-RPC 2.0 spec retains batch processing
([Section 6 — Batch][spec-batch]). `turul-rpc-jsonrpc` v0.1 implements it:

- A request body that parses to a JSON **array** is a batch.
- Empty batch (`[]`) → respond with a single `Invalid Request` error
  (`-32600`), `id: null`, NOT an empty array.
- Each batch member is parsed and dispatched independently. Per-member
  parse/invalid-request errors are returned in the response array with
  `id: null` and the appropriate error code.
- Notifications inside a batch produce **no** entry in the response array.
- A batch consisting **entirely** of notifications produces **no response
  body** (transport returns 204 / closes the stream — caller's choice).
- Response array order is **not required** to match request order, per
  spec. This implementation preserves request order for ergonomics; tests
  do not assert order to keep room for future concurrent dispatch.

Public API:

```rust
// turul-rpc-jsonrpc
pub enum BatchOrSingle {
    Single(Result<JsonRpcMessage, JsonRpcError>),
    Batch(Vec<Result<JsonRpcMessage, JsonRpcError>>),
    EmptyBatch,
}
pub fn parse_json_rpc_batch(json_str: &str) -> BatchOrSingle;

// turul-rpc-server
impl<E: ToJsonRpcError> JsonRpcDispatcher<E> {
    /// Dispatch a request body that may be a single message or a batch.
    /// Returns `Some(json_string)` for the response, or `None` if no
    /// response is required (all-notifications batch).
    pub async fn handle_batch(&self, body: &str) -> Option<String>;
}
```

The legacy `parse_json_rpc_messages(json_str) -> Vec<…>` is **retained** as
a compatibility shim that always returns a single-element vec for
non-array bodies and the parsed batch entries for array bodies. It is
deprecated in favour of `parse_json_rpc_batch`.

### Request id rules

JSON-RPC 2.0 ([Section 4.2 — id][spec-id]) allows `String`, `Number`, or
`Null`. Null is **discouraged** for client-to-server requests and
reserved for server responses to unparseable requests.

`turul-rpc-core::RequestId = enum { String(String), Number(i64), Null }`.

**Compliance posture summary**: spec-compliant on both request and
response id handling. Null ids are accepted; the only narrowing is that
fractional numeric ids are rejected (which the spec itself SHOULD-NOTs).

- **Incoming requests** with `"id": null` are **accepted** and parse to
  `RequestId::Null` (§4.2 permits null, discouraged). A test asserts this
  (`accepts_null_id`).
- **Outgoing requests** MAY carry a null id at the type level
  (`RequestId::Null`), though it remains discouraged and is useless for
  response correlation.
- **Outgoing error responses** for unparseable / unidentifiable requests
  emit `"id": null` via `JsonRpcError { id: Option<RequestId>, ... }`
  where `id` is `None`. Spec-required behaviour; constructors
  `JsonRpcError::parse_error()` and `JsonRpcError::invalid_request(None)`
  implement it. A test asserts the wire shape
  (`parse_error_response_serializes_with_null_id`). Note: `None` and
  `Some(RequestId::Null)` both serialize to `null`; `None` is the
  canonical "id could not be determined", `Some(Null)` is "the client
  explicitly sent null".
- **Fractional numeric ids** (e.g. `1.5`) are rejected as `Invalid
  Request`. JSON-RPC 2.0 SHOULD-NOT for fractional parts; this is
  enforced at the type level (`as_i64()` only).

#### Resolution (0.2.0)

Implemented in 0.2.0 by adding `RequestId::Null` **directly**, not the
separate codec-level `WireRequestId` this ADR originally proposed. That
indirect approach existed only to avoid changing `RequestId` while the
`turul-mcp-json-rpc-server 0.3.x` shim required type identity (ADR-003).
That shim-preservation constraint is retired, so the direct variant is
simpler and needs no parallel codec type.

### Standard error codes

`turul-rpc-core::error_codes` exposes the spec constants:

| Constant | Value | Meaning |
|---|---|---|
| `PARSE_ERROR` | `-32700` | JSON parse failure |
| `INVALID_REQUEST` | `-32600` | JSON not a valid Request object |
| `METHOD_NOT_FOUND` | `-32601` | Method does not exist / unavailable |
| `INVALID_PARAMS` | `-32602` | Invalid method parameter(s) |
| `INTERNAL_ERROR` | `-32603` | Internal JSON-RPC error |
| `SERVER_ERROR_START` | `-32099` | Inclusive lower bound of server-defined range |
| `SERVER_ERROR_END` | `-32000` | Inclusive upper bound of server-defined range |

`JsonRpcErrorObject::server_error()` panics if given a code outside
`-32099..=-32000`. A test asserts this.

### Notification semantics (no response)

Notifications (request without `id`) MUST NOT produce a response.

- `JsonRpcDispatcher::handle_notification()` returns `Result<(), E>`.
- Errors from notification handlers are **swallowed** at the dispatcher
  boundary (logged via `tracing` if available; never serialized to the
  wire). A test asserts that a notification dispatched through
  `handle_batch` produces no response body even when the handler errors.

### Version strictness

`"jsonrpc"` field MUST be the literal string `"2.0"`. Any other value
(including `2`, `2.0` as a number, `"1.0"`, missing) → `Invalid Request`
(`-32600`). The custom `Deserialize` for `JsonRpcVersion` enforces this.

### Method-not-found id echo

`Method not found` responses MUST echo the request id (when one was
present). The dispatcher does this via `JsonRpcError::method_not_found(id,
method)`. A test asserts the id round-trip for both string and number ids.

### MCP transport rejection of batch (note, not requirement)

The MCP protocol's Streamable HTTP transport may choose to reject batch
requests for protocol-policy reasons (e.g. session-bound dispatch
ordering). That is an MCP layer decision, **not** a `turul-rpc-jsonrpc`
limitation. If MCP rejects batch, it does so with its own `-32600`
response and a test in the MCP repo proves the rejection path; this crate
remains spec-complete on its own.

## Consequences

**Positive**

- A user reading the README sees `turul-rpc` claim JSON-RPC 2.0
  compliance, opens the spec, and finds the implementation matches each
  numbered section. No "almost compliant" footnotes.
- Batch is no longer an unimplemented stub with a misleading comment.
- Null request ids are accepted (§4.2), so the id handling is fully
  spec-conformant on both directions; the only narrowing is fractional
  numeric ids, which the spec itself SHOULD-NOTs.

**Negative**

- Accepting null ids means a client *can* send a request the server
  cannot correlate a response to. This is spec-permitted but discouraged;
  callers should prefer String or Number ids.

## References

- [spec-batch]: https://www.jsonrpc.org/specification#batch
- [spec-id]: https://www.jsonrpc.org/specification#request_object

## Revision log

- 2026-05-10: Initial proposal accepted.
- 2026-05-24: Strict-id departure resolved. `RequestId` gains a `Null`
  variant; incoming null id is accepted (§4.2). Chosen over the
  `WireRequestId` approach this ADR originally proposed because the
  shim-preservation constraint (ADR-003) is retired. Test renamed
  `rejects_null_id_per_strict_posture` → `accepts_null_id`.
