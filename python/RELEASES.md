# QueryGraph Python releases

The Python API shares the version line and codenames of the Rust
implementation. The canonical release scheme and codename pool (birds of prey)
live in `../RELEASES.md`. Detailed per-release notes are in
[`CHANGELOG.md`](CHANGELOG.md).

## Release log

| Version | Codename | Notes |
|---|---|---|
| 0.4.1 | Sentinel Patch | Python API relocated into the canonical `querygraph/querygraph` repository under `python/`; public imports and CLI remain compatible. |
| 0.4.0 | Sentinel | The **governed-answer** release. `GovernedNavigatorLoop` (CLI `querygraph answer`, MCP tool `answer_question`): semantic search → RBAC+ODRL receipts → SQL plans over allowed sources → any LLM or the deterministic baseline → signed envelope + schema-valid OpenLineage + attestation. `api_auth` mints the path- and body-bound Ed25519 `x-qg-envelope` headers for qg-rust's `--require-auth` mode, proven live in the equivalence suite. Tracks Grust 0.12.0 "Lobster", TypeSec 0.12.0 "Torcello", LakeCat 0.3.0 "Ocelot". |
| 0.3.0 | Goshawk | The **interoperability** release. Real Ed25519 signing (`crypto` extra) with `did:key` verification methods, verified cross-language by qg-rust. MCP server (`mcp-serve`), A2A Agent Card, OpenAI/Anthropic tool-schema export, async LangChain adapters, enriched OSI model, official OpenLineage 2-0-2 schema validation with spec-conformant UUIDv5 run ids, PyPI-ready packaging, CI. |
| 0.2.0 | Peregrine | Tracks Grust 0.11.0 "Crab", TypeSec 0.11.0 "Burano", LakeCat 0.2.1 "Lynx". TypeDID attestation parity (privacy / profile / envelope digest) with the Rust port. |
| 0.1.0 | — | (pre-codename) Initial Python port. |
