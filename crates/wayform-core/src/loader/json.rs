use super::{Document, LoadError};
pub use crate::json::MAX_DOCUMENT_BYTES;
use crate::{JsonError, parse_bounded_json};
use serde_json::Value;

fn check_tree(value: &Value, depth: usize) -> bool {
    if depth > 64 {
        return false;
    }
    match value {
        Value::String(value) => !value.is_empty(),
        Value::Array(values) => values.iter().all(|value| check_tree(value, depth + 1)),
        Value::Object(values) => values
            .iter()
            .all(|(key, value)| !key.is_empty() && check_tree(value, depth + 1)),
        _ => true,
    }
}

pub fn parse(bytes: &[u8], document: Document) -> Result<Value, LoadError> {
    let value = parse_bounded_json(bytes).map_err(|error| match error {
        JsonError::InvalidJson => LoadError::InvalidJson(document),
        JsonError::ResourceLimit => LoadError::ResourceLimit,
    })?;
    // Every string value in this closed draft schema has a minimum length of one.
    if !check_tree(&value, 0) {
        return Err(LoadError::contract(
            document,
            "empty string or excessive nesting",
        ));
    }
    Ok(value)
}
