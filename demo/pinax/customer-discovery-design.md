# Customer discovery demonstration

Part I uses Sail, LakeCat, Pinax and MCP to answer “What are customers called where?” Part II retains graph and specialist operations behind an explicit collapsed section.

The synthetic inventory adds CRM customers, billing accounts and product users. Concepts and bindings are reviewed independently. Customer is intentionally ambiguous across three table meanings. Only the master customer identifier is shared; billing account IDs and product user IDs remain distinct. Definitions record population and cardinality. Sources and stewards identify where the interpretation came from. No alias establishes a join.

The customer-discovery purpose exposes the three business tables; the existing analytics-purpose fixture remains available for regression checks. Every business table has real Iceberg metadata and Parquet rows loaded into the authenticated LakeCat owner. MCP responses, rather than browser constants, supply the answer's table names, fields, definitions, concept identities, and digests. The client follows registry pagination and refuses truncated semantic pages. It never chooses the first ambiguous match or executes a graph workflow.

Authoring changes stay in a new retained fixture directory. The old fixture and ontology are preserved. Activation pins the new registry and ontology and requires consumer restart; no live ontology HEAD is silently replaced. Request paths, signed intent validation, authorization and existing bounded process execution remain unchanged.

Validation: customer/account/user discovery, shared identifier bindings, distinct user/account identifiers, inaccessible-purpose non-disclosure, live catalog activation, all Part I browser actions, Part II hidden on load, desktop/mobile layout and slide overflow.
