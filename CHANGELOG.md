# Changelog

## Unreleased

- Bind live TPC-DS semantic answers to exact physical snapshots, upstream model
  and artifact, policy, plans, graph replay, and OpenLineage replay; fail proof
  verification under deliberate drift in every required dimension.
- Canonicalize Spark decimal metric values as exact strings before hashing and
  serializing the semantic answer proof.
- Record the pinned Ossie Polaris converter’s live TPC-DS structural,
  semantic, extension, and loss outcomes; prepare an evidence-linked upstream
  report-contract proposal and clean one-command demonstration guide.

- Derive five deterministic, relationship-connected physical TPC-DS Iceberg
  fixtures from the exact pinned Ossie model and create them through stock
  Spark's Iceberg REST catalog integration.
  The live runner binds Spark's standard S3 FileIO to the isolated benchmark
  object-store endpoint explicitly and loads the fixture planner without
  importing QueryGraph's Python 3.11 package facade into Spark's Python 3.10.

- Validate realized Spark schemas, bind every TPC-DS table by schema hash,
  create the governed policy binding, and CAS-publish/read back the exact
  upstream Ossie artifact through LakeCat's management boundary.

- Add an ordered semantic-publication admission pipeline proving malformed,
  unauthorized, missing-physical, schema/model drift, and unknown-version
  failures occur before catalog publication, graph, or lineage promotion.

- Pin Apache Ossie's schema, validator, and TPC-DS model by upstream commit and
  artifact hashes, with a fetch-and-verify workflow that keeps upstream bytes
  outside QueryGraph source ownership.

- Add a typed, multi-model Ossie artifact envelope with JSON/YAML round trips,
  Draft 2020-12 structural errors, and explicit loss reporting while preserving
  unknown model keys, custom extensions, and every dialect expression.

- Add the executable HadoopCatalog-to-LakeCat legacy migration cookbook for the
  catalog-community acceptance path.

- Add the live QueryGraph catalog-migration harness for protocol-native
  LakeCat, Polaris, and Lakekeeper interoperability evidence.

All notable changes to the QueryGraph Rust reference implementation are
recorded here. The codename pool and the shared version line live in
[`RELEASES.md`](RELEASES.md).

## Unreleased — unified repository

### Changed
- Add the QueryGraph-owned pure semantic boundary for LakeCat, Polaris, and
  Lakekeeper migration/federation verification; live adapters cannot classify
  registration as preserved when schema, spec, snapshot, ref, or pointer state
  differs.
- Add the audited AgentGym adversarial-agent results to both QueryGraph books:
  846 applicable case-runs across Pydantic AI, LangChain, and CrewAI, with
  runtime grade-A parity for OPA-mediated, Cerbos-mediated, and TypeSec and
  explicit simulation, provider, framework, receipt, and compile-time limits.
- Pin and verify the QueryGraph graph-enabled Sail revision that carries the
  open upstream performance PR, then pass the live Dataverse, graph/Cypher,
  OpenLineage-to-file-and-Sail, TypeDID authorization, and DID-ledger path with
  source-verified, fully optimized production Sail and QueryGraph executables.
- Expand the book with the released crate refactoring, standalone Marciana
  boundaries, the `adversarial-cognition` benchmark and its comparative
  results, and the Rust/Python/TypeScript release surfaces.
- Consolidate the Rust runtime and Python API in the canonical
  `querygraph/querygraph` repository. The Rust crate remains `querygraph`; the
  Python distribution is now maintained under `python/` with its public import
  and CLI names unchanged.
- Replace obsolete qg-rust/qg-python checkout assumptions in the shared CI and
  package metadata while preserving cross-language TypeDID fixtures.
- Track the migration, dependency matrix, compatibility contract, and Fable
  review gates in [`QGQG.md`](QGQG.md).
- Add the public TypeScript API under `typescript/`, with shared semantic and
  TypeDID contracts, Node tests, and npm publication support.

## 0.5.0-dev — unreleased

### Added
- **Registry-backed 0.4.1 release line:** QueryGraph now publishes against
  TypeSec `0.13.1`, Marciana `0.12.1`, Grust `0.12.1`, and LakeCat `0.3.0`
  crates rather than development Git dependencies.
- **TypeSec–Marciana stack facade:** QueryGraph now owns modular `stack`
  boundaries for request authentication, Marciana memory, cognition exports,
  and product-level errors. The service uses the facade for HTTP envelope
  verification and persistent memory while compatibility paths remain stable;
  the unused direct `marciana-catalog` dependency was removed.
- **Registry release dependencies:** QueryGraph now resolves TypeSec `0.13.1`,
  Marciana `0.12.1`, Grust `0.12.1`, and LakeCat `0.3.0` from crates.io rather
  than development Git branches.
- **TypeSec–Marciana facade plan:** documents the proposed QueryGraph-owned
  security, memory, cognition, and error boundary that will contain upstream
  TypeSec and Marciana implementation details without changing wire or route
  compatibility.
- **Native Marciana cognition consumer:** QueryGraph now delegates governed
  cognition composition to `marciana-cognition`; production retains only the
  opaque `improve` operation, while explicit feature-gated seams preserve the
  focused failure tests.
- **Marciana-owned protected-memory errors:** the fixed, non-disclosing
  TypeSec-to-product error mapping now belongs to `marciana-cognition`.
  QueryGraph consumes and re-exports that public boundary rather than owning a
  second implementation.
- **Marciana-owned binding errors:** the stable fail-closed error taxonomy for
  governed TypeDID, LakeCat, authority, projection, and proposal validation
  now belongs to `marciana-cognition` and is re-exported by QueryGraph.
- **Opaque governed `improve`:** governed cognition now exposes one public
  authenticated `improve(read, write)` operation. Planning and proposal
  application are crate-internal seams, so a QueryGraph caller cannot receive
  or replay a transient cognition proposal outside the authoritative path.
- **Standalone Marciana path:** the cognition integration now resolves
  `querygraph-memory` from `~/src/marciana` while retaining the `0.12.0`
  package and wire contract.
- **Durable cognition outcomes:** the QueryGraph application now carries the
  typed `Mutated`/`NoChange` effect through proposal, guarded Grust commit,
  recovery, audit, and TypeDID receipt paths, with no-change replay covered by
  separate application tests.
- **Owner-bound governed cognition sources:** LakeCat now owns the canonical
  snapshot and grant-aware source-scope digests; TypeSec attaches that scope
  only after exact governed-draft verification and requires it again during
  reveal, authoritative reload, derivation, audit, and receipt issuance.
  QueryGraph consumes those owner APIs, rejects local or mixed records before
  an engine sees plaintext, and pins application to the canonical proposal
  digest actually returned by its host-bound engine.
- **Commit-bound governed cognition:** QueryGraph binds bounded job, operation,
  sources, native algorithm, catalog, grant, and field mapping to a
  gateway-verified `memory:improve` TypeDID intent. Sender privacy may only
  narrow an independently authorized clearance, arbitrary sender claims never
  become policy authority, and a closed host-selected engine binding rejects
  algorithm substitution before protected input is loaded. LakeCat proofs are
  bounded before integrity work and freshly revalidated; TypeSec remains the
  sole mutation path. Expiry is rechecked inside its authority callback,
  public adapter errors are fixed categories, and deterministic receipts use
  immutable TypeSec preparation time, exposed as `preparedAt`, so backend clock
  skew and exact recovery cannot change signed bytes.
- **Cognition intent v2:** cognition requests now sign the exact native
  algorithm and version. Legacy v1 and unknown intent versions fail closed.
- **Closed native cognition engine API:** `GovernedCognitionApplication::new`
  now requires a host-selected `CognitionEngineBinding`, and `plan` no longer
  accepts an arbitrary engine. Production bindings are limited to `reference`
  and `live_sail`; the latter binds an established `Arc<SailGraphStore>` and
  enables the `querygraph-memory/sail` feature.
- **Verified cognition context:** governed cognition now accepts TypeSec's
  borrow-scoped verified TypeDID context and derives subject, signed purpose,
  and request identity from it; missing or mismatched purpose fails closed.
- **Non-blocking governed cognition:** QueryGraph now awaits Grust cognition
  engines, allowing the live Sail Spark Connect executor to run without
  blocking a runtime worker.
- **Governed Marciana cognition composition:** qg-rust now cross-checks the
  verified TypeDID subject and purpose against a hash-bound LakeCat/Iceberg
  governed Sail-scan proof before invoking Grust's native cognition engine;
  the result is an inert TypeSec proposal, never a direct store mutation.
  LakeCat's secret-free `GovernedScanProof` converts directly into this Grust
  cognition source contract without exposing its Sail token or receipt.
- **Persistent capability-secured agent memory**: `querygraph-memory` now
  connects qg-rust to a bootstrapped, file-backed Turso/libSQL graph through
  TypeSec's `MemoryVault`. `serve --memory-policy … --memory-db …` enables
  signed-only `/v1/memory/{remember,recall,forget}` routes. The verified
  TypeDID `did:key`—not a body field—is the policy subject; the signature key
  must match that sender, and the envelope recipient must be the QueryGraph
  service DID. Calls pass through `ToolCallGuard`, typed capability minting,
  clearance-aware recall, and the persistent vault. Router tests prove
  unsigned rejection, cross-recipient replay rejection, body-subject spoof
  resistance, RBAC denial, and close/reopen persistence.
- **The stack guide restructured as a full book** (`docs/guide`): executive
  summary and overview up front; four Parts — I. The Substrate (Grust, the
  query language, TypeSec, TypeDID, LakeCat, the bootstrap handoff, Sail),
  II. The Semantic Layer (a chapter per standard: Croissant, CDIF, DID,
  ODRL, plus OSI, the dual gate, lineage, the lakehouse path, the QGLake
  story, qg-python), III. The Interoperability Surfaces (`/v1` + envelope
  auth, MCP, A2A + tool schemas, the navigator loop, the cross-language
  contract), IV. Integration in Practice (the eleven-step assembly, catalog to
  governed answer, plugging in agent frameworks, operating and releasing) —
  closed by Future Work and a glossary/link appendix. 27 chapters; worked
  Rust/Python examples throughout, with outputs (bundle layers, receipts,
  verification reports, MCP transcripts) captured from real runs.
- **Per-chapter API references in the stack guide**: compact reference tables
  for every surface — Grust builder/stores/Cypher, TypeSec capabilities and
  TypeDID, LakeCat REST + the bundle crate, the four projection types in both
  languages, OSI, governance, lineage, the qg-python package map, `/v1` auth,
  the MCP tools, the navigator loop, and both CLIs.
- **A second integration walkthrough over live Dataverse data** (guide
  Chapter 25): `dataverse-e2e` against Harvard Dataverse, with output from a
  real run — live search staged into Sail, derived semantics, the dual-gate
  receipt, the `typedid/a2a` envelope, and DOI-level OpenLineage with a
  UUIDv5 run id and Ed25519 attestation.
- **Dual typesettings for the stack guide**: `-typst` and `-troff` PDF/EPUB
  editions alongside the canonical build. The troff PDF is set with
  `groff -Tpdf -P-e -k -t -ms` (embedded fonts, preconv, tbl) over a
  regenerated gropdf font map, with a `pdffonts` embed assertion; code-fence
  language tags are stripped for the ms writer (pandoc's highlight token
  macros are standalone-only and render blank otherwise). Both books' iCloud
  publishing now prunes superseded versioned copies before delivering.

### Changed
- **Marciana-owned LakeCat source translation:** QueryGraph now consumes the
  native `marciana-catalog` adapter for proof-to-cognition-source mapping and
  Marciana's stricter cognition table-identity boundary, retaining only
  application integration and public error translation.
- **Marciana-owned engine binding:** QueryGraph now consumes Marciana's closed
  reference/live-Sail engine binding. Arbitrary bindings remain available only
  through a test-only feature, never to production application code.
- **Reachable Marciana stack pins:** QueryGraph now resolves Marciana, Grust,
  TypeSec, and LakeCat from exact reviewed Git revisions rather than sibling
  paths, so the cognition integration can be built outside the local
  multi-repository checkout.
- **Complete memory failure routing:** the generic memory API now classifies
  proposal-free cognition recovery failures with the other fixed backend
  failures, keeping its TypeSec error mapping exhaustive as recovery evolves.
- **Current TypeSec consumers:** QueryGraph now reads verified DID and TypeDID
  values only through their non-forgeable accessors.
- **Current Sail consumers:** All live Sail entry points share a default-derived
  configuration helper, preserving Grust's session semantics without
  duplicating configuration across the lakehouse, lineage, and graph loaders.
- **Enforced TypeDID profile obligations:** every QueryGraph-produced agent,
  lineage, QGLake, and server envelope now carries the negotiated organization,
  verified agent id, and purpose claims instead of relying on descriptive
  profile metadata.
- **TypeSec Lido alignment**: the qg-rust agent, policy, TypeDID, and Marciana
  dependencies now resolve the `0.13.0` TypeSec release, keeping fresh local
  builds and the persistent memory integration on the same substrate line.
- **Marciana guide refresh**: the stack book now treats TypeSec 0.13.0
  "Lido" as the current security substrate, explains its capability-secured
  memory contract and QueryGraph `/v1` boundary, walks the Pydantic AI v2
  restart proof, and separates shipped v1 guarantees from post-v1 scale and
  hosted-service work. The verified suite ledger is now 41 Rust and 52 Python
  tests.

## 0.4.0 "Sentinel" — 2026-07-04

The governed-answer release: where Goshawk opened the doors (MCP, A2A, `/v1`,
cross-language crypto), Sentinel stands guard over what comes through them —
envelope auth on the API, the governed navigator loop with receipts, Rust
minting the envelopes Python verifies, and the whole stack realigned to the
0.12 substrate wave (Grust "Lobster", TypeSec "Torcello", LakeCat "Ocelot").
Ships alongside qg-python 0.4.0 "Sentinel".

### Changed
- **Stack alignment to the 0.12 substrate wave**: Grust `0.11.0 "Crab"` →
  `0.12.0 "Lobster"` (merged Full39075 GQL profile, atomic Cypher transaction
  batches), TypeSec `0.11.0 "Burano"` → `0.12.0 "Torcello"` (the
  agent-interoperability platform release), LakeCat `0.2.1 "Lynx"` → `0.3.0
  "Ocelot"` (stock-client Iceberg REST conformance). All 40 tests green
  against the new line; both books, the stack guide, the deck, the one-pager,
  and the READMEs updated accordingly.

### Added
- **The QueryGraph Stack guide** (`docs/guide`) — a second book: the
  definitive stack-wide guide (Grust, TypeSec, LakeCat, Sail, QueryGraph)
  with an executive summary and link index up front, built to EPUB/PDF/MOBI
  with versioned delivery links like the dedicated book, which remains in
  `docs/book` and gains a Goshawk interoperability chapter.
- **Stack review deck** (`docs/slides`, typst → PDF) and a **one-pager**
  (`docs/onepager`) in three typesettings: HTML, typst PDF, and troff/ms PDF.
  The troff build applies the omnighost findings — `groff -Tpdf -P-e -t -ms`
  (embedded fonts, `tbl` preprocessing, ragged-right) — regenerates the
  gropdf font map against the installed ghostscript, and asserts every font
  embeds via `pdffonts`.
- **MCP server over stdio** (`mcp` module; CLI: `mcp-serve`). A
  dependency-free JSON-RPC 2.0 implementation of the MCP handshake
  (protocol 2024-11-05) exposing the same governed surface as `/v1` and
  qg-python's FastMCP server: `build_navigator_bundle`, `run_qglake_story`,
  `verify_envelope`, `import_semantic_model` (OSI or Croissant),
  `search_semantic_models`, and `answer_question` (shared deterministic
  answer core with `/v1/answer`). Pointable at Claude Code/Desktop and any
  MCP client.
- **TypeDID envelope auth on `/v1`** (`serve --require-auth`). Governed routes
  (`models/import/*`, `answer`) demand a signed envelope in `x-qg-envelope`:
  `action == "invoke"`, `resource` bound to the request path (no cross-route
  replay), `payload.bodySha256` bound to the body, Ed25519 signature checked
  against the envelope's did:key. Failures are 401s carrying a receipt and
  the auth contract. Open routes (health, GETs, agent card, verify) stay open.
- **Rust now mints qg-python-compatible envelopes**
  (`PyTypeDidEnvelope::signed`): identical seed → did:key derivation as
  Python's `Ed25519Signer.from_seed`, closing the reverse crypto direction
  (Rust signs → Python verifies).
- **`POST /v1/answer`, first slice**: semantic search over the model
  registry, SQL plans for the matches, deterministic synthesis, and a signed
  TypeDID envelope plus an OpenLineage run with a spec-conformant UUID. The
  fully governed loop (RBAC+ODRL receipts, pluggable LLMs) is qg-python's
  `GovernedNavigatorLoop`; Rust parity follows with envelope auth.

## 0.3.0 "Goshawk" — 2026-07-03

The interoperability release, implementing FABLE-REVIEW-1 alongside qg-python
0.3.0 "Goshawk" (see the workspace `FABLE-REVIEW-1.md` §9).

### Added
- **A2A Agent Card** (`a2a` module; served at `/.well-known/agent-card.json`;
  CLI: `agent-card`). Aligns the existing `typedid/a2a` protocol label with
  the Linux Foundation Agent2Agent protocol: skills mirror the `/v1` surface
  and the security scheme documents the TypeDID envelope contract. The skill
  list is a cross-language contract asserted against qg-python by the
  equivalence suite.
- **`/v1` semantic-model registry**: `POST /v1/models/import/{osi,croissant}`
  (Croissant JSON-LD projects to OSI via the new
  `OsiDocument::from_croissant_json`, mirroring qg-python), `GET /v1/models`,
  `GET /v1/models/{name}`, and `GET /v1/search?q=` over names, descriptions,
  ai_context, semantic types, and ontology terms.
- **Cross-language envelope verification** (`agent::interop`). Rust now
  verifies qg-python's Ed25519-signed TypeDID envelopes with no shared state:
  `did:key` resolution (multibase/multicodec), reconstruction of the documented
  `querygraph-typedid-signing-v1` signing payload, `ed25519-dalek`
  verification, and byte-exact recomputation of Python's canonical payload JSON
  (`json.dumps(..., sort_keys=True, separators=(",", ":"))` with
  `ensure_ascii` escaping). Golden fixture generated by qg-python is tested in
  `cargo test`; the live round-trip (Python signs → Rust verifies, tampering
  rejected) runs in qg-python's equivalence suite.
- **`verify-envelope` CLI command**: reads an envelope JSON from a file or
  stdin, prints the verification report, exits non-zero unless the signature
  verifies.
- **`/v1` HTTP API, first slice** (`server` module, axum; CLI: `serve --port`).
  The platform is reachable over a network for the first time
  (FABLE-REVIEW-1 §4.1): `GET /v1/health`, `POST /v1/navigator/bundle`
  (four-layer Croissant/CDIF/DID/ODRL bundle), `GET /v1/qglake/story` (the
  governed multi-agent evidence chain), and `POST /v1/audit/verify-envelope`
  (verifies qg-python Ed25519 envelopes; an invalid signature is a 200 with a
  receipt, not a server error). Router-level tests cover all endpoints,
  including tamper rejection.
- **GitHub Actions CI**: fmt, clippy `-D warnings`, and tests against
  checkouts of `querygraph/grust` and `querygraph/lakecat` assembled to
  satisfy the `../..` path dependencies.

### Changed
- **OpenLineage run ids are now spec-conformant UUIDs**: the official 2-0-2
  JSON Schema requires `run.runId` to be a UUID, so run ids are deterministic
  UUIDv5 values under the QueryGraph namespace
  (`uuid5(NAMESPACE_URL, "https://querygraph.ai/openlineage")`), derived from
  the same seeds as before (envelope signatures, bundle hashes). qg-python
  derives identical ids; both CLIs' emitted events now validate against the
  official schema in the equivalence suite.

## 0.2.0 "Peregrine" — 2026-06-26

See the release log in [`RELEASES.md`](RELEASES.md).
