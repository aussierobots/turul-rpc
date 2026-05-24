# turul-rpc-core

> JSON-RPC 2.0 wire types for the [`turul-rpc`] framework. Pure data, no async.

[`turul-rpc`]: https://crates.io/crates/turul-rpc

This crate ships the request/response/notification/error types that any
JSON-RPC 2.0 implementation needs. It has no async runtime dependency
and no codec/parser logic — those live in [`turul-rpc-jsonrpc`] and
[`turul-rpc-server`] respectively. **Most consumers should depend on the
[`turul-rpc`] facade**, which re-exports this crate's items at the same
paths.

[`turul-rpc-jsonrpc`]: https://crates.io/crates/turul-rpc-jsonrpc
[`turul-rpc-server`]: https://crates.io/crates/turul-rpc-server

## Public surface

| Module | Items |
|---|---|
| `types` | `RequestId`, `JsonRpcVersion` |
| `request` | `JsonRpcRequest`, `RequestParams` |
| `response` | `JsonRpcResponse` (the §5 Response union: `Success \| Error`), `JsonRpcSuccessResponse`, `ResponseResult` |
| `notification` | `JsonRpcNotification` |
| `error` | `JsonRpcError`, `JsonRpcErrorObject`, `JsonRpcErrorCode`, `JsonRpcTransportError` |
| `error_codes` | Spec constants: `PARSE_ERROR`, `INVALID_REQUEST`, `METHOD_NOT_FOUND`, `INVALID_PARAMS`, `INTERNAL_ERROR`, `SERVER_ERROR_START`, `SERVER_ERROR_END` |

Plus the crate-root `JSONRPC_VERSION` constant.

## When to depend on this crate directly

- You're writing a JSON-RPC client without an async runtime.
- You're embedding the wire types in a non-async context (parser only,
  codec only, schema generation).
- You want the smallest possible dependency footprint — this crate's
  only deps are `serde`, `serde_json`, `thiserror`.

For a typical async server, depend on [`turul-rpc`] instead.

## `RequestId`

```rust
pub enum RequestId {
    String(String),
    Number(i64),
    Null,
}
```

`RequestId` is spec-conformant per JSON-RPC 2.0 §4.2: incoming `"id":
null` is **accepted** (permitted, though the spec discourages it).
Fractional numeric ids are rejected — only `i64` is representable.
`From` is implemented for `i64`, `i32`, `u32`, `&str`, and `String`, so
most call sites can write `1.into()` or `"abc".into()` without a cast.
See [ADR-002] in the workspace for the full posture.

[ADR-002]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/002-json-rpc-2-compliance.md

## License

Dual-licensed under [MIT](https://github.com/aussierobots/turul-rpc/blob/main/LICENSE-MIT)
or [Apache-2.0](https://github.com/aussierobots/turul-rpc/blob/main/LICENSE-APACHE)
at your option. See the [workspace README] for details.

[workspace README]: https://github.com/aussierobots/turul-rpc#readme
