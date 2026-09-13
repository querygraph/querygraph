//! Fingerprint the retained transaction log, rejecting changed or added commits.
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn fingerprint(table: &Path, version: i64) -> Result<String> {
    ensure!(
        (0..=100).contains(&version),
        "Demo Delta version out of bounds"
    );
    let log = table.join("_delta_log");
    let mut commits = std::fs::read_dir(&log)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    commits.retain(|p| p.extension().is_some_and(|e| e == "json"));
    commits.sort();
    ensure!(
        commits.len() == version as usize + 1,
        "Delta log changed since catalog registration"
    );
    let mut hash = Sha256::new();
    for (index, path) in commits.iter().enumerate() {
        ensure!(
            path.file_name().and_then(|s| s.to_str()) == Some(format!("{index:020}.json").as_str()),
            "Unexpected Delta commit sequence"
        );
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= 1024 * 1024,
            "Delta commit exceeds demo bound"
        );
        hash.update(index.to_le_bytes());
        hash.update(bytes.len().to_le_bytes());
        hash.update(bytes);
    }
    Ok(format!("sha256:{:x}", hash.finalize()))
}
