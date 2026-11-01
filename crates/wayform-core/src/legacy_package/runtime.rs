//! Visitor metadata derived from the pinned VoiceGIS compiledBuilding runtime.
//! See docs/compatibility.md and compat/voicegis/source-lock.json.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyPoi {
    pub source_id: String,
    pub name: String,
    pub category: String,
    pub icon: String,
    pub description: String,
    pub floor_id: String,
    pub floor_name: String,
    pub space_id: String,
    pub space_name: String,
    pub r#where: String,
    pub public: bool,
    /// The author's legacy flag, not a measurement or suitability assessment.
    pub accessible: bool,
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyRuntimeNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub floor: String,
    pub floor_name: String,
    pub r#type: String,
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poi: Option<LegacyPoi>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyRuntimeEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub distance: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corridor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_type: Option<String>,
    pub accessible: bool,
    pub restricted: bool,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_kind: Option<String>,
    pub source_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyRuntime {
    nodes: Vec<LegacyRuntimeNode>,
    edges: Vec<LegacyRuntimeEdge>,
    default_start_node_id: String,
    #[serde(skip)]
    node_index: HashMap<String, usize>,
    #[serde(skip)]
    edge_index: HashMap<String, usize>,
}

pub(super) fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().expect("validated package string")
}
fn records(value: &Value) -> HashMap<&str, &Value> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| (text(v, "id"), v))
        .collect()
}

fn icon(category: &str) -> String {
    match category {
        "medical" => "M",
        "diagnostic" => "D",
        "pharmacy" => "Rx",
        "service" => "S",
        "entrance" => "E",
        "restroom" => "WC",
        "staff" => "ID",
        "emergency" => "ER",
        "landmark" => "A",
        "food" => "F",
        "family" => "♥",
        "research" => "R",
        "education" => "ED",
        // Unknown non-BMP category names use a whole Unicode character here.
        // The old JS runtime sliced a UTF-16 code unit (a lone surrogate).
        _ => {
            return category
                .chars()
                .next()
                .map(|c| c.to_uppercase().collect())
                .unwrap_or("P".into());
        }
    }
    .into()
}

fn where_at(package: &Value, floor_name: &str, point: &Value) -> String {
    let Some(site) = package.get("site") else {
        return floor_name.into();
    };
    let x = point[0].as_f64().unwrap();
    let y = point[1].as_f64().unwrap();
    for building in site["buildings"].as_array().unwrap() {
        let ring = building["footprint"].as_array().unwrap();
        let mut inside = false;
        let mut j = ring.len() - 1;
        for i in 0..ring.len() {
            let xi = ring[i][0].as_f64().unwrap();
            let yi = ring[i][1].as_f64().unwrap();
            let xj = ring[j][0].as_f64().unwrap();
            let yj = ring[j][1].as_f64().unwrap();
            if (yi > y) != (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi {
                inside = !inside;
            }
            j = i;
        }
        if inside {
            return format!(
                "{} · {}",
                text(building, "name"),
                floor_name.split(" · ").next().unwrap()
            );
        }
    }
    "Grounds".into()
}

// Keep the original string key and last-edge-wins behavior for legacy parity,
// including its separator ambiguity. WAYFORM directed edges use edge IDs.
fn pair_key(a: &str, b: &str) -> String {
    if a.encode_utf16().cmp(b.encode_utf16()).is_lt() {
        format!("{a}::{b}")
    } else {
        format!("{b}::{a}")
    }
}

impl LegacyRuntime {
    pub(super) fn from_validated(package: &Value) -> Self {
        let floors = records(&package["floors"]);
        let spaces = records(&package["spaces"]);
        let pois = records(&package["pois"]);
        let connectors = records(&package["verticalConnectors"]);
        let nodes: Vec<_> = package["routing"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| {
                let floor_id = text(node, "floorId");
                let floor_name = text(floors[floor_id], "name");
                let poi = (text(node, "kind") == "poi").then(|| {
                    let poi = pois[text(node, "sourceId")];
                    let space = spaces[text(poi, "spaceId")];
                    LegacyPoi {
                        source_id: text(poi, "id").into(),
                        name: text(poi, "name").into(),
                        category: text(poi, "category").into(),
                        icon: icon(text(poi, "category")),
                        description: format!("{} · {floor_name}", text(space, "name")),
                        floor_id: floor_id.into(),
                        floor_name: floor_name.into(),
                        space_id: text(space, "id").into(),
                        space_name: text(space, "name").into(),
                        r#where: where_at(package, floor_name, &poi["position"]),
                        public: poi["public"].as_bool().unwrap()
                            && space["public"].as_bool().unwrap(),
                        accessible: poi["accessible"].as_bool().unwrap()
                            && space["accessible"].as_bool().unwrap(),
                        aliases: poi
                            .get("aliases")
                            .map(|a| {
                                a.as_array()
                                    .unwrap()
                                    .iter()
                                    .map(|s| s.as_str().unwrap().into())
                                    .collect()
                            })
                            .unwrap_or_default(),
                    }
                });
                LegacyRuntimeNode {
                    id: text(node, "id").into(),
                    x: node["position"][0].as_f64().unwrap(),
                    y: node["position"][1].as_f64().unwrap(),
                    floor: floor_id.into(),
                    floor_name: floor_name.into(),
                    r#type: text(node, "kind").into(),
                    source_id: text(node, "sourceId").into(),
                    poi,
                }
            })
            .collect();
        let edges: Vec<_> = package["routing"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| {
                let vertical = text(edge, "kind") == "vertical-connector";
                let space = edge["spaceId"]
                    .as_str()
                    .and_then(|id| spaces.get(id))
                    .copied();
                LegacyRuntimeEdge {
                    id: text(edge, "id").into(),
                    from: text(edge, "from").into(),
                    to: text(edge, "to").into(),
                    distance: edge["distanceMeters"].as_f64().unwrap(),
                    corridor: if vertical {
                        Some(text(connectors[text(edge, "sourceId")], "name").into())
                    } else {
                        space.map(|s| text(s, "name").into())
                    },
                    space_type: if vertical {
                        None
                    } else {
                        space.map(|s| text(s, "type").into())
                    },
                    accessible: edge["accessible"].as_bool().unwrap(),
                    restricted: edge["restricted"].as_bool().unwrap(),
                    kind: text(edge, "kind").into(),
                    connector_kind: edge["connectorKind"].as_str().map(str::to_owned),
                    source_id: text(edge, "sourceId").into(),
                }
            })
            .collect();
        let default_start_node_id = nodes
            .iter()
            .find(|node| {
                node.poi.as_ref().is_some_and(|poi| {
                    poi.public && poi.space_id == text(&package["building"], "entrySpaceId")
                })
            })
            .unwrap()
            .id
            .clone();
        let node_index = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();
        let edge_index = edges
            .iter()
            .enumerate()
            .map(|(i, e)| (pair_key(&e.from, &e.to), i))
            .collect();
        Self {
            nodes,
            edges,
            default_start_node_id,
            node_index,
            edge_index,
        }
    }

    pub fn nodes(&self) -> &[LegacyRuntimeNode] {
        &self.nodes
    }
    pub fn edges(&self) -> &[LegacyRuntimeEdge] {
        &self.edges
    }
    pub fn default_start_node_id(&self) -> &str {
        &self.default_start_node_id
    }
    pub fn node(&self, id: &str) -> Option<&LegacyRuntimeNode> {
        self.node_index.get(id).map(|&i| &self.nodes[i])
    }
    pub fn edge_between(&self, from: &str, to: &str) -> Option<&LegacyRuntimeEdge> {
        self.edge_index
            .get(&pair_key(from, to))
            .map(|&i| &self.edges[i])
    }
    pub fn public_destinations(&self) -> impl Iterator<Item = &LegacyRuntimeNode> {
        self.nodes
            .iter()
            .filter(|node| node.poi.as_ref().is_some_and(|poi| poi.public))
    }
}
