//! Coarse mobile bindings over the separately tested navigation contracts.
use serde_json::{Value, json};
use std::{
    fmt,
    sync::{Arc, Mutex},
};
use wayform_core::{
    LegacyCompiledOptions, LegacyGuidanceError, LegacyGuidanceRead, LegacyGuidanceSession,
    LegacySessionInvalidation, LegacySessionRequest, LegacyVenuePackage, MobilityRouteOutcome,
    MobilitySnapshot, parse_bounded_json,
};

uniffi::setup_scaffolding!();

#[derive(Debug, uniffi::Error)]
pub enum MobileError {
    Failed { message: String },
}
impl fmt::Display for MobileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed { message } => f.write_str(message),
        }
    }
}
impl std::error::Error for MobileError {}
fn failed(message: impl ToString) -> MobileError {
    MobileError::Failed {
        message: message.to_string().chars().take(512).collect(),
    }
}
fn encode(value: &Value) -> Result<String, MobileError> {
    let text = serde_json::to_string(value).map_err(failed)?;
    if text.len() > 16 * 1024 * 1024 {
        return Err(failed("mobile response exceeds 16 MiB"));
    }
    Ok(text)
}

fn check_stairs(route: &Value, avoid_stairs: bool) -> Result<(), MobileError> {
    if avoid_stairs
        && route["receipt"]["selectedConnectors"]
            .as_array()
            .is_some_and(|items| {
                items
                    .iter()
                    .any(|item| matches!(item["kind"].as_str(), Some("stairs" | "escalator")))
            })
    {
        return Err(failed(
            "The authored step-free route contains stairs or an escalator.",
        ));
    }
    Ok(())
}

#[derive(uniffi::Object)]
pub struct MobileVenue {
    venue: Arc<LegacyVenuePackage>,
}

#[uniffi::export]
impl MobileVenue {
    #[uniffi::constructor]
    pub fn new(package: Vec<u8>) -> Result<Arc<Self>, MobileError> {
        Ok(Arc::new(Self {
            venue: Arc::new(LegacyVenuePackage::from_json(&package).map_err(failed)?),
        }))
    }
    pub fn catalog(&self) -> Result<String, MobileError> {
        let package = self.venue.document();
        encode(
            &json!({"version":"0.1.0","building":package["building"],"packageHash":self.venue.content_hash(),"floors":package["floors"],"spaces":package["spaces"],"site":package.get("site"),"destinations":self.venue.runtime().public_destinations().collect::<Vec<_>>(),"defaultStartNodeId":self.venue.runtime().default_start_node_id(),"anchors":package["localizationAnchors"].as_array().unwrap().iter().filter(|a|a["kind"]=="qr").collect::<Vec<_>>()}),
        )
    }
    pub fn check_in(&self, payload: String) -> Result<String, MobileError> {
        if payload.len() > 4096 {
            return Err(failed("check-in payload exceeds 4096 bytes"));
        }
        encode(&self.venue.check_in(&payload).map_err(failed)?)
    }
    /// A map-only preview. Starting guidance requires a current policy below.
    pub fn preview(
        &self,
        start_id: String,
        destination_id: String,
        avoid_stairs: bool,
    ) -> Result<String, MobileError> {
        let options = LegacyCompiledOptions {
            accessible_only: avoid_stairs,
            ..Default::default()
        };
        let route = self
            .venue
            .calculate_route(&start_id, &destination_id, &options)
            .map_err(failed)?;
        check_stairs(&route, avoid_stairs)?;
        encode(&route)
    }
    // Keep the two clock samples explicit at the foreign-language boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn start_trip(
        &self,
        start_id: String,
        destination_id: String,
        avoid_stairs: bool,
        policy: Vec<u8>,
        wall_ms: f64,
        monotonic_ms: f64,
    ) -> Result<Arc<MobileTrip>, MobileError> {
        let overlay = parse_bounded_json(&policy).map_err(failed)?;
        let request = LegacySessionRequest {
            start_id,
            destination_id,
            accessible_only: avoid_stairs,
            allow_restricted: false,
        };
        let session =
            LegacyGuidanceSession::new(self.venue.clone(), request, overlay, wall_ms, monotonic_ms)
                .map_err(|error| match error {
                    LegacyGuidanceError::RouteUnavailable(value) => {
                        failed(value["error"].as_str().unwrap_or("route unavailable"))
                    }
                    other => failed(other),
                })?;
        Ok(Arc::new(MobileTrip {
            session: Mutex::new(session),
            avoid_stairs,
        }))
    }
}

#[derive(Clone, Copy, Debug, uniffi::Enum)]
pub enum TripStopReason {
    Cancelled,
    PolicyChanged,
    PolicyRemoved,
    ContextChanged,
}
#[derive(uniffi::Object)]
pub struct MobileTrip {
    session: Mutex<LegacyGuidanceSession>,
    avoid_stairs: bool,
}
#[uniffi::export]
impl MobileTrip {
    pub fn read(&self, wall_ms: f64, monotonic_ms: f64) -> Result<String, MobileError> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| failed("journey state unavailable"))?;
        let result = match session.read(wall_ms, monotonic_ms).map_err(failed)? {
            LegacyGuidanceRead::Ready(route) => {
                if let Err(error) = check_stairs(route, self.avoid_stairs) {
                    session.invalidate(LegacySessionInvalidation::RouteContextChanged);
                    return Err(error);
                }
                json!({"status":"ready","route":route})
            }
            LegacyGuidanceRead::PolicyUnavailable(state) => {
                json!({"status":"policy-unavailable","reason":state.reason})
            }
            LegacyGuidanceRead::Invalidated(reason) => {
                json!({"status":"invalidated","reason":match reason {
                    LegacySessionInvalidation::Cancelled=>"cancelled",LegacySessionInvalidation::PolicyChanged=>"policy-changed",LegacySessionInvalidation::PolicyRemoved=>"policy-removed",LegacySessionInvalidation::RouteContextChanged=>"context-changed",
                }})
            }
        };
        encode(&result)
    }
    pub fn next_check_delay(&self, wall_ms: f64, monotonic_ms: f64) -> Result<f64, MobileError> {
        Ok(self
            .session
            .lock()
            .map_err(|_| failed("journey state unavailable"))?
            .next_check_delay(wall_ms, monotonic_ms))
    }
    pub fn invalidate(&self, reason: TripStopReason) -> Result<(), MobileError> {
        let reason = match reason {
            TripStopReason::Cancelled => LegacySessionInvalidation::Cancelled,
            TripStopReason::PolicyChanged => LegacySessionInvalidation::PolicyChanged,
            TripStopReason::PolicyRemoved => LegacySessionInvalidation::PolicyRemoved,
            TripStopReason::ContextChanged => LegacySessionInvalidation::RouteContextChanged,
        };
        self.session
            .lock()
            .map_err(|_| failed("journey state unavailable"))?
            .invalidate(reason);
        Ok(())
    }
}

/// Separate evidence-based snapshot API. It never upgrades legacy flags to measurements.
#[uniffi::export]
pub fn plan_mobility(
    bundle: Vec<u8>,
    profile: Vec<u8>,
    context: Vec<u8>,
    start_id: String,
    destination_id: String,
) -> Result<String, MobileError> {
    let snapshot = MobilitySnapshot::from_json(&bundle, &profile, &context).map_err(failed)?;
    let result = match snapshot.route(&start_id, &destination_id).map_err(failed)? {
        MobilityRouteOutcome::Routed(assessment) => {
            json!({"outcome":"routed","assessment":assessment})
        }
        MobilityRouteOutcome::InsufficientEvidence(diagnostic) => {
            json!({"outcome":"insufficient-evidence","diagnostic":diagnostic})
        }
        MobilityRouteOutcome::Infeasible(diagnostic) => {
            json!({"outcome":"infeasible","diagnostic":diagnostic})
        }
    };
    encode(&result)
}

#[uniffi::export]
pub fn mobile_contract_version() -> String {
    "0.1.0".into()
}
