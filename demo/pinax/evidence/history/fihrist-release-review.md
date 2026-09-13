# Fihrist integration release review

Fihrist 0.1.1 was published to crates.io on September 12, 2026 with explicit user
authorization. QueryGraph's lockfile now resolves the registry package, whose
checksum matches the reviewed ontology-inclusive candidate. The integration and
EC2 demonstration retain their prepared-source acceptance evidence.

The user clarified that Sail is consumed from our source checkout. That is the
supported integration target, not a temporary fallback awaiting upstream
acceptance. No upstream Sail review, PR, merge or release is required to finish
this work. The earlier upstream-review blocker was incorrect and is removed.

## Current acceptance evidence

The September 12 stateless-MCP increment supersedes the earlier transport-only
acceptance summary: 121 Rust unit tests, five integration tests, three console
tests and 82 Python tests passed, with no skipped Python conformance cases.
The modern server and ontology code remain Rust 2024. The independent Python
test tooling validates actual Rust wire responses against the vendored official
2026-07-28 schema; it does not implement another modern server.

| Requirement | Evidence | Result |
| --- | --- | --- |
| MCP compatibility, bounds and cancellation | QueryGraph Rust tests and `python/tests/test_mcp_bridge.py`; real direct and Python CLI sessions in the EC2 execution report | Passed in the recorded integration runs |
| Stateless MCP 2026-07-28 | `grust-mcp-stateless.json`, `grust-mcp-stateless-stdio.json`, `mcp-stateless-schema-validation.json` under `demo/fihrist/evidence/` | HTTP and both stdio entry paths passed; 12 live responses and two signed requests conform |
| Central ontology and semantic discovery | Same modern MCP reports; `ontology-authoring-cli.json` and the separate Fihrist ontology tests | Reviewed mappings, policy filtering, revision publication, stale-CAS rejection and ontology-derived reads passed |
| Trusted Fihrist registry, identity, policy and catalog composition | `src/registry_service/`; signed discovery, planning and execution in `demo/fihrist/evidence/grust-console.json` | Passed against the retained EC2 owner |
| Real rows, mandatory filter and evidence | `demo/fihrist/evidence/grust-fihrist-execution-final.json` | Passed; exact allowed row is `{"id":2}` |
| Denials, drift and backend failure | Same execution report, including mutations after nonempty real reads | Passed; buffers withheld |
| Reviewed deployment and consumer reconciliation | Same report's revision-two consumer cutover; `integration/fihrist/run.py` | Passed prepared-source acceptance |
| Full live semantic graph and lineage | `demo/fihrist/evidence/grust-live-services-final.json` | 34 nodes, 33 edges, node readback and Sail lineage append |
| Released dependency resolution | QueryGraph `Cargo.lock`, ordinary `cargo check --locked` | Published Fihrist 0.1.1 resolves without a source override |
| Stack matrix | `uv run --project python python scripts/check-stack-dependencies.py --check` | Aligned; existing unpublished Grust warnings remain |

All five grust services are active, including `querygraph-fihrist-mcp` on
loopback 18082. The stateless-MCP source archive before crate publication had SHA-256
`b4703370d384e8453dba71ac16abe32f4316fe438b072ed4dbeecbeebc29d5f5`;
all 3,248 source checksum entries passed verification. Subsequent publication
bundles carry their current checksum in the adjacent `.sha256` file. The updated 14-slide
PDF, narrative, MCP guide and archive are delivered in `~/icloud/fihrist`.

The earlier registry-availability blocker is closed. `cargo publish --allow-dirty
--locked` verified and uploaded the exact reviewed package, and Cargo confirmed
registry availability. No additional Sail upstream action is required.

Post-publication checks passed without Cargo source overrides: ordinary locked
check/build, 121 unit tests, five integration tests, three console tests, strict
all-target Clippy, documentation/doctests and 82 Python regression/conformance
tests. Stack dependency alignment remains green with the previously recorded
unpublished Grust warnings. The EC2 build script now consumes released Fihrist
through the ordinary lockfile; Sail still uses the selected source checkout.

## Fihrist package candidate

Local commit `c51b3cc` excludes book output, blog textpacks and artwork from the
Cargo package. These assets remain in the repository and in the delivered book
and announcement. The crate retains source, examples and the v1 specification.
The refreshed ontology-inclusive working-tree candidate is approximately 99.5 KiB
compressed, containing 40 files. `cargo package --allow-dirty` passed, including
compilation of the packaged source, on September 12, 2026. The archive includes
the ontology operator guide, reference context, implementation and separate tests;
publishing artwork and textpacks remain excluded. Its SHA-256 is
`88958f70ce619caea5492fdf6717290ab0a3247870bcb2f849a25bc76fb1af0c`.
This candidate includes uncommitted ontology changes beyond `c51b3cc`.

The current Rust source passes all 33 tests, strict all-target Clippy,
formatting, documentation with warnings denied and doc tests. Publication
verification compiles the packaged source successfully. Publication completed;
the registry checksum is the candidate SHA-256 above. QueryGraph's lockfile now
records the crates.io source and checksum instead of the validation-only patch.

## Owner and engine changes

LakeCat's actual dependency contract and formatting checks pass in the isolated
worktree. The existing dependency check runs without build-time Cargo source
overrides: it checks the committed fallback Sail git source in the lock.
The executable integration separately selects the user-owned Sail checkout
through `--sail-root` and generated Cargo patches, as intended for this work.
The check's temporary resolution was restored after validation. Its earlier
1,411-test workspace result and the final EC2 real-row run remain the owner
behavior evidence; this audit does not relabel metadata checks as execution.

Sail commit `ecb3e8ef` migrates `sail-function` to Rust 2024 and preserves fixed
chunk iteration and UTF-16 behavior. All 265 package tests and strict
all-target/all-feature package Clippy pass on grust with Rust 1.98.1. New tests
exercise sliced null bitmaps around 64-element boundaries, both UTF-16 byte
orders, surrogate pairs and malformed inputs. Touched inline tests were moved
to separate files. The existing unsafe hashing operations now have explicit
blocks and safety explanations; the hash algorithm is unchanged.

A broader five-package Clippy command proceeds past the function crate and
reports ten findings in unchanged `sail-delta-lake` (formatting borrows and one
optional-value simplification). That command is not green. Package-level
success must not be presented as full Sail CI success.

The focused command over all five changed packages now passes with
`--all-targets --all-features --no-deps -- -D warnings`. It covers
`sail-execution`, `sail-function`, `sail-data-source`, `sail-logical-plan` and
`sail-iceberg`; the log is `logs/changed-packages-clippy.log`. The Delta findings
are broader dependency diagnostics, not evidence that the Fihrist read path failed.
Avoid expanding the integration's behavior changes solely to silence unrelated
dependency diagnostics.

The combined five-package `cargo test --all-targets --all-features` command
also completed successfully: 427 unit tests passed, and all benchmark
executables completed. The log is `logs/changed-packages-tests.log`. These
debug-mode benchmark runs establish successful execution only, not performance.

Before publication, ordinary QueryGraph checks failed because crates.io contained
only Fihrist 0.1.0. This historical failure is superseded by the published-package
resolution and ordinary build above; source-override checks were not substituted
for release acceptance.

Release-preparation validation uses
`/home/admin/querygraph-fihrist-release-check-20260912` on grust. Its logs are
`logs/sail-function-tests.log`, `logs/sail-function-final-clippy.log` and
`logs/sail-integration-clippy.log`. The delivered demo and its source archive
remain the previously verified snapshot under
`/home/admin/querygraph-fihrist-demo-20260912`; these later preparation changes
have not been substituted into it.

An upstream Sail PR is outside the requested workflow, so its review and
coordination rules do not block this checkout-based integration. The validated
owner boundary remains:
Sail owns snapshot reads and predicates; LakeCat owns execution authority and
post-read release checks; Fihrist owns registry contracts; QueryGraph composes
those services and verifies returned scope and evidence. Acceptance is against
the selected checkout and recorded executable results; it does not depend on
an upstream merge.

## Reviewable local commits

The isolated Sail and LakeCat worktrees are clean after these local commits:

| Repository | Commit | Change |
| --- | --- | --- |
| Fihrist | `c51b3cc` | Exclude publishing assets from the verified crate candidate |
| Sail | `12b127b3` | Rust 2024 in catalog execution dependencies |
| Sail | `ecb3e8ef` | Rust 2024 function migration and chunk/encoding regression tests |
| Sail | `a5607cb9` | Preserve shared delete references over the existing remote-plan wire format |
| Sail | `efab86db` | Bounded snapshot reads and REST predicate compilation |
| LakeCat | `4b6aab05` | Authenticated execution, independent permission and post-read release checks |

Sail remains on local branch `codex/fihrist-execution`; LakeCat remains on
`codex/fihrist-integration`. The LakeCat commit explicitly depends on the
prepared Sail helper for feature-enabled validation; its committed dependency
pin has not been silently moved. These commits define the supported source
integration. No upstream Sail push, PR or merge was performed. The later,
explicitly authorized Fihrist 0.1.1 publication closes its crate-resolution gate.
