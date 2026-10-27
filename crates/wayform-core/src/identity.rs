use std::cmp::Ordering;
use std::fmt;

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::graph::compare_ids;

const MAX_CANONICAL_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityError {
    InvalidStructure,
    InvalidNumber,
    ResourceLimit,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidStructure => "invalid identity input structure",
            Self::InvalidNumber => "identity input requires finite binary64 numbers",
            Self::ResourceLimit => "identity input exceeds canonicalization limits",
        })
    }
}
impl std::error::Error for IdentityError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDigests {
    pub bundle_digest: String,
    pub profile_digest: String,
    pub context_digest: String,
}

fn array_index(key: &str) -> Option<u32> {
    let number = key.parse::<u32>().ok()?;
    (number != u32::MAX && number.to_string() == key).then_some(number)
}

fn compare_keys(left: &str, right: &str) -> Ordering {
    // JSON.stringify enumerates array-index property names numerically, even
    // after Object.fromEntries inserted all keys in UTF-16 lexical order.
    match (array_index(left), array_index(right)) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => compare_ids(left, right),
    }
}

fn write_value(value: &Value, output: &mut String, depth: usize) -> Result<(), IdentityError> {
    if depth > MAX_DEPTH {
        return Err(IdentityError::ResourceLimit);
    }
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => {
            let number = value
                .as_f64()
                .filter(|number| number.is_finite())
                .ok_or(IdentityError::InvalidNumber)?;
            if number == 0.0 {
                output.push('0');
            } else {
                output.push_str(ryu_js::Buffer::new().format(number));
            }
        }
        Value::String(value) => output
            .push_str(&serde_json::to_string(value).map_err(|_| IdentityError::InvalidStructure)?),
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                write_value(value, output, depth + 1)?;
            }
            output.push(']');
        }
        Value::Object(values) => {
            let mut keys: Vec<_> = values.keys().collect();
            keys.sort_by(|left, right| compare_keys(left, right));
            output.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key).map_err(|_| IdentityError::InvalidStructure)?,
                );
                output.push(':');
                write_value(&values[key], output, depth + 1)?;
            }
            output.push('}');
        }
    }
    if output.len() > MAX_CANONICAL_BYTES {
        return Err(IdentityError::ResourceLimit);
    }
    Ok(())
}

/// Reproduce the draft's ECMAScript serialization, not a generic JCS encoding.
/// This performs serialization checks only, not mobility schema validation.
pub fn canonical_json(value: &Value) -> Result<String, IdentityError> {
    let mut output = String::new();
    write_value(value, &mut output, 0)?;
    Ok(output)
}

fn digest(value: &Value) -> Result<String, IdentityError> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(canonical_json(value)?.as_bytes()) {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 15)]));
    }
    Ok(output)
}

fn array<'a>(value: &'a mut Value, field: &str) -> Result<&'a mut Vec<Value>, IdentityError> {
    value
        .get_mut(field)
        .and_then(Value::as_array_mut)
        .ok_or(IdentityError::InvalidStructure)
}

fn sort_records(value: &mut Value, field: &str, key: &str) -> Result<(), IdentityError> {
    let values = array(value, field)?;
    if values
        .iter()
        .any(|value| value.get(key).and_then(Value::as_str).is_none())
    {
        return Err(IdentityError::InvalidStructure);
    }
    values.sort_by(|left, right| {
        compare_ids(left[key].as_str().unwrap(), right[key].as_str().unwrap())
    });
    Ok(())
}

fn sort_strings(value: &mut Value, field: &str) -> Result<(), IdentityError> {
    let values = array(value, field)?;
    if values.iter().any(|value| !value.is_string()) {
        return Err(IdentityError::InvalidStructure);
    }
    values.sort_by(|left, right| compare_ids(left.as_str().unwrap(), right.as_str().unwrap()));
    Ok(())
}

/// Hash a draft bundle after normalizing its unordered collections. This does
/// not validate references, establish survey truth or authenticate a publisher.
pub fn bundle_digest(bundle: &Value) -> Result<String, IdentityError> {
    let mut normalized = bundle.clone();
    for field in [
        "frames",
        "nodes",
        "edges",
        "evidence",
        "observations",
        "transforms",
        "policyRequirements",
    ] {
        sort_records(&mut normalized, field, "id")?;
    }
    for transform in array(&mut normalized, "transforms")? {
        let controls = array(transform, "controls")?;
        let mut ordered = controls
            .iter()
            .map(|control| Ok((canonical_json(control)?, control.clone())))
            .collect::<Result<Vec<_>, IdentityError>>()?;
        ordered.sort_by(|left, right| compare_ids(&left.0, &right.0));
        *controls = ordered.into_iter().map(|(_, value)| value).collect();
    }
    for requirement in array(&mut normalized, "policyRequirements")? {
        sort_strings(requirement, "edgeIds")?;
    }
    digest(&normalized)
}

pub fn input_digests(
    bundle: &Value,
    profile: &Value,
    context: &Value,
) -> Result<InputDigests, IdentityError> {
    let mut profile = profile.clone();
    let hard = profile
        .get_mut("hard")
        .ok_or(IdentityError::InvalidStructure)?;
    for field in ["allowedSurfaces", "allowedDoorOperations"] {
        if !hard
            .get(field)
            .ok_or(IdentityError::InvalidStructure)?
            .is_null()
        {
            sort_strings(hard, field)?;
        }
    }
    sort_strings(
        profile
            .get_mut("preferences")
            .ok_or(IdentityError::InvalidStructure)?,
        "avoidKinds",
    )?;
    sort_strings(
        profile
            .get_mut("evidencePolicy")
            .ok_or(IdentityError::InvalidStructure)?,
        "acceptedSources",
    )?;
    let mut context = context.clone();
    sort_strings(&mut context, "trustedIssuerIds")?;
    sort_records(&mut context, "policies", "requirementId")?;
    for policy in array(&mut context, "policies")? {
        sort_strings(policy, "closedEdgeIds")?;
    }
    Ok(InputDigests {
        bundle_digest: bundle_digest(bundle)?,
        profile_digest: digest(&profile)?,
        context_digest: digest(&context)?,
    })
}
