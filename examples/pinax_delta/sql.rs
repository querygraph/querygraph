//! Closed predicate subset for the retained demo; unsupported input fails closed.
use anyhow::{Result, ensure};
use serde_json::Value;

pub fn identifier(value: &str) -> Result<String> {
    ensure!(
        !value.is_empty()
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        "Invalid fixture identifier"
    );
    Ok(format!("`{value}`"))
}

pub fn literal(value: &Value) -> Result<String> {
    match value {
        Value::String(s) => {
            ensure!(!s.contains(['\0', '\\']), "Unsupported SQL literal");
            Ok(format!("'{}'", s.replace('\'', "''")))
        }
        Value::Number(n) if n.is_i64() => Ok(n.to_string()),
        Value::Null => Ok("NULL".into()),
        _ => anyhow::bail!("Unsupported fixture value"),
    }
}
