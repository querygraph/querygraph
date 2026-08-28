# Changelog

- Add a transport-free Iceberg REST migration verifier that canonicalizes and
  compares table UUID, format, schemas, partition specs, sort orders, non-empty
  snapshots, refs, current IDs, and metadata pointer with an explicit loss report.

All notable changes to the QueryGraph Python port are recorded here. The
codename pool and the shared version line live in [`RELEASES.md`](RELEASES.md);
the canonical scheme is in `../RELEASES.md`.

## 0.5.0-dev — unreleased

### Added
- **Pydantic AI v2 credential-and-memory capabilities**:
  `querygraph.pydantic_ai_capabilities` composes two native `Capability`
  objects on every agent. One keeps the TypeDID signing credential in typed
  runtime dependencies and performs authenticated QueryGraph access; the
  other remembers, recalls, and forgets through the root service's
  TypeSec/Grust/Turso Marciana boundary. The provider-free `TestModel` demo
  starts the root QueryGraph service,
  generates exact-DID policy, stores a governed specialist finding, restarts
  the server, lets a supervisor recover it, and proves both unsigned access
  and an unassigned signed identity are denied.

### Security
- Governed HTTP envelopes now use the credential's signing `did:key` as
  `sender`, and fail early without the crypto extra. qg-rust requires that
  sender to match the envelope verification method before using it as the
  TypeSec policy subject, closing sender-impersonation ambiguity between a
  display DID and the actual signing credential.

## 0.4.0 "Sentinel" — 2026-07-04

The governed-answer release, alongside QueryGraph 0.4.0 "Sentinel": the
navigator loop answers under receipts, and `api_auth` mints the envelope
headers that qg-rust's guarded `/v1` demands. The stack beneath moved to the
0.12 substrate wave (Grust "Lobster", TypeSec "Torcello", LakeCat "Ocelot").

### Added
- **Envelope auth client** (`querygraph.api_auth`): `mint_envelope_header`
  and `governed_post` mint the `x-qg-envelope` header qg-server's
  `--require-auth` mode demands — path-bound, body-bound, Ed25519-signed.
  The equivalence suite proves it live against a running qg-server (401
  without the header, 200 with it).
- **The governed navigator loop** (`querygraph.navigator_loop`, CLI:
  `querygraph answer`, MCP tool: `answer_question`) — FABLE-REVIEW-1 P1-8.
  question → semantic-model search (synonyms + bigrams + containment) →
  RBAC+ODRL gate with receipts (denials are first-class and named in the
  prompt as off-limits) → SQL plans over allowed Sail sources → synthesis via
  any `Callable[[str], str]` (`openai_compatible_llm` binds Ollama, vLLM,
  llama.cpp, LM Studio, OpenRouter; `llm=None` is the deterministic golden
  baseline) → answer in a signed TypeDID envelope with the OpenLineage event
  (schema-validated) and Ed25519 attestation.

## 0.3.0 "Goshawk" — 2026-07-03

The interoperability release, implementing the FABLE-REVIEW-1 P0 items and
quick wins alongside qg-rust 0.3.0 "Goshawk" (see the workspace
`FABLE-REVIEW-1.md` §9).

### Added
- **Real Ed25519 signing** (`querygraph.crypto`, extra: `crypto`). TypeDID
  envelopes and lineage attestations are now signed with real Ed25519 keys
  derived deterministically from agent seeds (mirroring Rust TypeSec
  `Ed25519DidKey::from_seed`), carry a W3C `did:key` `verification_method`,
  and verify with `TypeDidEnvelope.verify_signature()` /
  `LineageAttestation.verify()`. Without the extra, digests are explicitly
  prefixed `unsigned:sha256:` so they can never be mistaken for signatures.
- **MCP server** (`querygraph.mcp_server`, extra: `mcp`; CLI:
  `querygraph mcp-serve`). Exposes the governed semantic layer over the Model
  Context Protocol: `search_semantic_model`, `resolve_metric` (with dialect
  fallback), `check_access` (RBAC+ODRL dual gate; denials are receipts, not
  errors), `build_navigator_bundle`, `run_qglake_story`, `verify_envelope`,
  plus `qg://` resources.
- **Tool-schema export.** `TypeDidAgent.to_tool_schema(flavor="openai"|"anthropic")`
  emits standard JSON-Schema tool definitions accepted by OpenAI, Anthropic,
  Mistral, vLLM, Ollama, and most agent frameworks.
- **Async adapters.** `TypeDidLangChainToolAdapter.ainvoke()` and
  `as_async_tool()` for async agent runtimes.
- **Richer OSI model** ported from the semantic-layer research repos:
  structured `ai_context` (instructions/synonyms/examples), relationships,
  primary/unique keys, `SUPPORTED_DIALECTS`, `resolve_metric()` with
  ANSI_SQL/SAIL_SQL fallback, `find_by_synonym()`, and acceptance of upstream
  OSI documents that spell `semantic_model` as a list.
- **A2A Agent Card** (`querygraph.a2a`, CLI: `querygraph agent-card`): the
  Agent2Agent v0.3.0 card qg-rust serves at `/.well-known/agent-card.json`.
  The skill list and TypeDID security scheme are a cross-language contract
  asserted by the equivalence suite.
- **Official OpenLineage schema validation**
  (`validation.validate_openlineage_schema`, extra: `validation`): the 2-0-2
  spec schema is vendored and format-checked, so interop with OSS consumers
  (Marquez, openlineage-python) is proven rather than asserted — for events
  emitted by both qg-python and qg-rust.
- **Cross-language qglake equivalence test** asserting governance semantics
  (specialist roster, denial pattern, evidence chain) match the Rust CLI,
  plus live Ed25519 round-trip (Python signs → Rust verifies) and A2A card
  parity.
- **Packaging**: `py.typed` marker, authors/urls/keywords/classifiers, new
  `crypto` and `mcp` extras, GitHub Actions CI (pytest matrix + build +
  twine check).

### Changed
- `TypeDidEnvelope` gains `verification_method`; unsigned envelopes are
  labelled `unsigned:sha256:` instead of `sha256:`. `LineageAttestation`
  signature types are now `QueryGraphEd25519Signature` /
  `QueryGraphUnsignedDigest` (was `QueryGraphDemoSha256Signature`).
- `langchain-core` dependency bounded `<2`.
- **OpenLineage run ids are now spec-conformant UUIDs**: deterministic UUIDv5
  under the QueryGraph namespace (`lineage.run_id_for`), replacing the
  `querygraph-python-…` prefixed hashes. qg-rust derives identical ids.

## 0.2.0 "Peregrine" — 2026-06-26

First **named** release. Tracks the QueryGraph stack at Grust 0.11.0 "Crab",
TypeSec 0.11.0 "Burano", and LakeCat 0.2.1 "Lynx", and stays in lock-step with
the Rust implementation (`querygraph/qg-rust` v0.2.0).

### Added
- **TypeDID attestation parity.** `TypeDidEnvelope` gains `privacy`, `profile`,
  and `envelope_digest` fields, plus a `TYPEDID_PROFILE` constant, mirroring the
  Rust port's adoption of TypeSec 0.11 "Burano"'s audit-safe
  `VerifiedTypeDidMessage::attestation()`. `TypeDidEnvelope.create()` accepts
  `privacy` / `profile` (defaulting to `"secret"` and the
  `ed25519-x25519-chacha20` profile) and computes a deterministic
  `envelope_digest` that binds the attestation to the exact envelope without
  revealing the payload.
- **`RELEASES.md`** recording the shared version line and birds-of-prey codename
  pool; **`CHANGELOG.md`** (this file).
- **README "Stack versions"** section naming the three coordinated releases
  (Grust "Crab", TypeSec "Burano", LakeCat "Lynx") and pointing to the stack
  announcement.

### Changed
- Bumped the package version `0.1.0` → `0.2.0`.
- Updated stack references throughout to TypeSec 0.11.0 "Burano" and LakeCat
  0.2.1 "Lynx" (README, `typedid.py` docstrings).

### Compatibility
- The new `TypeDidEnvelope` fields are additive and default-valued, so existing
  callers and serialized payloads keep working.
- The Rust navigator-bundle equivalence test (`tests/test_rust_equivalence.py`)
  still passes: the `navigator` semantic bundle is byte-identical between the
  Python and Rust implementations. Full suite: 12 tests passing.

## 0.1.0 — initial Python port

Initial port of the QueryGraph AI Navigator semantic layer: Croissant/CDIF/DID/
ODRL projections, OSI semantic models, Pydantic TypeDID agents, optional
LangChain adapters, OpenLineage/DID attestations, PySpark/Sail helpers, and a
CLI compatible with the Rust semantic-bundle commands.
