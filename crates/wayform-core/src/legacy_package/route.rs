//! Complete legacy result composition from the pinned compiledRoutePolicy.ts.
use super::{LegacyDirections, LegacyVenuePackage, clearance, directions, landmarks};
use crate::{
    LegacyPolicyError, LegacyPolicyIndex, LegacyRouteOptions, LegacySearchError,
    LegacySearchOutcome,
};
use serde_json::{Value, json};
use std::{collections::HashMap, fmt};

#[derive(Clone, Debug, Default)]
pub struct LegacyCompiledOptions {
    pub accessible_only: bool,
    pub allow_restricted: bool,
    pub operational_overlay: Option<Value>,
    pub evaluated_at: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyRouteError {
    Search(LegacySearchError),
    Policy(LegacyPolicyError),
    UnsupportedConnectorId,
}
impl fmt::Display for LegacyRouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Search(e) => e.fmt(f),
            Self::Policy(e) => e.fmt(f),
            Self::UnsupportedConnectorId => f.write_str(
                "legacy connector receipt collation requires ASCII letters, digits and hyphens",
            ),
        }
    }
}
impl std::error::Error for LegacyRouteError {}
impl From<LegacySearchError> for LegacyRouteError {
    fn from(e: LegacySearchError) -> Self {
        Self::Search(e)
    }
}
impl From<LegacyPolicyError> for LegacyRouteError {
    fn from(e: LegacyPolicyError) -> Self {
        Self::Policy(e)
    }
}

fn selected_connectors(
    venue: &LegacyVenuePackage,
    path_ids: &[String],
) -> Result<Vec<Value>, LegacyRouteError> {
    let mut selected = HashMap::new();
    for pair in path_ids.windows(2) {
        let edge = venue
            .runtime
            .edge_between(&pair[0], &pair[1])
            .ok_or(LegacySearchError::InvalidGraph)?;
        if edge.kind != "vertical-connector" {
            continue;
        }
        let from = venue.runtime.node(&pair[0]).unwrap();
        let to = venue.runtime.node(&pair[1]).unwrap();
        let item=selected.entry(edge.source_id.as_str()).or_insert_with(||json!({"sourceId":edge.source_id,"kind":edge.connector_kind.as_deref().unwrap_or("vertical-connector"),"fromFloorId":from.floor,"toFloorId":to.floor}));
        item["toFloorId"] = json!(to.floor);
    }
    if selected
        .keys()
        .any(|id| !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
    {
        return Err(LegacyRouteError::UnsupportedConnectorId);
    }
    let mut ids: Vec<_> = selected.keys().copied().collect();
    ids.sort_by(|a, b| {
        a.bytes()
            .map(|c| c.to_ascii_lowercase())
            .cmp(b.bytes().map(|c| c.to_ascii_lowercase()))
            .then_with(|| {
                a.bytes()
                    .map(|c| c.is_ascii_uppercase())
                    .cmp(b.bytes().map(|c| c.is_ascii_uppercase()))
            })
    });
    Ok(ids.into_iter().map(|id| selected[id].clone()).collect())
}

pub(super) fn calculate(
    venue: &LegacyVenuePackage,
    start: &str,
    destination: &str,
    options: &LegacyCompiledOptions,
) -> Result<Value, LegacyRouteError> {
    let resolution = options
        .operational_overlay
        .as_ref()
        .map(|overlay| {
            LegacyPolicyIndex::new(venue.policy_package())?
                .resolve(overlay, options.evaluated_at.as_deref().unwrap_or(""))
        })
        .transpose()?;
    let profile = if options.accessible_only {
        "wheelchair"
    } else {
        "standard"
    };
    let mut receipt = json!({
        "receiptVersion":"0.1.0","buildingId":venue.building_id(),"packageHash":venue.content_hash(),"profile":profile,
        "startId":start,"destinationId":destination,
        "operationalOverlayId":options.operational_overlay.as_ref().and_then(|o|o.get("id")),
        "evaluatedAt":if options.operational_overlay.is_some() { options.evaluated_at.as_deref() } else { None },
        "appliedClosureIds":resolution.as_ref().map(|r|r.active_closure_ids.as_slice()).unwrap_or(&[]),
        "excludedEdges":{
            "restricted":if options.allow_restricted { 0 } else { venue.edges.iter().filter(|e|e.restricted).count() },
            "inaccessible":if options.accessible_only { venue.edges.iter().filter(|e|!e.accessible).count() } else { 0 },
            "closed":resolution.as_ref().map_or(0,|r|r.closed_edge_ids.len()),
        },
        "overlayIssues":resolution.as_ref().map(|r|r.issues.as_slice()).unwrap_or(&[]),
        "status":"unroutable","totalDistanceMeters":null,"selectedConnectors":[],
    });
    if let Some(rejected) = resolution.as_ref().filter(|r| !r.valid) {
        receipt["status"] = json!("rejected");
        return Ok(
            json!({"found":false,"error":format!("Operational overlay rejected: {}.",rejected.issues.iter().map(|i|i.code).collect::<Vec<_>>().join(", ")),"receipt":receipt}),
        );
    }
    let route_options = LegacyRouteOptions {
        accessible_only: options.accessible_only,
        allow_restricted: options.allow_restricted,
        closed_edge_ids: resolution.map(|r| r.closed_edge_ids).unwrap_or_default(),
    };
    let result = venue.route(start, destination, &route_options)?;
    let (path_ids, total_distance) = match result {
        LegacySearchOutcome::Routed {
            path_ids,
            total_distance,
        } => (path_ids, total_distance),
        LegacySearchOutcome::NoRoute => {
            return Ok(
                json!({"found":false,"error":"No route satisfies the selected constraints.","receipt":receipt}),
            );
        }
        LegacySearchOutcome::UnknownEndpoint => {
            return Ok(
                json!({"found":false,"error":"Start or destination does not exist in the building graph.","receipt":receipt}),
            );
        }
    };
    let path: Vec<_> = path_ids
        .iter()
        .map(|id| venue.runtime.node(id).unwrap().clone())
        .collect();
    let display_clearance = clearance::assess(venue, &path, &route_options)?;
    if !display_clearance.centerline_valid {
        receipt["status"] = json!("rejected");
        return Ok(
            json!({"found":false,"error":"The mapped route geometry is inconsistent. Choose another destination or ask venue staff for directions.","displayClearance":display_clearance,"receipt":receipt}),
        );
    }
    let steps = directions::generate(&venue.runtime, &path)?;
    let mut directions = LegacyDirections { path, steps };
    landmarks::describe(&mut directions, &venue.landmarks)?;
    receipt["status"] = json!("routed");
    receipt["totalDistanceMeters"] = json!(total_distance);
    receipt["selectedConnectors"] = json!(selected_connectors(venue, &path_ids)?);
    Ok(
        json!({"found":true,"algorithm":"a-star","pathIds":path_ids,"path":directions.path,"steps":directions.steps,"totalDistance":total_distance,"displayClearance":display_clearance,"receipt":receipt}),
    )
}
