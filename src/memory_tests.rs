use typesec_memory::{CognitionRecoveryError, GovernedSourceVerificationError, MemoryError};

use crate::memory::{MemoryApiError, memory_api_error};

#[test]
fn cognition_recovery_errors_are_service_failures() {
    let cases = [
        CognitionRecoveryError::PolicyUnavailable,
        CognitionRecoveryError::InvalidRequest,
        CognitionRecoveryError::Unavailable,
    ];

    for error in cases {
        assert!(matches!(
            memory_api_error(MemoryError::CognitionRecovery(error)),
            MemoryApiError::Failed(_)
        ));
    }
}

#[test]
fn governed_source_errors_keep_public_authorization_and_service_categories() {
    assert!(matches!(
        memory_api_error(MemoryError::GovernedSourceScopeMismatch),
        MemoryApiError::Denied(_)
    ));
    assert!(matches!(
        memory_api_error(MemoryError::GovernedSourceVerification(
            GovernedSourceVerificationError::Unavailable
        )),
        MemoryApiError::Failed(_)
    ));
}
