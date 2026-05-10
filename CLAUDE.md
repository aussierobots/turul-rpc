# CLAUDE.md

1. Don't assume. Don't hide confusion. Surface tradeoffs.
2. Minimum code that solves the problem. Nothing speculative.
3. Touch only what you must. Clean up only your own mess.
4. Define success criteria. Loop until verified.

Generic JSON-RPC 2.0 framework for Rust — handlers return domain errors; the
dispatcher owns the wire. Extracted from `turul-mcp-json-rpc-server 0.3.x`
(turul-mcp-framework).

> **Source of Truth**
> - **`docs/adr/`** — architectural decisions (start here)
> - **`README.md`** — user-facing intro
> - **`CHANGELOG.md`** — release notes
> - **CLAUDE.md** — operator playbook (this file)
> - If conflict: ADRs win.

## Workspace layout

| Crate | Purpose | Deps |
|---|---|---|
| `turul-rpc-core` | wire types (request, response, notification, error, id, version) | `serde`, `serde_json`, `thiserror` |
| `turul-rpc-jsonrpc` | codec, parser, JSON-RPC 2.0 §6 batch | `turul-rpc-core` |
| `turul-rpc-server` | async dispatcher, handler trait, session, streaming | `turul-rpc-core`, `turul-rpc-jsonrpc`, `async-trait`, `futures` (optional) |
| `turul-rpc` | facade — single `pub use` import path mirroring v0.3.38 layout | all of the above |

Most consumers depend on `turul-rpc` (the facade). The split exists so a
future no-async client can depend on `turul-rpc-core` alone.

## Critical Rules

### Type identity for the shim contract (ADR-003)

`turul-mcp-json-rpc-server 0.3.39` is a re-export shim over this workspace.
Every public path the original crate published MUST resolve through the
shim with the same nominal type. Achieved exclusively via `pub use`
chains rooted in the turul-rpc crates — **no newtype wrappers**.

Verification on the framework side:
- `crates/turul-mcp-json-rpc-server/tests/symbol_coverage.rs` — every
  v0.3.38 public path imported via `use`; fails to compile if any path
  becomes unreachable.
- `crates/turul-mcp-json-rpc-server/tests/shim_compat.rs` — type identity
  asserted across paths.

A breaking change here cascades into a behaviour change in the shim,
defeating its preservation purpose.

### Hide new APIs behind separate modules

When adding a new public API in v0.1.x patches, put it in a module the
shim does NOT re-export, so the shim's surface stays the v0.3.38 surface.
Example: `parse_json_rpc_batch` and `BatchOrSingle` live in
`turul-rpc-jsonrpc::batch`, not `dispatch`. The framework shim's curated
re-export of `dispatch` excludes batch APIs by construction.

Methods on existing re-exported types (e.g. `JsonRpcDispatcher::handle_batch`)
DO leak through the shim because re-exporting a type brings its methods.
Such additions MUST be listed in the framework `CHANGELOG.md` under
`### Added` with an explicit acknowledgement that they are reachable
through the shim. See ADR-003.

### JSON-RPC 2.0 strict-id posture (ADR-002)

`RequestId = enum { String(String), Number(i64) }` — null is rejected at
the parser. This is **stricter than the spec** (which permits null with
discouragement). The strict posture is inherited verbatim from
v0.3.38; relaxing it would be a behaviour change in the shim. The README
and ADR-002 are explicit about this departure; do NOT claim full
JSON-RPC 2.0 compliance unqualified.

A v0.2 candidate is to introduce a permissive codec-level `WireRequestId`
at the parser boundary, leaving `RequestId` itself unchanged.

### v0.1 non-goals (ADR-004)

- No MCP-specific types or methods (those live in `turul-mcp-protocol`,
  `turul-mcp-server`).
- No HTTP/SSE/stdio/Lambda transports — bring your own.
- **No `turul-rpc-client` crate.** No client code exists in
  `turul-mcp-json-rpc-server` to extract; building a generic typed RPC
  client is new design, not extraction. Defer to a future minor when a
  real, small client surface is identified (request id generation,
  request/notification builders, response/error parsing, batch
  request/response correlation — none of which exist today).
- **No `turul-rpc-derive` crate.** All derives in `turul-mcp-derive` are
  MCP-domain.
- No removal of inherited dead code (`JsonRpcTransportError`,
  `FunctionHandler`) — public items in v0.3.38, dropping them = breaking
  change. v0.2 candidates.

### Workspace dependencies

All inter-crate deps use `workspace = true`. Versions declared in root
`Cargo.toml` `[workspace.dependencies]`. The `turul-rpc-server` workspace
entry sets `default-features = false` so the facade can opt-in to
features explicitly.

## Quick Reference

### Build / test

```bash
cargo build --workspace --all-features
cargo test --workspace --all-features
cargo test -p turul-rpc --no-default-features         # facade builds without async
cargo test -p turul-rpc-jsonrpc --test spec_conformance   # 29 spec tests
```

### Run examples

```bash
cargo run -p turul-rpc --example simple_calculator    # stdin JSON-RPC
cargo run -p turul-rpc --example batch_dispatch       # §6 batch demo
```

### Compliance / batch tests

- `crates/turul-rpc-jsonrpc/tests/spec_conformance.rs` — 29 tests covering
  every numbered section of JSON-RPC 2.0 (parse error, invalid request,
  method not found, invalid params, internal error, server-error range,
  batch with empty/mixed/all-invalid/all-notifications, id type variants).
- `crates/turul-rpc-server/src/dispatcher.rs` `tests` — 7 batch dispatch
  tests covering the spec's response-array assembly rules.

## Release / publish

Order (dependency-first):

```
turul-rpc-core → turul-rpc-jsonrpc → turul-rpc-server → turul-rpc
```

Each crate inherits `version` from `[workspace.package]`. Bump the
workspace version before publishing.

After publishing v0.1.0, the framework's
`crates/turul-mcp-json-rpc-server` shim's path dep must be replaced
with the crates.io version-only form (see framework Cargo.toml comment
at `[workspace.dependencies] turul-rpc = ...`). That swap is the merge
gate for `turul-mcp-framework` branch `extract/turul-rpc-shim`.

## ADRs

| ADR | Decision |
|---|---|
| [001](docs/adr/001-crate-boundaries.md) | Four crates: core, jsonrpc, server, facade. No client/derive in v0.1. |
| [002](docs/adr/002-json-rpc-2-compliance.md) | JSON-RPC 2.0 with documented strict-id posture; batch implemented. |
| [003](docs/adr/003-compatibility-with-turul-mcp-json-rpc-server.md) | Shim contract: type identity, no removals/signature changes; additions reviewed and listed. |
| [004](docs/adr/004-non-goals-for-v0-1.md) | What v0.1 explicitly does NOT do. |

## Generally Safe Dev Commands

`cargo build/check/test/run/clippy/fmt/clean/doc` — including with
`--package`, `--example`, `--no-default-features`, `--all-features`,
`--workspace`.

Commands requiring explicit user approval:

```bash
cargo publish     # irreversible
git checkout      # discards uncommitted work
git restore       # discards uncommitted work
git reset --hard  # irreversible
git commit        # only when user explicitly requests
```

### Commit Message Style

- **No `Co-Authored-By` attribution** — omit AI co-author trailers.
- **Succinct** — one-line summary, optional body only if non-obvious.
