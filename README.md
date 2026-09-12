# QueryGraph

QueryGraph is the governed semantic layer for enterprise agentic AI. The
canonical repository contains the Rust runtime and the Python API together:

| Path | Boundary | Distribution |
| --- | --- | --- |
| `src/`, `tests/`, `Cargo.toml` | Rust semantic projections, governance, Sail/lakehouse loaders, lineage, agent envelopes, memory and CLI | crates.io `querygraph` |
| `python/querygraph/`, `python/tests/` | Python projection for notebooks, agents, MCP, LangChain, Pydantic AI v2 and PySpark | PyPI `querygraph` |
| `sail/` | Sail checkout and integration assets | upstream Sail |
| `semantic/` | Semantic-model research and fixtures | documentation/research |

The Rust and Python APIs share the same TypeDID, ODRL, Semantic Croissant,
OpenLineage, OSI, and cross-language fixture contracts. Python is an ergonomic
client projection; it is not a second memory or policy authority. TypeSec's
capability-gated `MemoryVault` remains the only protected-memory authority.

[Fihrist](https://github.com/querygraph/fihrist) is the standalone Typesafe
Enterprise Lakehouse Registry:
versioned HR, customer, transaction and custom table contracts, explicit column
types, stewardship metadata, and TypeSec security bindings. Its Rust API checks
compatibility, exports and submits LakeCat create requests, and authorizes
schema-checked Sail scan planning. Start with `querygraph fihrist --help`;
agents can check drafts through the `validate_fihrist` MCP tool. Both use the
standalone Fihrist crate; see [integration touchpoints](docs/frqg.md).

## Quick start

```bash
# Rust runtime and CLI
cargo test
cargo run -- qglake-story --json

# Python API and CLI
cd python
uv sync --extra test --extra crypto --extra agents --extra mcp
uv run pytest
uv run querygraph qglake-story --pretty
```

The Python package keeps its public import and command names:

```python
from querygraph import AiNavigator, TypeDidAgent
```

The full migration contract, dependency matrix, release gates, and status are
maintained in [`QGQG.md`](QGQG.md). The architecture review remains in
[`FABLE-REVIEW-1.md`](FABLE-REVIEW-1.md), and the stack guide is in
[`docs/guide`](docs/guide).

## Stack boundaries

QueryGraph consumes versioned TypeSec, Grust, Marciana, LakeCat, and Fihrist
crates and an explicit Sail upstream revision. Those projects never depend on QueryGraph.
Cognee and Fluree are comparative benchmark references only; neither is a
runtime dependency.

```text
Sail lakehouse <- Grust graph <- QueryGraph Rust <- Python API <- agents
       ^              ^              ^              ^
       |              |              |              |
  LakeCat proofs  Marciana memory  TypeSec law  Pydantic/MCP clients
```

## Release discipline

Rust and Python release metadata lives beside each distribution. A release
must pass formatting, strict lint, complete tests, clean package builds, the
cross-language TypeDID fixtures, and the live Sail gate applicable to the
change. See [`RELEASES.md`](RELEASES.md) and [`python/RELEASES.md`](python/RELEASES.md).
