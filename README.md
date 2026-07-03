# QueryGraph workspace

A governed semantic layer for enterprise agentic AI: four semantic projections
(Semantic Croissant, CDIF, W3C DID, ODRL) plus OSI business semantics, RBAC+ODRL
dual policy gating, TypeDID signed agent envelopes, and OpenLineage audit with
Ed25519-anchored attestations — over a Sail (Spark-compatible) lakehouse with a
Grust property graph and a Cypher extension compiled into Sail itself.

See `FABLE-REVIEW-1.md` for the full architecture review, findings, and roadmap.

## Components (this directory)

| Directory | What it is | Language |
|---|---|---|
| `qg-rust/` | Reference implementation: semantic projections, governance, lakehouse loaders, lineage, agent envelopes, CLI | Rust |
| `qg-python/` | Pythonic mirror (Pydantic v2): same layers plus LangChain adapter, tool-schema export, MCP server, PySpark/Sail helpers | Python |
| `sail/` | Fork of [lakehq/sail](https://github.com/lakehq/sail), branch `grust`, adding a Cypher graph query extension | Rust |
| `semantic/` | Research repos (`claude/`, `codex/`): Polaris `SemanticModel` architecture, OSI↔OpenLineage↔Navigator bundle round-trips, OPA/Rego policies | Python/docs |

## Sibling repositories (required for qg-rust builds)

`qg-rust` uses path dependencies that expect this exact layout:

```
~/src/
├── querygraph/          # this directory
│   ├── qg-rust/
│   ├── qg-python/
│   ├── sail/
│   └── semantic/
├── grust/               # github.com/querygraph/grust   (property graph + Cypher)
├── lakecat/             # github.com/querygraph/lakecat (catalog bootstrap bundles)
└── typesec/             # optional; typesec-* crates come from crates.io
```

## Quick start

```bash
# Rust reference CLI (needs ../grust and ../lakecat checkouts)
cd qg-rust && cargo test && cargo run -- qglake-story --json

# Python mirror
cd qg-python && uv sync --extra test --extra crypto --extra agents --extra mcp
uv run pytest                       # includes the Rust↔Python equivalence suite
uv run querygraph qglake-story --pretty

# Serve the governed semantic layer over MCP (Claude, LangChain, PydanticAI, …)
uv run querygraph mcp-serve --osi path/to/model.yaml
```

## Cross-language contract

`qg-python/tests/test_rust_equivalence.py` runs both CLIs and asserts:

- `navigator` bundles are byte-identical modulo timestamps (Croissant, CDIF,
  DID, ODRL layers, deterministic `did:oyd` identities);
- `qglake-story` governance semantics match: same specialist roster, the
  restricted broker (and only it) denied, complete OpenLineage + Ed25519
  attested evidence chain on both sides.

## Release discipline

Stack releases are coordinated by codename across `grust`, `typesec`,
`lakecat`, and `qg-*` (see each repo's `RELEASES.md`). Current: `qg-rust` /
`qg-python` 0.3.0 "Goshawk" — the interoperability release (MCP, A2A,
cross-language Ed25519, `/v1` API, official OpenLineage conformance).
