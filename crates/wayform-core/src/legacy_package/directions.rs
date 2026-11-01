//! Instruction behavior ported from the pinned VoiceGIS routingCore.ts.
//! The unchanged source and its attribution are retained in compat/voicegis.
use super::runtime::{LegacyRuntime, LegacyRuntimeEdge, LegacyRuntimeNode};
use crate::LegacySearchError;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyRouteStep {
    pub r#type: String,
    pub instruction: String,
    pub distance: f64,
    pub node_id: String,
    /// Clockwise from plan-up (-Y), not a compass heading.
    pub bearing: f64,
    pub floor_id: String,
}

#[derive(Debug, Serialize)]
pub struct LegacyDirections {
    pub path: Vec<LegacyRuntimeNode>,
    pub steps: Vec<LegacyRouteStep>,
}

pub(super) fn distance(dx: f64, dy: f64) -> f64 {
    let scale = dx.abs().max(dy.abs());
    if scale == 0.0 {
        return 0.0;
    }
    let x = dx.abs() / scale;
    let y = dy.abs() / scale;
    (x * x + y * y).sqrt() * scale
}
fn bearing(from: &LegacyRuntimeNode, to: &LegacyRuntimeNode) -> f64 {
    let value = (to.x - from.x).atan2(-(to.y - from.y)) * (180.0 / std::f64::consts::PI);
    if value < 0.0 { value + 360.0 } else { value }
}
fn turn(previous: f64, next: f64) -> &'static str {
    let mut difference = next - previous;
    if difference > 180.0 {
        difference -= 360.0;
    }
    if difference < -180.0 {
        difference += 360.0;
    }
    if difference.abs() < 20.0 {
        "straight"
    } else if (20.0..60.0).contains(&difference) {
        "slight_right"
    } else if (60.0..=150.0).contains(&difference) {
        "turn_right"
    } else if difference > -60.0 && difference <= -20.0 {
        "slight_left"
    } else if (-150.0..=-60.0).contains(&difference) {
        "turn_left"
    } else {
        "u_turn"
    }
}
fn name<'a>(node: &'a LegacyRuntimeNode, fallback: &'a str) -> &'a str {
    node.poi.as_ref().map_or(fallback, |p| &p.name)
}
fn step(
    kind: &str,
    instruction: String,
    distance: f64,
    node: &LegacyRuntimeNode,
    bearing: f64,
    floor: &str,
) -> LegacyRouteStep {
    LegacyRouteStep {
        r#type: kind.into(),
        instruction,
        distance,
        node_id: node.id.clone(),
        bearing,
        floor_id: floor.into(),
    }
}
struct Segment<'a> {
    from: &'a LegacyRuntimeNode,
    to: &'a LegacyRuntimeNode,
    edge: &'a LegacyRuntimeEdge,
    bearing: f64,
    distance: f64,
}
impl Segment<'_> {
    fn vertical(&self) -> bool {
        self.edge.kind == "vertical-connector"
    }
    fn corridor(&self) -> Option<&str> {
        self.edge.corridor.as_deref()
    }
    fn way(&self) -> bool {
        self.edge
            .space_type
            .as_deref()
            .is_none_or(|s| s == "corridor")
    }
    fn vertical_type(&self) -> &str {
        match self.edge.connector_kind.as_deref() {
            Some(kind @ ("elevator" | "stairs" | "ramp" | "escalator")) => kind,
            _ => "straight",
        }
    }
    fn vertical_instruction(&self) -> String {
        let connector = self.corridor().unwrap_or(
            self.edge
                .connector_kind
                .as_deref()
                .unwrap_or("vertical connector"),
        );
        let article = if self.corridor().is_some_and(|s| !s.is_empty()) {
            ""
        } else {
            "the "
        };
        format!("Take {article}{connector} to {}", self.to.floor_name)
    }
    fn vertical_step(&self) -> LegacyRouteStep {
        step(
            self.vertical_type(),
            self.vertical_instruction(),
            self.distance,
            self.from,
            0.0,
            &self.to.floor,
        )
    }
    fn flat_step(&self, kind: &str, instruction: String) -> LegacyRouteStep {
        step(
            kind,
            instruction,
            self.distance,
            self.from,
            self.bearing,
            &self.from.floor,
        )
    }
}

pub(super) fn generate(
    runtime: &LegacyRuntime,
    path: &[LegacyRuntimeNode],
) -> Result<Vec<LegacyRouteStep>, LegacySearchError> {
    let result = generate_inner(runtime, path)?;
    if result
        .iter()
        .any(|s| !s.distance.is_finite() || !s.bearing.is_finite())
    {
        return Err(LegacySearchError::UnrepresentableCost);
    }
    Ok(result)
}

fn generate_inner(
    runtime: &LegacyRuntime,
    path: &[LegacyRuntimeNode],
) -> Result<Vec<LegacyRouteStep>, LegacySearchError> {
    let Some(first) = path.first() else {
        return Ok(vec![]);
    };
    let destination = path.last().unwrap();
    if path.len() == 1 {
        return Ok(vec![step(
            "arrive",
            format!("You are already at {}", name(first, "the destination")),
            0.0,
            first,
            0.0,
            &first.floor,
        )]);
    }
    let mut segments: Vec<Segment<'_>> = Vec::new();
    let mut pending = 0.0;
    for pair in path.windows(2) {
        let from = &pair[0];
        let to = &pair[1];
        let edge = runtime
            .edge_between(&from.id, &to.id)
            .ok_or(LegacySearchError::InvalidGraph)?;
        if from.floor == to.floor && distance(to.x - from.x, to.y - from.y) < 1e-7 {
            pending += edge.distance;
            continue;
        }
        segments.push(Segment {
            from,
            to,
            edge,
            bearing: bearing(from, to),
            distance: edge.distance + pending,
        });
        pending = 0.0;
    }
    if let (true, Some(last)) = (pending > 0.0, segments.last_mut()) {
        last.distance += pending;
    }
    if segments.is_empty() {
        return Ok(vec![
            step(
                "start",
                format!("Start at {}", name(first, "your location")),
                pending,
                first,
                0.0,
                &first.floor,
            ),
            step(
                "arrive",
                format!("Arrive at {}", name(destination, "the destination")),
                0.0,
                destination,
                0.0,
                &destination.floor,
            ),
        ]);
    }
    let initial = &segments[0];
    let start_name = name(first, "your location");
    let mut instruction = format!("Start at {start_name}");
    if let Some(corridor) = initial.corridor().filter(|s| *s != start_name) {
        instruction += &format!(
            " {} {corridor}",
            if initial.way() {
                "and continue on"
            } else {
                "in"
            }
        );
    }
    let mut steps = vec![step(
        "start",
        instruction,
        0.0,
        first,
        initial.bearing,
        &first.floor,
    )];
    let mut active = if initial.vertical() { None } else { Some(0) };
    let mut active_distance = if initial.vertical() {
        0.0
    } else {
        initial.distance
    };
    let mut previous_bearing = if initial.vertical() {
        None
    } else {
        Some(initial.bearing)
    };
    let mut vertical_source = initial
        .vertical()
        .then_some(initial.edge.source_id.as_str());
    let mut active_corridor = if initial.vertical() {
        None
    } else {
        initial.corridor()
    };
    if initial.vertical() {
        steps.push(initial.vertical_step());
    }
    for (index, segment) in segments.iter().enumerate().skip(1) {
        if segment.vertical() {
            if let Some(i) = active {
                steps[i].distance = active_distance;
            }
            let last = steps.last_mut().unwrap();
            if vertical_source == Some(segment.edge.source_id.as_str())
                && last.r#type == segment.vertical_type()
            {
                last.instruction = segment.vertical_instruction();
                last.distance += segment.distance;
                last.floor_id = segment.to.floor.clone();
                continue;
            }
            steps.push(segment.vertical_step());
            active = None;
            active_distance = 0.0;
            previous_bearing = None;
            vertical_source = Some(&segment.edge.source_id);
            active_corridor = None;
            continue;
        }
        vertical_source = None;
        let Some(previous) = previous_bearing else {
            let instruction = if let Some(corridor) = segment.corridor() {
                format!(
                    "Continue {} {corridor}",
                    if segment.way() { "on" } else { "through" }
                )
            } else {
                format!("Continue on {}", segment.to.floor_name)
            };
            steps.push(segment.flat_step("straight", instruction));
            active = Some(steps.len() - 1);
            active_distance = segment.distance;
            previous_bearing = Some(segment.bearing);
            active_corridor = segment.corridor();
            continue;
        };
        let kind = turn(previous, segment.bearing);
        if kind == "straight" {
            if let Some(corridor) = segment.corridor().filter(|c| Some(*c) != active_corridor) {
                if let Some(i) = active {
                    steps[i].distance = active_distance;
                }
                steps.push(segment.flat_step(
                    "straight",
                    format!(
                        "Continue {} {corridor}",
                        if segment.way() { "on" } else { "into" }
                    ),
                ));
                active = Some(steps.len() - 1);
                active_distance = segment.distance;
                previous_bearing = Some(segment.bearing);
                active_corridor = segment.corridor();
                continue;
            }
            active_distance += segment.distance;
            previous_bearing = Some(segment.bearing);
            continue;
        }
        if index == segments.len() - 1
            && segment.distance < 2.5
            && segment.from.r#type == "space"
            && segment.to.r#type == "poi"
            && active.is_some()
        {
            active_distance += segment.distance;
            continue;
        }
        if let Some(i) = active {
            steps[i].distance = active_distance;
        }
        let stays = segment.corridor().is_some() && segment.corridor() == active_corridor;
        let next = segments.get(index + 1);
        let leads = stays
            && segment.distance <= 8.0
            && next.is_some_and(|next| {
                !next.vertical()
                    && next.corridor().is_some()
                    && next.corridor() != segment.corridor()
                    && turn(segment.bearing, next.bearing) == "straight"
            });
        let named = if leads { next.unwrap() } else { segment };
        let action = match kind {
            "turn_left" => "Turn left",
            "turn_right" => "Turn right",
            "slight_left" => "Bear left",
            "slight_right" => "Bear right",
            _ => "Turn around",
        };
        let instruction = if let Some(corridor) = named.corridor() {
            let preposition = if stays && !leads {
                "along"
            } else if named.way() {
                "onto"
            } else {
                "into"
            };
            format!("{action} {preposition} {corridor}")
        } else {
            action.into()
        };
        steps.push(segment.flat_step(kind, instruction));
        active = Some(steps.len() - 1);
        active_distance = segment.distance;
        previous_bearing = Some(segment.bearing);
        active_corridor = named.corridor();
    }
    if let Some(i) = active {
        steps[i].distance = active_distance;
    }
    steps.push(step(
        "arrive",
        format!("Arrive at {}", name(destination, "the destination")),
        0.0,
        destination,
        segments.last().unwrap().bearing,
        &destination.floor,
    ));
    Ok(steps)
}
