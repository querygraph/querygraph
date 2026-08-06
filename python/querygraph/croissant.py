from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class Field:
    name: str
    data_type: str
    description: str
    semantic_type_value: str | None = None

    def semantic_type(self, semantic_type: str) -> "Field":
        return Field(self.name, self.data_type, self.description, semantic_type)


@dataclass(frozen=True)
class FileObject:
    id: str
    name: str
    content_url: str
    encoding_format: str


@dataclass(frozen=True)
class RecordSet:
    id: str
    name: str
    fields: list[Field] = field(default_factory=list)


@dataclass(frozen=True)
class CroissantDataset:
    id: str
    name: str
    description: str
    license: str
    creators: list[str]
    files: list[FileObject]
    record_sets: list[RecordSet]
    keywords: list[str]

    def to_json_ld(self) -> dict:
        return {
            "@context": {
                "@vocab": "https://schema.org/",
                "cr": "http://mlcommons.org/croissant/",
                "dcat": "http://www.w3.org/ns/dcat#",
                "odrl": "http://www.w3.org/ns/odrl/2/",
            },
            "@type": "cr:Dataset",
            "@id": self.id,
            "name": self.name,
            "description": self.description,
            "license": self.license,
            "creator": [{"@type": "Person", "name": name} for name in self.creators],
            "keywords": self.keywords,
            "distribution": [
                {
                    "@type": "cr:FileObject",
                    "@id": file.id,
                    "name": file.name,
                    "contentUrl": file.content_url,
                    "encodingFormat": file.encoding_format,
                }
                for file in self.files
            ],
            "recordSet": [
                {
                    "@type": "cr:RecordSet",
                    "@id": record_set.id,
                    "name": record_set.name,
                    "field": [
                        {
                            "@type": "cr:Field",
                            "name": field.name,
                            "dataType": field.data_type,
                            "description": field.description,
                            "sameAs": field.semantic_type_value,
                        }
                        for field in record_set.fields
                    ],
                }
                for record_set in self.record_sets
            ],
        }
