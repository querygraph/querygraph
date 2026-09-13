# Authenticated registry deployment fixture

This isolated test runs real LakeCat, Sail Iceberg commits, and TypeSec gateway
verification. It checks schema evolution, all registry pins, metadata files,
reviewed-digest enforcement, restart reconciliation, stale owner tokens, and
drift rejection. It does not execute governed rows.

Prerequisites: the reviewed LakeCat state-token extension (`93a7673f`), a built
QueryGraph binary from this change, and the shared Python environment. Pinax
0.1.1 is published on crates.io; QueryGraph uses the released dependency without
a Pinax development override. Sail runs from the prepared source checkout.

```sh
cargo build --locked --bin querygraph
uv run --project python python integration/pinax/run.py \
  --lakecat-root /tmp/lakecat-pinax-integration \
  --querygraph-bin target/debug/querygraph \
  --report /tmp/querygraph-pinax-live-report.json
```

The runner generates an unpublished fixture manifest and lockfile in a temporary
directory, using explicit local owner sources only there. Cargo builds offline
using the owner's build cache. LakeCat listens on a loopback ephemeral port and
uses an in-memory catalog and temporary file warehouse. The fixture's fixed key
seed is test data only. Each request receives a fresh TypeSec envelope; replay
protection is not disabled. Identity configuration is generated only inside the
temporary directory.

The runner owns and stops the server process on success or failure and removes
the warehouse and credentials. The report records the tested owner commit and
QueryGraph binary hash, with no credentials or protected data. It is written
only after every assertion and cleanup succeeds. No service is deployed and no
crate, commit, or tag is published.

The deployment fixture also runs an activation-pinned MCP consumer, stops it
before applying the reviewed transition, and starts the target consumer after
full reconciliation. It checks discovered revisions and rejects unreviewed,
unfinished, stale, or subsequently drifted configurations before initialization.
The owner commit in the report is the base HEAD; when using prepared worktrees,
the compiled execution changes may still be uncommitted.

When validating LakeCat's full workspace against an unreleased Sail worktree,
its `trybuild` API tests launch nested Cargo processes. A command-line `--config`
patch is not inherited by those processes. Put the same temporary patch table
in a task-owned Cargo home's `config.toml` and pass that `CARGO_HOME` to the test
command; use the existing registry/git caches and restore the owner's original
lockfile afterward. Keep this development override outside committed manifests
and configuration. A passing override build does not establish released-pin
availability.

## Real Sail row fixture

The separate buffered-read fixture creates a real Iceberg table with PyIceberg
and reads it through the prepared Sail helper. It checks actual row filtering
before projection/limit, byte limits, exact old/new snapshots, invalid projections,
and missing backend metadata. This remains engine acceptance, not governed MCP
execution. The disposable Python environment adds only the pinned fixture
dependencies; the project manifest and lockfile are unchanged.

```sh
QG_SAIL_SOURCE=/tmp/sail-pinax-integration \
  QG_LAKECAT_SOURCE=/tmp/lakecat-pinax-integration \
  QG_RUST_MCP_BIN="$PWD/target/debug/querygraph" \
  uv run --project python --with 'pyiceberg[pyarrow,sql-sqlite]==0.10.0' \
  pytest integration/pinax/test_read_rows.py -q
```

For a retained, sanitized report, run `integration/pinax/read_rows.py` in the
same environment with `--sail-root` and `--report` arguments. The runner creates
its unpublished Rust fixture manifest outside the repository and records the
tested binary hash. Temporary table data and its SQLite catalog are removed.

The second pytest case composes real TypeSec authentication, LakeCat execution
permission and release checks, Sail mandatory row filtering, and lineage receipts.
It verifies returned row/evidence digests independently. `execute_rows.py` retains
an owner report when given `--lakecat-root`, `--sail-root`, and `--report`.
Set `QG_RUST_MCP_BIN` for pytest, or pass `--querygraph-bin` to that runner, to
verify signed execution through both direct and bridged MCP sessions as well.
The report explicitly distinguishes these runs from owner-only acceptance.

With the Rust binary configured, the row fixture also closes the old MCP
sessions, applies a reviewed additive registry revision, and starts both target
sessions with the activation digest. It verifies the same real filtered rows
with revision 2 before deleting data for the backend-failure case.

The owner fixture also injects purpose revocation, registry-pin drift, and a
valid snapshot transition after Sail returns a nonempty real buffer, before
the release guard runs. All three must return conflict without rows. A local
marker records only the buffered row count, proving the test reached the real
reader. These deterministic owner HTTP checks do not claim a race inside file
I/O or transport cancellation. The mutation adapter exists only in this fixture.

All Rust code changed for this integration uses edition 2024, including the
explicit `sail-iceberg` crate edition and temporary fixture manifests. Use
`rustfmt --edition 2024` for explicit formatter invocations.
