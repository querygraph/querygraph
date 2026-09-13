# Governed execution integration contract

Deployment and real bounded Sail rows through both MCP transports pass with
the prepared owner sources. Mutation before releasing real buffers also passes.
Transport cancellation and deadline tests cover pending backend futures.
Reviewed consumer cutover followed by real MCP rows also passes against prepared
sources. Owner release validation and released dependency resolution remain gates.

## Existing authority and the release guard

LakeCat grants bind the configured catalog, table version, snapshot, verified
subject, purpose, ordered projection, task digest, and original authorization
and policy evidence. Owner revalidation loads the exact durable grant and current
table/policy state. QueryGraph must consume its evidence rather than construct
an apparent owner decision from hashes.

The prepared LakeCat `read_with_governed_revalidation` boundary verifies the
current caller's capability against the proof, checks the grant before reading,
and checks it again before releasing a buffered result. Its tests cover caller
mismatch, unverified identity, revocation before/during execution, projection
drift, snapshot change, and backend failure. These are boundary tests with a
controlled callback, not evidence of real Sail row execution.

## Integration requirements

1. Compose an authenticated owner operation that plans and executes one bounded
   request. Retain its normalized filters and limit in owner memory through
   execution. The durable grant currently does not independently retain those
   inputs; accepting replacement execution filters from a client holding an old
   proof would not establish the original request's scope. Reuse the existing
   grant wire contract for evidence, without pretending it contains more fields.
2. Use Sail's public `Table::load_with_metadata_location` and snapshot-specific
   `IcebergTableProvider` through a Sail-owned execution adapter. Apply mandatory
   predicates as actual row filters, including residual predicates; metadata
   pruning alone is insufficient. Filter before projecting and limiting. Unknown
   predicate forms fail closed. Table locations and storage credentials come
   only from the owner, never from MCP arguments.
3. Buffer a bounded number of rows and bytes with a deadline and cancellation.
   Keep the expected catalog observation through the operation, so Pinax's
   checked registry/schema cannot change between QueryGraph's check and owner
   execution. Release only after the owner guard succeeds. The checks are fresh
   observations, not an atomic cross-service lease.
4. Bind the returned row digest and lineage to the exact proof, final owner
   decision, registry digest, snapshot, purpose, and ordered projection. Preserve
   original authorization separately from the final decision. Do not expose
   task paths, credentials, or rows through errors or discovery.
5. Add the MCP execution contract to the shared Rust/Python schema and verify
   direct and bridged results against a seeded real Iceberg table. Assert denied
   columns and purposes, mandatory row filtering, schema/registry drift,
   snapshot changes during execution, backend outage, and cancellation.

No Sail shared trait or internal format change has been made. Its existing
guidance requires owner design discussion before such changes. First use its
public execution interfaces; if those prove insufficient, prepare the exact
owner proposal before changing the shared surface.

## Engine acceptance now established

The isolated Sail checkout `/tmp/sail-pinax-integration`, based on `9f6f8065`,
adds `datasource::bounded_read::read_snapshot_buffered`. It uses the existing
provider and DataFusion planner, applies real row predicates before literal-name
projection and limit, and checks retained Arrow buffer bytes. The caller must
still supply bounded session memory, a deadline, and authorization. No catalog
trait, physical plan node, or format semantics changed.

QueryGraph's real-row Python fixture passed against that source: mixed-tenant
rows in one Iceberg data file are filtered correctly while the tenant field is
absent from output; row/byte limits, old and new snapshots after append, invalid
projections, missing metadata, and missing Parquet data are checked. A failed
read returns no partial result. `/tmp/pinax-sail-rows-report.json` retains the
binary hash and confirms temporary-data removal. This engine-only report is
separate from the subsequent owner/MCP acceptance described below.

The helper remains uncommitted. Nightly formatting of the added source passes.
Strict Clippy encounters existing warnings in `sail-logical-plan` and
`sail-iceberg` (formatting borrows, clone-on-copy, and test unwraps), including
with `--no-deps`. The fixture builds and runs, but no clean full-Clippy or release
claim is made. Those validation failures remain to resolve before release.

## Owner execution increment

The prepared owner now exposes POST
`/querygraph/v1/{warehouse}/namespaces/{namespace}/tables/{table}/execute`.
Its strict JSON body contains `projection`, `purpose`, `limit` (1–1000), and
`snapshot_id`. `x-lakecat-expected-table-state` must carry the observation that
the consumer checked. TypeSec authenticates the request, planning and execution
permissions are checked separately, and the configured purpose/projection are
enforced before execution. Clients cannot supply predicates or physical paths.
The owner retains scope through planning and passes its mandatory predicates to
Sail's strict scalar REST compiler and buffered reader. Unsupported predicates
fail closed. A 30-second operation deadline includes final checks and lineage I/O.

Rows are released with proof, row digest, lineage receipt, fresh execution and
grant decisions, and a digest binding that evidence. Final authorization follows
lineage I/O. The real owner fixture has passed positive rows and denials for
planning-only permission, columns, purpose, stale/missing state, and missing data.
The subsequent `/tmp/pinax-mcp-rows-report.json` run also passed direct and
bridged MCP execution, including separately signed operation scope and Pinax
denials. QueryGraph validates the owner response and freshly checks catalog state
and Pinax permission before releasing it. Mutation-during-read acceptance
is now covered at the owner release boundary by a deterministic fixture.
`/tmp/pinax-mutation-rows-report.json` verifies that a nonempty real Sail buffer
is discarded after purpose revocation, registry-pin drift, and a transition to
a valid second Iceberg snapshot. The fixture injects these changes after the
engine read and before owner revalidation; it does not test races inside the
physical file reader. Error responses contain no rows. The same run also passes
normal direct/bridged MCP execution; mutation cases themselves use owner HTTP.

The live run found that TypeSec's prior decision hash included request context,
making fresh revalidation always look like policy drift. The prepared adapter
uses a versioned stable decision domain; full authorization evidence retains
context, and restrictions remain separately hashed. Existing grants must be
replanned after upgrade. This migration is explicit and fail-closed.

Per the user's correction, all edited Rust and fixture manifests use edition
2024. The changed `sail-iceberg` crate overrides the upstream workspace default;
raw `r#gen` names and three pattern updates complete its edition migration.
