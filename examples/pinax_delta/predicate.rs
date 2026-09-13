use super::sql::{identifier, literal};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::Value;
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
enum Predicate {
    Eq {
        term: String,
        value: Value,
    },
    And {
        left: Box<Predicate>,
        right: Box<Predicate>,
    },
}
impl Predicate {
    fn sql(&self) -> Result<String> {
        match self {
            Self::Eq { term, value } => {
                ensure!(!value.is_null(), "Null equality is unsupported");
                Ok(format!("{} = {}", identifier(term)?, literal(value)?))
            }
            Self::And { left, right } => Ok(format!("({}) AND ({})", left.sql()?, right.sql()?)),
        }
    }
}

pub fn filters(values: &[Value]) -> Result<String> {
    values
        .iter()
        .map(|v| serde_json::from_value::<Predicate>(v.clone())?.sql())
        .collect::<Result<Vec<_>>>()
        .map(|parts| parts.join(" AND "))
}
