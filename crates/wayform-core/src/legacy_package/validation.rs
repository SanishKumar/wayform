//! Native loading boundary for the pinned venue contract. See the original
//! venuePackageContract.ts under compat/voicegis/upstream/ for provenance.

use super::LegacyPackageError;
use crate::validation::valid_digest;
use crate::{MAX_EDGES, MAX_NODES};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

type Result<T> = std::result::Result<T, LegacyPackageError>;
type Index<'a> = HashMap<&'a str, &'a Value>;

fn require(ok: bool, path: &str, code: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(LegacyPackageError::Invalid {
            path: path.into(),
            code,
        })
    }
}
fn text<'a>(value: &'a Value, path: &str) -> Result<&'a str> {
    value
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| LegacyPackageError::Invalid {
            path: path.into(),
            code: "required-string",
        })
}
fn number(value: &Value, path: &str) -> Result<f64> {
    value
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| LegacyPackageError::Invalid {
            path: path.into(),
            code: "required-number",
        })
}
fn array<'a>(value: &'a Value, path: &str, min: usize, max: usize) -> Result<&'a [Value]> {
    let items = value
        .as_array()
        .ok_or_else(|| LegacyPackageError::Invalid {
            path: path.into(),
            code: "required-array",
        })?;
    require((min..=max).contains(&items.len()), path, "collection-size")?;
    Ok(items)
}
fn records<'a>(value: &'a Value, path: &str, min: usize, max: usize) -> Result<Index<'a>> {
    let mut index = HashMap::new();
    for (i, value) in array(value, path, min, max)?.iter().enumerate() {
        let at = format!("{path}/{i}");
        require(value.is_object(), &at, "required-object")?;
        let id = text(&value["id"], &format!("{at}/id"))?;
        require(
            index.insert(id, value).is_none(),
            &format!("{at}/id"),
            "duplicate-id",
        )?;
    }
    Ok(index)
}
fn reference<'a>(value: &Value, index: &Index<'a>, path: &str) -> Result<&'a Value> {
    let id = text(value, path)?;
    index
        .get(id)
        .copied()
        .ok_or_else(|| LegacyPackageError::Invalid {
            path: path.into(),
            code: "unknown-reference",
        })
}
fn point(value: &Value, path: &str, dimensions: usize) -> Result<()> {
    for item in array(value, path, dimensions, dimensions)? {
        number(item, path)?;
    }
    Ok(())
}
fn polygon(value: &Value, path: &str) -> Result<()> {
    for item in array(value, path, 3, MAX_NODES)? {
        point(item, path, 2)?;
    }
    Ok(())
}
fn fields(value: &Value, path: &str, strings: &[&str], booleans: &[&str]) -> Result<()> {
    for key in strings {
        text(&value[key], &format!("{path}/{key}"))?;
    }
    for key in booleans {
        require(
            value[key].is_boolean(),
            &format!("{path}/{key}"),
            "required-boolean",
        )?;
    }
    Ok(())
}
fn one_of(value: &Value, path: &str, choices: &[&str]) -> Result<()> {
    require(
        value.as_str().is_some_and(|s| choices.contains(&s)),
        path,
        "unsupported-value",
    )
}
fn same_floor(a: &Value, b: &Value, path: &str) -> Result<()> {
    require(a["floorId"] == b["floorId"], path, "floor-mismatch")
}

pub(super) fn validate(value: &Value) -> Result<()> {
    require(value.is_object(), "/", "required-object")?;
    for (key, version) in [
        ("packageVersion", "0.2.0"),
        ("sourceSchemaVersion", "0.1.0"),
        ("compilerVersion", "0.2.0"),
    ] {
        require(
            value[key] == version,
            &format!("/{key}"),
            "unsupported-version",
        )?;
    }
    let building = &value["building"];
    require(building.is_object(), "/building", "required-object")?;
    fields(building, "/building", &["id", "name", "entrySpaceId"], &[])?;
    require(
        building["units"] == "meters",
        "/building/units",
        "unsupported-value",
    )?;
    let coordinates = &building["coordinateSystem"];
    require(
        coordinates.is_object() && coordinates["type"] == "local-cartesian",
        "/building/coordinateSystem",
        "unsupported-value",
    )?;
    point(
        &coordinates["origin"],
        "/building/coordinateSystem/origin",
        3,
    )?;
    number(
        &coordinates["northOffsetDegrees"],
        "/building/coordinateSystem/northOffsetDegrees",
    )?;
    let floors = records(&value["floors"], "/floors", 1, MAX_NODES)?;
    let spaces = records(&value["spaces"], "/spaces", 1, MAX_NODES)?;
    let portals = records(&value["portals"], "/portals", 0, MAX_NODES)?;
    let connectors = records(
        &value["verticalConnectors"],
        "/verticalConnectors",
        0,
        MAX_NODES,
    )?;
    let pois = records(&value["pois"], "/pois", 1, MAX_NODES)?;
    records(
        &value["localizationAnchors"],
        "/localizationAnchors",
        0,
        MAX_NODES,
    )?;
    reference(&building["entrySpaceId"], &spaces, "/building/entrySpaceId")?;

    for (i, floor) in value["floors"].as_array().unwrap().iter().enumerate() {
        let p = format!("/floors/{i}");
        fields(floor, &p, &["name"], &[])?;
        for key in ["level", "elevation", "clearHeight"] {
            number(&floor[key], &format!("{p}/{key}"))?;
        }
        require(
            floor["clearHeight"].as_f64().unwrap() > 0.0,
            &p,
            "invalid-height",
        )?;
        polygon(&floor["outline"], &format!("{p}/outline"))?;
    }
    for (i, space) in value["spaces"].as_array().unwrap().iter().enumerate() {
        let p = format!("/spaces/{i}");
        fields(space, &p, &["name"], &["public", "accessible"])?;
        reference(&space["floorId"], &floors, &format!("{p}/floorId"))?;
        one_of(
            &space["type"],
            &format!("{p}/type"),
            &[
                "entrance",
                "room",
                "corridor",
                "lobby",
                "service",
                "restricted",
                "vertical-circulation",
            ],
        )?;
        polygon(&space["polygon"], &format!("{p}/polygon"))?;
    }
    for (i, portal) in value["portals"].as_array().unwrap().iter().enumerate() {
        let p = format!("/portals/{i}");
        fields(portal, &p, &[], &["accessible"])?;
        if portal.get("restricted").is_some() {
            fields(portal, &p, &[], &["restricted"])?;
        }
        reference(&portal["floorId"], &floors, &format!("{p}/floorId"))?;
        one_of(
            &portal["kind"],
            &format!("{p}/kind"),
            &["door", "opening", "gate"],
        )?;
        point(&portal["position"], &format!("{p}/position"), 2)?;
        require(
            number(&portal["width"], &format!("{p}/width"))? > 0.0,
            &p,
            "invalid-width",
        )?;
        let ends = array(&portal["connects"], &format!("{p}/connects"), 2, 2)?;
        require(ends[0] != ends[1], &p, "duplicate-connection")?;
        for end in ends {
            same_floor(portal, reference(end, &spaces, &p)?, &p)?;
        }
    }
    let mut connector_floors = HashMap::new();
    for (i, connector) in value["verticalConnectors"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let p = format!("/verticalConnectors/{i}");
        fields(connector, &p, &["name"], &["accessible"])?;
        if connector.get("restricted").is_some() {
            fields(connector, &p, &[], &["restricted"])?;
        }
        one_of(
            &connector["kind"],
            &p,
            &["elevator", "stairs", "ramp", "escalator"],
        )?;
        let mut stops = HashSet::new();
        for stop in array(&connector["stops"], &format!("{p}/stops"), 2, MAX_NODES)? {
            require(stop.is_object(), &p, "required-object")?;
            reference(&stop["floorId"], &floors, &p)?;
            let space = reference(&stop["spaceId"], &spaces, &p)?;
            same_floor(stop, space, &p)?;
            require(
                stops.insert(stop["floorId"].as_str().unwrap()),
                &p,
                "duplicate-stop",
            )?;
            point(&stop["position"], &p, 2)?;
        }
        connector_floors.insert(connector["id"].as_str().unwrap(), stops);
    }
    let mut payloads = HashSet::new();
    for key in ["pois", "localizationAnchors"] {
        for (i, item) in value[key].as_array().unwrap().iter().enumerate() {
            let p = format!("/{key}/{i}");
            reference(&item["floorId"], &floors, &p)?;
            same_floor(item, reference(&item["spaceId"], &spaces, &p)?, &p)?;
            point(&item["position"], &p, 2)?;
            if key == "pois" {
                fields(item, &p, &["name", "category"], &["public", "accessible"])?;
                if let Some(aliases) = item.get("aliases") {
                    for alias in array(aliases, &p, 0, MAX_NODES)? {
                        text(alias, &p)?;
                    }
                }
            } else {
                one_of(&item["kind"], &p, &["qr", "apriltag", "image", "nfc"])?;
                number(&item["headingDegrees"], &p)?;
                require(
                    payloads.insert(text(&item["payload"], &p)?),
                    &p,
                    "duplicate-payload",
                )?;
            }
        }
    }
    let routing = &value["routing"];
    require(routing.is_object(), "/routing", "required-object")?;
    let nodes = records(&routing["nodes"], "/routing/nodes", 1, MAX_NODES)?;
    records(&routing["edges"], "/routing/edges", 0, MAX_EDGES)?;
    let sources: Index<'_> = spaces
        .iter()
        .chain(portals.iter())
        .chain(connectors.iter())
        .chain(pois.iter())
        .map(|(k, v)| (*k, *v))
        .collect();
    for (i, node) in routing["nodes"].as_array().unwrap().iter().enumerate() {
        let p = format!("/routing/nodes/{i}");
        reference(&node["floorId"], &floors, &p)?;
        point(&node["position"], &p, 2)?;
        let source = match node["kind"].as_str() {
            Some("space" | "waypoint") => reference(&node["sourceId"], &spaces, &p)?,
            Some("portal") => reference(&node["sourceId"], &portals, &p)?,
            Some("poi") => reference(&node["sourceId"], &pois, &p)?,
            Some("connector-stop") => reference(&node["sourceId"], &connectors, &p)?,
            _ => {
                require(false, &p, "unsupported-node-kind")?;
                unreachable!()
            }
        };
        if node["kind"] == "connector-stop" {
            require(
                connector_floors[node["sourceId"].as_str().unwrap()]
                    .contains(node["floorId"].as_str().unwrap()),
                &p,
                "floor-mismatch",
            )?;
        } else {
            same_floor(node, source, &p)?;
        }
    }
    for (i, edge) in routing["edges"].as_array().unwrap().iter().enumerate() {
        let p = format!("/routing/edges/{i}");
        let from = reference(&edge["from"], &nodes, &p)?;
        let to = reference(&edge["to"], &nodes, &p)?;
        reference(&edge["sourceId"], &sources, &p)?;
        fields(edge, &p, &[], &["accessible", "restricted"])?;
        require(
            number(&edge["distanceMeters"], &p)? > 0.0,
            &p,
            "invalid-distance",
        )?;
        let ids = array(&edge["floorIds"], &p, 1, 2)?;
        for id in ids {
            reference(id, &floors, &p)?;
        }
        require(
            ids.contains(&from["floorId"]) && ids.contains(&to["floorId"]),
            &p,
            "floor-mismatch",
        )?;
        match edge["kind"].as_str() {
            Some("within-space") => {
                let space = reference(&edge["spaceId"], &spaces, &p)?;
                same_floor(from, space, &p)?;
                same_floor(to, space, &p)?;
            }
            Some("vertical-connector") => {
                let connector = reference(&edge["sourceId"], &connectors, &p)?;
                require(
                    edge["connectorKind"] == connector["kind"],
                    &p,
                    "connector-kind-mismatch",
                )?;
                require(
                    from["kind"] == "connector-stop"
                        && to["kind"] == "connector-stop"
                        && from["sourceId"] == edge["sourceId"]
                        && to["sourceId"] == edge["sourceId"],
                    &p,
                    "connector-endpoint-mismatch",
                )?;
            }
            _ => require(false, &p, "unsupported-edge-kind")?,
        }
    }
    let entry = &building["entrySpaceId"];
    require(
        routing["nodes"].as_array().unwrap().iter().any(|node| {
            node["kind"] == "poi"
                && pois
                    .get(node["sourceId"].as_str().unwrap())
                    .is_some_and(|poi| {
                        poi["spaceId"] == *entry
                            && poi["public"] == true
                            && spaces[entry.as_str().unwrap()]["public"] == true
                    })
        }),
        "/pois",
        "missing-entry-poi",
    )?;
    if let Some(site) = value.get("site") {
        validate_site(site, &floors)?;
    }
    let manifest = &value["manifest"];
    require(
        manifest.is_object() && manifest["hashAlgorithm"] == "sha256",
        "/manifest",
        "unsupported-hash",
    )?;
    require(
        manifest["contentHash"].as_str().is_some_and(valid_digest),
        "/manifest/contentHash",
        "invalid-content-hash",
    )?;
    Ok(())
}

fn validate_site(site: &Value, floors: &Index<'_>) -> Result<()> {
    require(site.is_object(), "/site", "required-object")?;
    reference(&site["floorId"], floors, "/site/floorId")?;
    for key in ["buildings", "grounds", "features"] {
        let p = format!("/site/{key}");
        records(&site[key], &p, 0, MAX_NODES)?;
        for item in site[key].as_array().unwrap() {
            match key {
                "buildings" => {
                    text(&item["name"], &p)?;
                    polygon(&item["footprint"], &p)?;
                    let n = number(&item["storeys"], &p)?;
                    require(
                        n.fract() == 0.0 && (1.0..=60.0).contains(&n),
                        &p,
                        "invalid-storeys",
                    )?;
                }
                "grounds" => {
                    polygon(&item["polygon"], &p)?;
                    one_of(
                        &item["kind"],
                        &p,
                        &["lawn", "planting", "water", "paving", "parking", "road"],
                    )?;
                }
                _ => {
                    point(&item["position"], &p, 2)?;
                    one_of(
                        &item["kind"],
                        &p,
                        &["tree", "statue", "fountain", "bench", "lamp"],
                    )?;
                }
            }
        }
    }
    Ok(())
}
