# QGQG: QueryGraph Unified Repository Goal

Status: **in progress — TypeScript API and npm release handoff**  
Owner: QueryGraph maintainers  
Review audience: Fable  
Started: 2026-08-06

## Objective

Make `querygraph/querygraph` the single source repository for the QueryGraph
runtime and its Python and TypeScript APIs. The former `querygraph/qg-rust` implementation
becomes the repository's root Rust crate (`querygraph` on crates.io), while the
former `querygraph/qg-python` distribution becomes the sibling `python/`
project in the same repository. The public Python import remains `querygraph`
and the TypeScript package is `@querygraph/querygraph`.

The existing `querygraph/querygraph` GitHub repository is a workspace
meta-repository. Because that canonical destination already exists, this goal
consolidates the qg-rust implementation into it; it does not attempt to rename
one repository over another or discard the workspace history.

## Non-negotiable boundaries

```text
querygraph/querygraph
├── Rust crate at the repository root      -> cargo package: querygraph
├── python/querygraph/                     -> Python package: querygraph
├── python/tests/                          -> Python API tests
├── typescript/src/                         -> TypeScript package: @querygraph/querygraph
├── typescript/tests/                       -> Node contract tests
└── docs/, lakehouse/, scripts/            -> shared product/docs tooling

TypeSec, Grust, Marciana, LakeCat, and Sail remain independent upstream
projects. QueryGraph depends on their released crates; they never depend on
this product repository.
```

- The Rust crate remains the governed service/composition boundary.
- The Python API is a client-facing projection and must not duplicate Rust
  authority or protected-memory storage.
- `MemoryVault` remains the only TypeSec capability-gated memory authority;
  adapters translate and delegate.
- Cognee and Fluree remain benchmark/comparative references only, never runtime
  dependencies or facades.
- Public routes, wire formats, durable identifiers, and database compatibility
  are preserved unless a versioned migration is explicitly recorded.
- Production files stay small and cohesive; tests remain in separate targets.

## Migration phases and acceptance gates

| Phase | Work | Acceptance evidence | Status |
| --- | --- | --- | --- |
| 1 | Inventory repositories, manifests, CI, docs, and release pins | Dependency matrix and compatibility risks in this document | **complete** |
| 2 | Establish this goal document and target layout | QGQG.md records boundaries, matrix, gates, rollback, and Fable questions | **complete** |
| 3 | Consolidate qg-rust into the canonical repository root | Root Cargo build/tests pass; crate metadata points to `querygraph/querygraph` | **complete** |
| 4 | Roll qg-python into `python/` | `uv build`, 52 Python tests, CLI imports, and cross-language tests pass | **complete** |
| 5 | Update organization references and dependency manifests | Unified CI/scripts/docs and released registry pins resolve; active runtime audit is clean | **complete** |
| 6 | Release and publish | crates.io `0.4.2` and PyPI `0.4.1` are published; authenticated workflow is retained for future releases | **complete** |
| 7 | Closeout | QGQG.md status, changelog, compatibility pin, and unified CI are committed and pushed; legacy-repository deprecation is a follow-up | **complete** |
| 8 | TypeScript API | TypeScript mirrors Python semantic/security modules with shared contract tests | **complete** |
| 9 | npm release | Public `@querygraph/querygraph@0.1.2` release candidate; registry tag/access verified after publication; clean-install verification is pending metadata propagation | **in progress** |

## Dependency and repository matrix

| Consumer | Current dependency/reference | Target | Planned action |
| --- | --- | --- | --- |
| QueryGraph Rust crate | `querygraph/qg-rust`, crates.io `querygraph` | `querygraph/querygraph`, crates.io `querygraph` | Change repository metadata; publish the next compatible release |
| QueryGraph Python package | `querygraph/qg-python`, PyPI `querygraph` | `querygraph/querygraph/python`, PyPI `querygraph` | Move project, preserve import and CLI names; publish next release |
| QueryGraph TypeScript package | none | `querygraph/querygraph/typescript`, npm `@querygraph/querygraph` | Share semantic/security wire contracts; publish public package |
| Marciana | released `marciana-*` crates | unchanged released crates | Keep QueryGraph as consumer; update docs/paths only |
| TypeSec | released `typesec-*` crates | unchanged released crates | No reverse dependency on QueryGraph |
| Grust | released `grust-*` crates | unchanged released crates | No reverse dependency on QueryGraph |
| LakeCat | released `lakecat-*`/`qglake-*` crates | unchanged released crates | Update local integration paths/documentation |
| Sail | explicit upstream revision/binary | unchanged Sail contract | Keep live Sail gate; only generic fixes go upstream |
| Bootstrap/publishing tooling | qg-rust/qg-python checkout paths | one `querygraph` checkout plus `python/` | Update scripts, CI, and docs |

## Versioning and compatibility

The Rust package keeps its `querygraph` crate name. Repository consolidation is
not a wire or API break, so the next Rust release is a patch release unless the
verification gates uncover an API change. The Python distribution keeps the
`querygraph` project name and imports; its version advances independently under
the same release note. The previous repositories receive a deprecation/redirect
notice after the canonical repository is pushed. No sibling path dependency is
allowed in a published manifest.

Every release records:

1. exact released TypeSec, Grust, Marciana, LakeCat, and Sail compatibility;
2. the crate/package version and source revision;
3. clean-install and checksum evidence; and
4. the migration status in `CHANGELOG.md` and this file.

## Verification matrix

- Rust: `cargo fmt --check`, strict Clippy, all unit/integration tests,
  `cargo package --allow-dirty`, clean `cargo publish --dry-run`, and the live
  Sail binary gate applicable to the release.
- Python: `uv sync`, the complete pytest suite, CLI smoke tests, `uv build`,
  `twine check`, and a clean virtual-environment install of the built wheel.
- TypeScript: `npm ci`, strict `tsc`, Node contract tests, `npm pack --dry-run`,
  and a clean Node install from the published npm tarball.
- Cross-language: TypeDID signing/verification fixtures and semantic output
  equivalence continue to run from `python/tests` against the root binary.
- Organization: targeted search confirms no active manifest, workflow, or
  bootstrap script still requires sibling `qg-rust`/`qg-python` checkouts.

## Rollback

Before publishing, the original `qg-rust` and `qg-python` branches remain
available. If a gate fails, revert the consolidation commit in
`querygraph/querygraph`, keep the prior repositories as release sources, and
do not publish a partially integrated artifact. After a successful release,
rollback means selecting the previous crate/PyPI versions; it does not delete
published artifacts.

## Status log

- **2026-08-06:** Confirmed the canonical `querygraph/querygraph` repository
  already exists as the workspace meta-repo. Confirmed qg-rust's released
  registry dependency line and qg-python's publishable `querygraph` package.
- **2026-08-06:** Started the explicit QGQG goal. Inventory found stale sibling
  checkout paths in the workspace README, bootstrap script, CI, and downstream
  documentation; these are migration work, not runtime reverse dependencies.
- **2026-08-06:** Imported the verified qg-rust release branch into the root,
  moved qg-python into `python/`, changed Rust metadata to crate `0.4.2`,
  changed Python metadata to package `0.4.1`, and added one unified CI workflow.
- **2026-08-06:** Rust `cargo fmt --check`, strict Clippy, 71 unit tests, and
  2 integration tests pass. Python `uv build`, `twine check`, and 52 tests
  pass, including the root-binary cross-language equivalence suite. Cargo
  package and publish dry-run pass for `querygraph 0.4.2`.
- **2026-08-06:** Published `querygraph 0.4.2` to crates.io and `querygraph
  0.4.1` to PyPI through the authenticated GitHub Actions workflow. The
  repository is public OSS; future Python releases use the same workflow.
- **2026-08-06:** Started the TypeScript API goal. Added the modular
  `typescript/` package with Croissant, CDIF, OSI, ODRL, TypeDID/Ed25519,
  lineage, navigator, MCP, Dataverse, lakehouse, and capability surfaces.
  Initial build and four Node contract tests pass; npm publication remains.
- **2026-08-06:** Published `@querygraph/querygraph@0.1.0` with public access.
  `npm test`, strict TypeScript compilation, npm pack, npm dist-tag, and
  package access checks pass. The npm registry currently reports the release
  tag but its metadata endpoint has not yet become install-visible, so clean
  registry installation remains an explicit follow-up gate.
- **Handoff:** the former repositories remain recoverable outside the canonical
  checkout; deprecation/redirect notices can be added in a separate
  compatibility window without changing the unified runtime.

## Fable review questions

1. Is the root Rust crate plus `python/` packaging boundary clear enough for
   independent Rust and Python releases?
2. Should the compatibility window for the old repository names be one release
   or two, given GitHub redirects and existing local scripts?
3. Are the retained cross-language fixtures sufficient evidence that the Python
   API remains a projection rather than a second authority implementation?
