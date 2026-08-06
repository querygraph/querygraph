# QueryGraph TypeSec–Marciana Facade

## Status

Proposed refactoring plan. This document describes an internal ownership
boundary; it does not change the HTTP routes, MCP tools, TypeDID wire format,
memory identifiers, database schema, or Sail protocol.

## Objective

QueryGraph should continue to depend on TypeSec and Marciana because it is the
composition layer that exposes governed agent workflows. It should not spread
their implementation types throughout HTTP handlers, MCP code, demos, and
application orchestration.

The target is a small QueryGraph-owned facade:

```text
HTTP / MCP / agents / demos
            │
            ▼
   QueryGraph stack facade
   ├── security
   ├── memory
   ├── cognition
   └── errors
            │
            ├── TypeSec / TypeDID / ODRL
            └── Marciana / Grust memory adapter
```

The facade owns translation and orchestration. TypeSec remains the authority
for protected-memory access and mutation. Marciana remains the cognition and
memory composition layer. Grust, Sail, LakeCat, TypeSec, and Marciana must not
depend on QueryGraph.

## Dependency policy

These are intentional runtime dependencies:

- `typesec-core` for subjects, capabilities, policy contexts, and permissions;
- `typesec-integrations` for TypeDID envelopes and receipts;
- `typesec-memory` for `MemoryVault`, memory verbs, and cognition commit types;
- `typesec-agent` for the tool-call guard;
- `typesec-rbac` for policy loading and enforcement;
- `querygraph-memory` for Marciana's Grust-backed memory adapter;
- `marciana-cognition` for governed cognition application types.

The direct `marciana-catalog` dependency is not needed: source inspection found
no production QueryGraph import. It remains in the lockfile only where it is a
transitive dependency of `marciana-cognition`. LakeCat owns the catalog and
governed-scan boundary currently consumed by QueryGraph.

Cognee and Fluree remain comparative systems only. They must not become Rust
dependencies, runtime services, stores, or compatibility facades.

## Proposed module layout

```text
src/stack/
  mod.rs
  error.rs
  security.rs
  memory.rs
  cognition.rs
```

Production code outside `src/stack/` should use QueryGraph-owned request,
result, and error types. Upstream TypeSec and Marciana types should be imported
directly only by the facade and its focused tests.

### `stack::security`

This module is the single TypeSec integration point for request authority. It
should own:

- TypeDID envelope verification;
- recipient, path, and body binding;
- verified subject extraction;
- capability and ODRL checks;
- policy-context construction;
- receipt validation;
- conversion into a QueryGraph-owned `VerifiedAgent`.

Illustrative boundary types:

```rust
pub struct VerifiedAgent {
    pub subject: String,
    pub receipt: VerificationReceipt,
}

pub struct VerificationRequest {
    pub route: String,
    pub body: serde_json::Value,
    pub envelope: TypeDidEnvelope,
}
```

Handlers must receive `VerifiedAgent`; they must not reconstruct
`SubjectId`, `RequestContext`, or TypeDID verification independently.

### `stack::memory`

This module should contain the only production integration with:

- `querygraph-memory`;
- `MemoryVault`;
- `MemoryToolRouter`;
- `TursoMemoryStore`;
- Marciana memory error conversion.

The current `MemoryApi` should become an implementation of a QueryGraph-owned
gateway:

```rust
pub trait MemoryGateway {
    fn execute(
        &self,
        agent: &VerifiedAgent,
        request: MemoryRequest,
    ) -> Result<MemoryResult, StackError>;
}
```

The gateway continues to enforce the existing sequence:

```text
verified TypeDID
      → ToolCallGuard
      → typed capability
      → MemoryVault
      → Marciana/Grust store
```

No handler or agent may mutate a store directly.

### `stack::cognition`

This module should contain the only QueryGraph production integration with
`marciana-cognition` and `querygraph-memory::cognition`.

It should translate QueryGraph-owned requests into Marciana's governed
cognition application, including formation profiles, source proofs, proposal
validation, commit outcomes, recovery, and receipts.

The public shape should remain proposal-oriented and authenticated:

```rust
pub trait CognitionGateway {
    fn improve(
        &self,
        agent: &VerifiedAgent,
        request: CognitionRequest,
    ) -> Result<CognitionResult, StackError>;
}
```

The authority flow remains:

```text
verified request
      → Marciana proposal
      → TypeSec validation and commit
      → durable outcome and receipt
```

QueryGraph must not expose transient proposals that can be replayed outside
the authoritative path.

### `stack::error`

Define a small, stable error taxonomy for callers. It should distinguish:

- authentication or verification failure;
- authorization denial;
- invalid request;
- durable store failure;
- cognition failure;
- recovery or receipt failure.

Error conversion from TypeSec and Marciana belongs here. Messages must remain
non-disclosing for protected-memory failures.

## Migration sequence

1. Add `stack::error` and QueryGraph-owned request/result types.
2. Extract TypeSec verification and policy preparation into `stack::security`.
3. Implement `MarcianaMemoryGateway` and migrate `MemoryApi` callers.
4. Implement `MarcianaCognitionGateway` and migrate cognition callers.
5. Migrate HTTP, MCP, agent, and demo paths to the facade.
6. Remove duplicated TypeSec/Marciana error mapping and direct imports.
7. Remove unused direct dependencies; the direct `marciana-catalog` entry has
   been removed while its Marciana-transitive lock entry remains.
8. Add boundary and compatibility tests.
9. Update documentation and `CHANGELOG.md` in the same logical change.
10. Commit each cohesive extraction separately, then run the full stack gate.

## Testing plan

Keep tests separate from production modules:

```text
tests/
  stack_security.rs
  stack_memory.rs
  stack_cognition.rs
  stack_compatibility.rs
```

The tests should cover:

- valid and invalid TypeDID envelopes;
- recipient, path, and body binding;
- deny-by-default policy behavior;
- remember, recall, and forget;
- create, close, reopen, retry, collision, and recovery paths;
- cognition proposal acceptance and rejection;
- receipt verification and deterministic replay;
- unchanged HTTP and MCP behavior;
- dependency-direction and no-Cognee/no-Fluree checks.

Fixtures must pass through real authority boundaries. Do not add public
convenience constructors solely to make tests easier.

## Compatibility and release gates

This refactoring must preserve routes, MCP tool names, TypeDID envelopes,
durable identifiers, database schemas, and Sail integration. Any change to
those surfaces requires an explicit versioned migration.

During active development, TypeSec and Marciana may continue to track their
`main` branches. Release candidates must use reviewed reachable revisions or
released versions, record them in `Cargo.lock` and `COMPATIBILITY.md`, and run
the explicit verified Sail binary.

The completion gate is:

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
cargo test --all-targets
dependency-direction checks
TypeSec conformance tests
persistence/recovery tests
live Sail integration gate
```

The outcome is a deliberately small QueryGraph integration boundary: TypeSec
and Marciana remain real dependencies, but their implementation details no
longer leak into every QueryGraph feature.
