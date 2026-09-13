# Synthetic fixture ontology review

The demo-maintained central ontology describes the existing acme/rows fixture.
The publisher example refuses any other table or field identity. Its reviewed
concepts are Customer records, Customer identifier, Tenant identifier and Private
email address. Customer identifier aliases include account number and customer
id. An email address is not an identifier alias. All four mappings are explicit.

The agent is allowed to discover/read id only. Tenant and email concepts remain
in the operator's ontology but must not appear in agent search or standards
exports. Tenant filtering is still enforced by the owner. The plan/read client
must discover Customer identifier centrally, resolve its stable field binding,
then sign a fresh plan/read intent using the resolved table and column.

This review applies only to synthetic demonstration data. Real imports start
with draft concepts AND draft bindings; stewards review them separately.

## Customer discovery extension

The reviewed extension is exactly `customer-meaning.json`: `crm_customers`,
`billing_accounts`, and `product_users`, with five fields in total. The publisher
compares the complete contracts to that file before approving them. The existing
`rows` schema and its analytics-purpose restrictions remain unchanged.

The table concepts CRM customer, Billing account, and Product user each carry
`customer` as an intentionally ambiguous alias. Their definitions and department
stewards distinguish an organization, a billing relationship, and a person with
a login. Account and user are not declared equivalent to the organization.

Field bindings to Enterprise customer identifier are approved for
`crm_customers.customer_id` (1), `billing_accounts.customer_ref` (2), and
`product_users.customer_ref` (2). Billing account identifier and Product user
identifier are distinct concepts bound to their respective field ID 1. The
synthetic fixture deliberately shares master customer IDs across the reference
fields; this review does not assert universal cross-system key equivalence.

The new purpose `customer_discovery` exposes only the three business tables.
The original analytics fixture remains separate. These are reviewed synthetic
mappings, not automatic approvals for imported production schemas. See
`customer-discovery-design.md` for the semantics and validation contract.
