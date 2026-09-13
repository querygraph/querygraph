# Pinax integration acceptance

Pinax succeeds Fihrist under the `querygraph/pinax` repository. The published
package is `pinax-registry` 0.2.0, with Rust library and CLI name `pinax`.
QueryGraph resolves it through crates.io, with no sibling source override.

The rename covers public APIs, CLI commands, MCP tool names and signing
resources, catalog properties, ontology URNs, registry and ontology wire
versions, demos and publishing artifacts. See the [owner migration guide](https://github.com/querygraph/pinax/blob/main/docs/migration.md)
for existing documents. Old signed evidence remains historical, unmodified.

## Accepted implementation

- Pinax owns the reviewed table registry and complete central ontology.
- Agents consult approved, policy-filtered mappings for Semantic Croissant,
  CDIF and SKOS discovery before governed reads.
- Stateless MCP 2026-07-28 supports stdio and Streamable HTTP, bounded execution,
  cancellation, and per-request protocol context.
- LakeCat and Sail execute real Iceberg reads with mandatory filtering and
  independently verified row, authorization and lineage digests.
- Schema reconciliation and reviewed consumer restart reject stale state.
- All Rust code uses edition 2024. Sail runs from our selected source checkout.

## September 13, 2026 verification

35 Pinax tests, 121 QueryGraph unit tests, five integration tests, three console
tests and 82 Python tests passed, along with strict Clippy and documentation
checks. Local and grust integration suites passed. All nine live browser actions
passed against the new central ontology and stateless MCP endpoint.

The five boot-enabled `querygraph-pinax-*` services run from
`/home/admin/querygraph-pinax-demo-20260913`. The prior demo is disabled and
retained for rollback. Current reports live in `demo/pinax/evidence/`;
pre-rename reports are explicitly under its `history/` directory.

The 14-slide PDF, narrative, source deployment archive and reports are delivered
under `~/icloud/pinax`. The book is rebuilt as Pinax, and the announcement
textpack uses `announcing-pinax`. [Release review](pinax-release-review.md).
