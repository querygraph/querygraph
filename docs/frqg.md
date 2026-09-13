# Pinax and QueryGraph touchpoints

Pinax is maintained in the standalone [Pinax repository](https://github.com/querygraph/pinax)
(`~/src/pinax` locally). The current QueryGraph checkout consumes the released
`pinax-registry` 0.2.0 crate, imported as `pinax`. Pinax owns `pinax.v1`, its
validation and digest rules, the central ontology, and catalog/scan adapters.
The new namespaces require explicit migration from pre-rename documents.

| Touchpoint | Ownership and integration |
| --- | --- |
| Rust API | QueryGraph re-exports the crate as `querygraph::pinax`, using the renamed import path. There is no embedded implementation. |
| CLI | `querygraph pinax init`, `validate`, `check`, and `catalog-plan` use `pinax::cli::PinaxCommands`. The standalone `pinax` binary uses the same commands without the `querygraph pinax` prefix. |
| MCP | QueryGraph owns the `validate_pinax` tool definition and JSON-RPC handling; its handler calls `pinax::validate_proposal`. Inputs are JSON-string `document` and optional `previous`; output includes validity, enterprise, table count, and digest. |
| LakeCat / Iceberg | Pinax's `CatalogBinding` maps logical names to warehouse / enterprise namespace / table. It generates create requests and can submit them with a host-configured authenticated client. |
| TypeSec | Pinax's `GovernedRegistry` checks the configured policy identity and revision, allowed purpose, and read permission for the table and every requested column. Only explicit allow decisions proceed. |
| Sail planning | `GovernedRegistry::plan_scan` checks physical schema and registry pins, then calls LakeCat's `SailCatalogEngine` with an explicit projection, bounded row count, and exact snapshot. |
| Execution | LakeCat retains governed execution authorization, snapshot revalidation, and proof binding. A Pinax scan plan is not an execution capability. |
| Releases | `QUERYGRAPH.md` and `scripts/check-stack-dependencies.py` track Pinax and its sibling pins. Pinax releases after LakeCat and TypeSec, before QueryGraph. |

## Shared contracts

The extraction preserves the `pinax.v1` wire format and digest encoding.
Iceberg properties bind the table to `pinax.version`, `pinax.enterprise`,
`pinax.table`, `pinax.revision`, `pinax.registry-digest`, and the serialized
`pinax.contract`. Whole-registry digests require updating all bound tables when
a new registry snapshot is deployed.

Policy resources remain `pinax/{enterprise}/{table}/revisions/{revision}` and
`/columns/{field-id}` beneath that path. Classification and retention are
metadata declarations; they do not grant access or delete stored rows.

## Service composition boundary

The intended flow is:

```text
Pinax registry → TypeSec authorization → catalog/schema checks
                → Sail scan plan → LakeCat governed execution
```

QueryGraph's CLI and MCP integration is active. The unreleased
`discover_pinax_tables` and `plan_pinax_scan` MCP tools compose the library through a deployment-owned
`RegistryService`; `mcp-serve --registry-config` configures the REST adapter.
The Python `--rust-backend` bridge forwards the same contract. See the
[service configuration and verification guide](mcp-pinax-service.md).
`registry-deployment plan` reviews the full successor transition, and
`registry-deployment reconcile` reads every table binding and rejects incomplete
or drifted deployments. Neither command performs catalog mutations or cutover.
`registry-deployment apply` requires the reviewed target digest, preflights all
tables, and submits conditional Iceberg commits through the owner. It requires
the prepared LakeCat state-token extension and freshly minted TypeSec credentials.
It reconciles after application; consumer cutover remains separate.
The host must supply authenticated principals, trusted catalog
metadata, a deployment-owned TypeSec policy engine, and a real Sail backend.
Authentication, registry publication, catalog migrations, and backend
credentials remain the host's responsibility. MCP validation does not publish
state or authorize scans. The planning tool returns bounded planning evidence,
not rows, backend task paths, or an execution capability.

## Compatibility and verification

QueryGraph keeps CLI and MCP boundary tests; Pinax owns the domain and adapter
tests, including denial, unresolved policy, drift, and local HTTP submission.
The compatibility re-export shares the actual Pinax types, so callers can
pass values directly between the two crate paths. No sibling path or git
dependency is committed. Release lockfiles must resolve Pinax from crates.io.

The checker reads `../pinax` by default; `QG_STACK_PINAX` overrides its path.
Run the shared Python bootstrap and matrix checks from QueryGraph:

```sh
publishing/scripts/ensure-python-env.sh python
uv run --project python python scripts/check-stack-dependencies.py --write
uv run --project python python scripts/check-stack-dependencies.py --check
```

See the authoritative [Pinax v1 specification](https://github.com/querygraph/pinax/blob/main/docs/pinax.md),
[enterprise example](https://github.com/querygraph/pinax/blob/main/examples/pinax/enterprise.json),
and [stack dependency matrix](../QUERYGRAPH.md).
