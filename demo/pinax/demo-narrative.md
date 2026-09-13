# Part I — What are customers called where?

A 10–15 minute catalog discovery demo. The four main console steps exercise
Sail, LakeCat, Pinax, and MCP. The default slides contain only Part I.
Graph storage and specialist workflows are a separately opened Part II.

## Open with the business question

“Sales calls them customers. Finance calls them accounts. Product calls them
users. Across the company, where do those meanings live, and which fields
represent them?”

The useful answer is a set of locations with definitions, not a list of
synonyms. A billing account can belong to a customer organization. A product
user can belong to that organization, or be a free user with no customer
reference. Calling all three “customer” does not make their populations or
identifiers interchangeable.

Open the live console through the existing tunnel:

```bash
ssh -N -L 18081:127.0.0.1:18081 grust
# http://localhost:18081
```

## 1. Inspect the company inventory

Click **Inspect lakehouse & catalog**. Three departments have real Iceberg
metadata and Parquet records in the synthetic `acme` namespace:

| Department | Table | Fields | What one row means |
| --- | --- | --- | --- |
| Sales | `crm_customers` | `customer_id` | One customer organization |
| Finance | `billing_accounts` | `account_id`, `customer_ref` | One billing relationship |
| Product | `product_users` | `user_id`, `customer_ref` | One person with a login |

The fixture contains two organizations, two billing accounts for organization
101, and two users, one of whom has no customer reference. These records make
one-to-many and missing relationships concrete. Part I discovers metadata; it
does not execute a join or count these populations.

Sail supplies lakehouse execution. LakeCat catalogs physical table state.
The operator schema display reads retained Iceberg metadata; the fresh service
activation checks registered contracts against live LakeCat. Do not call a
seed display a remote catalog enumeration. The next step is the agent's
actual authorized discovery response.

## 2. Discover tables and fields

Click **Discover company tables**. Pinax returns the three registered tables
for the `customer_discovery` purpose, with field names, stable field IDs,
types, descriptions, owners and stewards. TypeSec still authenticates the
agent and filters discovery; it is supporting infrastructure, not an extra
main-stage workflow.

Explain that departments retain their names. A company does not have to
rename every account or user table to customers before agents can find it.
The registry provides physical contracts; the ontology provides reviewed meaning.

## 3. Ask what customers are called where

Click **Find customer meanings**. The search for `customer` intentionally
returns three different table concepts:

- **CRM customer**: an organization with a commercial relationship, excluding prospects.
- **Billing account**: a billing relationship; several may belong to one organization.
- **Product user**: a person with a login, potentially a free user.

All three accept the business alias `customer`. Each has its own stable concept
ID, definition, steward, source reference, and reviewed table binding. The
ambiguity is recorded, not erased. `account` finds the Finance meaning, and
`user` finds the Product meaning.

Then show field-level meaning. The same **Enterprise customer identifier**
concept binds these physical fields:

```text
crm_customers.customer_id       field ID 1
billing_accounts.customer_ref   field ID 2
product_users.customer_ref      field ID 2
```

`billing_accounts.account_id` and `product_users.user_id` each have a separate
identifier concept. The repeated field ID 2 is meaningful only together with
its table and revision; it is not a globally unique column identity.

This is how Pinax answers the question: business wording selects candidate
concepts; reviewed bindings identify the tables and stable fields; registry
metadata supplies the actual column names. Definitions explain when two names
mean different things. A shared concept does not, by itself, prove key
uniqueness, foreign-key integrity, completeness, or permission to join tables.

### How the organization records this in Pinax

The concrete reviewed synthetic vocabulary is
[`customer-meaning.json`](customer-meaning.json). Its `meanings` section is an
operator fixture input, not a new Pinax wire format. The publisher materializes
standard `pinax.ontology.v1` concepts and bindings using the registry's exact
digest. The source tables remain ordinary `pinax.v1` contracts.

1. Inventory tables in the catalog and register their schemas, field IDs,
   owners, and stewards in Pinax.
2. Bootstrap or import draft ontology concepts and bindings.
3. Have domain stewards define the populations, identifiers, aliases, and
   distinctions. Reuse a concept ID only when meaning truly agrees.
4. Review both the concepts and each table/field binding. Record source and
   review evidence. Publish the reviewed digest.
5. Update the consumer's registry and ontology pins and restart it at the
   reviewed revision. Reconcile schema changes before publishing successors.

Pinax's aliases belong to concepts. They are not a global replacement rule
saying `customer = account = user`. Department scope comes from distinct
concept definitions, stewardship, and the physical binding. Broader and related
references can express additional vocabulary structure, but `related` does
not assert equivalence. This example needs no graph workflow.

## 4. Let the agent ask through MCP

Click **Ask: what are customers called where?** The Rust protocol client:

1. Discovers the MCP server and tools.
2. Calls `discover_pinax_ontology` for `customer`, `account`, `user`, and
   `enterprise customer id`, using a signed intent and `customer_discovery` purpose.
3. Follows registry cursors, including pages with no matches; rejects truncated
   semantic result pages rather than silently presenting an incomplete answer.
4. Reads definitions and bindings from the responses and resolves field IDs
   against the authorized exported schemas.
5. Displays department terminology, table and field names, stewards, and the
   registry/ontology digests that support the answer.

An intent inside a signed MCP tool call looks like:

```json
{"purpose":"customer_discovery","query":"customer","limit":100,"cursor":null}
```

The tool arguments are the exact serialized `intent` and its TypeSec `envelope`.
The client never accepts a publisher path or ontology replacement from an agent.

Expected business answer:

“Sales calls customer organizations **customers**, in `crm_customers`.
Finance calls billing relationships **accounts**, in `billing_accounts`.
Product calls people with logins **users**, in `product_users`.
The customer reference fields share the enterprise customer identifier;
account IDs and user IDs have different meanings. Do you need organizations,
billing relationships, or people?”

The displayed locations and definitions come from the live responses. This is
an executable MCP client, not a fresh language-model conversation. It performs
metadata discovery and does not run graph algorithms, workflows, or joins.

```bash
DEMO_ROOT=/home/admin/querygraph-pinax-demo-20260913
"$DEMO_ROOT/src/querygraph/target/debug/examples/pinax-mcp-client" \
  --url http://127.0.0.1:18082/mcp --scenario customer-discovery
```

## What “across the company” means

The answer covers the authorized **registered** inventory, not unregistered or
inaccessible systems. Domain stewardship and explicit ingestion are required
at enterprise scale. Cursors make inventory pagination visible; ontology
and registry digests pin each interpretation. This small fixture demonstrates
the process and does not claim enterprise-scale performance or automatic
semantic inference. If two definitions still conflict, the agent presents both
and asks for the intended population instead of choosing its first result.

## Optional authoring, reads, and access checks

These are collapsed outside the four-step introduction. Shared profile generation
previews reusable templates. The original `rows` table, analytics purpose, and
its governed read `[{"id":2}]` remain as a separate regression fixture. They
must not be presented as a read from the three department tables. The original
MCP read client remains available with `--scenario governed-read`.

# Part II — Graphs and specialist workflows

Open **Part II · Graphs and specialist workflows** explicitly. Its component
buttons and operations remain hidden on initial load. The presentation's
**Open Part II** link enables the appendix; the default arrow-key route ends
with the catalog discovery demo.

The retained semantic workflow writes 34 nodes and 33 edges for two descriptive
Dataverse fixtures through Sail, reads a dataset node back, and appends lineage.
The Cypher summary and fixture answer are computed locally. Navigator produces
semantic metadata; the Resilience Desk is deterministic specialist orchestration.
These are separate datasets and capabilities, not prerequisites for the customer
naming answer. External model inference is not enabled.

## Operating notes

The five existing services and SSH tunnel remain the entrypoints. Prepared
customer inventory lives in a new retained directory; preserve the original
fixture and ontology on cutover. The owner remains an in-memory catalog restored
from seed metadata on restart. This is a correctness demo, not a durability claim.

See [customer-discovery-design.md](customer-discovery-design.md) for the change
contract and [README.md](README.md) for build, lifecycle, and source packaging.
