# QueryGraph compatibility baseline

Verified baseline for the unified `querygraph/querygraph` repository (2026-08-06):

| Component | Version/revision | Role |
| --- | --- | --- |
| QueryGraph Rust crate | 0.4.2 | Root runtime and published `querygraph` crate |
| QueryGraph Python API | 0.4.1 | `python/` project and published wheel/sdist candidate |
| TypeSec | 0.13.1 | Capability and TypeDID authority |
| Marciana | 0.12.1 | Cognition and governed memory |
| Grust | 0.12.1 | Graph and Sail integration substrate |
| LakeCat | 0.3.0 | Catalog/bootstrap proof boundary |
| Sail | `d9e0fa42c3238c34ba3223336e9396291a31d1a4` | Explicit local upstream integration revision; see `compat/sail-revision.txt` |

The Rust package resolves all QueryGraph stack dependencies from released
registries. No published manifest uses sibling path or Git dependencies.
The Python API is packaged independently under `python/` while retaining the
`querygraph` import and CLI names.

Verification on this baseline:

- Rust formatting, strict Clippy, 71 unit tests, and 2 integration tests;
- Python 52-test suite, including cross-language TypeDID/equivalence tests;
- `cargo package --locked`, `cargo publish --locked` (`querygraph 0.4.2`);
- `uv build` and `twine check` for `querygraph 0.4.1`.
