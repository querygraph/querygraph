# Pinax customer discovery demo

Part I answers **“What are customers called where?”** through four catalog, lakehouse, Pinax, and MCP steps. Three real synthetic business tables distinguish CRM customers, billing accounts, and product users. Reviewed concepts and bindings identify their actual fields. Graph operations and specialist workflows are hidden in an explicitly opened Part II.

Start with [the presenter narrative](demo-narrative.md), [reviewed fixture](customer-meaning.json), and [design contract](customer-discovery-design.md).

Sail is built from our selected source checkout. The demo and integration do
not depend on upstream Sail review, merge or release. `build-ec2.sh` compiles
`DEMO_ROOT/src/sail`; the owner fixture explicitly selects the same checkout
through `--sail-root`. The packaged source checksums identify the delivered
snapshot. Later checkout validation is recorded in the integration release audit.

Pinax is the `pinax-registry` 0.2.0 crate, imported as `pinax`. QueryGraph uses
the released registry dependency. Temporary fixture manifests select the
prepared LakeCat and Sail sources without changing release pins.

## EC2 layout and commands

Stage the prepared source trees under `DEMO_ROOT/src/{querygraph,pinax,sail,lakecat}`.
The configured `grust` host currently uses
`/home/admin/querygraph-pinax-demo-20260913` for this run. Existing services and
source checkouts are outside this directory.

For a fresh source bundle, set `DEMO_ROOT` to its absolute extraction directory:

```bash
bash "$DEMO_ROOT/src/querygraph/demo/pinax/deploy.sh" "$DEMO_ROOT"
```

The existing deployment is ready to use. From your laptop:

```bash
ssh -N -L 18081:127.0.0.1:18081 grust
# Open http://localhost:18081
```

The browser console runs four main discovery actions, with authoring, reads, access checks, and Part II collapsed. It serves the presentation and links
to the published book. It uses a signed Rust fixture client. The separate
acceptance runner exercises both MCP entry paths and the mutation scenarios.
No EC2 security-group change is needed for the SSH tunnel.

The source archive is `dist/querygraph-pinax-ec2.tar.gz`, with a sibling SHA-256
file. Extract into a new directory and follow `START-HERE.txt`. Its internal
`SOURCE-SHA256SUMS` identifies every source file; `PROVENANCE.json` records base
commits and prepared working-tree status. The binary checksums identify the
demonstrated build; binaries and runtime credentials are excluded.

The host needs Rust supporting edition 2024, Cargo, a C/C++ build toolchain,
protobuf compiler, OpenSSL development files, Python development libraries
matching the interpreter selected by `uv`, and `uv`. The runner uses four
Cargo jobs by default and disables development debug information. This is a
correctness demonstration, with no performance claim.

`run-ec2.sh` creates a new report directory each time. The original semantic
workflow is available only with `--part-ii`, and writes its report and signed lineage. The Pinax phase reads actual
Iceberg/Parquet rows, verifies output and evidence digests, rejects unauthorized
columns and purposes, discards buffers after policy/registry/snapshot mutation,
and restarts consumers after a reviewed registry revision.

`QG_FIXTURE_FETCH=1` permits dependency downloads before the fixture's offline
build. `QG_FIXTURE_BUILD_TIMEOUT` allows a cold EC2 build up to two hours.
Fixture data and services are temporary and clean up after each phase.

## Repeat the live presentation

```bash
ssh grust 'bash /home/admin/querygraph-pinax-demo-20260913/src/querygraph/demo/pinax/run-live.sh /home/admin/querygraph-pinax-demo-20260913'
```

`run-live.sh` runs only the four Part I discovery actions. Pass `--part-ii` explicitly to exercise the retained Spark Connect graph workflow, governed reads, access checks, and QGLake story. Each run has an isolated absolute warehouse
selected by `QG_SAIL_WAREHOUSE`. Omitting this environment variable preserves
the library's server-managed warehouse default; relative values fail validation.

The retained Part II live run loaded **34 graph nodes and 33 edges** for two Dataverse datasets,
read a dataset node back from Sail, and appended lineage to
`qg_audit.openlineage_events`. The report's Cypher summary is computed in memory;
graph storage and node readback are verified through Spark Connect.

## Runnable Rust and MCP snippets

The complete Rust 2024 example is [`examples/pinax-client.rs`](../../examples/pinax-client.rs).
It signs the exact intent body and calls the trusted service. On the host:

```bash
DEMO_ROOT=/home/admin/querygraph-pinax-demo-20260913
CLIENT="$DEMO_ROOT/src/querygraph/target/debug/examples/pinax-client"
CONFIG="$DEMO_ROOT/run/active-pinax/registry-service.json"
"$CLIENT" --config "$CONFIG" discover
"$CLIENT" --config "$CONFIG" plan
"$CLIENT" --config "$CONFIG" execute
"$CLIENT" --config "$CONFIG" deny-column  # expected nonzero; no result output
"$CLIENT" --config "$CONFIG" deny-purpose # expected nonzero; no result output

# MCP stdio server for a client that supplies the fixture's signed envelopes:
"$DEMO_ROOT/src/querygraph/target/debug/querygraph" mcp-serve --registry-config "$CONFIG"
```

The allowed intent is `{"table":"rows","columns":["id"],"purpose":"analytics","limit":10}`.
Expected rows: `[{"id":2}]`; tenant `other` and private email are excluded.
This public fixture identity is for synthetic demo records only.

## Services and lifecycle

`install-services.sh` installs five named systemd services:

| Service | Endpoint | Purpose |
| --- | --- | --- |
| `querygraph-pinax-console` | loopback 18081 | Browser console and slides |
| `querygraph-pinax-mcp` | loopback 18082 | MCP 2026-07-28 and central ontology |
| `querygraph-pinax-sail` | loopback 15051 | Spark Connect and Grust storage |
| `querygraph-pinax-owner` | loopback 18181 | Authenticated catalog and governed reads |
| `querygraph-pinax-api` | 18080 | Original QueryGraph HTTP API with `--require-auth` |

Services restart on failure and are enabled at boot. The installer refuses to
replace an identically named service from a different deployment directory.
The original API retains its existing network binding; no firewall ports were
opened. The console admits only local-host requests with its custom header,
runs one fixed operation at a time, bounds output and cancels children at its
120-second deadline. It does not accept shell commands, paths or credentials.

The active fixture is selected by `run/active-pinax`, pointing to `run/customer-discovery`. The original `run/pinax` fixture and `run/ontology` publication remain intact. Its **in-memory catalog resets to
the seed state on owner restart**. Data files and per-run semantic warehouses
remain for inspection. Plan-signing material is generated on the destination
with mode 0600 and excluded from the source bundle. `prepare-services.sh`
refuses to overwrite an existing fixture. Fault/cutover acceptance runs use
separate temporary data and never mutate this retained presentation fixture.

Stop only these demo services when finished:

```bash
sudo systemctl disable --now querygraph-pinax-{console,mcp,api,owner,sail}
```

## Presentation and evidence

- [`evidence/grust-customer-discovery-console.json`](evidence/grust-customer-discovery-console.json): live four-step customer discovery, shared/distinct field meanings, Part II hidden on load, all optional operations, and desktop/mobile checks. Earlier evidence below retains its original scope.

- [`evidence/grust-whole-stack-console.json`](evidence/grust-whole-stack-console.json): all ten main actions, two optional checks, and component/layout regressions against the redesigned live console.
- [`demo-narrative.md`](demo-narrative.md): complete presenter walkthrough, commands, and evidence.
- [`slides.html`](slides.html): 10 main slides with an explicitly opened Part II appendix and embedded speaker notes.
- [`dist/querygraph-pinax-slides.pdf`](dist/querygraph-pinax-slides.pdf): exported Part I deck.
- [`evidence/grust-live-services.json`](evidence/grust-live-services.json): persistent live semantic run.
- [`evidence/grust-pinax-execution.json`](evidence/grust-pinax-execution.json): both MCP paths, denials, real-read mutations and revision cutover.
- [`evidence/grust-pinax-execution-final.json`](evidence/grust-pinax-execution-final.json): the repeated acceptance run after the final source changes.
- [`evidence/grust-console.json`](evidence/grust-console.json): the original eight browser operations against EC2.
- [Pinax book](https://firstpair.org/read/pinax/), [PDF](https://firstpair.org/pinax/pdf/), [EPUB](https://firstpair.org/pinax/epub/).

The published library story is `https://querygraph.ai/announcing-pinax/`.
The announcement is also delivered as an iCloud textpack. Released dependency resolution remains a separate integration
gate. This package records prepared-source correctness, not release acceptance
or a performance benchmark.

## Central ontology

The demo includes a Pinax-owned central ontology, signed semantic
discovery, and ontology-resolved plan/read requests. See [the integration guide](../../docs/central-ontology.md),
[fixture review](ontology-review.md), and [presenter narrative](demo-narrative.md).
The seed example installs the reviewed synthetic vocabulary in run/ontology.
The three business tables use the `customer_discovery` purpose. New schema imports remain drafts; operators publish revisions with reviewed
digests and compare-and-swap. Runtime store contents stay out of source bundles.

## Customer discovery fixture

`integration/pinax/prepare_customers.py` extends a preserved single-table fixture into a **new** directory, creating three Iceberg tables and Parquet files. It never overwrites the old fixture. `pinax-ontology-seed` reviews only the exact source-owned synthetic contracts and publishes into a new central store. After verifying catalog activation, point the retained services at the new directory and restart consumers with its exact registry/ontology pins. Keep the original directory for rollback.

The MCP customer-discovery scenario is metadata-only. The original `--scenario governed-read` still checks the original analytics fixture. The source archive and older evidence files describe their recorded runs; new customer-discovery evidence is stored separately.

### Choose Iceberg or Delta Lake

Open https://demo.rust.ai/ and choose **Iceberg** or **Delta Lake** in the header.
The same selection is carried into Slides. Direct links:

- `https://demo.rust.ai/?format=iceberg`
- `https://demo.rust.ai/?format=delta`

Both use real table files on grust, independent catalog owners and MCP endpoints,
with the same business inventory, reviewed ontology and access policy. Switching
clears the current result; it never changes another visitor's selection.
Delta's transaction-log version appears in the existing `snapshot_id` wire field.
The Delta bridge is read-only and scoped to this retained demo; see
[table-formats.md](table-formats.md) for the exact compatibility boundary.
Grust's optional graph extension uses Delta internally with either selection.

Fresh `deploy.sh` builds and installs both. To add Delta to a running prepared
Iceberg deployment, rebuild the Rust examples and owner using the current
sources, then run `install-delta.sh DEMO_ROOT`. The extra services are
`querygraph-pinax-delta-owner` (18182) and `querygraph-pinax-delta-mcp` (18084),
both loopback-only. Keep the existing owner (18181) and MCP (18082) running.

To export both complete decks through the local SSH tunnel:

```bash
node demo/pinax/render-slides.mjs 'http://localhost:18081/slides?format=iceberg&part=all' /tmp/pinax-slides-iceberg
node demo/pinax/render-slides.mjs 'http://localhost:18081/slides?format=delta&part=all' /tmp/pinax-slides-delta
```
