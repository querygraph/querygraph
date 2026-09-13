# Central ontology integration

The authoritative model, validation, lifecycle, schema import and exports live
in Pinax's `ontology` module. QueryGraph owns the signed agent boundary and
host configuration. See the sibling Pinax `docs/ontology.md` for operator
commands and data shapes.

Add this host-controlled section to registry-service.json:

```json
{
  "ontology": {
    "store": "../ontology",
    "expected_digest": "sha256:<reviewed ontology digest>"
  }
}
```

Relative paths resolve beside that configuration. Bootstrap validates the store
and registry pin. Each ontology request rereads the central publication; changing
HEAD without updating the consumer's reviewed digest fails closed. After an
operator-reviewed publication, update consumer pins and restart consumers during
the maintenance window. This is an explicit deployment lifecycle, not silent
in-process adoption of changed business meaning.

## Signed MCP discovery

`discover_pinax_ontology` uses the same exact-byte signing rules as existing
Pinax tools, with resource `/mcp/discover_pinax_ontology`:

```json
{"purpose":"analytics","query":"account number","limit":100}
```

Tool arguments are `intent` (the exact JSON string) and its signed `envelope`.
No store path, ontology document, publisher identity or policy override is
accepted. The response has `search`, `exports` and `next_cursor`. Both direct
Rust MCP and Python's Rust bridge use the shared tool contract. Authoring stays
in the operator-only Pinax CLI.

The original substring search over imported OSI models remains available for
legacy local models. The governed central ontology is a separate explicit tool;
legacy search must not be presented as authorization or as central consultation.
OSI import recognizes the new `qg:concept` reference and still accepts legacy
field `sameAs` input.

## Demo agent behavior

`pinax-client ontology --query 'account number'` signs a central consultation.
Plan and execute first resolve Customer identifier to exactly one approved
field binding, retrieve that stable field's authorized column name, then sign
a fresh bounded plan/read intent. Missing and ambiguous resolution fails.
The returned `ontology_consultation` records the evidence used to construct the
request. The synthetic denied-column and denied-purpose actions still submit
explicit forbidden requests to verify enforcement.

The browser has a central ontology action. Original Navigator, QGLake and semantic
workflow actions perform and report a central consultation before their existing
workflow. Their original fixture-specific semantic narratives are preserved;
that preflight is not a claim that their older signatures bind the new ontology
receipt. The new governed plan/read flow actually uses the resolved binding.

The fixture ontology inventories one table and all three physical fields; only
id is discoverable by the demo identity. The private email and tenant concepts
are present centrally but absent from agent search and exports. The reviewed
fixture is documented in `demo/pinax/ontology-review.md` and installed by
`pinax-ontology-seed`, which refuses unrelated registries.

## Standards and source evidence

The lakehouse loader already profiles physical columns. It no longer turns
column-name substring guesses into authoritative semantic identifiers. Actual
Iceberg schema imports enter Pinax as draft inventory; stewards approve meaning.
Croissant contexts now declare the schema namespace and record/field/type terms,
and datasets use Schema.org Dataset. Loadable governed results declare Croissant
1.1; discovery descriptions explicitly identify themselves as metadata only.
Source overrides remain explicit during prepared builds; no sibling path/git
pin is committed and Sail continues to come from our source checkout.

With ontology configured, successful `execute_pinax_scan` also returns
`semantic_exports`. These contain a loadable Croissant 1.1 inline-data document
and CDIF metadata, bound to the same ontology digest. Export preparation occurs
before QueryGraph's final catalog-state and authorization rechecks. Existing
clients without ontology configuration retain the prior execution response.
Discovery-only Croissant descriptions explicitly carry `qg:metadataOnly`; they
make no claim that a reader can fetch protected rows without a governed read.
