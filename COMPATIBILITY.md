# QueryGraph compatibility baseline

Verified baseline for the unified `querygraph/querygraph` repository (2026-08-09):

| Component | Version/revision | Role |
| --- | --- | --- |
| QueryGraph Rust crate | 0.4.2 | Root runtime and published `querygraph` crate |
| QueryGraph Python API | 0.4.1 | `python/` project and published wheel/sdist candidate |
| TypeSec | 0.13.1 | Capability and TypeDID authority |
| Marciana | 0.12.1 | Cognition and governed memory |
| Grust | 0.12.1 | Graph and Sail integration substrate |
| LakeCat | 0.3.0 | Catalog/bootstrap proof boundary |
| Sail | `c530936541da340d3d466fff8ea17f8b41542017` | Exact reachable QueryGraph graph revision; see `compat/sail-revision.txt` |

The Rust package resolves all QueryGraph stack dependencies from released
registries. No published manifest uses sibling path or Git dependencies.
The Python API is packaged independently under `python/` while retaining the
`querygraph` import and CLI names.

The Sail revision merges the performance work proposed in
[lakehq/sail#2400](https://github.com/lakehq/sail/pull/2400) at `ce5dada0` with
QueryGraph's native Cypher graph extension. Its graph frontend is 4.44% faster
to parse and 4.20% faster end to end than the previous `d9e0fa42` baseline in
the same production-profile Docker benchmark. Parser/analyzer, planner,
Spark Connect, and strict Clippy gates pass at the recorded source revision.

The live release gate used one source-verified Sail executable built from
`c5309365` with Rust 1.96.0, native CPU instructions, `-O3`, thin LTO, one
codegen unit, aborting panics, no incremental state, `clang`/`lld`, and stripped
symbols. The 115,921,992-byte binary has SHA-256
`937eb113b5a03384a400cb659d12157a9fdc42fbfc19f660325708983e989199`.
It is an AArch64 PIE with no debug sections.

`scripts/bootstrap-debian-demo.sh` reads the machine pin by default, fetches
and verifies that exact detached revision, builds it with an optimized locked
release profile, and records the binary's source in `bin/sail.revision`.
`QG_SAIL_REVISION` is an explicit 40-hex override for candidate validation; a
moving branch name is never treated as the executable compatibility datum.

Verification on this baseline:

- Rust formatting, strict Clippy, 71 unit tests, and 2 integration tests;
- Python 52-test suite, including cross-language TypeDID/equivalence tests;
- `cargo package --locked`, `cargo publish --locked` (`querygraph 0.4.2`);
- `uv build` and `twine check` for `querygraph 0.4.1`;
- a locked production QueryGraph build using the same native-CPU, `-O3`, thin
  LTO, one-codegen-unit, aborting-panic, `clang`/`lld`, stripped profile. Its
  23,218,904-byte executable has SHA-256
  `38492297a4a1b8b041200093baadfca900fd2e88626834eb45abba2dfe7a3f00`;
- the production Dataverse gate loaded 2 datasets as 34 graph nodes and 33
  edges, verified all 4 one-row Sail views and a 2-dataset Cypher count,
  authorized the `typedid/a2a` answer path, appended the same OpenLineage event
  to JSONL and `qg_audit` in Sail, and appended its signed DID-ledger
  attestation; and
- the same Sail binary passed Grust's 30 live adapter/cognition checks,
  Marciana's 2 cognition parity/secrecy tests, GrustFrames' write/read,
  distributed-degree, and triplet gate, and Typesec's least-privilege company
  graph proof with 5 nodes and 4 edges.
