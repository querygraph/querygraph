"""Client-side TypeDID envelope auth for the qg-server `/v1` API.

The server's governed routes (`serve --require-auth`) demand an
`x-qg-envelope` header: a signed TypeDID envelope with `action == "invoke"`,
`resource` bound to the request path, and `payload.bodySha256` bound to the
request body — so an envelope can be neither replayed against another
endpoint nor attached to a different body. Requires the `crypto` extra.
For HTTP authorization, `sender` is the credential's public `did:key`;
qg-rust requires it to match `verification_method` and requires recipient
`did:web:qg-server` before using the sender as the TypeSec policy subject.
"""

from __future__ import annotations

import json
from typing import Any

from querygraph.typedid import TypeDidAgent, TypeDidEnvelope, sha256_hex

ENVELOPE_HEADER = "x-qg-envelope"


def mint_envelope_header(
    agent: TypeDidAgent,
    *,
    path: str,
    body: bytes | str = b"",
    recipient: str = "did:web:qg-server",
) -> str:
    """The `x-qg-envelope` header value authorizing one request."""
    signing_did = agent.did_key()
    if signing_did is None:
        raise RuntimeError(
            "governed HTTP requests require the querygraph[crypto] extra and a signing seed"
        )
    envelope = TypeDidEnvelope.create(
        # HTTP authorization uses the signing DID itself as the principal.
        # The server rejects a sender that differs from verification_method.
        sender=signing_did,
        recipient=recipient,
        action="invoke",
        resource=path,
        payload={"bodySha256": sha256_hex(body)},
        signer=agent.signer,
    )
    return envelope.model_dump_json(exclude_none=True)


def governed_post(
    base_url: str,
    path: str,
    payload: dict[str, Any],
    agent: TypeDidAgent,
    *,
    timeout: float = 30.0,
) -> dict[str, Any]:
    """POST to a governed `/v1` route with envelope auth. Stdlib-only."""
    import urllib.request

    body = json.dumps(payload).encode()
    request = urllib.request.Request(
        f"{base_url.rstrip('/')}{path}",
        data=body,
        headers={
            "Content-Type": "application/json",
            ENVELOPE_HEADER: mint_envelope_header(agent, path=path, body=body),
        },
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.loads(response.read())
