//! QR payload resolution from the pinned VoiceGIS anchorCheckIn.ts.
use super::LegacyVenuePackage;
use super::directions::distance;
use crate::LegacySearchError;
use serde_json::{Value, json};

fn js_whitespace(c: char) -> bool {
    matches!(c,'\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{00a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}'|'\u{feff}')
}

pub(super) fn resolve(
    venue: &LegacyVenuePackage,
    payload: &str,
) -> Result<Value, LegacySearchError> {
    let wanted = payload.trim_matches(js_whitespace);
    let anchor = venue.value["localizationAnchors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["payload"].as_str() == Some(wanted));
    let Some(anchor) = anchor.filter(|_| !wanted.is_empty()) else {
        return Ok(json!({"ok":false,"reason":"unknown-code"}));
    };
    if anchor["kind"] != "qr" {
        return Ok(json!({"ok":false,"reason":"not-a-checkin-code"}));
    }
    let on_floor: Vec<_> = venue
        .runtime
        .nodes()
        .iter()
        .filter(|n| anchor["floorId"] == n.floor)
        .collect();
    let in_space: Vec<_> = on_floor
        .iter()
        .copied()
        .filter(|n| {
            anchor["spaceId"] == n.source_id && matches!(n.r#type.as_str(), "space" | "waypoint")
        })
        .collect();
    let standable: Vec<_> = on_floor
        .iter()
        .copied()
        .filter(|n| !matches!(n.r#type.as_str(), "portal" | "connector-stop"))
        .collect();
    let candidates = if !in_space.is_empty() {
        in_space
    } else if !standable.is_empty() {
        standable
    } else {
        on_floor
    };
    let x = anchor["position"][0].as_f64().unwrap();
    let y = anchor["position"][1].as_f64().unwrap();
    let mut best: Option<(&super::LegacyRuntimeNode, f64)> = None;
    for node in candidates {
        let length = distance(node.x - x, node.y - y);
        if !length.is_finite() {
            return Err(LegacySearchError::UnrepresentableCost);
        }
        if best.is_none_or(|(previous, d)| {
            length < d
                || (length == d
                    && node
                        .id
                        .encode_utf16()
                        .cmp(previous.id.encode_utf16())
                        .is_lt())
        }) {
            best = Some((node, length));
        }
    }
    Ok(match best {
        Some((node, length)) => {
            json!({"ok":true,"anchor":anchor,"nodeId":node.id,"distanceMeters":length})
        }
        None => json!({"ok":false,"reason":"no-node-on-floor"}),
    })
}
