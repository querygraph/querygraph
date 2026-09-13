# The QueryGraph stack: presenter narrative

A 15–20 minute introduction for an audience seeing the stack for the first time.
The companion [14-slide deck](dist/querygraph-pinax-slides.pdf) and live console
follow the same story. No prior Pinax version or migration demo is assumed.

## The opening

“A large company already has data. Different teams collect it in different
systems, name similar fields differently, and use business terms differently.
We want an agent to find the appropriate tables, understand their shape and
meaning, and operate on them with the right access.”

“We have a lakehouse in Sail, a physical catalog in LakeCat, and an enterprise
registry in Pinax. TypeSec gives agents identity and access. MCP provides their
tool interface. Marciana contributes semantic and cognition capabilities, and
Grust connects datasets and their meanings in a graph. QueryGraph composes the
whole stack.”

Use slides 1–2. Do not open with denied requests, drift, or revision cutover.
The central question is how the company makes data understandable and usable.

## Open the console

Keep the laptop tunnel running:

```bash
ssh -N -L 18081:127.0.0.1:18081 grust
```

Open http://localhost:18081. The presentation is at
http://localhost:18081/slides. Component names across the top are buttons:
clicking one explains its role on the left and selects its related step on the
right. The explanation includes a source/documentation link.

Results remain on the left. The walkthrough scrolls independently on the right.
Clicking an action aligns its card with the result pane. During execution, the
result header shows elapsed time. Each completed response has a readable
summary, followed by expandable complete JSON. Long results scroll inside the
left pane, independently of the controls. On a narrow screen the two scrollable
panes stack vertically, with the result above the controls.

## 1. A lakehouse with a schema — slides 3–4

Click **Inspect lakehouse & catalog**.

“We begin with a real physical table. Sail executes reads over Iceberg metadata
and Parquet files. LakeCat catalogs the table and the snapshot used by a read.”

The small customer fixture is physically named `rows`. It has three fields:

| Stable field ID | Column | Iceberg type | Required |
| --- | --- | --- | --- |
| 1 | id | long | yes |
| 2 | tenant | string | no |
| 3 | private_email | string | no |

The console displays the physical schema from the retained synthetic seed and
runs a fresh signed plan against the live catalog. The plan verifies agreement
between the registered contract and current catalog state. The seed display is
an operator view of this public synthetic fixture; it is not the agent's
filtered discovery response. Do not present a seed read as a remote catalog
listing. The subsequent plan and row operations supply the live evidence.

“The physical catalog tells us what table and snapshot exist. The registry
tells the organization what shape and meaning it has agreed to use.”

## 2. Standard table shapes — slide 5

Click **Generate enterprise standards**.

“A large company should not need to rediscover the meaning and representation
of customer, employee, and transaction records in every team. Pinax provides
shared typed profiles with stable field identities, semantic terms, owners,
and stewards. Teams can adopt, review, and maintain these definitions.”

This runs the actual authoring command:

```bash
querygraph pinax init --enterprise acme \
  --owner platform --steward data --policy enterprise
```

The output contains `employees`, `customers`, and `transactions` profiles. Show
the returned columns, types, and semantic keys. These are generated templates,
not newly registered or populated physical tables. The live customer fixture
remains the small `rows` table throughout the walkthrough.

Explain the practical adoption process: inventory existing schemas, choose
shared profiles and definitions, map existing fields, validate the contracts,
and reconcile the catalog before deploying consumers. An agent can help propose
mappings; data stewards supply and review business meaning. Pinax validates
representable shapes and compatibility. It does not infer currencies, units,
or business keys from an ambiguous column name.

## 3. Find the registered tables — slide 7

Click **Discover tables**.

“The analytics agent now approaches the registry with an authenticated TypeSec
identity and a purpose. Pinax returns the tables and fields it can use.”

The response identifies enterprise `acme`, the `rows` table, its customer
identifier description, owner `platform`, steward `data`, and field `id` with
stable field ID 1 and type `int64`. Its registry digest identifies the exact
inventory consulted. This is the agent's usable view, unlike the operator
schema display in step 1.

Keep the emphasis on successful access: TypeSec lets identified agents work
with the appropriate data. Optional negative checks are available at the end.

## 4. Find data by meaning — slide 6

Click **Consult shared ontology**.

“An agent should be able to begin with the organization's vocabulary. Pinax's
central ontology binds a reviewed concept to the physical field.”

Show **Customer identifier**, its definition, and aliases **account number**
and **customer id**. The binding identifies `rows`, field ID 1. The result
contains the ontology digest and registry digest. The next MCP step uses the
alias `account number` and derives the operation from this returned binding.

A schema describes representation. An ontology explains meaning: preferred
labels, definitions, aliases, and broader/related concepts. Bindings connect
that meaning to tables and fields. New imports remain drafts until their
concepts and bindings are reviewed. Coverage refers to the complete registered
inventory; remote tables must first be inventoried and registered.

## 5. Let an agent use MCP — slide 8

Click **Discover & read through MCP**.

This runs `pinax-mcp-client` against the deployed HTTP MCP service. It discovers
the server and its tools, signs a semantic search, resolves `account number`,
derives the table and column from the reviewed result, and calls
`execute_pinax_scan`. The read travels through LakeCat and Sail. The left pane
shows the match and actual returned rows.

```bash
DEMO_ROOT=/home/admin/querygraph-pinax-demo-20260913
"$DEMO_ROOT/src/querygraph/target/debug/examples/pinax-mcp-client" \
  --url http://127.0.0.1:18082/mcp
```

This is an executable agent-protocol client using a public fixture identity.
It does not invoke an external language model. Its complete JSON includes
requests, responses, additional access assertions, and signature checks. Lead
with the successful discovery-to-read path rather than those assertions.

## 6–7. Inspect the plan and read — slide 9

Click **Inspect scan plan**, then **Read selected rows**.

The signed client consults the central ontology, resolves the selected field,
and constructs this bounded operation:

```json
{"table":"rows","columns":["id"],"purpose":"analytics","limit":10}
```

The plan displays the table, selected columns, purpose, limit, and snapshot.
The read returns:

```json
[{"id":2}]
```

The seed contains two records belonging to different synthetic tenants. This
agent's context selects the Acme identifier. The returned projection carries
execution evidence and semantic exports. The result is a physical read from
Parquet, not a prefilled browser response. The direct actions show individual
stages; step 5 shows the complete protocol journey over MCP.

## 8. Connect the data as a graph — slide 10

Click **Build graph & run workflow**.

“An organization also needs the relationships between datasets, files, fields,
and concepts. Grust represents those relationships as a property graph.”

The live workflow describes two Dataverse fixtures: Bay Area building energy
observations and Enterprise data access survey. It writes **34 nodes and 33
edges** through Sail Spark Connect, reads a dataset node back, and appends
OpenLineage to `qg_audit.openlineage_events`. The result shows the loaded views
and a readable fixture answer.

These descriptive datasets are separate from the retained customer-row fixture.
The graph's Cypher summary and agent answer are computed locally; graph storage,
node readback, and lineage append use live Sail. Optional external model
inference is not enabled. Each invocation gets an isolated report and warehouse.

## 9–10. Describe data and coordinate agents — slide 11

Click **Build semantic bundle**.

“Marciana's semantic and cognition capabilities are composed into QueryGraph's
workflows. Navigator packages a dataset description for other agents: Croissant
fields, CDIF data elements, vocabulary references, identity, and usage terms.”

Show the bundle's layer names and expand JSON if the audience wants the
structure. The hazard vocabulary and its URLs are descriptive fixture metadata;
this step does not download a remote dataset. It consults the central ontology
before generating the bundle.

Click **Run specialist workflow**.

The Resilience Desk supervisor delegates to finance, energy, mobility,
climate-health, reference, and restricted-data specialists. The synthesis agent
combines their signed summaries into a briefing. TypeSec identities and
operation scopes accompany the delegation; lineage connects the outputs to the
workflow. Show the briefing and specialist inputs first.

This is a deterministic orchestration fixture. Do not claim it runs a fresh
language-model conversation or scans every named source. The real table read
is demonstrated in steps 5–7. The central ontology consultation is a preflight
for these workflows; it does not replace their domain-specific vocabulary.

## Semantic interoperability — slide 12

The shared ontology projects into Croissant descriptions, CDIF-oriented data
elements, and a SKOS vocabulary. They refer to the same reviewed concepts.
Discovery descriptions are explicitly metadata-only. Successful bounded reads
include loadable Croissant inline records, validated separately with the
MLCommons reference loader. CDIF support is the implemented projection, not a
claim to implement every CDIF profile.

## Close — slides 13–14

“We started with company data. Sail executes it, LakeCat catalogs it, Pinax
standardizes its shape and meaning, and TypeSec enables appropriate agent
access. MCP makes the operations available; Marciana and Grust support the
workflows and relationships around them. Pinax is where an agent begins.”

The [book](https://firstpair.org/books/pinax/) and
[announcement](https://querygraph.ai/announcing-pinax/) provide the longer
explanation. The [ontology guide](../../docs/central-ontology.md) documents
construction and maintenance.

## Optional checks and operator notes

The collapsed **Optional access checks** section contains field and purpose
denials. These remain useful engineering checks, but are outside the main
introduction. Mutation and registry-cutover scenarios remain in the separate
acceptance suite; they are not audience prerequisites or slide topics.

Five services support the retained deployment:

| Service suffix (`querygraph-pinax-…`) | Port | Role |
| --- | --- | --- |
| console | 18081 | Walkthrough and slides |
| mcp | 18082 | Agent MCP endpoint |
| owner | 18181 | LakeCat catalog and table operations |
| sail | 15051 | Spark Connect |
| api | 18080 | QueryGraph HTTP API |

Services are enabled at boot. The owner's catalog is an in-memory fixture and
returns to its seed on restart. Destination-generated credentials stay outside
the source archive. All Rust uses edition 2024. Sail runs from our selected
source checkout; upstream merge is not required.

Repeat live integration checks:

```bash
ssh grust 'bash /home/admin/querygraph-pinax-demo-20260913/src/querygraph/demo/pinax/run-live.sh /home/admin/querygraph-pinax-demo-20260913'
```

Verify every console action and the desktop/mobile layout through the tunnel:

```bash
node demo/pinax/verify-console.mjs /tmp/querygraph-stack-console-check
```

The prepared archive can deploy a new isolated instance using its
`START-HERE.txt`. This demonstration establishes working integration for small
fixtures; it makes no throughput or production-durability claim.
