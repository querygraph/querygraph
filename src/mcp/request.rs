//! Typed tool arguments. Unknown fields fail at this transport boundary.

use serde::Deserialize;
use serde_json::Value;

use crate::{agent::PyTypeDidEnvelope, navigator::NavigatorInput, osi::OsiDocument};

#[derive(Deserialize)]
#[serde(tag = "name", content = "arguments")]
pub(super) enum ToolCall {
    #[serde(rename = "validate_pinax")]
    ValidatePinax(RegistryProposal),
    #[serde(rename = "build_navigator_bundle")]
    BuildBundle(BundleArguments),
    #[serde(rename = "run_qglake_story")]
    Story(EmptyArguments),
    #[serde(rename = "verify_envelope")]
    Verify(EnvelopeArguments),
    #[serde(rename = "import_semantic_model")]
    Import(ImportArguments),
    #[serde(rename = "search_semantic_models", alias = "search_semantic_model")]
    Search(SearchArguments),
    #[serde(rename = "answer_question")]
    Answer(QuestionArguments),
}

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RegistryProposal {
    document: String,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    previous: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BundleArguments {
    dataset_name: String,
    description: String,
    landing_page: String,
    data_url: String,
    creator: String,
    agent_name: String,
}

impl From<BundleArguments> for NavigatorInput {
    fn from(value: BundleArguments) -> Self {
        Self {
            dataset_name: value.dataset_name,
            description: value.description,
            landing_page: value.landing_page,
            data_url: value.data_url,
            creator: value.creator,
            agent_name: value.agent_name,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EmptyArguments {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EnvelopeArguments {
    pub envelope: PyTypeDidEnvelope,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImportArguments {
    #[serde(default, deserialize_with = "present")]
    pub osi: Option<OsiDocument>,
    #[serde(default, deserialize_with = "present")]
    pub croissant: Option<Value>,
}

fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SearchArguments {
    pub term: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct QuestionArguments {
    pub question: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SignedRegistryArguments {
    pub intent: String,
    pub envelope: PyTypeDidEnvelope,
}
