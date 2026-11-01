//! Route graphic assessment ported from the pinned VoiceGIS routeClearance.ts.
//! A graphic envelope is not a measurement of a person's required clearance.
use super::directions::distance;
use super::{LegacyRuntimeNode, LegacyVenuePackage};
use crate::{LegacyRouteOptions, LegacySearchError};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

type Point = [f64; 2];
type Segment = [Point; 2];
type Index<'a> = HashMap<&'a str, &'a Value>;
const EPS: f64 = 1e-7;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyClearanceIssue {
    pub code: &'static str,
    pub source_id: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyDisplayClearance {
    pub basis: &'static str,
    pub package_hash: String,
    pub profile: &'static str,
    pub allow_restricted: bool,
    pub width_meters: f64,
    pub status: &'static str,
    pub centerline_valid: bool,
    pub issues: Vec<LegacyClearanceIssue>,
}
#[derive(Default)]
struct Issues {
    values: Vec<LegacyClearanceIssue>,
    seen: HashSet<(&'static str, String)>,
}
impl Issues {
    fn add(&mut self, code: &'static str, source: &str) {
        if self.seen.insert((code, source.into())) {
            self.values.push(LegacyClearanceIssue {
                code,
                source_id: source.into(),
            });
        }
    }
}
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap()
}
fn index(value: &Value) -> Index<'_> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| (text(v, "id"), v))
        .collect()
}
fn point(value: &Value) -> Point {
    [value[0].as_f64().unwrap(), value[1].as_f64().unwrap()]
}
fn xy(n: &LegacyRuntimeNode) -> Point {
    [n.x, n.y]
}
fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: Point, b: Point) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn lerp([a, b]: Segment, t: f64) -> Point {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}
fn sides(polygon: &[Point]) -> impl Iterator<Item = Segment> + '_ {
    (0..polygon.len()).map(|i| [polygon[i], polygon[(i + 1) % polygon.len()]])
}
fn point_distance(p: Point, [a, b]: Segment, epsilon_squared: f64) -> f64 {
    let direction = sub(b, a);
    let length_squared = dot(direction, direction);
    let t = if length_squared < epsilon_squared {
        0.0
    } else {
        (dot(sub(p, a), direction) / length_squared).clamp(0.0, 1.0)
    };
    let nearest = lerp([a, b], t);
    distance(p[0] - nearest[0], p[1] - nearest[1])
}
fn inside(point: Point, polygon: &[Point]) -> bool {
    // The compiler boundary predicate uses EPS for squared length, while the
    // route guard uses EPS squared. Preserve both original thresholds.
    if sides(polygon).any(|side| point_distance(point, side, EPS) <= EPS) {
        return true;
    }
    let mut result = false;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + polygon.len() - 1) % polygon.len()];
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            result = !result;
        }
    }
    result
}
fn cuts([a, b]: Segment, [c, d]: Segment) -> Vec<f64> {
    let ab = sub(b, a);
    let cd = sub(d, c);
    let ca = sub(c, a);
    let denominator = cross(ab, cd);
    if denominator.abs() > EPS {
        let t = cross(ca, cd) / denominator;
        let u = cross(ca, ab) / denominator;
        return if (-EPS..=1.0 + EPS).contains(&t) && (-EPS..=1.0 + EPS).contains(&u) {
            vec![t.clamp(0.0, 1.0)]
        } else {
            vec![]
        };
    }
    let squared = dot(ab, ab);
    if squared < EPS * EPS || cross(ca, ab).abs() > EPS {
        return vec![];
    }
    let t0 = dot(ca, ab) / squared;
    let t1 = dot(sub(d, a), ab) / squared;
    if t0.max(t1) < -EPS || t0.min(t1) > 1.0 + EPS {
        return vec![];
    }
    vec![t0.clamp(0.0, 1.0), t1.clamp(0.0, 1.0)]
}
fn segment_distance(a: Segment, b: Segment) -> f64 {
    if !cuts(a, b).is_empty() {
        return 0.0;
    }
    point_distance(a[0], b, EPS * EPS)
        .min(point_distance(a[1], b, EPS * EPS))
        .min(point_distance(b[0], a, EPS * EPS))
        .min(point_distance(b[1], a, EPS * EPS))
}
fn segment_inside(segment: Segment, polygon: &[Point]) -> bool {
    if !segment.into_iter().all(|p| inside(p, polygon)) {
        return false;
    }
    let mut values = vec![0.0, 1.0];
    for side in sides(polygon) {
        values.extend(cuts(segment, side));
    }
    values.sort_by(f64::total_cmp);
    values.dedup();
    values
        .windows(2)
        .all(|v| inside(lerp(segment, (v[0] + v[1]) / 2.0), polygon))
}
fn opening_for(portal: &Value, polygons: [&[Point]; 2]) -> Option<Segment> {
    let position = point(&portal["position"]);
    let containing = polygons.map(|polygon| {
        sides(polygon)
            .filter(|&s| point_distance(position, s, EPS * EPS) < EPS)
            .collect::<Vec<_>>()
    });
    if containing.iter().any(Vec::is_empty) {
        return None;
    }
    let first = containing[0][0];
    let delta = sub(first[1], first[0]);
    let length = distance(delta[0], delta[1]);
    let width = portal["width"].as_f64().unwrap();
    if length < EPS {
        return None;
    }
    let tangent = [delta[0] / length, delta[1] / length];
    if containing
        .iter()
        .flatten()
        .any(|s| cross(sub(s[1], s[0]), tangent).abs() > EPS)
    {
        return None;
    }
    let opening = [
        [
            position[0] - tangent[0] * width / 2.0,
            position[1] - tangent[1] * width / 2.0,
        ],
        [
            position[0] + tangent[0] * width / 2.0,
            position[1] + tangent[1] * width / 2.0,
        ],
    ];
    if containing.iter().any(|list| {
        opening
            .iter()
            .any(|&p| !list.iter().any(|&s| point_distance(p, s, EPS * EPS) < EPS))
    }) {
        return None;
    }
    let normal = [-tangent[1] * 0.0001, tangent[0] * 0.0001];
    let forward = lerp(
        [position, [position[0] + normal[0], position[1] + normal[1]]],
        1.0,
    );
    let back = [position[0] - normal[0], position[1] - normal[1]];
    if inside(forward, polygons[0]) == inside(forward, polygons[1])
        || inside(back, polygons[0]) == inside(back, polygons[1])
    {
        return None;
    }
    Some(opening)
}
fn subtract_openings(wall: Segment, openings: &[Segment]) -> Vec<Segment> {
    let direction = sub(wall[1], wall[0]);
    let squared = dot(direction, direction);
    if squared < EPS * EPS {
        return vec![];
    }
    let mut removed: Vec<_> = openings
        .iter()
        .filter(|opening| {
            opening
                .iter()
                .all(|&p| cross(sub(p, wall[0]), direction).abs() < EPS)
        })
        .map(|opening| {
            let a = dot(sub(opening[0], wall[0]), direction) / squared;
            let b = dot(sub(opening[1], wall[0]), direction) / squared;
            [a.min(b), a.max(b)]
        })
        .filter(|&[a, b]| b > 0.0 && a < 1.0)
        .collect();
    removed.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut result = vec![];
    let mut cursor = 0.0;
    for [a, b] in removed {
        let start = a.max(0.0);
        if start > cursor + EPS {
            result.push([lerp(wall, cursor), lerp(wall, start)]);
        }
        cursor = cursor.max(b.min(1.0));
    }
    if cursor < 1.0 - EPS {
        result.push([lerp(wall, cursor), wall[1]]);
    }
    result
}
fn add_used<'a>(id: &'a str, list: &mut Vec<&'a str>, seen: &mut HashSet<&'a str>) {
    if seen.insert(id) {
        list.push(id);
    }
}
fn stop_on<'a>(connector: &'a Value, floor: &str) -> Option<&'a Value> {
    connector["stops"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["floorId"] == floor)
}
fn owner<'a>(
    node: &'a LegacyRuntimeNode,
    pois: &Index<'a>,
    connectors: &Index<'a>,
) -> Option<&'a str> {
    match node.r#type.as_str() {
        "poi" => Some(text(pois[node.source_id.as_str()], "spaceId")),
        "connector-stop" => {
            stop_on(connectors[node.source_id.as_str()], &node.floor).map(|s| text(s, "spaceId"))
        }
        "space" | "waypoint" => Some(&node.source_id),
        _ => None,
    }
}

pub(super) fn assess(
    venue: &LegacyVenuePackage,
    path: &[LegacyRuntimeNode],
    options: &LegacyRouteOptions,
) -> Result<LegacyDisplayClearance, LegacySearchError> {
    let document = venue.document();
    let spaces = index(&document["spaces"]);
    let pois = index(&document["pois"]);
    let portals = index(&document["portals"]);
    let connectors = index(&document["verticalConnectors"]);
    let raw_edges = index(&document["routing"]["edges"]);
    let polygons: HashMap<_, Vec<_>> = spaces
        .iter()
        .map(|(&id, s)| {
            (
                id,
                s["polygon"].as_array().unwrap().iter().map(point).collect(),
            )
        })
        .collect();
    let vertices: usize = polygons.values().map(Vec::len).sum();
    let mut work = vertices
        .saturating_add(portals.len().saturating_mul(4))
        .saturating_mul(path.len().saturating_mul(10));
    for pair in path.windows(2) {
        if let Some(polygon) = venue
            .runtime()
            .edge_between(&pair[0].id, &pair[1].id)
            .and_then(|edge| raw_edges[edge.id.as_str()]["spaceId"].as_str())
            .and_then(|id| polygons.get(id))
        {
            work = work.saturating_add(
                polygon
                    .len()
                    .saturating_mul(polygon.len())
                    .saturating_mul(4),
            );
        }
    }
    if work > 10_000_000 {
        return Err(LegacySearchError::WorkLimitExceeded);
    }
    if polygons
        .values()
        .flatten()
        .flatten()
        .any(|v| v.abs() > 1e100)
        || path.iter().any(|p| p.x.abs() > 1e100 || p.y.abs() > 1e100)
        || portals.values().any(|p| {
            point(&p["position"]).iter().any(|v| v.abs() > 1e100)
                || p["width"].as_f64().unwrap() > 1e100
        })
        || connectors
            .values()
            .flat_map(|c| c["stops"].as_array().unwrap())
            .any(|s| point(&s["position"]).iter().any(|v| v.abs() > 1e100))
    {
        return Err(LegacySearchError::UnrepresentableCost);
    }
    let mut issues = Issues::default();
    let mut used_spaces = vec![];
    let mut space_seen = HashSet::new();
    let mut used_portals = vec![];
    let mut portal_seen = HashSet::new();
    let mut planar = vec![];
    if path.is_empty() {
        issues.add("invalid-path", "empty");
    }
    for node in path {
        if node.r#type == "poi" {
            let poi = pois[node.source_id.as_str()];
            if !inside(xy(node), &polygons[text(poi, "spaceId")]) {
                issues.add("invalid-path", &node.id);
            }
            if (!options.allow_restricted && poi["public"] != true)
                || (options.accessible_only && poi["accessible"] != true)
            {
                issues.add("space-policy", text(poi, "id"));
            }
        }
    }
    for pair in path.windows(2) {
        let from = &pair[0];
        let to = &pair[1];
        let runtime_edge = venue
            .runtime()
            .edge_between(&from.id, &to.id)
            .ok_or(LegacySearchError::InvalidGraph)?;
        let edge = raw_edges[runtime_edge.id.as_str()];
        let id = text(edge, "id");
        if (!options.allow_restricted && edge["restricted"] == true)
            || (options.accessible_only && edge["accessible"] != true)
        {
            issues.add("space-policy", id);
        }
        if edge["kind"] == "vertical-connector" {
            let connector = connectors[text(edge, "sourceId")];
            let stops = [
                stop_on(connector, &from.floor),
                stop_on(connector, &to.floor),
            ];
            if from.floor == to.floor
                || stops.iter().zip([from, to]).any(|(stop, node)| {
                    stop.is_none_or(|stop| {
                        node.r#type != "connector-stop"
                            || node.source_id != text(connector, "id")
                            || {
                                let p = point(&stop["position"]);
                                distance(p[0] - node.x, p[1] - node.y) > EPS
                            }
                    })
                })
            {
                issues.add("invalid-path", id);
            }
            for stop in stops {
                if let Some(stop) = stop {
                    let space = spaces[text(stop, "spaceId")];
                    let space_id = text(space, "id");
                    if space["floorId"] == stop["floorId"]
                        && inside(point(&stop["position"]), &polygons[space_id])
                    {
                        add_used(space_id, &mut used_spaces, &mut space_seen);
                    } else {
                        issues.add("invalid-path", id);
                    }
                } else {
                    issues.add("invalid-path", id);
                }
            }
            if (!options.allow_restricted && connector["restricted"] == true)
                || (options.accessible_only && connector["accessible"] != true)
            {
                issues.add("space-policy", text(connector, "id"));
            }
            continue;
        }
        let space_id = text(edge, "spaceId");
        let space = spaces[space_id];
        if space["floorId"] != from.floor || space["floorId"] != to.floor {
            issues.add("invalid-path", id);
            continue;
        }
        add_used(space_id, &mut used_spaces, &mut space_seen);
        for node in [from, to] {
            if node.r#type != "portal" && owner(node, &pois, &connectors) != Some(space_id) {
                issues.add("invalid-path", &node.id);
            }
        }
        let segment = [xy(from), xy(to)];
        if !segment_inside(segment, &polygons[space_id]) {
            issues.add("centerline-outside", id);
        }
        planar.push((segment, text(space, "floorId"), id));
    }
    if path.len() == 1 {
        let node = &path[0];
        let space_id = pois
            .get(node.source_id.as_str())
            .map_or(node.source_id.as_str(), |p| text(p, "spaceId"));
        if spaces.contains_key(space_id) {
            add_used(space_id, &mut used_spaces, &mut space_seen);
            if !inside(xy(node), &polygons[space_id]) {
                issues.add("centerline-outside", &node.id);
            }
        } else {
            issues.add("invalid-path", &node.id);
        }
    }
    for &id in &used_spaces {
        let space = spaces[id];
        if (!options.allow_restricted && space["public"] != true)
            || (options.accessible_only && space["accessible"] != true)
        {
            issues.add("space-policy", id);
        }
    }
    for (i, node) in path.iter().enumerate() {
        let adjacent = [i.checked_sub(1).map(|j| &path[j]), path.get(i + 1)];
        let adjacent_spaces: HashSet<_> = adjacent
            .into_iter()
            .flatten()
            .filter_map(|other| venue.runtime().edge_between(&other.id, &node.id))
            .filter_map(|e| raw_edges[e.id.as_str()]["spaceId"].as_str())
            .collect();
        if node.r#type != "portal" {
            if adjacent_spaces.len() > 1 {
                issues.add("invalid-path", &node.id);
            }
            continue;
        }
        let portal = portals[node.source_id.as_str()];
        let p = point(&portal["position"]);
        if portal["floorId"] != node.floor
            || distance(p[0] - node.x, p[1] - node.y) > EPS
            || adjacent_spaces.iter().any(|id| {
                !portal["connects"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v.as_str() == Some(id))
            })
        {
            issues.add("invalid-path", &node.id);
            continue;
        }
        if adjacent_spaces.len() != 2 {
            continue;
        }
        let id = text(portal, "id");
        if (!options.allow_restricted && portal["restricted"] == true)
            || (options.accessible_only && portal["accessible"] != true)
        {
            issues.add("portal-policy", id);
        }
        add_used(id, &mut used_portals, &mut portal_seen);
    }
    let mut openings: HashMap<&str, Vec<Segment>> = HashMap::new();
    for id in used_portals {
        let portal = portals[id];
        let connects = portal["connects"].as_array().unwrap();
        let a = connects[0].as_str().unwrap();
        let b = connects[1].as_str().unwrap();
        if let Some(opening) = opening_for(portal, [&polygons[a], &polygons[b]]) {
            for id in [a, b] {
                openings.entry(id).or_default().push(opening);
            }
        } else {
            issues.add("portal-geometry", id);
        }
    }
    let mut walls: HashMap<&str, Vec<Segment>> = HashMap::new();
    let mut floor_polygons: HashMap<&str, Vec<&[Point]>> = HashMap::new();
    for id in used_spaces {
        let floor = text(spaces[id], "floorId");
        let polygon = &polygons[id];
        for wall in sides(polygon) {
            walls.entry(floor).or_default().extend(subtract_openings(
                wall,
                openings.get(id).map_or(&[], Vec::as_slice),
            ));
        }
        floor_polygons.entry(floor).or_default().push(polygon);
    }
    let centerline_valid = !issues.values.iter().any(|i| i.code != "portal-geometry");
    for (segment, floor, id) in planar {
        if walls.get(floor).is_some_and(|walls| {
            walls
                .iter()
                .any(|&wall| segment_distance(segment, wall) + EPS < 0.35)
        }) {
            issues.add("insufficient-clearance", id);
        }
    }
    if path.len() == 1 {
        let node = &path[0];
        let p = xy(node);
        let fits = centerline_valid
            && floor_polygons
                .get(node.floor.as_str())
                .is_some_and(|polygons| polygons.iter().any(|polygon| inside(p, polygon)))
            && walls.get(node.floor.as_str()).is_none_or(|walls| {
                walls
                    .iter()
                    .all(|&wall| point_distance(p, wall, EPS * EPS) + EPS >= 0.35)
            });
        if !fits {
            issues.add("insufficient-clearance", &node.id);
        }
    }
    Ok(LegacyDisplayClearance {
        basis: "authored-geometry",
        package_hash: venue.content_hash().into(),
        profile: if options.accessible_only {
            "wheelchair"
        } else {
            "standard"
        },
        allow_restricted: options.allow_restricted,
        width_meters: 0.7,
        status: if issues.values.is_empty() {
            "checked"
        } else {
            "withheld"
        },
        centerline_valid,
        issues: issues.values,
    })
}
