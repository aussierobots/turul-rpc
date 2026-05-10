# ADR-003: Compatibility with `turul-mcp-json-rpc-server`

**Status**: Accepted
**Date**: 2026-05-10

## Context

`turul-mcp-json-rpc-server 0.3.x` is shipped by turul-mcp-framework. Many
projects depend on it transitively (and a few directly). The extraction of
`turul-rpc` must not break those projects.

The constraint is asymmetric on the framework side:

- **0.3.x line**: `turul-mcp-json-rpc-server` continues to be published as a
  thin re-export shim. There is one final shim release: **0.3.39**.
  Existing 0.3 users keep building forever.
- **0.4 line and beyond**: `turul-mcp-json-rpc-server` is **not republished**.
  Framework 0.4.0 removes the dependency. Users who want to follow MCP onto
  0.4 import `turul-rpc` directly.

This ADR captures the technical contract `turul-rpc` must honour for the
0.3.39 shim to exist.

## Decision

### Type identity

Every public type in `turul-mcp-json-rpc-server::*` (and its re-exported
modules) MUST be the **same nominal type** as the corresponding type in
`turul-rpc-*`. Achieved exclusively via `pub use` chains rooted at
`turul-rpc-core` (or, where applicable, `turul-rpc-jsonrpc` /
`turul-rpc-server`):

```text
turul-rpc-core::error::JsonRpcError
   ▲ pub use turul_rpc_core::error;
turul-rpc::error::JsonRpcError                  (same type)
   ▲ pub use turul_rpc::error;
turul-mcp-json-rpc-server::error::JsonRpcError  (same type)
```

**No newtype wrappers anywhere along the chain.** Pattern matches,
trait impls, and trait-object coercions written against
`turul_mcp_json_rpc_server::JsonRpcError` continue to compile and behave
identically when re-exported from `turul-rpc-core::JsonRpcError`.

### Public surface preservation

The shim re-exports every item the original published, at every original
path. Concretely, every `pub use` line in the original `lib.rs` and
`prelude.rs` is mirrored. The verification gate is:

```bash
# in turul-mcp-framework, before and after the shim swap:
cargo public-api -p turul-mcp-json-rpc-server > /tmp/api.txt
diff /tmp/api-before.txt /tmp/api-after.txt
# MUST be empty
```

Any non-empty diff blocks the shim release.

### Feature flag forwarding

`turul-mcp-json-rpc-server` defines `default = ["async"]`, `async`,
`streams`. The shim's Cargo.toml forwards them to `turul-rpc`:

```toml
[features]
default = ["async"]
async = ["turul-rpc/async"]
streams = ["async", "turul-rpc/streams"]
```

A consumer using `turul-mcp-json-rpc-server` without enabling `async` MUST
get a build identical to v0.3.38 — no surprise tokio dependency.

### Allowed changes within `turul-rpc 0.1.x`

Patch releases of `turul-rpc-*` MAY:

- Add new public items.
- Add new variants to non-exhaustive enums (none currently exist; would
  require explicit `#[non_exhaustive]` first).
- Improve documentation.
- Fix bugs that do not change observable wire behaviour.

Patch releases MUST NOT:

- Remove or rename existing public items.
- Change field types, trait bounds, or method signatures of items the
  shim re-exports.
- Add variants to existing exhaustive enums (would break downstream
  pattern matches).

Any change requiring a minor bump to `turul-rpc` requires a coordinated
review against the shim's `cargo public-api` snapshot in turul-mcp-framework.

### What the shim crate looks like

```rust
// turul-mcp-json-rpc-server/src/lib.rs (entire body)
//! Compatibility shim. All implementation lives in `turul-rpc`.
//! New code should depend on `turul-rpc` directly.
//! turul-mcp-framework 0.4.0 removes this dependency; this crate ships
//! its final release as 0.3.39 and is not republished.
pub use turul_rpc::*;
pub use turul_rpc::{dispatch, error, notification, prelude, request, response, types};
#[cfg(feature = "async")] pub use turul_rpc::r#async;
pub use turul_rpc::{JSONRPC_VERSION, error_codes};
```

The original `src/{types,error,request,response,notification,dispatch,async,prelude}.rs`
files are deleted in the shim release.

### Out of scope for the shim

The shim **does not** add new functionality. JSON-RPC 2.0 batch is
implemented in `turul-rpc-jsonrpc` and reachable through the shim
(`turul_mcp_json_rpc_server::dispatch::parse_json_rpc_batch` resolves via
`pub use`), but the shim's own crate documentation continues to describe
the original surface only. Users who want the batch APIs are expected to
import `turul-rpc` directly — that's the whole point of the migration.

## Consequences

**Positive**

- Existing `turul-mcp-json-rpc-server = "0.3"` consumers see no change.
- New consumers find `turul-rpc` first via crates.io search, README
  pointers, and CHANGELOG callouts.
- `turul-rpc 0.1` is free to grow within its semver discipline without
  worrying about specific framework crates' downstream needs.

**Negative**

- The historical `JsonRpcMessage` name collision (response union vs.
  dispatch incoming union) is preserved on the shim path because the
  original crate published both names. Cleanup deferred to `turul-rpc 0.2`
  / framework 0.5.
- The `SessionContext.broadcaster: Option<Arc<dyn Any + Send + Sync>>` and
  `SessionContext.extensions: HashMap<String, Value>` fields are
  structurally generic but were introduced for MCP transport reasons.
  v0.1 keeps them as-is to preserve type identity. ADR-004 documents this
  as accepted inherited cruft.

## Revision log

- 2026-05-10: Initial proposal accepted.
