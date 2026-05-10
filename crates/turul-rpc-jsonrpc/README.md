# turul-rpc-jsonrpc

> JSON-RPC 2.0 codec, parser, and §6 batch processing for the
> [`turul-rpc`] framework.

[`turul-rpc`]: https://crates.io/crates/turul-rpc

Built on [`turul-rpc-core`] wire types. **Most consumers should depend
on the [`turul-rpc`] facade**, which re-exports this crate's items.

[`turul-rpc-core`]: https://crates.io/crates/turul-rpc-core

## What's here

- `parse_json_rpc_message` — single-message parser with spec-correct
  error mapping (parse error → `-32700` with `id: null`; invalid request
  → `-32600` echoing the id when parseable; etc.).
- `parse_json_rpc_batch` + `BatchOrSingle` — JSON-RPC 2.0 §6 batch
  parsing. Distinguishes single-object body, non-empty batch array, and
  empty batch (which the dispatcher answers with a single
  `Invalid Request -32600` per spec).
- `JsonRpcMessage` — incoming union (`Request | Notification`).
- `JsonRpcMessageResult` — outcome shape consumed by dispatchers.
- Helper constructors: `create_success_response`, `create_error_response`.

## JSON-RPC 2.0 §6 batch

Per spec:

- Empty batch (`[]`) → respond with single `Invalid Request -32600`,
  `id: null`. NOT an empty array.
- Each batch member is parsed independently. Per-member parse failures
  produce `Err(JsonRpcError)` entries; the dispatcher emits them in the
  response array.
- Notifications inside a batch produce **no** entry in the response
  array. An all-notifications batch produces no response body at all.

29 spec-conformance tests in `tests/spec_conformance.rs` lock these
rules in. See [ADR-002] in the workspace for the full compliance contract.

[ADR-002]: https://github.com/aussierobots/turul-rpc/blob/main/docs/adr/002-json-rpc-2-compliance.md

## Compatibility note

The `parse_json_rpc_batch` and `BatchOrSingle` items live in this
crate's `batch` module, intentionally separate from the legacy
`dispatch` module. The `turul-mcp-json-rpc-server 0.3.39` shim
re-exports `dispatch::*` (preserving the v0.3.38 surface) but
**not** `batch::*` — so 0.3.x consumers see no API addition through the
shim. New code that wants batch processing should depend on
`turul-rpc` directly.

## License

Dual-licensed under [MIT](https://github.com/aussierobots/turul-rpc/blob/main/LICENSE-MIT)
or [Apache-2.0](https://github.com/aussierobots/turul-rpc/blob/main/LICENSE-APACHE)
at your option. See the [workspace README] for details.

[workspace README]: https://github.com/aussierobots/turul-rpc#readme
