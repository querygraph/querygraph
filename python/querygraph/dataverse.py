from __future__ import annotations

import json
from pathlib import Path
from typing import Any
from urllib.request import urlopen

from pydantic import BaseModel, Field

from querygraph.croissant import CroissantDataset, Field as CroissantField, FileObject, RecordSet


class DataverseFile(BaseModel):
    id: int | str
    label: str
    download_url: str
    content_type: str = "application/octet-stream"


class DataverseDataset(BaseModel):
    id: int | str
    persistent_id: str
    title: str
    description: str = ""
    landing_page: str
    subjects: list[str] = Field(default_factory=list)
    keywords: list[str] = Field(default_factory=list)
    files: list[DataverseFile] = Field(default_factory=list)

    @classmethod
    def from_native_api(cls, payload: dict[str, Any]) -> "DataverseDataset":
        data = payload.get("data", payload)
        citation = data.get("latestVersion", data).get("metadataBlocks", {}).get("citation", {})
        fields = citation.get("fields", [])
        values = {_field_name(field): _field_value(field) for field in fields}
        files = []
        for file_entry in data.get("latestVersion", data).get("files", []):
            data_file = file_entry.get("dataFile", file_entry)
            file_id = data_file.get("id", file_entry.get("id", "file"))
            files.append(
                DataverseFile(
                    id=file_id,
                    label=data_file.get("filename", file_entry.get("label", str(file_id))),
                    download_url=data_file.get(
                        "downloadUrl",
                        f"https://dataverse.harvard.edu/api/access/datafile/{file_id}",
                    ),
                    content_type=data_file.get("contentType", "application/octet-stream"),
                )
            )
        return cls(
            id=data.get("id", values.get("datasetId", "dataset")),
            persistent_id=data.get("persistentId", values.get("persistentId", "")),
            title=values.get("title", data.get("title", "Dataverse dataset")),
            description=_first_text(values.get("dsDescription")) or data.get("description", ""),
            landing_page=data.get("persistentUrl", data.get("url", "")),
            subjects=_as_text_list(values.get("subject")),
            keywords=_keyword_values(values.get("keyword")),
            files=files,
        )

    @classmethod
    def from_json_file(cls, path: str | Path) -> "DataverseDataset":
        return cls.from_native_api(json.loads(Path(path).read_text()))

    @classmethod
    def fetch(cls, url: str) -> "DataverseDataset":
        with urlopen(url) as response:  # nosec - user-supplied CLI/library URL.
            return cls.from_native_api(json.loads(response.read().decode("utf-8")))

    def to_croissant(self) -> CroissantDataset:
        dataset_id = f"{self.landing_page.rstrip('/')}/#dataset" if self.landing_page else f"urn:dataverse:{self.id}"
        return CroissantDataset(
            id=dataset_id,
            name=self.title,
            description=self.description,
            license="https://creativecommons.org/licenses/by/4.0/",
            creators=["Dataverse"],
            files=[
                FileObject(
                    id=f"{dataset_id}/file/{file.id}",
                    name=file.label,
                    content_url=file.download_url,
                    encoding_format=file.content_type,
                )
                for file in self.files
            ],
            record_sets=[
                RecordSet(
                    id=f"{dataset_id}/recordset/files",
                    name="Dataverse files",
                    fields=[
                        CroissantField(
                            "dataset_persistent_id",
                            "sc:Text",
                            "Dataverse persistent dataset identifier.",
                        ).semantic_type("https://schema.org/identifier"),
                        CroissantField(
                            "file_name",
                            "sc:Text",
                            "Dataverse file name.",
                        ).semantic_type("https://schema.org/name"),
                        CroissantField(
                            "download_url",
                            "sc:URL",
                            "Dataverse file download URL.",
                        ).semantic_type("https://schema.org/contentUrl"),
                    ],
                )
            ],
            keywords=[*self.subjects, *self.keywords],
        )


def _field_name(field: dict[str, Any]) -> str:
    return str(field.get("typeName", field.get("name", "")))


def _field_value(field: dict[str, Any]) -> Any:
    return field.get("value", field.get("values"))


def _first_text(value: Any) -> str | None:
    if isinstance(value, list) and value:
        item = value[0]
        if isinstance(item, dict):
            return str(item.get("dsDescriptionValue", item.get("value", "")))
        return str(item)
    if isinstance(value, dict):
        return str(value.get("dsDescriptionValue", value.get("value", "")))
    if value:
        return str(value)
    return None


def _as_text_list(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, list):
        return [str(item) for item in value]
    return [str(value)]


def _keyword_values(value: Any) -> list[str]:
    if not isinstance(value, list):
        return _as_text_list(value)
    out = []
    for item in value:
        if isinstance(item, dict):
            keyword = item.get("keywordValue") or item.get("value")
            if keyword:
                out.append(str(keyword))
        else:
            out.append(str(item))
    return out
