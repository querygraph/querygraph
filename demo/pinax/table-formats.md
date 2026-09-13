# Two live table formats

The format is selected per HTTP request (closed Iceberg/Delta enum), not by
mutating a shared symlink. Iceberg uses the retained customer-discovery owner;
Delta uses separate tables, catalog owner, configuration and MCP service.
Both use the same registry, reviewed ontology and TypeSec policy.

The demo Delta adapter implements LakeCat's Sail engine seam. It normalizes
Delta schema and version into the existing catalog contract, explicitly tagged
`qg.table-format=delta`; no Iceberg manifests or Iceberg physical tables are
created for Delta. The existing `snapshot_id` wire field carries Delta version
1 in this adapter. The operator seeds immutable local Delta tables with Sail.
The catalog envelope's required `format-version=1` describes this compatibility
record; it is not an assertion of Iceberg storage or the Delta protocol version.
Stable field IDs come from the shared Pinax contract, rather than Delta column mapping.
Native Delta transaction-log reads execute through Sail Spark Connect. The
owner retains authentication, projection/row constraints, fresh policy checks,
and evidence validation before releasing rows. Unsupported predicates,
mutations, versions and changed log fingerprints fail closed.

The native Delta adapter is scoped to this retained, read-only demo. It does not
claim general Delta catalog write support in the released LakeCat API.
Grust's separate graph extension uses Delta internally for both selections.

Process output and Arrow collection remain bounded; operation deadlines and
the console semaphore apply to both formats. A format change reloads the page,
clearing previous results and preserving the selected format in slides links.
