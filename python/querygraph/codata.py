from __future__ import annotations

import json
from dataclasses import dataclass
from urllib.parse import urlencode
from urllib.request import urlopen


@dataclass(frozen=True)
class StoredPayload:
    url: str | None = None
    timestamp: str | None = None
    title: str | None = None
    is_rdf: bool | None = None


@dataclass(frozen=True)
class AnchoredDid:
    did: str
    doc: dict | None = None
    stored_payload: StoredPayload | None = None


class CodataOdrlClient:
    def __init__(self, base_url: str = "https://odrl.dev.codata.org") -> None:
        self.base_url = base_url.rstrip("/")

    def create_did_from_url(self, url: str) -> AnchoredDid:
        query = urlencode({"url": url})
        with urlopen(f"{self.base_url}/api/did/create_from_url?{query}") as response:
            payload = json.loads(response.read().decode())

        stored_payload = payload.get("stored_payload")
        return AnchoredDid(
            did=payload["did"],
            doc=payload.get("doc"),
            stored_payload=StoredPayload(**stored_payload) if stored_payload else None,
        )
