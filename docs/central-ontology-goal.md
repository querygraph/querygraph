# Central lakehouse ontology goal

Requested September 12, 2026. Extends the existing Pinax integration goal.
Implement a Pinax-owned central ontology for all registered tables and fields,
agent discovery, Semantic Croissant and CDIF. Use Rust 2024 and our Sail checkout.

## Design and acceptance

- Preserve pinax.v1; add pinax.ontology.v1 pinned to an exact registry digest.
- Stable concepts, definitions, aliases, broader/related edges, provenance,
  stewardship, draft/approved/deprecated lifecycle and stable field-ID bindings.
- Inventory every table and field, including mappings awaiting review. Never
  automatically approve inferred business meaning.
- Validate references, cycles, replacements, duplicate IDs, coverage and bounds.
- Reconcile schema changes without losing reviewed mappings; reject stale pins.
- Immutable durable snapshots, compare-and-swap publication, reviewed target
  digest. Agent discovery cannot publish or choose host storage paths.
- Authorize table/field discovery before disclosing mappings, concepts or
  exports. Return explanation and ontology digest; matches never grant reads.
- Host-configured central store; demo agents actually consult it, failing closed
  for missing/stale configured ontology.
- Standard JSON-LD contexts, physical types, explicit extensions; never invent
  file extraction mappings or infer unknown units.
- Test validation, reconciliation, publication conflicts, metadata denial,
  aliases/relationships and standards exports. Build and run on grust.
- Update demo narrative/slides and iCloud artifacts with verified evidence.

## Status

Implemented and verified on grust on September 12, 2026. The demo inventories
one registered table and all three fields; only the authorized identifier is
visible to the demo agent. The authoring CLI also passed a three-table,
15-binding lifecycle, including revision publication and stale-CAS rejection.

- All nine browser actions passed at desktop and mobile sizes; four services active.
- Direct Rust MCP and Python-to-Rust handoff passed ontology alias discovery,
  metadata denial, signature rejection and governed execution.
- MLCommons Croissant 1.1 loaded the actual authorized result (identifier 2).
  Optional citation/date/semantic-version warnings remain; no validation errors.
- Pinax: 33 tests. QueryGraph: 113 unit, five integration and three console tests.
  Python regression suite: 79 passed, two skipped. Formatting, strict Clippy,
  documentation checks and stack dependency alignment passed.
- Fourteen-slide PDF, narrative, operator guide and prepared EC2 source bundle
  delivered to `~/icloud/pinax`.

Evidence is in `demo/pinax/evidence/grust-ontology-*.json`,
`croissant-ontology-validation.json` and `ontology-authoring-cli.json`.
Coverage applies to registered inventory; enrolling additional production
catalogs and steward approval of their meanings remain operator work.

The goal API refused replacing the unfinished integration goal; this file records
expanded scope and verified implementation without claiming its outstanding
release work complete.

## Explicit semantic-layer scope confirmation

The user reaffirmed that all semantic-layer work belongs to this goal: actual
schema ingestion, maintained ontology concepts and physical mappings, standards
exports, agent consultation and explained discovery, governed execution,
interoperability validation and deployment/demo artifacts. Implementation and
verification continue as one goal, rather than separate optional follow-up work.

## Stateless MCP acceptance

The user added current MCP support. MCP 2026-07-28 now serves this ontology over
stateless Streamable HTTP and modern stdio, preserving legacy stdio compatibility.
Per-request metadata, header checks, cache hints, explicit imported-model handles
and disconnect cancellation are implemented in Rust 2024. The persistent grust
service is `querygraph-pinax-mcp` on loopback 18082. See
[mcp-stateless.md](mcp-stateless.md) for evidence and runnable examples.
