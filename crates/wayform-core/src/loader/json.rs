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

pub fn timestamp(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 24 {
        return None;
    }
    for (index, expected) in [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'.'),
        (23, b'Z'),
    ] {
        if bytes[index] != expected {
            return None;
        }
    }
    let number = |start: usize, end: usize| -> Option<i64> {
        bytes[start..end].iter().try_fold(0, |value, digit| {
            digit
                .is_ascii_digit()
                .then(|| value * 10 + i64::from(digit - b'0'))
        })
    };
    let year = number(0, 4)?;
    let month = number(5, 7)?;
    let day = number(8, 10)?;
    let hour = number(11, 13)?;
    let minute = number(14, 16)?;
    let second = number(17, 19)?;
    let millisecond = number(20, 23)?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&month)
        || day < 1
        || day > months[(month - 1) as usize]
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    // Count Gregorian days from year 0000, including its leap day.
    let before_year =
        |year: i64| 365 * year + (year + 3) / 4 - (year + 99) / 100 + (year + 399) / 400;
    let days = before_year(year) - before_year(1970)
        + months[..(month - 1) as usize].iter().sum::<i64>()
        + day
        - 1;
    Some(((days * 24 + hour) * 60 + minute) * 60_000 + second * 1000 + millisecond)
}

#[cfg(test)]
mod tests {
    use super::timestamp;
    #[test]
    fn utc_format_calendar_and_extremes() {
        for (text, expected) in [
            ("0000-01-01T00:00:00.000Z", -62_167_219_200_000),
            ("1970-01-01T00:00:00.000Z", 0),
            ("1969-12-31T23:59:59.999Z", -1),
            ("2000-03-01T00:00:00.000Z", 951_868_800_000),
            ("9999-12-31T23:59:59.999Z", 253_402_300_799_999),
        ] {
            assert_eq!(timestamp(text), Some(expected), "{text}");
        }
        for text in [
            "1900-02-29T00:00:00.000Z",
            "2026-02-29T00:00:00.000Z",
            "2026-04-31T00:00:00.000Z",
            "2026-01-01T24:00:00.000Z",
            "2026-01-01T00:00:60.000Z",
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00.000z",
            "2026-01-01T00:00:00.000+00:00",
        ] {
            assert_eq!(timestamp(text), None, "{text}");
        }
        assert!(timestamp("0000-02-29T00:00:00.000Z").is_some());
        assert!(timestamp("2000-02-29T00:00:00.000Z").is_some());
    }
}
