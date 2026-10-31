use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use super::{Document, LoadError};

pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

// Preserve JSON numbers while rejecting duplicate names at every nesting level.
// Decoding directly into Value would silently keep only the final occurrence.
struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON without duplicate object names")
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|value| UniqueValue(Value::Number(value)))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut input: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(UniqueValue(value)) = input.next_element()? {
                    values.push(value);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut input: A) -> Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some(key) = input.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom("duplicate object name"));
                    }
                    let UniqueValue(value) = input.next_value()?;
                    values.insert(key, value);
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(JsonVisitor)
    }
}

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
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(LoadError::ResourceLimit);
    }
    let UniqueValue(value) =
        serde_json::from_slice(bytes).map_err(|_| LoadError::InvalidJson(document))?;
    // Every string value in this closed draft schema has a minimum length of one.
    if !check_tree(&value, 0) {
        return Err(LoadError::contract(
            document,
            "empty string or excessive nesting",
        ));
    }
    Ok(value)
}
