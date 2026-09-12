# Fihrist and QueryGraph touchpoints

Fihrist is maintained in the standalone [Fihrist repository](https://github.com/querygraph/fihrist)
(`~/src/fihrist` locally). QueryGraph 0.5.1 consumes the released
`fihrist 0.1.0` crate from crates.io. Fihrist owns the `fihrist.v1` contract, validation, revision compatibility,
digests, profiles, catalog adapters, and governed scan planning.

| Touchpoint | Ownership and integration |
| --- | --- |
| Rust API | QueryGraph re-exports the crate as `querygraph::fihrist`, preserving existing import paths. There is no embedded implementation. |
| CLI | `querygraph fihrist init`, `validate`, `check`, and `catalog-plan` use `fihrist::cli::FihristCommands`. The standalone `fihrist` binary uses the same commands without the `querygraph fihrist` prefix. |
| MCP | QueryGraph owns the `validate_fihrist` tool definition and JSON-RPC handling; its handler calls `fihrist::validate_proposal`. Inputs are JSON-string `document` and optional `previous`; output includes validity, enterprise, table count, and digest. |
| LakeCat / Iceberg | Fihrist's `CatalogBinding` maps logical names to warehouse / enterprise namespace / table. It generates create requests and can submit them with a host-configured authenticated client. |
| TypeSec | Fihrist's `GovernedRegistry` checks the configured policy identity and revision, allowed purpose, and read permission for the table and every requested column. Only explicit allow decisions proceed. |
| Sail planning | `GovernedRegistry::plan_scan` checks physical schema and registry pins, then calls LakeCat's `SailCatalogEngine` with an explicit projection, bounded row count, and exact snapshot. |
| Execution | LakeCat retains governed execution authorization, snapshot revalidation, and proof binding. A Fihrist scan plan is not an execution capability. |
| Releases | `QUERYGRAPH.md` and `scripts/check-stack-dependencies.py` track Fihrist and its sibling pins. Fihrist releases after LakeCat and TypeSec, before QueryGraph. |

## Shared contracts

The extraction preserves the `fihrist.v1` wire format and digest encoding.
Iceberg properties bind the table to `fihrist.version`, `fihrist.enterprise`,
`fihrist.table`, `fihrist.revision`, `fihrist.registry-digest`, and the serialized
`fihrist.contract`. Whole-registry digests require updating all bound tables when
a new registry snapshot is deployed.

Policy resources remain `fihrist/{enterprise}/{table}/revisions/{revision}` and
`/columns/{field-id}` beneath that path. Classification and retention are
metadata declarations; they do not grant access or delete stored rows.

## Service composition boundary

The intended flow is:

```text
Fihrist registry → TypeSec authorization → catalog/schema checks
                → Sail scan plan → LakeCat governed execution
```

QueryGraph's CLI and MCP integration is active. Governed scan planning is a
library integration point, not a new QueryGraph HTTP/MCP scan endpoint. An
embedding service must supply authenticated principals, trusted catalog
metadata, a deployment-owned TypeSec policy engine, and a real Sail backend.
Authentication, registry publication, catalog migrations, and backend
credentials remain the host's responsibility. MCP validation does not publish
state or authorize scans.

## Compatibility and verification

QueryGraph keeps CLI and MCP boundary tests; Fihrist owns the domain and adapter
tests, including denial, unresolved policy, drift, and local HTTP submission.
The compatibility re-export shares the actual Fihrist types, so callers can
pass values directly between the two crate paths. No sibling path or git
dependency is committed. Release lockfiles must resolve Fihrist from crates.io.

The checker reads `../fihrist` by default; `QG_STACK_FIHRIST` overrides its path.
Run the shared Python bootstrap and matrix checks from QueryGraph:

```sh
publishing/scripts/ensure-python-env.sh python
uv run --project python python scripts/check-stack-dependencies.py --write
uv run --project python python scripts/check-stack-dependencies.py --check
```

See the authoritative [Fihrist v1 specification](https://github.com/querygraph/fihrist/blob/main/docs/fihrist.md),
[enterprise example](https://github.com/querygraph/fihrist/blob/main/examples/fihrist/enterprise.json),
and [stack dependency matrix](../QUERYGRAPH.md).
