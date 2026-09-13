# Fihrist integration execution plan

## Final acceptance — September 12, 2026

Fihrist 0.1.1 is published with explicit user authorization. Its registry
checksum matches the reviewed ontology-inclusive package. QueryGraph now builds
and validates without source overrides: 121 unit tests, five integration tests,
three console tests, 82 Python tests, strict Clippy, docs and stack alignment.
Grust rebuilt using the published Fihrist crate and passed the live stack run.
The central ontology and stateless MCP 2026-07-28 increments are included.

The [release review](fihrist-release-review.md) and
`demo/fihrist/evidence/fihrist-publication.json` supersede historical publication
blockers below. Sail remains the selected source checkout, as requested.
The book, announcement textpack, slides, narrative and runnable EC2 bundle have
been delivered; current archive checksums accompany the bundle in iCloud.

## Expanded goal: central ontology and semantic layer

The MCP increment also targets the published **2026-07-28 stateless protocol**:
per-request context, modern stdio and Streamable HTTP, explicit application
handles, central ontology discovery, conformance tests and a persistent grust
endpoint. See [the transport guide](mcp-stateless.md).

Explicitly added by the user on September 12, 2026. The active implementation
scope includes the [central ontology goal](central-ontology-goal.md) and the
[semantic-layer implementation](central-ontology.md):

- Fihrist-owned, versioned ontology covering every registered table and field.
- Actual schema ingestion with stable IDs/types, provenance and coverage checks.
- Separate concept and mapping review; draft, approved and deprecated lifecycle;
  aliases, definitions, relationships and validated registry revision bindings.
- Durable central publication, reviewed target digests, conflict detection,
  reconciliation and stale-consumer rejection.
- Semantic Croissant, CDIF and SKOS projections from the same reviewed meanings;
  loadable Croissant results from verified governed reads.
- Signed, policy-filtered discovery with match explanations and ontology evidence;
  demo agents consult the central ontology and derive read requests from bindings.
- Rust 2024 implementation, our Sail source checkout, regression/negative tests,
  actual end-to-end grust runs, refreshed slides/narrative, source deployment
  bundle and iCloud artifacts.

The goal API cannot edit the objective of the existing unfinished goal. This
repository record extends its scope without marking outstanding release work
complete or making upstream Sail merge a dependency.


The target is a governed query through MCP, from a reviewed Fihrist registry
to Sail results carrying LakeCat authorization evidence and lineage.

The current release audit is [fihrist-release-review.md](fihrist-release-review.md).
It distinguishes the completed source-checkout/demo evidence from unresolved
published QueryGraph-family dependencies.

**Current Sail target, clarified by the user:** work against our Sail source
checkout. Upstream Sail review, PR acceptance, merge and release are not
completion gates. The supported build selects that checkout explicitly and
records its revision or source checksums. Earlier progress notes that treated
upstream acceptance or a released Sail dependency as required are superseded.
The changed-checkout validation passes 427 tests and strict package Clippy for
all five changed crates. Unrelated dependency-wide lint findings remain recorded
without becoming an upstream-release requirement for this integration.

## Stages and acceptance gates

1. MCP contract: retain existing names, define shared Rust/Python fixtures,
   negotiate supported protocol versions, enforce session ordering, bound
   messages, and distinguish protocol errors from tool failures.
2. Trusted host composition: load immutable validated registries and
   deployment-owned policy/catalog/backend configuration. Request arguments
   cannot establish identity, select policy engines, or supply trusted metadata.
3. Discovery and planning: authorize metadata disclosure and explicit projected,
   purpose-bound, bounded scans through Fihrist's released adapters.
4. Execution: use LakeCat's governed execution authority and snapshot
   revalidation. Verify real Sail results, denied columns, rejected purposes,
   schema/registry drift, snapshot changes, and backend unavailability.
5. Deployment: plan and reconcile whole-registry pin updates with explicit
   recovery after ambiguous writes. Never retry a possibly committed mutation
   without reading its resulting state.
6. Documentation and release validation: update operational guidance, run Rust
   and Python checks, and check the stack dependency matrix. Sibling changes
   require released versions before published QueryGraph-family consumer pins
   can change. Sail is built from the explicitly selected source checkout;
   upstream Sail acceptance or publication is not required.

## Expanded deliverables requested 2026-09-12

- Extend the existing complete QueryGraph stack demo with Fihrist, retaining
  Grust, TypeSec, Marciana, LakeCat and Sail coverage.
- Prepare presentation slides, runnable code snippets and an executable EC2
  deployment package. Build the complete stack on the configured `grust` SSH
  machine and record end-to-end results there.
- Write the Fihrist book and announcement blog using the Fihrist `cover/`
  artwork. Publish the book to firstpair.org and prepare the blog as an iCloud
  textpack. Use `announcing-fihrist` as the story slug in the library link.
- Use Rust edition 2024 for all new or changed Rust crates and commands.

These deliverables extend the active integration goal. Completion requires
the remote execution evidence, published book and delivered presentation/blog
artifacts as well as the integration acceptance gates above.

### EC2 and publication progress

The configured `grust` SSH host is reachable. Prepared sources are staged in
`/home/admin/querygraph-fihrist-demo-20260912`. QueryGraph built there using
Rust 1.98.1 and the explicit Fihrist source override. Its binary SHA-256 is
`0a5b2ecd8c69a579a6f2a2ded892cd8ead7848c17687d00ab090547c475a9df6`.
The successful combined run is `reports/run-nkdEW8dY/`. Its original semantic
phase did not enable live Spark Connect. Its authenticated Fihrist phase passed
actual owner reads, both MCP entry paths, projection/purpose denial, backend
failure, buffer discard after three real-read mutations, and real rows after a
reviewed revision-two consumer cutover. Copies of both JSON reports are in
`demo/fihrist/evidence/`. A first attempt failed at linking because the host
lacked Python 3.13 development libraries; installing the matching development
package resolved it. No application check was weakened.

The repeatable build and run scripts are in `demo/fihrist/`. The 13-slide browser
deck is `slides.html`; its verified PDF is `dist/querygraph-fihrist-slides.pdf`.
All slides were rendered and visually inspected. The PowerPoint runtime named
by the presentation skill is unavailable, so the deck uses portable HTML and
PDF, with editable source and embedded speaker notes. Live-service coverage now
passes: 34 graph nodes and 33 edges loaded into Sail, a dataset node read back,
and lineage appended through Spark Connect. The repeated report is
`demo/fihrist/evidence/grust-live-services.json`. The optional absolute
`QG_SAIL_WAREHOUSE` configuration fixes the embedded demo warehouse location
without changing the server-managed library default. Relative paths fail closed.

Four persistent systemd services run on grust: `querygraph-fihrist-sail`,
`querygraph-fihrist-owner`, `querygraph-fihrist-api` and
`querygraph-fihrist-console`. The browser console is available through
`ssh -N -L 18081:127.0.0.1:18081 grust` at `http://localhost:18081`.
All eight browser actions passed against EC2: original Navigator bundle,
QGLake specialist story, live semantic graph, Fihrist discovery, planning,
actual execution, column denial and purpose denial. Returned rows were exactly
`[{"id":2}]`. Browser output and evidence are retained in
`demo/fihrist/evidence/grust-console.json`; desktop/mobile layouts were inspected.
The owner catalog is a synthetic in-memory fixture and resets to its seed on
restart. Systemd persistence does not imply durable production catalog state.

The executable fresh-deployment pipeline and exact source archive are in
`demo/fihrist/`. The bundle excludes runtime credentials, data, build targets
and caches; it carries per-file source checksums and base-commit provenance.
Released Grust, TypeSec and Marciana dependencies compile from Cargo sources
on EC2. Fihrist and the prepared LakeCat/Sail extensions use explicit source
overrides, not committed sibling path pins.

Cold-building Sail's CLI exposed a codec mismatch between shared Iceberg
delete-file references and deserialized owned values. The codec now preserves
shared references over the existing JSON wire format. Its new round-trip test
covers both position and equality deletes, sequence numbers and field schema.
All 19 execution-crate tests and strict all-target/all-feature package Clippy
pass on EC2. The changed crate uses Rust 2024, its generated-module references
are updated, and its tests are in separate files. Dependency-wide Clippy on
Rust 1.98 initially reported function-crate lints; that crate has since been
migrated and validated. All five changed crates now pass strict package Clippy
and 427 combined tests. Broader checking reports unrelated Delta Lake lints;
it is not claimed as green and is not an upstream-acceptance gate for our
source-checkout integration.

QueryGraph's latest full suite passes 111 unit and five integration tests.
Three separate console tests cover cross-origin/unknown-action rejection,
capacity admission and output bounds. Strict all-target Clippy passed; the
stack matrix is aligned with the existing unpublished Grust dependency warnings.

The final prepared-source EC2 row acceptance run is
`reports/run-iGjK2QtX/`, copied as
`demo/fihrist/evidence/grust-fihrist-execution-final.json`. It again passed both
MCP paths, separate execution authority, mandatory filtering, projection/purpose
denials, missing-data rejection, row/evidence digests, all three real-read
mutation checks and revision-two consumer cutover. The final persistent semantic
run is `reports/live-tmpV76xa/`, copied as `grust-live-services-final.json`.
The remaining default-build dependency gate is Fihrist 0.1.1 availability from
crates.io. Sail source-checkout integration is the intended target and does not
await upstream review, merge or release.
Fihrist commit `9422922` pushed the existing cover/headboard artwork and the
preceding integration source commit. This did not publish a Cargo crate.
FirstPair's canonical clean/pushed preflight passed for Fihrist and FirstPair
before editorial work began. Fihrist source commit `b2d323c` contains the book
and announcement; artifact commit `41d50ad` contains the built book and textpack.
The full book has 18 pages at 6-by-9-inch trim, plus EPUB, MOBI and hosted HTML
editions. The unified builder and artifact validators passed, and sampled PDF
pages were visually inspected.

The book is published at `https://firstpair.org/read/fihrist/`, with working
`/fihrist/pdf/`, `/fihrist/epub/` and `/read/fihrist/chapters/` routes. All four
returned HTTP 200 after deployment. The live catalog contains the cover,
headboard and story `https://querygraph.ai/announcing-fihrist/`. FirstPair
publication metadata is committed and pushed as `9d244c1`.

The announcement textpack is delivered to
`~/icloud/blogs/announcing-fihrist (0.1.1-b2d323).textpack`, with source commit
provenance and a verified byte match. Versioned PDF and EPUB copies in
`~/icloud/books` also match the canonical book bytes. The blog itself has not
been published; the user requested an iCloud textpack and the library story link.

## Implementation constraints

MCP sessions own their imported semantic models and progress from uninitialized
to initializing to ready. Invalid transitions do not execute tools. Transport
parsing and error rendering stay outside domain validation. Fihrist remains the
single source for registry validity, compatibility, and digest semantics.

Runtime services own trusted configuration; individual requests borrow it.
Plans are not execution capabilities. Authorization is checked at the operation
boundary, without caching decisions across policy or snapshot changes.
Backend awaits must remain cancellable, with no lock spanning an await and no
unowned tasks. Mutation outcomes distinguish success, conflict, and uncertain
commit state; diagnostics must not expose protected rows or credentials.

Existing public names and wire outputs require compatibility fixtures or an
explicit migration. No performance improvement is claimed by this work.

## Progress

Implemented: MCP lifecycle/version negotiation, bounded input and model state,
typed tool arguments, shared schemas, search alias, Python CLI handoff to Rust,
signed Fihrist host composition, an authenticated REST metadata/planning adapter,
CLI configuration, async MCP planning, and filtered signed discovery with no
catalog I/O. Tests cover direct/bridged sessions, identity/body binding, denied
columns/purposes, unresolved policy, drift and backend failure. Deployment review
and read-only reconciliation include all registry pins and distinguish partial
deployment, authoritative table absence, and drift.

Real owner-authorized rows and lineage now pass through direct and bridged MCP
using prepared isolated owner sources. Deterministic mutations after real engine
reads also verify the owner release guard. The transport now supports bounded
governed concurrency and cancellation. A reviewed activation check and live
stop/apply/restart fixture now cover controlled consumer transition. Remaining:
owner release checks and released dependency
resolution. The execution extension and Sail helper are not yet released.
Do not replace this release gap with committed path dependencies.

Operational details and verification commands are in
[the MCP Fihrist service guide](mcp-fihrist-service.md).

Fihrist 0.1.1 is prepared in local commit `6c8e95b`: all 24 tests, Clippy, docs,
and a clean-package publication dry run pass. Explicit publication approval is
pending. QueryGraph currently uses an explicit command-line Cargo patch for
development checks; resolve its lockfile from crates.io after publication before
committing/releasing the dependency update. The matrix is regenerated alongside
the pending pin change; alignment alone does not establish publication.

Latest verification: Rust all-target tests passed (102 unit and five integration
tests). Strict Clippy, rustdoc with warnings denied, formatting, and whitespace
checks passed; doc tests passed in the earlier MCP stage. All 81 Python tests pass
with the temporary Cargo override propagated to subprocesses, including both
real SDK tests. Running Python's Rust-equivalence tests without that override
fails dependency resolution for unpublished Fihrist 0.1.1; this is an outstanding
release gate, not a passing default build. The stack checker reports alignment
with pre-existing warnings in Grust's unpublished querygraph-memory manifest.
No release or live row-execution claim has been made.

## Owner-side work identified from live source

- LakeCat's `revalidate_governed_scan_grant` reloads the durable grant, compares
  the exact proof and catalog identity, checks current table version/metadata/
  snapshot, and freshly authorizes policy. Its result is neither deserializable
  nor a lease. Execution must bind the authenticated caller to that grant and
  preserve these checks through result release.
- Sail's `IcebergTableProvider` already implements snapshot reads, projections,
  filters, and delete application. Reusable bounded row execution should call
  this provider in Sail; QueryGraph must not reconstruct physical reads from
  exposed file tasks. The current LakeCat adapter only implements planning,
  commit preparation, and task fetches.
- LakeCat's commit path already uses `TableCommitSnapshot` for its internal
  compare-and-swap. It lacks a caller-supplied predecessor precondition covering
  metadata-only changes before the server loads that snapshot. Add an optional
  owner-checked precondition, bind it into idempotency request identity, and
  verify stale/duplicate/malformed preconditions before enabling automatic
  registry mutation. Existing unconditioned Iceberg clients must keep working.
- LakeCat's service binary does not configure a TypeDID gateway from its current
  environment settings. A live signed fixture must compose
  `TypeSecTypeDidVerifier` explicitly, as the existing owner integration test
  does; the conservative verifier must never be replaced by an unsigned fallback.

LakeCat follow-up is in the isolated worktree `/tmp/lakecat-fihrist-integration`,
branch `codex/fihrist-integration`, commit `93a7673f` (based on `27e18f39`). The state-token
extension returns `x-lakecat-table-state` on table loads and accepts
`x-lakecat-expected-table-state` on commits, binding that precondition into exact
retry identity. Its three targeted tests, 508 Sail/TypeSec service tests, 490
default-feature service tests, strict Clippy, formatting, and dependency-contract
checks pass. The dependency checker now supports isolated worktrees with
`QUERYGRAPH_RUST_DIR` instead of hard-coded checkout paths. The updated book
built and passed artifact/layout validation in
`/tmp/lakecat-fihrist-book-check`. QueryGraph now wires reviewed conditional
application with all-table preflight, stable field IDs, and restart recovery.
The LakeCat commit has not been pushed or deployed. The baseline LakeCat/Sail service build and real TypeSec envelope
verification test also passed; neither establishes governed row execution.

The first real authenticated deployment fixture exposed static TypeSec envelope
reuse: the first request succeeded and the next was rejected as a replay. The
adapter now owns a host-configured signer and mints fresh envelopes per request;
real gateway tests accept both fresh envelopes and reject replay of the first.
Legacy static credentials are explicitly single-use. The isolated fixture lives
in `integration/fihrist`; its temporary manifest composes real LakeCat, Sail,
and TypeSec without committing local-source consumer dependencies.

The authenticated live deployment run now passes, with evidence in
`/tmp/querygraph-fihrist-live-report.json`: reviewed-digest enforcement, real Sail
schema evolution with stable IDs, all registry pins, physical Iceberg metadata
readback, restart without repeating completed updates, stale-token conflict, and
drift blocking. The file comparison excludes only `lakecat:version` and
`lakecat:last-request-hash`, which CatalogStore adds after Sail persists the file.
The server stopped and its temporary warehouse/credentials were removed. The
report binds LakeCat commit `93a7673fb62209869cb0b32424da1ee60f9d0f6f` and binary
SHA-256 `1e2cfe442098bc596458a89088269ec1fc62a5b88f0a7ba90167850a4a3fd285`.
This is catalog deployment acceptance, explicitly not governed row execution.

Completion audit: the goal remains active. Row execution, fresh authority at
result release, denial/drift/snapshot/backend failure acceptance for actual rows,
lineage, controlled consumer cutover, and released dependency resolution are
still required. This turn made progress by implementing conditional application,
repairing real replay failures, and passing authenticated deployment acceptance;
it is not a blocked turn. No publication, push, or consumer switch occurred.

The next execution increment adds LakeCat's in-process
`read_with_governed_revalidation` boundary. It checks the current verified caller
and durable grant before invoking the backend, then checks current authority and
catalog evidence again before returning the buffer with final evidence. Seven
targeted tests and the 515-test Sail/TypeSec service suite pass, as does strict
Clippy. The tests use controlled callback results and do not claim real Sail
rows. See [the execution integration contract](governed-execution-contract.md)
for the remaining request binding, engine, lineage, and MCP acceptance work.

Source inspection confirmed that grants do not separately retain the original
filters and limit. The owner execution operation must retain its normalized
request from planning through execution; a client-presented old proof plus
replacement filters is not sufficient. No Sail shared surface has been changed.
This increment is progress on result-release authorization, not goal completion
or a blocked turn. Publication approval remains pending independently.

The buffered-read increment is committed locally as LakeCat `1097fca1` on the
same isolated branch. All 497 default-feature tests also pass, alongside
formatting, dependency-contract checks, and book artifact/layout validation
(69 pages). No push or deployment occurred. The prior live deployment report
continues to identify the exact earlier commit it tested; it is not silently
relabelled as acceptance evidence for the new guard.

The next increment implements Sail's real buffered snapshot reader in isolated
worktree `/tmp/sail-fihrist-integration` (base `9f6f8065`, branch
`codex/fihrist-execution`). The Python acceptance fixture has passed against
physical Iceberg/Parquet rows, including filtering before projection/limit,
byte bounds, exact snapshots across append, and missing-data-file rejection
without partial rows. Its report is `/tmp/fihrist-sail-rows-report.json`, with
binary SHA-256 `676fa7c2222994cddb3a3cb1b9d235585ab3bc4ba6467ba4d1c21f72a49d94c5`.
Temporary data was removed. The helper and fixture remain uncommitted; strict
Sail Clippy is not green due to existing warnings in unchanged source. See the
execution contract for the precise scope and outstanding validation.

Completion audit remains negative: real engine reads now work, but the owner
endpoint must still retain normalized filters/limit, apply mandatory predicates,
use the release guard, bind evidence and lineage, and serve the MCP contract.
Consumer cutover and released dependency resolution also remain. This turn is
progress, not a wait or blocker; no release, push, or deployment occurred.

The user explicitly requires Rust 2024. The earlier explicit 2021 formatter flag
was corrected. `sail-iceberg` now declares edition 2024 instead of inheriting the
upstream default; raw `r#gen` identifiers and three pattern changes compile under
2024. QueryGraph, LakeCat, and both temporary fixture manifests already use 2024.
All subsequent explicit formatter commands must specify 2024.

The owner execution increment is implemented but uncommitted: a strict
authenticated QueryGraph integration route retains purpose/projection/limit and
observed catalog state; requires separate execution permission; applies mandatory
predicates through Sail; and returns buffered rows with final owner and lineage
evidence. The real TypeSec run exposed a context-dependent policy hash that
prevented revalidation. A versioned stable decision hash fixes it while keeping
request context in authorization evidence; old grants require replanning.

Latest verification: 497 default LakeCat service tests, 28 security tests, and
both real-row pytest cases pass. The owner fixture verifies mandatory filtering,
planning-only denial, denied columns/purpose, state preconditions, missing data,
and independently recomputed row/evidence digests. The earlier deployment fixture
also passes with the prepared Sail source override. These checks do not establish
MCP execution, real mutation-during-read acceptance, or a clean full release build.

Goal audit: progress; not blocked and not complete. Next connect the verified
owner operation to Fihrist/MCP, test real revocation/drift during reads, finish
consumer cutover, resolve broader Sail lint/release gates, and complete released
dependency resolution. No push, publication, or deployment occurred this turn.

The Rust 2024 MCP execution composition now passes a real authenticated
Iceberg/Parquet acceptance run through both direct and Python-bridged SDK
sessions. The report is `/tmp/fihrist-mcp-rows-report.json`; it verifies mandatory
filtering, planning-only denial, forbidden columns/purpose, distinct operation
signatures, state preconditions, missing-data failure, and recomputed digests.
The fixture's bridge entry point was corrected to `python -m querygraph`.
Three separate-file Rust tests cover tampered rows, rehashed scope expansion,
missing release evidence, another caller, and changed/missing observation.
The implementation checks RFC3339 release timestamps without claiming a lease.
Real mutation during reads, consumer cutover, broader release checks, and
released sibling resolution remain outstanding. The goal remains active.

Verification after MCP execution composition: 105 Rust unit tests and five
integration tests pass, strict Clippy and rustdoc with warnings denied pass,
and all 81 Python tests pass with the explicit temporary Fihrist override.
Both physical-row pytest cases pass (47.86 seconds), with the owner case
including direct and bridged MCP execution against the latest QueryGraph binary.
Formatting and whitespace checks pass. The stack matrix remains aligned with
the previously recorded Grust warnings. No publication, push, or deployment
occurred; the prepared execution source changes remain uncommitted.

The next turn adds a Rust 2024 fixture-only engine decorator that forwards to
real Sail and then injects a policy/catalog change before owner release. A
retained run `/tmp/fihrist-mutation-rows-report.json` passes purpose revocation,
registry-pin drift, and a valid second snapshot produced by a real append.
Each case proves one real row was buffered and that the owner returned 409 with
no rows. The same run passes normal direct/bridged MCP checks and removes all
temporary data. Mutation checks use owner HTTP, not MCP; they do not claim
races within physical file I/O. The initial test expected 403 for revocation;
inspection confirmed the existing grant-revalidation contract returns 409, and
the assertion was corrected before the successful run.

Cancellation inspection confirms the current stdio loop waits for each governed
call before reading more input. Transport cancellation is therefore not
implemented despite cancellable library futures. Next implement bounded,
session-owned in-flight requests with concurrent control-message intake and
verify cancellation/disconnect cleanup. Consumer cutover and owner release
checks remain. This turn is progress, with no publication or deployment.

The cancellation increment now runs up to 16 governed requests in session-owned
tasks while the input thread continues reading controls. Cancellation aborts
the matching task; numeric/string IDs remain distinct; EOF/error paths abort and
join remaining work. Requests have a 30-second deadline, and cancelled/completed
IDs can be reused without an old task removing a replacement handle. Synchronous
tools retain ordering; completed output writes still use stdio backpressure.
This does not guarantee cancellation of work inside a remote HTTP server.

Inspection of the Python SDK showed that cancellation of its local wait did not
notify the upstream Rust request. The optional Python CLI now replaces itself
with the selected Rust executable, preserving stdio and protocol IDs rather than
creating a second MCP session. The original Python FastMCP mode remains available.
All 81 Python tests pass after this handoff. The latest real-row/mutation run
also passes and is retained at `/tmp/fihrist-cancellation-transport-rows-report.json`;
that report verifies rows and mutations, while Rust transport tests separately
verify cancellation. No push, publication, or deployment occurred.

Final checks for this increment: 109 Rust unit tests plus five integration
tests pass, including the actual 30-second timeout/drop check. Strict all-target
Clippy, rustdoc with warnings denied, formatting, and whitespace checks pass.
The stack matrix is aligned with the previously recorded Grust warnings.
The goal remains active; next complete the controlled consumer transition and
the owner/released-dependency validation gates.

The consumer transition increment adds optional `activation.expected_registry_digest`
to trusted service configuration. Startup first validates the immutable digest,
then requires every catalog table's schema/pins to match within one 30-second
deadline before MCP initialization. Existing discovery-only configurations remain
compatible. The operator owns the maintenance window; startup is an observation,
not a lock, automatic process manager, or rollback mechanism.

`/tmp/fihrist-consumer-cutover-report.json` passes a real authenticated
stop/apply/restart sequence: the old consumer exits before mutation, all schema
and registry pins migrate, the target consumer starts and discovers the expected
revisions, and unreviewed/premature/stale/drifted configurations fail startup.
The report identifies QueryGraph binary SHA-256
`723df4c9098e7049805cc22f2654ef0956070088717102320e28dc508adf08af`.
Owner HEAD is `1097fca1` with the previously documented uncommitted execution
changes; this is prepared-source acceptance, not release acceptance. Temporary
processes/data were removed. A separate Rust test proves wrong-digest rejection
before catalog I/O and preserves legacy discovery-only startup. The goal remains
active; no publication, push, or production deployment occurred.

The combined row/cutover run also passes:
`/tmp/fihrist-cutover-execution-report.json` verifies that old MCP sessions close,
a reviewed additive revision is applied/reconciled, and activation-pinned target
sessions return real policy-filtered Iceberg rows with revision 2 through both
direct and Python CLI entry points. This closes the gap between revision
discovery and actual reads after migration. The same run retains all earlier
denial, mutation-before-release, digest, and missing-data checks.

Validation for this increment: 110 Rust unit tests and five integration tests
pass; strict all-target Clippy, rustdoc with warnings denied, formatting, and
whitespace checks pass. Stack alignment retains the existing Grust warnings.
The goal remains active with owner release checks and released dependency
resolution outstanding. No live fixture process remains running.

The owner release audit found additional Rust 2024 Clippy findings in
`sail-iceberg`. Mechanical let-chain/format-borrow/copy fixes are now applied in
the isolated worktree, and affected utility tests were moved into separate files.
Unrelated formatter-only diffs were removed. Strict package Clippy with
`--all-targets --all-features --no-deps`, all 88 Sail unit tests, and the real-row
pytest case pass. Broader Clippy still reports nine redundant formatting borrows
in unchanged `sail-logical-plan`; this is not a clean full-release claim.

LakeCat's updated book builds and passes EPUB/PDF/artifact validation (69 pages)
at `/tmp/lakecat-fihrist-execution-book-check`. The full workspace/all-feature
test command passed its main unit suites, including 515 service tests, but its
nested compile-fail build selected the old Sail pin because CLI overrides were
not inherited. That run failed and restored its lockfile. A corrected run uses
a temporary Cargo home with the same explicit source overrides so nested Cargo
processes inherit them; no committed manifest/config gains a path dependency.
The corrected full-suite result is still pending. This turn is progress; the
goal remains active and publication approval remains unanswered.

The inherited-override LakeCat run completed successfully: all 1,411 workspace
tests passed, including the compile-fail API test, and all-feature/all-target
workspace Clippy passed. Both commands restored the original owner lockfile.
The temporary Cargo home is `/tmp/fihrist-owner-cargo.0GT4kV`; its configuration
is validation-only and its registry/git directories reuse the existing caches.

The remaining Sail lint failures were fixed in `sail-logical-plan` and
`sail-data-source`, with both changed crates explicitly migrated to Rust 2024.
Generated-module references use `r#gen`; changes otherwise comprise mechanical
let-chains, formatting borrows, an ignored-field pattern, and a byte-array spelling.
Strict Clippy now passes for all three changed Sail crates with all targets,
all features, and dependency linting enabled. Final combined Sail and owner
tests are running after these changes; earlier owner results are not silently
relabelled as verification of the newer dependency source. Fihrist 0.1.1 remains
absent from crates.io on recheck. The goal remains active.
