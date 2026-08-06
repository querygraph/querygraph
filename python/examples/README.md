# QueryGraph Python Examples

These examples turn the Python package into the notebook-facing ecosystem for
the Rust lakehouse.

## Sail and PySpark

```bash
sail spark server --port 50051
querygraph lakehouse-register \
  --manifest ../.querygraph/lakehouse/manifest/load-report.json \
  --warehouse ../spark-warehouse
querygraph audit-register --warehouse ../spark-warehouse
python examples/pyspark_query_sail.py
```

Useful shell entry:

```bash
pyspark --remote sc://127.0.0.1:50051
```

Then:

```python
spark.sql("SELECT COUNT(*) FROM global_temp.government_finance__countydata").show()
spark.sql("SELECT quantity, value, unit FROM global_temp.codata_constants_2022__codata_constants_2022 LIMIT 5").show(truncate=False)
spark.sql("SELECT event_hash, event_type, job_name FROM global_temp.openlineage_events LIMIT 10").show(truncate=False)
```

## OSI and Semantic Croissant

```bash
python examples/osi_semantic_croissant.py
```

The example starts with concrete Semantic Croissant fields and projects them
into an OSI model with business terms, Sail expressions, and ontology terms.

## TypeDID, Pydantic, and LangChain

```bash
python examples/typedid_langchain_agents.py
```

For the LangChain adapter path:

```bash
uv sync --extra agents
python examples/typedid_langchain_agents.py
```

## Pydantic AI v2 Credentials + Persistent Memory

```bash
uv sync --extra crypto --extra pydantic-ai
uv run python examples/pydantic_ai_v2_memory_agents.py
```

This provider-free multi-agent demo composes a TypeDID credential capability
with a Marciana memory capability. It boots qg-rust with generated exact-DID
RBAC policy and Turso storage, restarts it between specialist write and
supervisor recall, and records the outsider denial receipt.
