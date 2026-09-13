use super::*;

#[test]
fn rejects_unsupported_predicates_and_sql_injection() {
    assert!(sql::identifier("id` FROM other").is_err());
    assert!(
        predicate::filters(&[json!({"type":"eq","term":"tenant","value":"acme","ignored":true})])
            .is_err()
    );
    assert!(predicate::filters(&[json!({"type":"or","left":{},"right":{}})]).is_err());
    assert_eq!(
        predicate::filters(&[json!({"type":"eq","term":"tenant","value":"acme"})]).unwrap(),
        "`tenant` = 'acme'"
    );
}

#[test]
fn log_fingerprint_rejects_extra_or_modified_commits() {
    let root = tempfile::tempdir().unwrap();
    let log_dir = root.path().join("_delta_log");
    std::fs::create_dir(&log_dir).unwrap();
    std::fs::write(log_dir.join("00000000000000000000.json"), b"initial").unwrap();
    std::fs::write(log_dir.join("00000000000000000001.json"), b"insert").unwrap();
    let before = log::fingerprint(root.path(), 1).unwrap();
    std::fs::write(log_dir.join("00000000000000000001.json"), b"changed").unwrap();
    assert_ne!(before, log::fingerprint(root.path(), 1).unwrap());
    std::fs::write(log_dir.join("00000000000000000002.json"), b"new version").unwrap();
    assert!(log::fingerprint(root.path(), 1).is_err());
}
