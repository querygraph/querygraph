from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum

from querygraph.croissant import CroissantDataset


class CdifProfile(Enum):
    DISCOVERY = "https://cdif.codata.org/profile/discovery"
    MANIFEST = "https://cdif.codata.org/profile/manifest"
    DATA_DESCRIPTION = "https://cdif.codata.org/profile/data-description"
    DATA_ACCESS = "https://cdif.codata.org/profile/data-access"
    ACCESS_RIGHTS = "https://cdif.codata.org/profile/access-rights"
    CONTROLLED_VOCABULARIES = "https://cdif.codata.org/profile/controlled-vocabularies"
    DATA_INTEGRATION = "https://cdif.codata.org/profile/data-integration"
    UNIVERSALS = "https://cdif.codata.org/profile/universals"
    PROVENANCE = "https://cdif.codata.org/profile/provenance"

    def iri(self) -> str:
        return self.value


@dataclass(frozen=True)
class CdifDistribution:
    id: str
    name: str
    content_url: str
    encoding_format: str


@dataclass(frozen=True)
class CdifDataElement:
    id: str
    name: str
    data_type: str
    description: str
    semantic_type: str | None
    record_set: str


@dataclass(frozen=True)
class CdifAccessRights:
    license: str
    policy_id: str | None = None
    rights_statement: str | None = None
    odrl_policy: dict | None = None


@dataclass(frozen=True)
class CdifResource:
    dataset_id: str
    title: str
    description: str
    profiles: list[CdifProfile]
    landing_page: str
    access_service: str
    distributions: list[CdifDistribution] = field(default_factory=list)
    data_elements: list[CdifDataElement] = field(default_factory=list)
    access_rights: CdifAccessRights | None = None
    temporal_coverage: str | None = None
    spatial_coverage: str | None = None
    units: list[str] = field(default_factory=list)
    vocabularies: list[str] = field(default_factory=list)
    keywords: list[str] = field(default_factory=list)

    @classmethod
    def from_croissant(
        cls, dataset: CroissantDataset, landing_page: str, access_service: str
    ) -> "CdifResource":
        distributions = [
            CdifDistribution(
                id=file.id,
                name=file.name,
                content_url=file.content_url,
                encoding_format=file.encoding_format,
            )
            for file in dataset.files
        ]
        data_elements = [
            CdifDataElement(
                id=f"{record_set.id}/field/{field.name}",
                name=field.name,
                data_type=field.data_type,
                description=field.description,
                semantic_type=field.semantic_type_value,
                record_set=record_set.id,
            )
            for record_set in dataset.record_sets
            for field in record_set.fields
        ]
        return cls(
            dataset_id=dataset.id,
            title=dataset.name,
            description=dataset.description,
            profiles=[
                CdifProfile.DISCOVERY,
                CdifProfile.MANIFEST,
                CdifProfile.DATA_DESCRIPTION,
                CdifProfile.DATA_ACCESS,
                CdifProfile.ACCESS_RIGHTS,
                CdifProfile.CONTROLLED_VOCABULARIES,
                CdifProfile.DATA_INTEGRATION,
                CdifProfile.UNIVERSALS,
            ],
            landing_page=landing_page,
            access_service=access_service,
            distributions=distributions,
            data_elements=data_elements,
            access_rights=CdifAccessRights(
                license=dataset.license,
                rights_statement=(
                    "Access and usage must satisfy the attached ODRL/TypeSec "
                    "policy before agent use."
                ),
            ),
            vocabularies=[
                element.semantic_type
                for element in data_elements
                if element.semantic_type is not None
            ],
            keywords=dataset.keywords,
        )

    def with_odrl_policy(self, policy_id: str, policy: dict) -> "CdifResource":
        rights = self.access_rights or CdifAccessRights(license="")
        return CdifResource(
            dataset_id=self.dataset_id,
            title=self.title,
            description=self.description,
            profiles=self.profiles,
            landing_page=self.landing_page,
            access_service=self.access_service,
            distributions=self.distributions,
            data_elements=self.data_elements,
            access_rights=CdifAccessRights(
                license=rights.license,
                policy_id=policy_id,
                rights_statement=rights.rights_statement,
                odrl_policy=policy,
            ),
            temporal_coverage=self.temporal_coverage,
            spatial_coverage=self.spatial_coverage,
            units=self.units,
            vocabularies=self.vocabularies,
            keywords=self.keywords,
        )

    def to_json_ld(self) -> dict:
        return {
            "@context": {
                "cdif": "https://cdif.codata.org/",
                "dcat": "http://www.w3.org/ns/dcat#",
                "dct": "http://purl.org/dc/terms/",
                "odrl": "http://www.w3.org/ns/odrl/2/",
            },
            "@type": "dcat:Dataset",
            "@id": self.dataset_id,
            "dct:title": self.title,
            "dct:description": self.description,
            "cdif:profile": [profile.iri() for profile in self.profiles],
            "dcat:landingPage": self.landing_page,
            "dcat:accessService": {
                "@type": "dcat:DataService",
                "endpointURL": self.access_service,
            },
            "dcat:distribution": [
                {
                    "@type": "dcat:Distribution",
                    "@id": distribution.id,
                    "dct:title": distribution.name,
                    "dcat:downloadURL": distribution.content_url,
                    "dcat:mediaType": distribution.encoding_format,
                }
                for distribution in self.distributions
            ],
            "cdif:dataElement": [
                {
                    "@type": "cdif:DataElement",
                    "@id": element.id,
                    "dct:title": element.name,
                    "dct:description": element.description,
                    "cdif:dataType": element.data_type,
                    "cdif:semanticType": element.semantic_type,
                    "cdif:recordSet": element.record_set,
                }
                for element in self.data_elements
            ],
            "dct:accessRights": (
                {
                    "@type": "dct:RightsStatement",
                    "@id": self.access_rights.policy_id,
                    "dct:license": self.access_rights.license,
                    "dct:description": self.access_rights.rights_statement,
                    "odrl:policy": self.access_rights.odrl_policy,
                }
                if self.access_rights is not None
                else None
            ),
            "dct:temporal": self.temporal_coverage,
            "dct:spatial": self.spatial_coverage,
            "cdif:unit": self.units,
            "cdif:controlledVocabulary": self.vocabularies,
            "dcat:keyword": self.keywords,
        }
