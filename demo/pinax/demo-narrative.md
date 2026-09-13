# QueryGraph with Pinax: the complete demo narrative

Presenter companion to the [14-slide deck](dist/querygraph-pinax-slides.pdf). Based on the prepared-source deployment and recorded EC2 evidence from September 12, 2026. Allow approximately 20 minutes for the presentation, with additional time for inspecting receipts.

## The story we are telling

QueryGraph connects discovery, semantic descriptions, agent identity, policy, graph storage, and governed access to data. Pinax adds a reviewed table contract to that journey. An agent can discover what a table means, request a bounded read, and receive rows only when the contract, policy, catalog state, and execution evidence agree.

The demonstration has two connected parts. The original QueryGraph workflows show semantic packaging and permissioned agent collaboration. The Pinax extension shows actual protected rows moving through an authenticated catalog owner and physical execution engine. These are composed in one console, but they are distinct operations: the semantic answer does not automatically launch the Pinax read.

We build Sail from our selected source checkout. Upstream Sail review, merge, and release are not prerequisites. The prepared build uses explicit source overrides, and the Rust implementation work uses edition 2024. The recorded run establishes correctness for that source snapshot; it makes no performance claim.

## Open the demo

From the laptop, keep this SSH tunnel running:

```bash
ssh -N -L 18081:127.0.0.1:18081 grust
```

Open `http://localhost:18081` for the console and `http://localhost:18081/slides` for the presentation. The console has nine fixed actions. Each produces inspectable JSON or an expected policy denial. Run them individually and wait for each to finish.

| Slides | Console action or evidence | Presenter emphasis |
| --- | --- | --- |
| 1-2 | Introduce the stack | Separate meaning, identity, authority, and execution |
| 3 | Navigator bundle; QGLake agent story; Semantic graph | Preserve the original QueryGraph story |
| 4 | Discover contracts | Introduce the reviewed table contract |
| 5-6 | Plan a scan | Explain the bounded request and its checks |
| 7 | Read governed rows | Show the real returned record and receipt |
| 6-8 | Protect private fields; Enforce purpose | Demonstrate explicit denials |
| 8 | Recorded mutation acceptance | Explain checks after actual reads |
| 9 | Recorded revision cutover | Show controlled contract evolution |
| 10 | MCP implementation and transport tests | Explain both entry paths and interruption |
| 11-12 | Deployment and evidence files | Describe repeatability and limits |
| 13 | Central ontology | Reviewed meanings drive the governed read |
| 14 | Book and announcement | Give the audience the longer explanation |

## Slides 1-2: who owns each responsibility?

Start with the question: “What must remain true between an agent discovering a dataset and receiving its rows?” The answer spans several components.

Grust supplies the property graph representation and graph APIs. TypeSec supplies identity and policy machinery. Marciana supplies semantic catalog and agent workflow capabilities. Pinax represents validated, versioned table contracts. LakeCat owns catalog authority and the governed operation. Sail interprets the physical table and executes the read. QueryGraph composes these capabilities and exposes trusted service and MCP entry points.

A table contract describes the agreed meaning and shape of data, including stable field identifiers, types, ownership, stewardship, policy binding, and revision compatibility. Physical catalog metadata describes the actual table and snapshot. The demo checks that these descriptions still agree when the table is used.

A classification such as “internal” is descriptive metadata, not an access grant. Likewise, a declared retention period does not demonstrate that a lifecycle service has deleted expired data. Keep those distinctions explicit when presenting the contract.

## Slide 3, action 1: Navigator bundle

Click **Navigator bundle**. This produces the original AI Navigator semantic bundle for a hazard vocabulary. Expand the result's `layers` object.

Semantic Croissant describes the dataset, distribution, record set, and fields. In this fixture the fields are `subject`, `value`, and `source`, connected to semantic terms for the observation subject, observed value, and evidence citation. CDIF presents discovery and integration descriptions, access information, and controlled vocabulary references. The DID layer identifies the publisher/controller. ODRL records permissions and prohibitions, including attribution and the requirement for a separate model-training agreement.

The point is that a dataset can carry machine-readable meaning, identity, and usage terms together. This action generates the bundle; its dataset URLs and access declarations are not evidence that the browser fetched those remote datasets or enforced every declared term.

Suggested transition: “We now have a description an agent can understand. Next we show how agents work within distinct permissions.”

## Slide 3, action 2: QGLake agent story

Click **QGLake agent story**. The scenario asks where fiscal capacity, energy burden, mobility disruption, and climate-health risk overlap.

A supervisor delegates to finance, energy, mobility, climate-health, reference, and restricted-data specialists. Each specialist has a role and a compartment-specific policy. The synthesis agent combines approved summaries. The restricted-data broker contributes a denial receipt instead of restricted raw health records.

Inspect the role assignments, ODRL policies, signed summaries and delegation envelopes, final briefing, OpenLineage event, and DID attestation. They connect the output to the identities and decisions in the workflow. The briefing explicitly says that restricted data contributed metadata and a denial receipt.

This is an executable, deterministic agent-story fixture. The presence of Sail table names in its catalog and lineage does not establish that this action scanned each named dataset. The subsequent semantic action and Pinax read provide the separate evidence for live storage and real row execution.

## Slide 3, action 3: Semantic graph

Click **1 · Semantic graph**. This runs `dataverse-e2e` with live Sail enabled, using a new report directory and an absolute warehouse path for this invocation.

The workflow describes two fixture datasets: “Bay Area building energy observations” and “Enterprise data access survey.” It connects datasets and files to semantic elements, ontology terms, Navigator bundles, and OSI models, fields, and metrics. The recorded result contains **34 graph nodes and 33 edges**, spanning nine node labels.

The graph is written through Grust to Sail over Spark Connect. The run reads a dataset node back and verifies its identity and label. It also exercises the semantic views and produces a policy-checked agent answer. Signed lineage is written to the local event file and DID attestation ledger, and an OpenLineage event is appended to Sail's `qg_audit.openlineage_events` table.

There are two important execution details. The report's Cypher summary is calculated in memory; it is not a demonstration of server-side Cypher pushdown. The answer is deterministic fixture output; optional Ollama inference is not enabled. Actual graph storage, node readback, and the lineage append do use the live Sail service.

To repeat this action directly on grust:

```bash
DEMO_ROOT=/home/admin/querygraph-pinax-demo-20260913
RUN=$(mktemp -d "$DEMO_ROOT/reports/narrative-XXXXXXXX")
export QG_SAIL_WAREHOUSE="$RUN/warehouse"
"$DEMO_ROOT/src/querygraph/target/debug/querygraph" dataverse-e2e \
  --live-sail --sail-endpoint http://127.0.0.1:15051 \
  --sail-dir "$RUN/sail" \
  --openlineage-file "$RUN/events.jsonl" \
  --did-ledger-file "$RUN/attestations.jsonl" \
  --question 'Which governed datasets mention access control?' \
  > "$RUN/semantic.json"
```

Suggested transition: “Discovery and graph evidence tell us what exists and how the answer was formed. Pinax now connects a reviewed contract to a real protected read.”

## Slide 4, action 4: Discover contracts

Click **2 · Discover contracts**. QueryGraph uses its trusted registry configuration and signed caller identity to discover contracts allowed for the requested purpose.

The retained fixture returns enterprise `acme`, table `rows`, revision `1`, and a registry digest. The visible column is `id`, with stable field ID `1`, required `int64` type, `internal` classification, and semantic identifier `row.id`. The metadata identifies the owner as `platform`, steward as `data`, and declared retention as 30 days.

The result is a governed description, not an unrestricted dump of the physical table. The private email column is absent from this permitted discovery result. The registry digest identifies the exact contract collection the consumer is using; later checks can detect that the collection has changed.

## Slides 5-6, action 5: Plan a scan

Click **3 · Plan a scan**. The executable client submits this exact bounded intent:

```json
{"table":"rows","columns":["id"],"purpose":"analytics","limit":10}
```

The client hashes the serialized intent and signs an operation-bound TypeDID envelope. Discovery, planning, and execution use distinct signed resource scopes. Changing the intent or reusing a signature for a different operation is not equivalent to the original request.

The caller chooses a table, projection, purpose, and limit. Trusted host configuration supplies the registry, policy context, identity handling, and catalog credentials. The request cannot substitute a storage path or a policy engine.

Pinax checks the request against the contract and allowed purpose. QueryGraph checks the registry and catalog agreement. The owner plans against a particular physical snapshot. The displayed plan identifies `rows`, revision `1`, columns `["id"]`, purpose `analytics`, limit `10`, the registry digest, and a snapshot ID. Snapshot IDs and receipt hashes can vary between freshly seeded fixtures.

Planning does not return rows. Permission to plan is distinct from permission to execute; the acceptance fixture explicitly verifies that a planning-only identity cannot use that permission to obtain data.

## Slides 6-7, action 6: Read governed rows

Click **4 · Read governed rows**. This is the central real-data demonstration.

The synthetic Iceberg table has actual metadata and Parquet files. One stored record has ID `1` and tenant `other`; another has ID `2` and tenant `acme`. The owner supplies the mandatory tenant predicate. Filtering determines the permitted rows, while projection restricts the returned fields. The request's limit bounds the result.

The expected rows are exactly:

```json
[{"id":2}]
```

ID `1` is excluded by the owner predicate. The private email field does not appear. This is a physical engine read, not a fabricated result inserted by the console.

Follow the request through the boundaries: QueryGraph validates the signed intent and contract; LakeCat checks separate execution authority; Sail reads the selected snapshot under the mandatory predicate; the owner buffers the result and revalidates authority and state before release. QueryGraph checks the returned scope and digests and reloads current catalog state before accepting the response.

Expand the evidence beside the rows. It binds the principal, purpose, table, effective projection, snapshot, plan task, authorization decisions, catalog state, and result digest. The fresh authorization digest and revalidation timestamp describe the release-time check. The independent acceptance runner recomputes row and evidence digests instead of trusting a success label.

The owner fixture uses `HashOnlyLineageSink`, reported as `lakecat-openlineage-hash`. Its receipt binds lineage hashes; that does not prove durable persistence of the owner's event in an external lineage system. The earlier semantic workflow separately demonstrates a real Sail lineage append and local event/attestation files.

## Slides 6-8, actions 7-8: Show the denials

Click **5 · Protect private fields**. This requests `private_email`, which is outside the allowed projection. Then click **6 · Enforce purpose**, which substitutes `marketing` for the allowed `analytics` purpose.

Both actions must be denied. A successful denial test requires the actual `Pinax scan denied` error, a nonzero process exit, and empty result stdout. The console does not classify an arbitrary connection failure as a successful policy denial.

Explain that identity answers who made the request, while authorization answers whether that identity may perform this operation for this purpose and projection. A valid signature does not make a forbidden request permissible.

## Slide 8: What if authority changes after rows have been read?

The presentation buttons preserve a retained fixture. More disruptive scenarios run separately in the acceptance harness against temporary data.

The mutation fixture waits until the real engine has returned a nonempty buffer, then changes one of three things: it revokes the purpose, changes the registry binding, or changes the snapshot state. In each case the owner detects the mismatch during revalidation and discards the buffer. No protected rows are released.

This matters because an admission check alone cannot establish that authority remained valid while work was running. The recorded acceptance also covers unauthorized projection and purpose, stale state, and missing physical data. Backend failure releases no rows.

Show [the final execution acceptance report](evidence/grust-pinax-execution-final.json), especially `real_buffer_discarded_after_mutations`, `missing_data_releases_no_rows`, and `row_and_evidence_digests_verified`. These are recorded acceptance results, not additional browser buttons.

## Slide 9: Evolve the contract and move consumers

The revision scenario adds a reviewed optional column and checks compatibility against the predecessor contract. Consumers tied to the old registry are closed. The operator applies the reviewed target and reconciles catalog pins, then starts consumers bound to the target digest. A real read succeeds under revision two.

The execution report records `real_rows_after_reviewed_consumer_cutover: true`. Related registry deployment tests cover creation, reconciliation, and rejection of stale activation observations.

Activation records what was checked at a point in time; it is not a distributed publication transaction or a guarantee that all consumers changed simultaneously. The operator owns the maintenance window and consumer lifecycle. The retained browser fixture stays at its seed revision, so it should not suddenly display revision two after discussing this separate test.

## Slide 10: MCP and interruption

The browser console calls the signed Rust `RegistryService` client directly. The separate acceptance harness exercises both direct Rust MCP and the Python CLI handoff to the Rust service. Its recorded result is `mcp_execution: "direct_and_bridged_passed"`.

MCP 2026-07-28 exposes these operations without a modern initialization handshake. Every request declares its version and capabilities; each governed tool still requires its own signed operation scope. Modern stdio and Streamable HTTP serve the central ontology, while older stdio clients retain the handshake. Imported semantic models return explicit handles, preventing unrelated requests on one connection from inheriting another conversation's data.

The HTTP endpoint accepts POST at loopback port 18082, validates version/method/name headers against the body, and returns request-scoped SSE for governed work. Dropping that stream cancels its pending future. The Rust transport bounds admission to 16 requests and governed work to 30 seconds. Local tests cover cancellation and bounds; live HTTP and both stdio entry paths passed ontology-derived reads, and captured responses validate against the official 2026-07-28 JSON Schema.

Cancellation has a precise boundary: it can stop local work still owned by the session. It cannot retract an already delivered response, and remote HTTP processing may continue after a local request is cancelled.

The complete runnable signing example is [pinax-client.rs](../../examples/pinax-client.rs). On grust:

```bash
DEMO_ROOT=/home/admin/querygraph-pinax-demo-20260913
CLIENT="$DEMO_ROOT/src/querygraph/target/debug/examples/pinax-client"
CONFIG="$DEMO_ROOT/run/pinax/registry-service.json"
"$CLIENT" --config "$CONFIG" discover
"$CLIENT" --config "$CONFIG" plan
"$CLIENT" --config "$CONFIG" execute
# The following commands intentionally return nonzero:
"$CLIENT" --config "$CONFIG" deny-column
"$CLIENT" --config "$CONFIG" deny-purpose
```

For an MCP client that supplies the signed envelopes, the stdio server is:

```bash
"$DEMO_ROOT/src/querygraph/target/debug/querygraph" \
  mcp-serve --registry-config "$CONFIG"
```

Starting that process alone does not perform a read. Modern clients submit a correctly signed tool request with per-request metadata; legacy clients initialize first. To exercise modern HTTP on grust:

```bash
"$DEMO_ROOT/src/querygraph/target/debug/examples/pinax-mcp-client" \
  --url http://127.0.0.1:18082/mcp
```

This client resolves `account number` through the central ontology, derives the table/field, signs the read separately, and checks denied purpose and altered intent. See [the stateless MCP guide](../../docs/mcp-stateless.md).

## Slides 11-12: Deployment, repetition, and operational limits

The prepared deployment lives at `/home/admin/querygraph-pinax-demo-20260913` on grust. Five systemd services support it:

| Service | Endpoint | Responsibility |
| --- | --- | --- |
| `querygraph-pinax-console` | Loopback 18081 | Console and slides |
| `querygraph-pinax-mcp` | Loopback 18082 | Stateless MCP and central ontology |
| `querygraph-pinax-sail` | Loopback 15051 | Spark Connect and graph storage |
| `querygraph-pinax-owner` | Loopback 18181 | Authenticated catalog and governed execution |
| `querygraph-pinax-api` | Port 18080 | Original QueryGraph HTTP API with required authentication |

The services restart on failure and are enabled at boot. The presentation tunnel requires no new EC2 security-group rule. The console runs one fixed operation at a time, bounds child output, and applies a 120-second operation deadline. It accepts no arbitrary shell command, path, or credentials.

Repeat the existing deployment's live checks without replacing its fixture:

```bash
ssh grust 'bash /home/admin/querygraph-pinax-demo-20260913/src/querygraph/demo/pinax/run-live.sh /home/admin/querygraph-pinax-demo-20260913'
```

That script creates a unique report directory, executes the live semantic workflow and the signed discovery/plan/read sequence, checks both denials, and checks the original API's health and QGLake story endpoints.

For a fresh deployment, extract the prepared source bundle into a new absolute directory, follow its `START-HERE.txt`, and run:

```bash
bash "$DEMO_ROOT/src/querygraph/demo/pinax/deploy.sh" "$DEMO_ROOT"
```

The deployment script builds the prepared sources, runs isolated acceptance scenarios, prepares a separate retained fixture, installs the services, and verifies live operation. Source checksums identify the delivered archive snapshot. Later source-checkout validation should not be described as though it changed that frozen archive.

The fixture uses synthetic records and a public demonstration identity. Its catalog is `MemoryCatalogStore`: an owner restart restores the seed catalog state even though data files and report directories remain. Boot-enabled services therefore do not imply a durable production catalog. Destination-generated signing material is excluded from the source bundle.

The demo retains its verified prepared-source snapshot. Pinax 0.2.0 (`pinax-registry`) is now published on crates.io, and QueryGraph resolves the reviewed package through its ordinary lockfile without a source override. Sail remains our selected source checkout; upstream merge is not a dependency. The separately requested Icecat checkout is not part of this deployed demonstration.

## Slide 13: Consult the central lakehouse ontology

Click **Consult central ontology**. The agent signs a query to the host-configured
Pinax ontology store. The current reviewed inventory contains the rows table
and all three physical fields. Only the customer identifier is visible to this
agent; tenant and private-email meanings are withheld with their fields.

Search for “account number” with the executable client's ontology operation to
show alias resolution. The result explains the match and returns a stable field
ID, table revision, registry digest and ontology digest. Concept review and
mapping review are separate. New schema imports remain drafts even when their
proposed concept already exists. An operator publishes immutable revisions with
an explicit reviewed digest and expected previous HEAD; stale consumer pins fail.

Plan and read actions now consult Customer identifier centrally, require an
unambiguous field match, resolve its permitted physical name and sign a fresh
request from that binding. Their output includes ontology_consultation. A read
also returns semantic_exports: Croissant 1.1 containing the exact authorized
rows as inline data, CDIF field descriptions, and the reviewed SKOS vocabulary.
Discovery descriptions alone are explicitly metadata-only.

Navigator, QGLake and semantic graph console actions also report their central
consultation preflight. Their original fixture story and signatures are
preserved; the governed plan/read flow is the demonstration that actually uses
the resolved mapping to choose a field. See [the ontology integration guide](../../docs/central-ontology.md)
and [the fixture review](ontology-review.md) for the authoring and publication workflow.

## Slide 14: Book, source, and story

Close with the Pinax book, which develops the contract model, implementation, and operational workflow beyond what fits in the live session. The slide artwork comes from Pinax's supplied `cover/` assets.

The book is published at [First Pair](https://firstpair.org/read/pinax/), with [PDF](https://firstpair.org/pinax/pdf/) and [EPUB](https://firstpair.org/pinax/epub/) editions. The library story slug is `announcing-pinax`, targeting `https://querygraph.ai/announcing-pinax/`. The announcement has been prepared as an iCloud textpack; the blog itself is not yet published.

Suggested closing sentence: “We can explain the dataset, identify the agent, check the intended use, execute against a specific table state, and verify the evidence before releasing the rows.”

## Evidence and implementation index

| File | What to inspect |
| --- | --- |
| [Browser results](evidence/grust-console.json) | All nine console actions and browser error results |
| [Final live services report](evidence/grust-live-services-final.json) | Live graph storage, readback, semantic workflow and lineage |
| [Final execution acceptance](evidence/grust-pinax-execution-final.json) | Real rows, both MCP paths, denied requests, post-read mutations and cutover |
| [Governed execution contract](../../docs/governed-execution-contract.md) | Request, authority, physical execution and release checks |
| [MCP service documentation](../../docs/mcp-pinax-service.md) | Trusted composition and transport behavior |
| [Executable Rust client](../../examples/pinax-client.rs) | Exact intent signing and service calls |
| [Console server](../../examples/pinax-demo.rs) | Fixed operations and child-process boundaries |
| [Acceptance harness](../../integration/pinax/execute_rows.py) | Independent row/evidence verification and fault cases |
| [Mutation fixture](../../integration/pinax/read_mutation.rs) | State changes after actual engine reads |
| [Deployment README](README.md) | Bundle, build, service lifecycle and repeat commands |
| [Release review](../../docs/pinax-release-review.md) | Prepared-source validation and published Pinax package |

Use the recorded reports to substantiate the September 12 run. Use a new `run-live.sh` report when presenting evidence from a later run; timestamps, snapshot identifiers, signatures, and hashes are expected to change.
