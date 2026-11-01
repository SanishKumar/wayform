//! Landmark wording ported from the pinned VoiceGIS routeLandmarks.ts.
use super::directions::distance;
use super::{LegacyDirections, LegacyRouteStep, LegacyRuntimeNode};
use crate::LegacySearchError;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

type Point = [f64; 2];
#[derive(Debug)]
pub(super) struct Landmark {
    id: String,
    name: String,
    floor: String,
    position: Point,
    outline: Vec<Point>,
    space_name: String,
}
fn point(value: &Value) -> Point {
    [value[0].as_f64().unwrap(), value[1].as_f64().unwrap()]
}
pub(super) fn from_package(package: &Value) -> Vec<Landmark> {
    let spaces: HashMap<_, _> = package["spaces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["id"].as_str().unwrap(), s))
        .collect();
    package["pois"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["public"] == true && !p["name"].as_str().unwrap().trim().is_empty())
        .map(|p| {
            let space = spaces[p["spaceId"].as_str().unwrap()];
            Landmark {
                id: p["id"].as_str().unwrap().into(),
                name: p["name"].as_str().unwrap().into(),
                floor: p["floorId"].as_str().unwrap().into(),
                position: point(&p["position"]),
                outline: space["polygon"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(point)
                    .collect(),
                space_name: space["name"].as_str().unwrap().into(),
            }
        })
        .collect()
}
fn dist(a: Point, b: Point) -> f64 {
    distance(a[0] - b[0], a[1] - b[1])
}
fn along(point: Point, from: Point, to: Point) -> f64 {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let squared = dx * dx + dy * dy;
    if squared == 0.0 {
        0.0
    } else {
        ((point[0] - from[0]) * dx + (point[1] - from[1]) * dy) / squared
    }
}
fn point_to_segment(point: Point, from: Point, to: Point) -> f64 {
    let t = along(point, from, to).clamp(0.0, 1.0);
    dist(
        point,
        [
            from[0] + (to[0] - from[0]) * t,
            from[1] + (to[1] - from[1]) * t,
        ],
    )
}
fn orientation(a: Point, b: Point, c: Point) -> i8 {
    let value = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if value > 1e-9 {
        1
    } else if value < -1e-9 {
        -1
    } else {
        0
    }
}
fn segment_to_segment(a: Point, b: Point, c: Point, d: Point) -> f64 {
    if orientation(a, b, c) != orientation(a, b, d) && orientation(c, d, a) != orientation(c, d, b)
    {
        return 0.0;
    }
    point_to_segment(a, c, d)
        .min(point_to_segment(b, c, d))
        .min(point_to_segment(c, a, b))
        .min(point_to_segment(d, a, b))
}
fn xy(node: &LegacyRuntimeNode) -> Point {
    [node.x, node.y]
}
impl Landmark {
    fn to_point(&self, point: Point) -> f64 {
        if self.outline.is_empty() {
            return dist(self.position, point);
        }
        (0..self.outline.len())
            .map(|i| {
                point_to_segment(
                    point,
                    self.outline[i],
                    self.outline[(i + 1) % self.outline.len()],
                )
            })
            .fold(f64::INFINITY, f64::min)
    }
    fn to_segment(&self, from: Point, to: Point) -> f64 {
        if self.outline.is_empty() {
            return point_to_segment(self.position, from, to);
        }
        (0..self.outline.len())
            .map(|i| {
                segment_to_segment(
                    self.outline[i],
                    self.outline[(i + 1) % self.outline.len()],
                    from,
                    to,
                )
            })
            .fold(f64::INFINITY, f64::min)
    }
}
fn is_turn(step: &LegacyRouteStep) -> bool {
    matches!(
        step.r#type.as_str(),
        "turn_left" | "turn_right" | "slight_left" | "slight_right" | "u_turn"
    )
}
fn nearest<'a>(node: &LegacyRuntimeNode, landmarks: &[&'a Landmark]) -> Option<&'a Landmark> {
    let mut best: Option<&Landmark> = None;
    let mut best_distance = 7.0;
    for &landmark in landmarks.iter().filter(|l| l.floor == node.floor) {
        let d = landmark.to_point(xy(node));
        if d < best_distance
            || (d == best_distance
                && best.is_some_and(|b| {
                    landmark
                        .name
                        .encode_utf16()
                        .cmp(b.name.encode_utf16())
                        .is_lt()
                }))
        {
            best = Some(landmark);
            best_distance = d;
        }
    }
    best
}
fn already_names(instruction: &str, landmark: &Landmark) -> bool {
    // The reference's / along .+$/ leaves an empty suffix untouched.
    let said = instruction
        .find(" along ")
        .filter(|&i| i + 7 < instruction.len())
        .map_or(instruction, |i| &instruction[..i]);
    said.contains(&landmark.name) || said.contains(&landmark.space_name)
}
fn split_corridor(instruction: &str) -> (&str, &str) {
    let found = [" onto ", " into ", " along "]
        .into_iter()
        .filter_map(|pattern| instruction.find(pattern).map(|i| (i, pattern)))
        .min_by_key(|(i, _)| *i);
    match found {
        None => (instruction, ""),
        Some((i, pattern)) => (
            &instruction[..i],
            if pattern == " along " {
                ""
            } else {
                &instruction[i..]
            },
        ),
    }
}
fn passed_along<'a>(
    path: &[LegacyRuntimeNode],
    start: usize,
    end: usize,
    landmarks: &[&'a Landmark],
) -> (Vec<(&'a Landmark, f64)>, f64) {
    let mut found = Vec::new();
    let mut ids = HashSet::new();
    let mut offset = 0.0;
    for pair in path[start..=end].windows(2) {
        let from = &pair[0];
        let to = &pair[1];
        if from.floor != to.floor {
            continue;
        }
        let a = xy(from);
        let b = xy(to);
        let length = dist(a, b);
        if length < 1e-6 {
            continue;
        }
        for &landmark in landmarks.iter().filter(|l| l.floor == from.floor) {
            if ids.contains(&landmark.id) {
                continue;
            }
            let t = along(landmark.position, a, b);
            if !(-0.01..=1.01).contains(&t) || landmark.to_segment(a, b) > 5.5 {
                continue;
            }
            ids.insert(&landmark.id);
            found.push((landmark, offset + t.clamp(0.0, 1.0) * length));
        }
        offset += length;
    }
    found.retain(|(_, along)| *along >= 0.05 * offset && *along <= 0.95 * offset);
    (found, offset)
}
fn arrival_side(path: &[LegacyRuntimeNode]) -> Option<&'static str> {
    let destination = path.last()?;
    for index in (1..path.len().saturating_sub(1)).rev() {
        let end = &path[index];
        let start = &path[index - 1];
        if start.floor != end.floor || end.floor != destination.floor {
            return None;
        }
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let length = distance(dx, dy);
        if length < 1e-6 {
            continue;
        }
        let ox = destination.x - end.x;
        let oy = destination.y - end.y;
        let offset = distance(ox, oy);
        if offset < 1.5 {
            continue;
        }
        let cross = dx * oy - dy * ox;
        let sine = cross.abs() / (length * offset);
        if sine < 0.5 {
            if length < 6.0 {
                continue;
            }
            return None;
        }
        return Some(if cross > 0.0 { "right" } else { "left" });
    }
    None
}

pub(super) fn describe(
    directions: &mut LegacyDirections,
    landmarks: &[Landmark],
) -> Result<(), LegacySearchError> {
    let path = &directions.path;
    let steps = &directions.steps;
    if path.is_empty() || steps.is_empty() {
        return Ok(());
    }
    // Bound products used by geometric predicates, in addition to the loader's
    // finite-coordinate checks. Extremely large coordinates cannot guide.
    if path.iter().any(|n| n.x.abs() > 1e100 || n.y.abs() > 1e100)
        || landmarks.iter().any(|l| {
            l.outline
                .iter()
                .chain(std::iter::once(&l.position))
                .flatten()
                .any(|v| v.abs() > 1e100)
        })
    {
        return Err(LegacySearchError::UnrepresentableCost);
    }
    let vertices: usize = landmarks.iter().map(|l| l.outline.len().max(1)).sum();
    if vertices
        .saturating_mul(path.len().saturating_add(steps.len()))
        .saturating_mul(4)
        > 5_000_000
    {
        return Err(LegacySearchError::WorkLimitExceeded);
    }
    let endpoints: HashSet<_> = [path[0].poi.as_ref(), path.last().unwrap().poi.as_ref()]
        .into_iter()
        .flatten()
        .map(|p| p.name.as_str())
        .collect();
    let usable: Vec<_> = landmarks
        .iter()
        .filter(|l| !endpoints.contains(l.name.as_str()))
        .collect();
    let mut cursor = 0;
    let anchors: Vec<_> = steps
        .iter()
        .map(|step| {
            if let Some(index) = path
                .iter()
                .enumerate()
                .skip(cursor)
                .find_map(|(i, n)| (n.id == step.node_id).then_some(i))
            {
                cursor = index;
            }
            cursor
        })
        .collect();
    let mut previous_turn = None;
    let mut placed = steps.clone();
    for (index, step) in steps.iter().enumerate() {
        let anchor = anchors[index];
        let node = &path[anchor];
        if is_turn(step) {
            let landmark = nearest(node, &usable);
            let repeat = landmark.is_some_and(|l| Some(l.id.as_str()) == previous_turn);
            previous_turn = landmark.map(|l| l.id.as_str());
            let Some(landmark) =
                landmark.filter(|l| !repeat && !already_names(&step.instruction, l))
            else {
                continue;
            };
            let (head, leads) = split_corridor(&step.instruction);
            placed[index].instruction = format!("{head} at {}{leads}", landmark.name);
            continue;
        }
        previous_turn = None;
        if matches!(step.r#type.as_str(), "straight" | "start") {
            if index + 2 == steps.len() && steps[index + 1].r#type == "arrive" {
                continue;
            }
            let end = anchors.get(index + 1).copied().unwrap_or(path.len() - 1);
            if end <= anchor {
                continue;
            }
            let (passed, length) = passed_along(path, anchor, end, &usable);
            if length < 8.0 || passed.is_empty() {
                continue;
            }
            let next_named = steps
                .get(index + 1)
                .filter(|s| is_turn(s))
                .and_then(|_| nearest(&path[end], &usable));
            let candidates: Vec<_> = passed
                .into_iter()
                .filter(|(l, _)| {
                    !next_named.is_some_and(|n| n.id == l.id)
                        && !already_names(&step.instruction, l)
                })
                .collect();
            let Some(mut selected) = candidates.first().copied() else {
                continue;
            };
            for &candidate in candidates.iter().skip(1) {
                if (step.r#type == "start" && selected.1 > candidate.1)
                    || (step.r#type != "start" && selected.1 < candidate.1)
                {
                    selected = candidate;
                }
            }
            placed[index].instruction = format!(
                "{}, {} {}",
                step.instruction,
                if step.r#type == "start" {
                    "towards"
                } else {
                    "past"
                },
                selected.0.name
            );
        } else if let (true, Some(side)) = (
            step.r#type == "arrive" && index + 1 == steps.len(),
            arrival_side(path),
        ) {
            placed[index].instruction = format!("{}, on your {side}", step.instruction);
        }
    }
    let mut run = 0;
    let original: Vec<_> = placed.iter().map(|s| s.instruction.clone()).collect();
    for (index, step) in placed.iter_mut().enumerate() {
        if index == 0 || original[index - 1] != original[index] || !is_turn(step) {
            run = 0;
            continue;
        }
        run += 1;
        if let Some(prefix) = [
            "Turn left",
            "Turn right",
            "Turn around",
            "Bear left",
            "Bear right",
        ]
        .into_iter()
        .find(|p| step.instruction.starts_with(p))
        {
            step.instruction.insert_str(
                prefix.len(),
                if run == 1 {
                    " again"
                } else if run == 2 {
                    " a third time"
                } else {
                    " once more"
                },
            );
        }
    }
    directions.steps = placed;
    Ok(())
}
