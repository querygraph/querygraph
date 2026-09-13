use super::*;
use std::sync::Arc;
use typesec_integrations::TypeDidGateway;

#[test]
fn fresh_credentials_pass_the_real_gateway_and_replays_remain_rejected() {
    let seed = [7_u8; 32];
    let sender_key = Ed25519DidKey::from_seed(seed);
    let verifying = ed25519_dalek::VerifyingKey::from_bytes(&sender_key.signing_public()).unwrap();
    let subject = format!("did:key:{}", crate::agent::ed25519_multibase(&verifying));
    let sender = Did::parse(&subject).unwrap();
    let receiver_key = Ed25519DidKey::from_seed([9_u8; 32]);
    let receiver = Did::key(receiver_key.signing_public());
    let document = receiver_key.document(receiver.clone());
    let credentials =
        TypeSecLakeCatCredentials::from_seed(&seed, &subject, document.clone()).unwrap();
    let resolver = StaticDidResolver::new()
        .with_document(sender_key.document(sender))
        .with_document(document);
    let gateway = TypeDidGateway::new(
        Arc::new(resolver),
        Arc::new(Ed25519DidKeyStore::new().with_key(receiver.clone(), receiver_key)),
        receiver,
    );
    let url = Url::parse("http://127.0.0.1/catalog/v1/warehouse/namespaces/acme/tables/customers")
        .unwrap();
    let first: DidEnvelope =
        serde_json::from_str(&credentials.envelope("GET", &url, b"").unwrap()).unwrap();
    let second: DidEnvelope =
        serde_json::from_str(&credentials.envelope("GET", &url, b"").unwrap()).unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(
        gateway
            .open_message(&first)
            .unwrap()
            .attestation()
            .subject
            .to_string(),
        subject
    );
    assert!(gateway.open_message(&second).is_ok());
    assert!(gateway.open_message(&first).is_err());
}

#[test]
fn configured_subject_cannot_claim_a_different_signing_key() {
    let receiver_key = Ed25519DidKey::from_seed([9_u8; 32]);
    let receiver = Did::key(receiver_key.signing_public());
    assert!(matches!(
        TypeSecLakeCatCredentials::from_seed(
            &[7; 32],
            "did:example:another",
            receiver_key.document(receiver)
        ),
        Err(LakeCatError::InvalidArgument(_))
    ));
}

#[test]
fn legacy_envelope_is_consumed_once_instead_of_replayed() {
    let credentials = OneShotEnvelope(std::sync::Mutex::new(Some("{}".into())));
    let url = Url::parse("http://127.0.0.1/").unwrap();
    assert_eq!(credentials.envelope("GET", &url, b"").unwrap(), "{}");
    assert!(matches!(
        credentials.envelope("GET", &url, b""),
        Err(LakeCatError::NotSupported(_))
    ));
}
