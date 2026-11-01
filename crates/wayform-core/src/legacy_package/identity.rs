//! Content identity used by VoiceGIS packageLifecycle.ts at commit
//! 305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f. Original source and rights remain
//! in compat/voicegis/upstream/. This is distinct from mobility draft identity.

use std::cmp::Ordering;

use serde_json::Value;
use crate::identity::sha256_hex;
use crate::{IdentityError, canonical_json};

const MAX_BYTES: usize = 16 * 1024 * 1024;

// The pinned package keys are ASCII identifiers. Restrict extensions to that
// grammar rather than pretend to reproduce arbitrary host-locale collation.
fn valid_key(key: &str) -> bool {
    key.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && key.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn compare_keys(a: &str, b: &str) -> Ordering {
    a.bytes()
        .map(|byte| byte.to_ascii_lowercase())
        .cmp(b.bytes().map(|byte| byte.to_ascii_lowercase()))
        .then_with(|| {
            a.bytes()
                .map(|byte| byte.is_ascii_uppercase())
                .cmp(b.bytes().map(|byte| byte.is_ascii_uppercase()))
        })
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn write(value: &Value, out: &mut String, depth: usize, root: bool) -> Result<(), IdentityError> {
    if depth > 64 || out.len() > MAX_BYTES {
        return Err(IdentityError::ResourceLimit);
    }
    match value {
        Value::Object(values) => {
            let mut keys: Vec<_> = values
                .keys()
                .filter(|key| !(root && *key == "manifest"))
                .collect();
            if keys.iter().any(|key| !valid_key(key)) {
                return Err(IdentityError::InvalidStructure);
            }
            keys.sort_by(|a, b| compare_keys(a, b));
            out.push('{');
            for (index, key) in keys.iter().enumerate() {
                out.push_str(if index == 0 { "\n" } else { ",\n" });
                indent(out, depth + 1);
                out.push_str(
                    &serde_json::to_string(key).map_err(|_| IdentityError::InvalidStructure)?,
                );
                out.push_str(": ");
                write(&values[*key], out, depth + 1, false)?;
            }
            if !keys.is_empty() {
                out.push('\n');
                indent(out, depth);
            }
            out.push('}');
        }
        Value::Array(values) => {
            out.push('[');
            for (index, value) in values.iter().enumerate() {
                out.push_str(if index == 0 { "\n" } else { ",\n" });
                indent(out, depth + 1);
                write(value, out, depth + 1, false)?;
            }
            if !values.is_empty() {
                out.push('\n');
                indent(out, depth);
            }
            out.push(']');
        }
        _ => out.push_str(&canonical_json(value)?),
    }
    if out.len() > MAX_BYTES {
        return Err(IdentityError::ResourceLimit);
    }
    Ok(())
}

/// Hash the legacy pretty-printed content, excluding only the root manifest.
/// This checks identity serialization, not package shape or publisher trust.
pub fn legacy_package_content_hash(value: &Value) -> Result<String, IdentityError> {
    if !value.is_object() {
        return Err(IdentityError::InvalidStructure);
    }
    let mut content = String::new();
    write(value, &mut content, 0, true)?;
    content.push('\n');
    if content.len() > MAX_BYTES {
        return Err(IdentityError::ResourceLimit);
    }
    Ok(sha256_hex(content.as_bytes()))
}
