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
