# QueryGraph Python

Python ecosystem for the QueryGraph AI Navigator.

It is the Python API shipped beside the Rust implementation in the canonical
`querygraph/querygraph` repository:

- Croissant JSON-LD dataset metadata
- CDIF discovery/access/profile projection
- deterministic `did:oyd` identity documents
- ODRL permissions and prohibitions
- OSI semantic models over Croissant fields and Sail columns, with structured
  `ai_context` (instructions/synonyms/examples), relationships, and
  dialect-fallback metric resolution
- TypeDID agents modeled with Pydantic, signed with real Ed25519 keys
  (`crypto` extra) under `did:key` verification methods
- an MCP server exposing the governed layer to any agent framework
  (`mcp` extra, `querygraph mcp-serve`)
- vendor-neutral tool-schema export (`TypeDidAgent.to_tool_schema()`) for
  OpenAI/Anthropic-style function calling
- optional LangChain adapters (sync + async) for governed agent tools
- native Pydantic AI v2 capabilities for TypeDID credentials and persistent
  TypeSec/Grust memory
- OpenLineage events and Ed25519-signed lineage attestations
- PySpark helpers for querying a local Sail warehouse
- a CLI compatible with the Rust semantic bundle commands

The design goal is Python-native ergonomics over the same governed lakehouse:
Rust loads and verifies the warehouse; Python gives notebooks, PySpark users,
LangChain agents, and data scientists a typed interop layer.

## Stack versions

This API tracks the same coordinated QueryGraph stack releases as the root
Rust crate:

- **Grust 0.12.0 "Lobster"** — the property-graph + GQL/Cypher substrate, with
  the merged Full39075 profile (CALL subqueries, table-valued functions,
  shortestPath, passthrough) and atomic Cypher transaction batches.
- **TypeSec 0.12.0 "Torcello"** — the typed security fabric, grown into an
  agent-interoperability platform (framework guards, MCP gate, enforcement
  proxy, signed decision receipts); the Pydantic `TypeDidEnvelope` mirrors its
  audit-safe attestation (action, resource, privacy level, negotiated profile,
  and an envelope digest).
- **LakeCat 0.3.0 "Ocelot"** — the thin Iceberg REST catalog boundary with
  stock-client Iceberg REST conformance, sharing its bootstrap-bundle wire
  format with the importer via `qglake-bundle`.

See `../docs/blog/announcing-querygraph-stack.md` for the full story.

## Install

Core metadata and TypeDID/Pydantic support:

```bash
uv sync
```

Optional PySpark/Sail support:

```bash
uv sync --extra lakehouse
```

Optional LangChain tool adapters:

```bash
uv sync --extra agents
```

Pydantic AI v2 capabilities (provider-free slim runtime):

```bash
uv sync --extra pydantic-ai --extra crypto
```

Real Ed25519 signing for envelopes and attestations:

```bash
uv sync --extra crypto
```

The MCP server:

```bash
uv sync --extra mcp
```

Everything:

```bash
uv sync --extra all
```

## Build a Semantic Bundle

```bash
python -m querygraph navigator \
  --dataset-name "Hazard vocabulary" \
  --description "Controlled vocabulary with multilingual technical terms" \
  --landing-page "https://querygraph.ai/datasets/hazards" \
  --data-url "https://querygraph.ai/datasets/hazards.csv"
```

## QG Lakehouse Agent Story

```bash
python -m querygraph qglake-story --pretty
```

This produces a Pydantic TypeDID multi-agent run: supervisor, finance, energy,
mobility, climate-health, reference, restricted-data broker, synthesis,
OpenLineage, and DID attestation.

## Query Sail with PySpark

Start Sail from the Rust project after the lakehouse has been loaded:

```bash
cd ..
sail spark server --port 50051
```

In another shell:

```bash
uv sync --extra lakehouse
uv run querygraph lakehouse-register \
  --manifest ../.querygraph/lakehouse/manifest/load-report.json \
  --warehouse ../spark-warehouse
uv run querygraph audit-register --warehouse ../spark-warehouse
uv run querygraph pyspark-examples
```

Open a shell:

```bash
uv run pyspark --remote sc://127.0.0.1:50051
```

Then query the registered views:

```python
spark.sql("SELECT COUNT(*) FROM global_temp.government_finance__countydata").show()
spark.sql("SELECT quantity, value, unit FROM global_temp.codata_constants_2022__codata_constants_2022 LIMIT 5").show(truncate=False)
spark.sql("SELECT event_hash, event_type, job_name FROM global_temp.openlineage_events LIMIT 10").show(truncate=False)
```

## OSI with Semantic Croissant

```bash
uv run python examples/osi_semantic_croissant.py
```

The example starts with concrete Semantic Croissant fields and projects them
into an OSI semantic model with ontology terms and Sail SQL expressions.

## TypeDID Agents with LangChain

```bash
uv sync --extra agents
uv run python examples/typedid_langchain_agents.py
```

The agents are Pydantic models first. When LangChain is installed, a
`TypeDidLangChainToolAdapter` exposes the same governed agent as a LangChain
`StructuredTool` — `as_tool()` for sync runtimes, `as_async_tool()` for async
ones. For every other framework, `TypeDidAgent.to_tool_schema()` emits a
standard JSON-Schema tool definition:

```python
from querygraph import TypeDidAgent

finance = TypeDidAgent.new("FinanceAgent")
finance.to_tool_schema()                     # OpenAI function-calling shape
finance.to_tool_schema(flavor="anthropic")   # Anthropic tool-use shape
```

## Pydantic AI v2: Credentials That Remember

The end-to-end demo gives every agent two native Pydantic AI v2
`Capability` objects:

- `querygraph.typedid-credential` keeps the private signing seed in typed
  runtime dependencies and exposes only signed access plus the public DID;
- `querygraph.marciana-memory` calls the root service's capability-gated,
  Turso-backed
  remember/recall/forget API.

It uses `TestModel`, so no OpenAI, Anthropic, or Google key is required. The
script builds and starts the root QueryGraph service, creates policy for a specialist and
supervisor, stores a governed answer, restarts the Rust service, recalls the
answer under the supervisor's distinct credential, and shows an outsider's
validly signed request being denied.

```bash
uv sync --extra test --extra crypto --extra pydantic-ai
uv run python examples/pydantic_ai_v2_memory_agents.py
```

The policy contains only public `did:key` values. Seeds never enter prompts,
tool arguments, policy files, or serialized agent models.

## Signed Envelopes (Ed25519)

With the `crypto` extra installed, agent envelopes and lineage attestations
are signed with real Ed25519 keys derived deterministically from agent seeds
(the same `from_seed` pattern as Rust TypeSec), and carry `did:key`
verification methods:

```python
from querygraph import TypeDidAgent

supervisor = TypeDidAgent.new("SupervisorAgent")
finance = TypeDidAgent.new("FinanceAgent")
request = supervisor.request(
    finance, action="summarize", resource="compartment:finance",
    payload={"question": "Where is fiscal stress highest?"},
)
assert request.verify_signature()      # verifies against verification_method
```

Without the extra, digests are prefixed `unsigned:sha256:` so they can never
be mistaken for signatures.

## MCP Server

Expose the whole governed layer to any MCP client — Claude Code/Desktop,
OpenAI Agents SDK, LangChain (`langchain-mcp-adapters`), PydanticAI,
LlamaIndex, CrewAI — with one command:

```bash
uv sync --extra mcp
uv run querygraph mcp-serve --osi path/to/semantic-model.yaml
```

Tools: `search_semantic_model`, `resolve_metric` (dialect fallback),
`check_access` (RBAC+ODRL dual gate — denials are receipts, not errors),
`build_navigator_bundle`, `run_qglake_story`, and `verify_envelope`. Pass
`--rights governance.json` to supply your own RBAC+ODRL policy and
`--transport streamable-http` for a network transport.

## Test

```bash
uv run python -m pytest
```

The test suite includes equivalence checks against the sibling Rust implementation.

## Releases

This is **QueryGraph Python 0.4.0 "Sentinel"**. See [`CHANGELOG.md`](CHANGELOG.md)
for per-release notes and [`RELEASES.md`](RELEASES.md) for the version/codename
log (shared with the root QueryGraph service).
