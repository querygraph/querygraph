# MCP and the Pinax service

The current Rust server implements **MCP 2026-07-28** over stateless stdio and
Streamable HTTP, including the central ontology. See
[the current transport guide](mcp-stateless.md) for per-request metadata,
explicit model handles, HTTP headers and the grust endpoint. The handshake
instructions below describe the retained legacy stdio mode.

The Rust server and Python's optional Rust-backend bridge share one tool
contract, embedded from `python/querygraph/mcp_contract.json`. The original
Python FastMCP server remains available, with its existing OSI and rights
configuration. It has different OSI representations and answer behavior; it is
not a substitute for the authoritative Pinax service.

```sh
cargo build --locked --bin querygraph
target/debug/querygraph mcp-serve
uv run --project python python -m querygraph mcp-serve \
  --rust-backend "$PWD/target/debug/querygraph"
```

The optional Python CLI replaces itself with the configured Rust executable,
preserving stdio and request IDs. In legacy mode, Rust owns the session and
imports persist across calls. In modern mode, imports require explicit handles
on each consuming request. Governed requests have a 30-second timeout and are not
automatically retried. For legacy initialization, the Rust server negotiates
`2025-11-25`, `2025-06-18`, or `2024-11-05`, returning its newest supported
version for other requests. Legacy clients receive JSON text without structured
output fields. Send `initialize` with protocol version, capabilities and
`clientInfo`, then `notifications/initialized`, before using tools.

The input thread continues reading controls while up to 16 governed requests
await backend work. Additional governed requests receive an overload error;
duplicate in-flight IDs are rejected. `notifications/cancelled` aborts the
matching request without a response; malformed/unknown cancellations are ignored.
EOF and input errors abort and join all owned tasks. Synchronous tools retain
their ordering and are not interruptible. Completed response writes are
synchronous and can wait on output backpressure. Cancellation cannot retract a completed response or guarantee
that a remote HTTP server stops its own work. This follows the
[MCP cancellation contract](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/cancellation).

Malformed JSON-RPC requests and unknown tools are protocol errors. Malformed
arguments to known tools and domain failures are tool results with `isError`.
Previously accepted empty search/question arguments, ambiguous dual imports,
and unknown argument fields are rejected. `search_semantic_model` is an alias
for `search_semantic_models` on the Rust surface.

Encoded messages are limited to 32 MiB; an oversized stdio message closes the
session after an error. Each session accepts at most 128 imported models and
64 MiB of serialized semantic models. Pinax independently limits each
registry document to 8 MiB. Scan intents are limited to 64 KiB.

## Configure governed discovery, planning, and execution

`discover_pinax_tables`, `plan_pinax_scan`, and `execute_pinax_scan` require an attached service. Without one, each reports that
the backend is not configured. The host supplies immutable registry and policy
files, an exact policy identity/revision, a server DID, and a LakeCat credential
for the agent. Tool arguments cannot override any of those choices.

Example `registry-service.json` (paths resolve beside this file):

```json
{
  "registry": "enterprise.json",
  "warehouse": "production",
  "policy": "registry-policy.yaml",
  "policy_id": "enterprise",
  "policy_revision": 1,
  "server_did": "did:example:querygraph-registry",
  "lakecat_origin": "https://catalog.example.com/",
  "lakecat_subject": "did:key:REPLACE_WITH_AGENT_DID",
  "lakecat_identity": "credentials/identity.json"
}
```

The identity file contains `{"seed":"agent.seed","recipient_document":"lakecat.json"}`;
these paths resolve beside the identity file. The seed is exactly 32 raw,
high-entropy bytes, and the recipient file is the host-pinned TypeSec DID
document of the catalog. TypeSec derives the signing/agreement keys. Configure
`lakecat_subject` as the canonical multicodec/base58 `did:key` of that signing
key; a different subject is rejected. TypeSec's seed derivation differs from
the Python interoperability signer's seed derivation, so equal seed strings
do not establish equal identities.

The adapter mints a fresh encrypted TypeSec envelope for every HTTP request.
The remote gateway rejects replay; a reusable static envelope cannot support
multi-request planning or deployment. Legacy `lakecat_envelope` configuration
is accepted only for a single request and then fails closed. Configure exactly
one of the two credential sources. These catalog credentials are separate from
the Python interoperability envelope used on MCP.
LakeCat's conservative default verifier rejects credentials; configure its
TypeDID gateway. The configured credential must authenticate the same subject
that signs MCP requests. An unverified identity header is never used as a
fallback. The adapter rejects redirects, limits responses to 8 MiB, and uses a
30-second request timeout. Use HTTPS outside isolated local fixtures.

A TypeSec RBAC policy can grant explicit resources:

```yaml
roles:
  - name: analyst
    permissions: [read]
    resources:
      - pinax/acme/customers/revisions/1
      - pinax/acme/customers/revisions/1/columns/1
assignments:
  - subject: "did:key:REPLACE_WITH_AGENT_DID"
    roles: [analyst]
```

Pinax checks the configured policy binding and purpose allowlist, then table
and column permissions on every request. The stock RBAC engine does not add
context-sensitive policy semantics; an embedding service can inject a different
`PolicyEngine` through `RegistryPolicy`.

```sh
target/debug/querygraph mcp-serve --registry-config registry-service.json
uv run --project python python -m querygraph mcp-serve \
  --rust-backend "$PWD/target/debug/querygraph" \
  --registry-config registry-service.json
```

The tool arguments are `intent` (JSON text) and `envelope` (an object). Sign a
QueryGraph interoperability envelope with action `invoke`, resource
`/mcp/plan_pinax_scan`, recipient equal to `server_did`, and
`payload.bodySha256` equal to the SHA-256 hex digest of the exact UTF-8 intent
text. Example intent:

```json
{"table":"customers","columns":["customer_id"],"purpose":"analytics","limit":100}
```

Successful output contains enterprise, table, revision, registry digest,
snapshot ID, requested columns, purpose, limit, and planner identity. It excludes
scan tasks, storage paths, credentials, and rows. The service reloads catalog
metadata after planning to reject concurrent registry/schema changes even when
the data snapshot ID is unchanged.

## Execute bounded governed reads

Call `execute_pinax_scan` with the same intent shape, limit 1–1000, and
resource `/mcp/execute_pinax_scan`. A planning signature cannot authorize
execution. The configured LakeCat owner must support the execution extension
and grant `table.execute_scan` independently of planning permission.

The Rust service checks Pinax authorization and physical registry/schema pins,
then binds the request to the observed catalog state and snapshot. LakeCat applies
mandatory predicates through Sail and revalidates authority before releasing its
buffer. QueryGraph checks the returned proof, exact scope, row/evidence digests,
and fresh catalog state and Pinax authorization before returning
`{summary, rows, evidence, evidence_digest}`. Errors release no partial rows.
Clients cannot supply SQL, storage paths, filters, or reusable scan tasks.

The owner extension currently requires prepared LakeCat/Sail sources; released
dependency resolution remains a release gate. Evidence hashes describe the
authenticated owner's response; they are not independently signed attestations.
See [the execution contract](governed-execution-contract.md) for engine bounds
and remaining acceptance work.

## Discover permitted metadata

Call `discover_pinax_tables` with intent text such as
`{"purpose":"analytics","limit":10}`. Sign the same envelope format using
resource `/mcp/discover_pinax_tables`. Signatures cannot be replayed between
discovery and planning. Discovery checks registry policy without catalog I/O;
it omits denied tables and columns, primary keys, and security configuration.
A disallowed purpose therefore produces an empty page. Unresolved policy fails
the request instead of returning partially authorized metadata.

The limit is 1–100 registry tables examined per page. A page can be empty while
still providing `next_cursor`; pass that object as the next intent's `cursor`.
Cursors bind the registry digest and become invalid when the registry changes.
Discovery does not establish physical table readiness or execution authority.

## Review and reconcile a registry deployment

```sh
target/debug/querygraph registry-deployment plan \
  --current enterprise.json --target enterprise-next.json --warehouse production
target/debug/querygraph registry-deployment reconcile \
  --registry-config registry-service.json --target enterprise-next.json
```

Planning validates the successor using Pinax and includes every target table,
including unchanged tables whose whole-registry digest must change. Existing
tables carry predecessor and target contracts; new tables carry create requests.
Reconciliation performs fresh authenticated reads and reports `target`,
`pending_update`, `pending_create`, `missing_existing`, or `drift` for each table.
Only an authoritative missing-table response permits `pending_create`; a missing
route or namespace is an error. The command exits successfully only when every
table matches the target. Backend failures produce no partial readiness report.

Planning and reconciliation never mutate the catalog. Reconciliation can inspect an uncertain
prior write before an operator decides how to recover. Its reads are sequential
observations, not an atomic snapshot or a lease authorizing consumer cutover.

Apply the reviewed target digest explicitly:

```sh
target/debug/querygraph registry-deployment apply \
  --registry-config registry-service.json --target enterprise-next.json \
  --expected-target-digest 'DIGEST_FROM_REVIEWED_PLAN'
```

Apply preflights every table before writing. Existing-table updates require the
owner's `x-lakecat-table-state` observation and send it as
`x-lakecat-expected-table-state`; unsupported owners fail closed. Sail applies
the standard Iceberg schema/property updates, retaining field IDs and rejecting
reuse of retired IDs. New tables use Pinax's create contract. Each write is
attempted once; an ambiguous result stops the deployment. Reconcile before
restarting: a fresh application skips tables already matching the target.
The command finishes with full reconciliation, but does not switch consumers
or rewrite the running service's immutable configuration.

## Activate a reviewed consumer

In the target consumer configuration, set `registry` to the reviewed target
file and add:

```json
"activation": {
  "expected_registry_digest": "DIGEST_FROM_REVIEWED_PLAN"
}
```

With this option, startup verifies the immutable registry digest and reads every
bound catalog table before accepting MCP initialization. All schemas and Pinax
pins must match within one 30-second deadline. Missing tables, drift, denial,
backend failure, and an unreviewed digest prevent startup. Existing configurations
without `activation` retain discovery-only startup without catalog readiness I/O.
Use a fresh-signing `lakecat_identity` for multi-table startup and later requests.

The operator owns the maintenance window and service processes:

1. Review the successor and prepare a target configuration with its exact digest.
2. Stop consumers and exclude competing catalog writers for the transition.
3. Apply the reviewed digest. On uncertain writes, reconcile before retrying.
4. Require a complete ready report, then start consumers with the target config.
5. Verify discovery and a governed query before ending the maintenance window.

The startup check is a fresh observation, not a cross-table lock or lease. Normal
scan checks still reject subsequent drift. Restarting an old activation-pinned
configuration after catalog migration fails; recovery requires a separately
reviewed compatible forward transition, not simply switching the old file back.
The deployment fixture verifies stop/apply/restart and revision discovery.
The row fixture also applies a reviewed additive revision after closing the old
MCP sessions, starts activation-pinned target sessions, and verifies real filtered
rows carrying revision 2 through both CLI entry points.

## Execution and deployment gates still outstanding

Planning is not execution authorization. The execution adapter uses the prepared
owner endpoint and does not expose file tasks or storage credentials. Real rows
pass through both MCP transports. Owner HTTP tests discard actual Sail buffers
after policy/catalog changes. Transport tests cover request cancellation,
disconnect, overload, and deadline cleanup of pending backend futures.
The owner extension and Sail helper must
complete release checks and released dependency resolution before this becomes
a reproducible release build.

Registry deployment must coordinate every bound table because digests cover the
whole registry. Pause consumers during a controlled transition, validate the
successor, migrate schemas through the catalog's commit protocol, update all
pins, and reconcile each table before switching the configured snapshot. Do not
retry an uncertain write until current state has been loaded and compared.
The conditional mutation extension is prepared in LakeCat commit `93a7673f`,
selected through the prepared source checkout. Consumer transition and governed
row execution pass both locally and on grust; this does not claim that the owner
extension has been published as a new LakeCat crate.

## Verification

Pinax is published as `pinax-registry` 0.2.0. The ordinary locked build resolves
it from crates.io with no Pinax source override. The live fixture separately
selects our LakeCat/Sail source checkouts through its command-line arguments.

```sh
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --bin querygraph
QG_RUST_MCP_BIN="$PWD/target/debug/querygraph" \
  uv run --project python pytest python/tests/test_mcp_bridge.py -q
```

The bridge test uses real SDK sessions against Rust directly and through Python.
Service tests verify signed body/identity binding, rejection before planning,
catalog drift, unresolved policy, backend failure, and snapshot changes. HTTP
tests use local fixtures to verify credential forwarding, principal confinement,
and redacted failures. They do not claim to be a live LakeCat/Sail deployment.

The separate [authenticated deployment fixture](../integration/pinax/README.md)
has also passed against the prepared owner extension. It verifies real Sail
schema commits and physical metadata readback, reviewed digests, stale-write
rejection, drift, and restart reconciliation. Its report explicitly excludes
governed row execution and records the tested source/binary identities.
