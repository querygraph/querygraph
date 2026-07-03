# FABLE-REVIEW-1: QueryGraph Enterprise Agentic AI + Semantic Layer

*A full-stack review of the QueryGraph workspace — findings, gaps, and a roadmap for
strengthening, completing, and opening the platform to the Pythonic agentic-AI and
OSS AI ecosystems.*

Date: 2026-07-02 · Reviewed: `qg-rust` 0.2.0 "Peregrine", `qg-python` 0.2.0 "Peregrine",
`sail` (fork, branch `grust`), `semantic/claude` + `semantic/codex`, plus the sibling
stack (`grust` 0.11 "Crab", `typesec` 0.11 "Burano", `lakecat` 0.2.1 "Lynx").

**Updated 2026-07-03:** the P0 items and quick wins have been implemented — see
§9 "Implementation status" at the end of this document.

---

## 1. Executive summary

You have built a coherent, unusually standards-dense **governed semantic layer for
agentic AI**: four semantic projections (Semantic Croissant, CDIF, DID, ODRL) plus OSI
business semantics, RBAC+ODRL dual gating, TypeDID signed agent envelopes, OpenLineage
audit with Ed25519-anchored attestations, all over a Sail (Spark-compatible) lakehouse
with a Grust property-graph and a Cypher extension compiled *into* Sail itself. The
Rust core is clean (~6,600 LOC, 25/25 tests passing, zero TODOs), the Python mirror is
lean and faithfully equivalence-tested, and the `semantic/` research repos already
contain the *next* architecture (Polaris `SemanticModel` entity + `/navigator-bundle`
projection + `OSIMetricFacet` for OpenLineage + ODS packaging).

The three findings that matter most:

1. **The platform is a library + CLI, not yet a service.** The documented `/v1` HTTP
   API (`docs/sail-typesec-grust-implementation.md` §"API Surface") is designed but
   unimplemented. Nothing can call QueryGraph over a network except Sail itself. This
   is the single biggest blocker to interoperability of any kind.
2. **The "agents" are governance fixtures, not agents.** `qglake-story` (Rust and
   Python) is fully deterministic — hardcoded summaries, no LLM in the loop. The only
   real inference path is the Rust `dataverse-e2e --call-ollama` flow. The platform
   *governs* agents beautifully but does not yet *host or navigate for* one.
3. **There is no MCP surface.** In 2026, MCP is the lingua franca between semantic/data
   layers and every agent framework (Claude, OpenAI Agents SDK, LangChain/LangGraph,
   PydanticAI, LlamaIndex, CrewAI all speak it). One MCP server would make the entire
   governed lakehouse reachable from every Pythonic framework at once — with far less
   code than N framework adapters.

Everything else — crypto parity for Python, JSON Schema validation, publishing
hygiene, Polaris integration, A2A alignment — slots in behind those three.

---

## 2. What exists today (component inventory)

### 2.1 `qg-rust` — the reference implementation (~6,600 LOC, 25 tests, all passing)

| Area | Modules | State |
|---|---|---|
| Semantic projections | `croissant`, `cdif`, `osi`, `did`, `odrl`, `validation` | Solid; validation is shape-checking, not JSON Schema |
| Governance | `rbac`, `agent/` (TypeDID envelopes), TypeSec 0.11 integration | Real Ed25519 via TypeSec; RBAC **and** ODRL both required to allow |
| Lakehouse | `lakehouse/` (infer/normalize/load/project), `sail/` (graph + views) | Streams CSV/TSV/XLSX → typed Parquet in Sail; Arrow IPC temp views; `SailGraphStore` round-trip |
| Lineage/audit | `lineage/` + `sail_sink` | OpenLineage COMPLETE events, canonical hash, signed root attestation, JSONL + Sail `qg_audit` sinks |
| Catalog boundary | `lakecat/` | Verifies LakeCat bootstrap bundles via shared `qglake-bundle` crate (no copied wire format) |
| Agent story | `qglake/` (Resilience Desk) | **Deterministic** multi-agent narrative; supervisor/specialists/broker/synthesis |
| LLM path | `agent/ollama.rs` | Real: DID-encrypted prompt → `DidMessageGateway` → `DidOllamaClient::chat_verified_prompt_bound` → signed reply envelope |
| Interfaces | `main.rs` CLI (9 commands) | **CLI-only.** No HTTP, no gRPC of its own, no MCP |

Dependencies: `grust`/`grust-cypher` and `qglake-bundle`/`lakecat-core` are **local path
deps** (`../../grust`, `../../lakecat`); `typesec-*` 0.11 from crates.io. Single crate —
the Phase-1 workspace split from the implementation doc has not happened.

Docs are a genuine asset: the implementation proposal (6 phases, `/v1` API design),
an architecture report (md/PDF), a stack-announcement blog with rendered Mermaid
diagrams, a full **book** pipeline (EPUB+PDF, versioned), and a Ulysses textpack
workflow. Release engineering is disciplined (SemVer + birds-of-prey codenames,
coordinated stack versions across Grust/TypeSec/LakeCat).

### 2.2 `qg-python` — the Pythonic mirror (~1,800 LOC, 12 tests)

Pydantic-v2-first port of the same layers, plus Python-only ecosystem pieces:
`typedid.py` (Pydantic TypeDID agents/envelopes), `agents.py`
(`TypeDidLangChainToolAdapter` → `langchain_core.tools.StructuredTool`),
`lakehouse.py` (PySpark → Sail Spark Connect helpers), `lineage.py`, `qglake.py`
(deterministic story), CLI with Rust-command parity. Modern packaging (hatchling, uv,
extras: `agents`, `lakehouse`, `all`), wheel + sdist already built in `dist/`.

Cross-language parity is enforced by a real test: `test_rust_equivalence.py` runs
`cargo run -- navigator` and `python -m querygraph navigator` and asserts JSON
equality modulo timestamp. (Only the `navigator` command is covered.)

Key deltas vs Rust:

- **Signatures are demo placeholders** — deterministic SHA-256 with seeds like
  `"querygraph-typedid-demo-signature-v1"` (`typedid.py`, `lineage.py`). The Rust side
  signs with TypeSec Ed25519; the Python side only *looks* signed.
- No `py.typed` marker, no async anywhere, no MCP, LangChain adapter is sync-only,
  validation is presence-checking, `pyproject.toml` missing `authors`/`urls`/classifiers.

### 2.3 `sail` — fork of lakehq/sail, branch `grust` (3 commits ahead of upstream)

Not a pristine clone: it adds a **Cypher graph query extension** (~5,600 insertions
across 20 files) — Cypher AST in `sail-sql-parser`, graph analysis in
`sail-sql-analyzer/src/graph.rs`, resolution in `sail-plan/src/resolver/query/graph.rs`
(~2,900 lines), Spark Connect integration, and a design doc
(`docs/development/graph-extension.md`). It reuses Grust's property-graph model and
schema validation rather than reimplementing them. This is a significant, upstreamable
piece of work — and also a fork-drift liability (see §4.8).

### 2.4 `semantic/` — the next architecture, already designed

Two research repos (`sl-claude`, `sl-codex`) that answer "where does the semantic layer
*live* in the open catalog ecosystem":

- **`claude/`**: `semantic-layer-report.md`, `ARCHITECTURE.md` (Apache Polaris
  `SemanticModel` entity; REST `/v1/{catalog}/namespaces/{ns}/semantic-models` CRUD +
  `/navigator-bundle` projection endpoint; privileges `USE_SEMANTIC_MODEL`,
  `MANAGE_SEMANTIC_MODEL`, `EXPORT_NAVIGATOR_BUNDLE`, `FEDERATE_SEMANTIC_MODEL`;
  4-phase roadmap), the **two-semantic-layers thesis** (Layer A physical/catalog vs
  Layer B business/agent, bridged by the Navigator bundle), 8 cross-spec example
  artifacts (OSI YAML ↔ OpenLineage `OSIMetricFacet` ↔ Iceberg ↔ Unity ↔ Gravitino ↔
  Navigator bundle ↔ Polaris resource ↔ ODS manifest), and 6 working Python modules
  (`osi_loader`, `navigator_from_osi`, `polaris_osi_plugin`, `openlineage_emitter`,
  `ods_packager`, `demo`) that round-trip the whole story.
- **`codex/`**: a minimal `qg_polaris_semantic` library, a Polaris OpenAPI sketch, and
  **OPA/Rego policy examples** for semantic-model authorization.

This work is ahead of the implementations: almost none of it has been folded back into
`qg-rust`/`qg-python` yet. That fold-back is most of the "complete it" story.

---

## 3. Strengths worth preserving

1. **The evidence chain is the product.** "The answer used OSI metric X, resolved to
   Sail table Y at snapshot Z, under capability C and odrl:read, emitting OpenLineage
   run R anchored by attestation A" — no mainstream agent framework offers this. Every
   interop decision below should *export* this chain, never dilute it.
2. **Dual-language parity with an executable contract.** The Rust↔Python equivalence
   test is the right idea; it just needs to cover more than `navigator`.
3. **Standards-first surface area.** Croissant 1.1, CDIF, W3C DID/ODRL, OSI,
   OpenLineage, Iceberg REST — each is a door to an existing community.
4. **Deterministic DIDs and canonical hashing everywhere** make golden-file testing and
   cross-language verification cheap.
5. **Compartmentalized agent topology** (specialists never share raw rows; synthesis
   sees only signed summaries; restricted broker returns denial receipts) is a genuinely
   good enterprise pattern — it maps directly onto LangGraph subgraphs and A2A tasks.
6. **Documentation and release discipline** far above prototype norm: book, blog,
   architecture PDF, coordinated codename releases, ≤500-line module rule.

---

## 4. Findings: gaps and weaknesses

### 4.1 No service surface (critical)

Everything is `cargo run --`/`python -m querygraph`. The `/v1` API
(`models/import/*`, `search`, `plan`, `answer`, `lineage/events`, `audit/verify`) is
specified in the implementation doc but does not exist. Consequences: no agent
framework, BI tool, or partner node can reach QueryGraph; the Polaris/ODS designs in
`semantic/` have nothing to call; every demo requires a local checkout of four repos.

### 4.2 Agents are scripted, not LLM-driven (critical for the "agentic" claim)

`qglake_story` hardcodes specialist summaries in both languages (e.g. Python
`qglake.py` maps `"FinanceAgent" → "Fiscal capacity summary…"`). The governance
plumbing is real; the intelligence is not. There is no navigator loop that takes a
question, searches the semantic graph, plans SQL against Sail, executes under
policy, and synthesizes an answer with the evidence chain. The Rust Ollama path proves
the envelope machinery works end-to-end with a real model — it needs to be promoted
from a demo flag into the core loop.

### 4.3 Python crypto is cosmetic (high)

Python TypeDID envelopes and lineage attestations use deterministic SHA-256 "demo"
signatures. Anything verified on the Rust side would reject them; worse, nothing
*marks* them as unverifiable, so a downstream consumer could mistake them for
signatures. This breaks the core value proposition (portable, verifiable receipts) the
moment a Python agent participates.

### 4.4 Validation is shape-checking, not schema validation (medium)

Both languages validate Croissant/CDIF/OpenLineage by field presence. The README's own
"Next Milestones" already names this. No `mlcroissant` validation, no official
OpenLineage JSON Schema check, no Dataverse/OSI schema enforcement. Interop with OSS
consumers (Hugging Face Croissant readers, Marquez) is asserted, not proven.

### 4.5 Test asymmetry and coverage gaps (medium)

Rust: 25 unit tests, no integration test against a live Sail (the richest code paths —
`dataverse-e2e --live-sail`, `lakehouse-load` — are manually verified only). Python:
12 tests; the LangChain adapter, CLI subcommands other than `navigator`, and all error
paths are untested. Equivalence covers one command. No CI is visible in either repo.

### 4.6 Publishing blockers (medium)

`qg-rust` depends on `../../grust` and `../../lakecat` path deps — not `cargo publish`-able
and not buildable by anyone without your exact `~/src` layout. It is also still one
crate rather than the proposed workspace (`qg-core`, `qg-osi`, `qg-croissant`, …).
`qg-python` is nearly publishable but lacks `py.typed`, `project.urls`/`authors`, and
the PyPI name `querygraph` should be verified/claimed early.

### 4.7 The `semantic/` designs are stranded (medium)

`OSIMetricFacet`, the Polaris `SemanticModel` REST shape, ODS packaging, OPA/Rego
authorization, and the two-layers doc live only in research repos. `qg-python` has no
`osi_loader`-grade multi-dialect expression support (`semantic/claude/osi_loader.py`
is actually *richer* than `querygraph/osi.py` — ai_context, synonyms, dialect
fallback); the emitters/packagers have no production home.

### 4.8 Workspace/ops hygiene (low, cumulative)

- The top-level `querygraph/` directory is not a git repo and has no README — there is
  no single place that explains the four-component layout, the sibling-repo
  requirement, or the run order. (`FABLE-REVIEW-1.md` §2 is currently the only map.)
- The Sail fork tracks a fast-moving upstream; 3 local commits will rot without a
  rebase cadence or an upstreaming plan for the Cypher extension.
- Version pins are aggressive on the Python lakehouse extra (`pandas>=3.0.0`,
  `pyspark>=4.1.2`) — fine for you, hostile to enterprise environments; consider
  widening lower bounds after testing.
- `langchain-core>=0.3` is unbounded above; pin `<2` or test against current majors.

---

## 5. Interoperability: the Pythonic agentic-AI and OSS AI plan

The strategic framing: **QueryGraph should not compete with agent frameworks — it
should be the governed data/semantics plane that every framework plugs into.** The
compartmentalized-specialist pattern, policy receipts, and evidence chain are the
differentiators; the frameworks bring the LLM loops. That means the priority order is
protocol surfaces first (MCP, A2A, OpenAI-schema tools), then thin per-framework
adapters, then standards round-trips.

### 5.1 MCP server — the single highest-leverage item

Expose the platform as a **Model Context Protocol server** in both languages:

- **Tools**: `search_semantic_graph(question)`, `resolve_metric(name, dialect)`,
  `plan_query(metric|question)`, `execute_governed_query(sql, envelope)`,
  `check_access(principal, resource, action)` (returns the RBAC+ODRL receipt),
  `verify_attestation(id)`, `anchor_url(url)`.
- **Resources**: `qg://bundles/{did}` (Navigator bundle JSON-LD), `qg://models/{name}`
  (OSI doc), `qg://lineage/{run_id}`, `qg://catalog/tables/{schema.table}` (Croissant
  field metadata from Sail schema).
- **Governance mapping**: MCP `_meta`/headers carry the TypeDID envelope; every tool
  result embeds the policy receipt and the OpenLineage run id. A denial is a
  first-class result (the RestrictedDataBroker pattern), not an error.
- Implementation: Python via the official `mcp` SDK (`FastMCP`) in a new
  `querygraph.mcp` module (extra: `querygraph[mcp]`); Rust via `rmcp` in a new
  `qg-mcp` crate. Both stdio and streamable-HTTP transports; the HTTP transport can
  share the axum server from §6 P1.

One MCP server instantly reaches Claude Code/Desktop, OpenAI Agents SDK, LangChain
(`langchain-mcp-adapters`), PydanticAI (`MCPServerStdio`/`MCPServerStreamableHTTP`),
LlamaIndex, CrewAI, AutoGen — with zero per-framework code.

### 5.2 A2A: make `typedid/a2a` real

The Rust agent run already labels its protocol `typedid/a2a`. Align it with the actual
Linux Foundation **Agent2Agent** protocol: publish an Agent Card
(`/.well-known/agent-card.json` — dovetails with the ODS `/.well-known/` manifest idea
in `semantic/claude`), map `QgLakeAgent` request/response to A2A task lifecycle
(submitted → working → completed, with artifacts), and position TypeDID envelopes as
A2A message signatures/extensions. The supervisor→specialist→synthesis topology is
exactly the A2A multi-agent story; QueryGraph would be one of the first *governed* A2A
implementations, which is a publishable result on its own.

### 5.3 Framework adapters (thin, on top of the same core)

Priority order, by fit and audience:

1. **PydanticAI** — the natural sibling: qg-python is already Pydantic-v2-first. Ship
   a `QueryGraphToolset` and an output-validator that attaches/verifies policy
   receipts. Near-zero impedance; likely ~150 LOC.
2. **LangGraph** (beyond the existing LangChain `StructuredTool`) — the
   supervisor/specialist/broker/synthesis topology *is* a `StateGraph`. Ship a
   prebuilt graph factory: nodes wrap TypeDID agents, edges are delegation, a policy
   gate node produces receipts/denials, `interrupt()` maps to human-in-the-loop
   approval for restricted data, and the checkpointer doubles as an OpenLineage
   emitter. This turns the deterministic qglake story into a real LLM-driven one with
   the same governance guarantees.
3. **OpenAI Agents SDK / function-calling schema export** — add
   `TypeDidAgent.to_tool_schema()` emitting standard JSON-Schema tool definitions
   (name/description/parameters), which OpenAI, Anthropic tool-use, Mistral, and local
   runtimes all accept. One exporter covers many frameworks without adapters.
4. **LlamaIndex** — Navigator bundles as document/index metadata; Croissant RecordSets
   → structured retrievers over Sail tables.
5. **CrewAI / AutoGen(AG2) / smolagents** — document recipes using the MCP surface
   rather than bespoke adapters; only build native adapters if users ask.

Cross-cutting adapter requirements: **async variants** (`as_async_tool()`,
`httpx.AsyncClient` in `codata.py`/`dataverse.py`), and every adapter returns
`(answer, receipt, lineage_ref)` — never a bare string.

### 5.4 LLM provider layer (general OSS AI)

- Generalize `agent/ollama.rs` to the **OpenAI-compatible chat API** (one client then
  covers Ollama, vLLM, llama.cpp server, LM Studio, TGI, OpenRouter, SGLang) while
  keeping the DID-encrypted prompt-binding wrapper. Add native Anthropic support.
- In Python, don't build a provider layer — bind to LangChain/PydanticAI model
  abstractions and keep QueryGraph focused on governance around the call:
  `GovernedPrompt` in, signed `AgentResponse` + receipt out.
- Idea: a **"governed inference proxy"** mode — an OpenAI-compatible endpoint that
  fronts any backend but requires a TypeDID envelope, enforces ODRL
  (`derive`/`train` actions matter here), and emits OpenLineage per call. That is a
  standalone OSS product enterprises currently lack.

### 5.5 Standards round-trips (prove interop, don't assert it)

| Standard | Action |
|---|---|
| Croissant | Validate generated JSON-LD with `mlcroissant`; add an importer for Hugging Face dataset Croissant exports (instant corpus of thousands of datasets); contribute the "Sail table location" recordSet extension pattern upstream |
| OpenLineage | Replace hand-rolled event models with the official `openlineage-python` client (Rust keeps its own but adds the official JSON Schema conformance test); register `OSIMetricFacet` as a documented custom facet and pursue upstreaming per `semantic/claude` Phase 3; integration-test against Marquez in CI |
| OSI | Merge `semantic/claude/osi_loader.py`'s multi-dialect/ai_context model into `querygraph/osi.py` (and mirror in Rust); track the OSI consortium spec version explicitly; add importers from **dbt MetricFlow** and **Cube** metric definitions → OSI (this is how existing enterprise semantic layers migrate in) |
| DID/VC | Real Ed25519 in Python (§6 P0); consider emitting attestations as W3C Verifiable Credentials for wallet/verifier interop |
| ODRL | Publish the QueryGraph ODRL profile (actions: read/index/derive/train/export/answer) as a documented JSON-LD profile; add the `semantic/codex` OPA/Rego bridge so enterprises can evaluate the same policies in their existing OPA fleets |
| Iceberg REST / Polaris | Implement the `semantic/claude` ARCHITECTURE: `SemanticModel` entity + `/navigator-bundle` endpoint — either as a Polaris plugin (Java, larger lift) or first in **LakeCat** (your Rust catalog, immediate) with the Polaris OpenAPI shape, so the REST contract is proven before the Polaris PR |
| ODS | Productionize `ods_packager.py` into `querygraph.ods`; emit the v1alpha1 manifest from any Navigator bundle |
| Arrow | Add an **ADBC/Arrow Flight SQL** path in qg-python for querying Sail without the heavyweight PySpark extra — notebooks and agents get `pip install querygraph[adbc]` with pyarrow only |

### 5.6 Packaging/distribution for the OSS audience

- Publish `grust`, `qglake-bundle`/`lakecat-core` to crates.io (or vendor via git
  tags) so `qg-rust` builds anywhere; split the workspace per the Phase-1 plan.
- Ship `querygraph` to PyPI with `py.typed`, URLs, classifiers; keep the core
  dependency footprint at exactly `pydantic`.
- One `docker compose up`: Sail + qg-server + Marquez + Ollama, loading a small bundled
  corpus — the 10-minute reproducible demo of the whole evidence chain. Publish images
  to GHCR.
- Top-level meta-repo (README + compose + versions matrix + the release codename log)
  so the four-component layout is discoverable.

---

## 6. Completion roadmap

### P0 — Trust and publishability (unblocks everything; ~small, mostly mechanical)

1. **Real Ed25519 signing in qg-python** (`cryptography` or `pynacl`): sign TypeDID
   envelopes and lineage attestations; add a cross-language test — Python signs, Rust
   verifies, and vice versa. Until then, rename the demo fields so they cannot be
   mistaken for signatures.
2. **JSON Schema validation** both languages: official OpenLineage schema, Croissant
   via `mlcroissant` (Python) / vendored schema (Rust), OSI schema, Dataverse payloads.
3. **CI (GitHub Actions)** on both repos: build, clippy/ruff, tests, plus the
   Rust↔Python equivalence suite extended beyond `navigator` (qglake story JSON, OSI
   projection, lineage event hash).
4. **Packaging**: `py.typed`, pyproject metadata, PyPI name; workspace split +
   dependency strategy for crates.io.
5. **Meta-repo README** at `~/src/querygraph/` tying the components together.

### P1 — Become a platform (the core build)

6. **`qg-server`** (axum): implement the documented `/v1` API — `models/import/{osi,croissant}`,
   `models/*`, `search`, `plan`, `answer`, `lineage/events/*`, `audit/verify` — with
   TypeDID envelope auth per the design.
7. **MCP servers** (Rust `rmcp` sharing qg-server internals; Python `FastMCP` for the
   notebook/local crowd) per §5.1.
8. **The real navigator agent loop**: question → semantic-graph search (Grust/Cypher,
   including through the Sail Cypher extension) → allowed-data filtering via
   RBAC+ODRL → SQL plan against Sail → execution → LLM synthesis (OpenAI-compatible +
   Anthropic + Ollama) → answer with full evidence chain. Convert `qglake-story` into
   an *eval* for this loop (the deterministic version becomes the golden baseline —
   same governance receipts, now with live inference).
9. **LangGraph prebuilt graph + PydanticAI toolset** (§5.3), async adapters, and
   `to_tool_schema()` export.

### P2 — Ecosystem embedding

10. LakeCat-first implementation of the Polaris `SemanticModel` + `/navigator-bundle`
    REST shape; then the upstream Polaris plugin conversation, armed with a working
    reference.
11. `OSIMetricFacet` upstreaming to OpenLineage; Marquez CI integration.
12. A2A Agent Cards + task mapping; ODS manifest endpoint under `/.well-known/`.
13. dbt MetricFlow / Cube importers → OSI; Hugging Face Croissant importer.
14. ADBC/Flight SQL Python path; governed inference proxy (§5.4) as an experiment.
15. Sail fork: agree a rebase cadence and open the upstream conversation about the
    Cypher graph extension (even a rejected RFC de-risks the fork).

### P3 — Differentiation and research

16. Batched **Merkle-root attestations** by tenant/model/time window (already on your
    milestone list) — makes the audit trail scale.
17. Ontology import (OWL/SKOS/JSON-LD → Grust) and cross-node **federation** with
    inbound signed bundles (Phase 6 of the implementation doc).
18. **Benchmark the semantic layer's effect on agent accuracy**: text-to-SQL with vs
    without OSI/Croissant context on BIRD/Spider-style tasks over Sail — a
    quantified "semantic layers make agents right" result is the strongest possible
    marketing for the whole stack, and nobody has published it for an open,
    end-to-end governed system.
19. Book/blog refresh once P1 lands: "the governed agent plane" story with the live
    loop replacing the scripted one.

---

## 7. Quick wins (each ≤ a day)

- Add `querygraph/py.typed` + pyproject `urls`/`authors`/classifiers.
- Rename Python demo-signature strings to `demo_digest`/`unsigned:` prefix until real
  crypto lands.
- Extend `test_rust_equivalence.py` to `qglake-story --json` (normalize timestamps).
- `TypeDidAgent.to_tool_schema()` → OpenAI/Anthropic-compatible JSON tool definition.
- `mlcroissant`-validate one generated bundle in the Python test suite.
- Async `as_async_tool()` on the LangChain adapter.
- Top-level `README.md` for `~/src/querygraph/` (component map, sibling-repo
  requirement, run order).
- GitHub Actions workflow: `cargo test` + `uv run pytest` on push.
- Widen/pin sanity pass on `lakehouse` and `agents` extras.
- Promote `semantic/claude/osi_loader.py`'s ai_context/synonyms/dialect model into
  `querygraph/osi.py` (pure Pydantic port, no new deps).

---

## 8. Closing assessment

The unusual thing about this stack is that the *hard, boring, differentiating* parts —
deterministic identity, dual policy gating, signed envelopes, canonical hashing,
lineage anchoring, cross-language parity, standards projections — are already built
and tested, while the parts every framework gives you for free — an HTTP server, an
LLM loop, an MCP wrapper — are the ones missing. That is the right kind of incomplete:
P1 is mostly assembly, not invention. Ship the MCP + `/v1` surface, put a real model
inside the envelope machinery you already trust, fold the `semantic/` designs back
into the implementations, and QueryGraph becomes the thing the agent-framework
ecosystem currently lacks: a governed, auditable, standards-native data plane that any
Pythonic agent can plug into and *prove* what it did.

---

## 9. Implementation status — 2026-07-03

The plan above started landing the day after the review. Everything below is
implemented, tested (37 Python tests — up from 12 — plus 25 Rust tests, all
passing), and verified end-to-end (`uv build` + `twine check` pass; the MCP
server boots on stdio).

### Landed (qg-python 0.3.0-dev)

| Roadmap item | What landed |
|---|---|
| **P0-1 Real Ed25519 signing** (§4.3) | New `querygraph/crypto.py` (extra: `crypto`): keys derived deterministically from agent seeds via SHA-256, mirroring Rust TypeSec `Ed25519DidKey::from_seed`; W3C `did:key` verification methods; `TypeDidEnvelope.verify_signature()` and `LineageAttestation.verify()`; tamper tests. Without the extra, digests are prefixed `unsigned:sha256:` — the quick-win rename — so nothing can mistake them for signatures. The qglake attestation is now really signed by the supervisor. |
| **P1-7 MCP server** (§5.1) | `querygraph/mcp_server.py` (extra: `mcp`; CLI `querygraph mcp-serve --osi model.yaml --rights governance.json --transport stdio\|streamable-http`). Tools: `search_semantic_model`, `resolve_metric` (dialect fallback), `check_access` (RBAC+ODRL dual gate — a denial is a receipt, not an error), `build_navigator_bundle`, `run_qglake_story`, `verify_envelope`; `qg://` resources for the story and loaded model. Reaches Claude, OpenAI Agents SDK, LangChain, PydanticAI, LlamaIndex, CrewAI with zero per-framework code. |
| **§5.3-3 tool-schema export** | `TypeDidAgent.to_tool_schema(flavor="openai"\|"anthropic")` — one exporter covers OpenAI, Anthropic, Mistral, vLLM, Ollama function-calling. |
| **§5.3 async adapters** | `TypeDidLangChainToolAdapter.ainvoke()` / `as_async_tool()`; every adapter result carries the envelope (signature + payload hash), never a bare string. |
| **§4.7 OSI enrichment** | `semantic/claude/osi_loader.py`'s richer model folded into `querygraph/osi.py` as Pydantic: structured `ai_context` (instructions/synonyms/examples, string-coercing), relationships, primary/unique keys, `SUPPORTED_DIALECTS` (+`SAIL_SQL`), `resolve_metric()` with ANSI_SQL fallback, `find_by_synonym()`, and acceptance of upstream OSI documents (`semantic_model` as list). |
| **P0-3 equivalence beyond `navigator`** | `test_rust_equivalence.py` now also runs both `qglake-story` CLIs and asserts governance semantics match: identical specialist roster, RestrictedDataBroker (and only it) denied, COMPLETE OpenLineage events, field-for-field identical attestation schemas, payload hashes on every envelope. |
| **P0-3 CI** | GitHub Actions in both repos: qg-python (uv, Python 3.11+3.13 matrix, pytest, `uv build`, `twine check`); qg-rust (assembles the sibling `grust`/`lakecat` layout, fmt + clippy `-D warnings` + test). |
| **P0-4 packaging** | `py.typed`, authors/urls/keywords/classifiers, `crypto`/`mcp` extras, `langchain-core<2` bound, version 0.3.0.dev0; wheel+sdist build and pass `twine check`. |
| **§4.8 meta-repo README** | `~/src/querygraph/README.md`: component map, sibling-repo layout, quick start, the cross-language contract, release discipline. |

### Landed 2026-07-03, second wave (qg-rust 0.3.0-dev)

| Roadmap item | What landed |
|---|---|
| **Cross-language signature verification** | `qg-rust` `agent::interop`: resolves Python's `did:key` verification methods, reconstructs the documented `querygraph-typedid-signing-v1` payload, verifies with `ed25519-dalek`, and recomputes Python's canonical payload JSON byte-exactly (`sort_keys` + compact separators + `ensure_ascii` escaping). Golden fixture in `cargo test`; live round-trip (Python signs → Rust `verify-envelope` accepts; tampering → exit 1) in the equivalence suite. |
| **P1-6 `/v1` API, first slice** (§4.1) | `server` module (axum) + `querygraph serve --port`: `GET /v1/health`, `POST /v1/navigator/bundle`, `GET /v1/qglake/story`, `POST /v1/audit/verify-envelope` — the platform is network-reachable for the first time, and a Python-signed envelope verifies over HTTP. Invalid signatures return 200 + receipt (findings, not errors). Router tests cover every endpoint. |

All work is committed as logical units and pushed to `querygraph/qg-python`
and `querygraph/qg-rust` (the two CI-workflow commits remain local until the
gh token gains the `workflow` scope).

### Deliberately deferred (unchanged from the roadmap)

- **P1-6 remaining `/v1` surface** (`models/import/*`, `search`, `plan`,
  `answer`, `lineage/events`, TypeDID envelope auth) and the Rust `rmcp` MCP
  crate sharing the axum internals.
- **P1-8 the real navigator LLM loop** (question → Cypher search → RBAC+ODRL →
  Sail SQL → synthesis) — the deterministic story is now positioned as its
  golden eval baseline.
- JSON Schema validation (`mlcroissant`, official OpenLineage schema), Polaris
  `SemanticModel` in LakeCat, `OSIMetricFacet` upstreaming, A2A Agent Cards,
  dbt/Cube importers, ADBC path, docker-compose demo — P1/P2 as planned.
