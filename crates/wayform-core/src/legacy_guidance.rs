//! A live policy lease around a complete verified legacy guidance result.
use crate::{
    LegacyCompiledOptions, LegacyFreshnessStatus, LegacyOperationalFreshness,
    LegacyOperationalLease, LegacyPolicyError, LegacyPolicyIndex, LegacyRouteError,
    LegacySessionInvalidation, LegacySessionRequest, LegacyVenuePackage,
};
use serde_json::Value;
use std::{fmt, sync::Arc};

#[derive(Debug)]
pub enum LegacyGuidanceError {
    Policy(LegacyPolicyError),
    Route(LegacyRouteError),
    PolicyUnavailable(LegacyOperationalFreshness),
    RouteUnavailable(Value),
}
impl fmt::Display for LegacyGuidanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(e) => e.fmt(f),
            Self::Route(e) => e.fmt(f),
            Self::PolicyUnavailable(state) => write!(
                f,
                "route policy unavailable: {}",
                state.reason.unwrap_or("unavailable")
            ),
            Self::RouteUnavailable(result) => {
                f.write_str(result["error"].as_str().unwrap_or("route unavailable"))
            }
        }
    }
}
impl std::error::Error for LegacyGuidanceError {}

#[derive(Debug)]
pub enum LegacyGuidanceRead<'a> {
    Ready(&'a Value),
    PolicyUnavailable(LegacyOperationalFreshness),
    Invalidated(LegacySessionInvalidation),
}
pub struct LegacyGuidanceSession {
    lease: LegacyOperationalLease,
    result: Option<Value>,
    invalidated: Option<LegacySessionInvalidation>,
}
impl fmt::Debug for LegacyGuidanceSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LegacyGuidanceSession")
            .field("invalidated", &self.invalidated)
            .finish_non_exhaustive()
    }
}
impl LegacyGuidanceSession {
    /// Plans without exposing guidance. Read with freshly sampled clocks after
    /// construction, before rendering or speaking any part of the result.
    pub fn new(
        venue: Arc<LegacyVenuePackage>,
        request: LegacySessionRequest,
        overlay: Value,
        wall_ms: f64,
        monotonic_ms: f64,
    ) -> Result<Self, LegacyGuidanceError> {
        let index = Arc::new(
            LegacyPolicyIndex::new(venue.policy_package()).map_err(LegacyGuidanceError::Policy)?,
        );
        let mut lease = LegacyOperationalLease::new(index, overlay.clone(), wall_ms, monotonic_ms)
            .map_err(LegacyGuidanceError::Policy)?;
        let freshness = lease
            .read(wall_ms, monotonic_ms)
            .map_err(LegacyGuidanceError::Policy)?;
        if freshness.status != LegacyFreshnessStatus::Current {
            return Err(LegacyGuidanceError::PolicyUnavailable(freshness));
        }
        let options = LegacyCompiledOptions {
            accessible_only: request.accessible_only,
            allow_restricted: request.allow_restricted,
            operational_overlay: Some(overlay),
            evaluated_at: Some(lease.resolution().unwrap().evaluated_at.clone()),
        };
        let result = venue
            .calculate_route(&request.start_id, &request.destination_id, &options)
            .map_err(LegacyGuidanceError::Route)?;
        if result["found"] != true {
            return Err(LegacyGuidanceError::RouteUnavailable(result));
        }
        Ok(Self {
            lease,
            result: Some(result),
            invalidated: None,
        })
    }

    pub fn read(
        &mut self,
        wall_ms: f64,
        monotonic_ms: f64,
    ) -> Result<LegacyGuidanceRead<'_>, LegacyGuidanceError> {
        if let Some(reason) = self.invalidated {
            return Ok(LegacyGuidanceRead::Invalidated(reason));
        }
        let freshness = match self.lease.read(wall_ms, monotonic_ms) {
            Ok(state) => state,
            Err(error) => {
                self.result = None;
                return Err(LegacyGuidanceError::Policy(error));
            }
        };
        if freshness.status != LegacyFreshnessStatus::Current {
            self.result = None;
            return Ok(LegacyGuidanceRead::PolicyUnavailable(freshness));
        }
        match self.result.as_ref() {
            Some(result) => Ok(LegacyGuidanceRead::Ready(result)),
            None => Ok(LegacyGuidanceRead::Invalidated(
                LegacySessionInvalidation::RouteContextChanged,
            )),
        }
    }

    pub fn invalidate(&mut self, reason: LegacySessionInvalidation) {
        self.invalidated.get_or_insert(reason);
        self.result = None;
    }

    /// A timer hint, never permission to retain or deliver a previous read.
    pub fn next_check_delay(&self, wall_ms: f64, monotonic_ms: f64) -> f64 {
        self.lease.delay(wall_ms, monotonic_ms)
    }
}
