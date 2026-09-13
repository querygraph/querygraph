# Pinax 0.2.0 release review

Pinax is published as `pinax-registry`, with library and executable name `pinax`.
The repository is https://github.com/querygraph/pinax. QueryGraph depends on the
released package without a source override. Rust uses edition 2024.

The rename changes the Rust API, CLI, MCP tools, signed resources, catalog
property keys, registry and ontology wire versions, and digest domains. Existing
Fihrist documents require explicit migration; see the owner migration guide.
Historical release evidence is retained unmodified under
`demo/pinax/evidence/history/`. It is not evidence for the renamed build.

Current acceptance reports are generated under `demo/pinax/evidence/` from the
renamed build. The deployment creates isolated synthetic LakeCat/Sail services
and a fresh Pinax ontology store. Sail is built from the selected source
checkout; upstream merging is not a release gate.
