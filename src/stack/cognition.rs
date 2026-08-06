//! Marciana cognition integration boundary.
//!
//! The existing governed cognition application remains the authoritative
//! implementation. This module is the reserved QueryGraph ownership point for
//! its adapter and will grow only with named domain operations.

pub use marciana_cognition::{
    CLAIM_ALGORITHM, CLAIM_ALGORITHM_VERSION, CLAIM_CATALOG_IDENTITY, CLAIM_FIELD_MAPPING_DIGEST,
    CLAIM_FORMATION_PROFILE, CLAIM_GRANT_ID, CLAIM_INTENT_VERSION, CLAIM_JOB_ID, CLAIM_OPERATION,
    CLAIM_SOURCE_SELECTION_DIGEST, COGNITION_ACTION, COGNITION_INTENT_VERSION,
    CognitionApplicationError, CognitionBindingError, CognitionEngineBinding, CognitionMemoryError,
    FormationProfile, FreshLakeCatAuthority, GovernedCognitionApplication, GovernedCognitionConfig,
    GovernedCognitionResult, LakeCatAuthorityError, LakeCatCognitionAuthority,
    cognition_field_mapping_digest, cognition_source_selection_digest,
};
