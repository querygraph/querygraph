# Stateless MCP and central ontology discovery

QueryGraph implements MCP **2026-07-28** over stdio and Streamable HTTP. Modern
requests need no initialization or protocol session. Each request supplies its
version and client capabilities. Existing stdio clients can still initialize
with the supported 2024/2025 revisions and retain their original behavior.
The Python CLI's Rust handoff executes the same server; the independent Python
FastMCP implementation is not the modern authoritative server.

## Start and connect

```sh
querygraph mcp-serve --registry-config /path/to/registry-service.json \
  --listen 127.0.0.1:18082
```

HTTP binds only to loopback. On grust the persistent service is
`querygraph-pinax-mcp`. Forward it with:

```sh
ssh -N -L 18082:127.0.0.1:18082 grust
```

Connect a modern MCP client to `http://127.0.0.1:18082/mcp`. To use stdio,
omit `--listen`. HTTP deliberately supports the modern revision only; older
clients retain the existing stdio path.

```sh
curl http://127.0.0.1:18082/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: server/discover' \
  --data '{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}'
```

Every successful result has `resultType: "complete"` and server information in
`_meta`. `tools/list` includes `discover_pinax_ontology`, table discovery,
planning and execution alongside the original semantic tools. Tool definitions
are stable across connections. No protocol session ID is issued or consumed.
Discovery and tool-list responses include `ttlMs: 0` and `cacheScope: "public"`:
these contain only fixed public protocol definitions. Governed ontology results
remain permission-filtered, with no shared-cache permission.

## Discover our ontologies

Call `discover_pinax_ontology` with `arguments.intent` containing serialized
purpose/query/limit/cursor and `arguments.envelope` containing its TypeDID
signature. The HTTP request also needs `Mcp-Method: tools/call` and
`Mcp-Name: discover_pinax_ontology`. Sign the exact intent bytes for resource
`/mcp/discover_pinax_ontology`; protocol metadata is not authentication.

The existing host-configured central store supplies reviewed concepts, aliases,
field mappings, match explanations and the exact ontology digest. Pinax
filters metadata using current table/field policy before constructing Croissant,
CDIF and SKOS exports. The endpoint never exposes the whole private ontology as
an unsigned resource, and discovery does not authorize a read. A governed read
requires its own purpose-bound execution signature and current authorization.

The executable [acceptance client](../examples/pinax-mcp-client.rs) performs
server discovery, lists tools, resolves `account number` through the central
ontology, derives the physical table/field, executes a separately signed read,
and verifies purpose denial and signature tampering:

```sh
target/debug/examples/pinax-mcp-client --url http://127.0.0.1:18082/mcp
```

It uses the public synthetic demo identity. Production clients supply their own
identity and permissions. HTTP supports our custom signed-intent authentication;
it does not advertise OAuth discovery or the optional Tasks, Apps, subscription,
sampling or elicitation extensions.

## Explicit application state

In modern mode, `import_semantic_model` returns `model_handle` and
`expires_in_seconds`. Pass `model_handles: ["..."]` in the arguments of
`search_semantic_models` (or its singular alias) and `answer_question`.
Omitting handles selects no imported models. Requests never inherit imports
from the same connection, client name, previous request or legacy session.

Handles refer to immutable documents, are server-generated random UUIDs, and
expire after one hour or process restart. Possession grants access to that
imported document; keep a handle private if the document is private. They never
grant lakehouse access. The store is bounded to 128 models and 64 MiB, with no
silent eviction of live entries. Multiple replicas require routing to the same
application-object store or a future shared handle backend; protocol-level
session affinity is not used. Central ontology discovery already reads the
configured durable ontology store independently on every request.

## Transport invariants and compatibility

- HTTP accepts one request per POST, never batches or client response messages.
  GET and DELETE return 405; unsupported RPC methods return 404 with `-32601`.
- Version/method/name headers must agree with the body. Missing, duplicate,
  malformed or mismatched required headers return 400/`-32020`. Encoded
  `Mcp-Name` values use the standard Base64 sentinel and are decoded once.
- Unsupported versions return 400/`-32022` with supported/requested versions.
  Missing required per-request metadata returns 400/`-32602`.
- Origin validation applies to all endpoint methods. No browser Origin is
  accepted by default; `--allow-origin` adds an exact trusted value. The endpoint
  is intended for machine clients/SSH or an authenticated reverse proxy, not
  anonymous Internet exposure. No OAuth interoperability is claimed.
- Governed HTTP requests return request-scoped SSE, with no replay event IDs,
  session headers or standalone GET streams. Dropping the stream cancels its
  future and releases its concurrency permit. No worker task is detached.
- Stdio cancellation retains the request-ID notification mechanism. Numeric
  and string IDs are distinct; modern and legacy requests can coexist without
  sharing imported models or protocol metadata.
- Input is bounded to 32 MiB, HTTP admission to 16 requests, body reads to
  30 seconds and governed work to 30 seconds. Central store reads already have
  bounded blocking I/O; cancellation cannot interrupt an OS read already in
  progress, but it cannot independently publish or release a response.
- Read-only protocol metadata uses JSON responses; governed operations use SSE.
  HTTP client notifications are rejected because no applicable notification
  extension is advertised. Modern calls do not change legacy response fields.

## Specification and validation

The implementation targets the published [2026-07-28 release](https://blog.modelcontextprotocol.io/posts/2026-07-28/),
[stateless protocol rules](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/basic/index.mdx),
[versioning](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/basic/versioning.mdx),
and [Streamable HTTP](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/basic/transports/streamable-http.mdx).

Separate Rust tests cover handshake-free requests, required metadata, unsupported
versions, explicit model isolation/expiry, HTTP header and Origin rejection,
Base64 names, response framing, signature-bound central discovery, and dropping
pending catalog work on disconnect. Legacy MCP regression tests remain active.
Live evidence and independent official JSON Schema validation are recorded under
`demo/pinax/evidence/`; this is not a claim to implement optional extensions.

Verified September 12, 2026: 121 Rust unit tests, five integration tests, three
console tests and 82 Python regression/conformance tests passed with no skips.
Strict all-target Clippy, formatting and documentation checks passed. Live
HTTP and direct/Python-handoff stdio resolve the reviewed ontology alias and
return exactly `{"id":2}` with the matching ontology digest. Independent schema
validation passed 12 captured responses and two signed requests.

- `demo/pinax/evidence/grust-mcp-stateless.json`: live HTTP exchanges.
- `demo/pinax/evidence/grust-mcp-stateless-stdio.json`: both modern stdio paths.
- `demo/pinax/evidence/mcp-stateless-schema-validation.json`: official schema check.

The official schema is vendored unchanged in
`python/tests/fixtures/mcp-2026-07-28/` with upstream license/provenance. CI's
`QG_RUST_MCP_BIN` enables independent wire conformance through the actual Rust
executable and Python handoff. This test tooling does not implement a second
modern protocol server.
