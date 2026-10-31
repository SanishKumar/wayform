//! Local expiry lease for the pinned VoiceGIS operational policy.
//! Source: SanishKumar/voicegis-indoor-ar, commit 305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f.
//! Original source and rights statement remain under compat/voicegis/upstream/.

use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::legacy_policy::overlay_time_bounds;
use crate::validation::{utc_from_millis, valid_time};
use crate::{LegacyOverlayResolution, LegacyPolicyError, LegacyPolicyIndex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LegacyFreshnessStatus {
    Current,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyOperationalFreshness {
    pub status: LegacyFreshnessStatus,
    pub overlay_id: String,
    pub reason: Option<&'static str>,
}

// JavaScript Date truncates fractional Unix milliseconds toward zero. The
// native contract additionally restricts dates to canonical four-digit years.
fn wall_millis(value: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    let clipped = value.trunc() as i64;
    valid_time(clipped).then_some(clipped)
}

/// Owns one overlay and its clock history. No system clocks or timers are read.
/// Hosts must retain this instance and check read() before using route guidance.
#[derive(Debug)]
pub struct LegacyOperationalLease {
    index: Arc<LegacyPolicyIndex>,
    overlay: Value,
    valid_from: Option<f64>,
    valid_until: Option<f64>,
    deadline: Option<f64>,
    last_wall: f64,
    last_monotonic: f64,
    clock_lost: bool,
    expired: bool,
    resolution: Option<Result<LegacyOverlayResolution, LegacyPolicyError>>,
}

impl LegacyOperationalLease {
    pub fn new(
        index: Arc<LegacyPolicyIndex>,
        overlay: Value,
        wall_ms: f64,
        monotonic_ms: f64,
    ) -> Result<Self, LegacyPolicyError> {
        let (from, until) = overlay_time_bounds(&overlay)?;
        let valid_until = until.map(|value| value as f64);
        let deadline = valid_until.map(|until| monotonic_ms + (until - wall_ms));
        let clock_lost = wall_millis(wall_ms).is_none()
            || !monotonic_ms.is_finite()
            || deadline.is_some_and(|value| !value.is_finite());
        Ok(Self {
            index,
            overlay,
            valid_from: from.map(|value| value as f64),
            valid_until,
            deadline,
            last_wall: wall_ms,
            last_monotonic: monotonic_ms,
            clock_lost,
            expired: false,
            resolution: None,
        })
    }

    /// Clock loss and observed expiry are permanent for this lease. Errors are
    /// also cached and must block guidance, never be treated as an open policy.
    pub fn read(
        &mut self,
        now_wall: f64,
        now_monotonic: f64,
    ) -> Result<LegacyOperationalFreshness, LegacyPolicyError> {
        let clipped = wall_millis(now_wall);
        if clipped.is_none()
            || !now_monotonic.is_finite()
            || now_wall < self.last_wall - 1000.0
            || now_monotonic < self.last_monotonic
        {
            self.clock_lost = true;
        }
        self.last_wall = self.last_wall.max(now_wall);
        self.last_monotonic = self.last_monotonic.max(now_monotonic);
        if self.valid_until.is_some_and(|until| {
            now_wall >= until
                || self
                    .deadline
                    .is_some_and(|deadline| now_monotonic >= deadline)
        }) {
            self.expired = true;
        }
        let activate = self.resolution.as_ref().is_some_and(|result| {
            result.as_ref().is_ok_and(|resolution| {
                resolution
                    .issues
                    .first()
                    .is_some_and(|issue| issue.code == "overlay-not-active")
                    && self.valid_from.is_some_and(|from| now_wall >= from)
            })
        });
        if self.resolution.is_none() || activate {
            let evaluated_at = clipped.and_then(utc_from_millis).unwrap_or_default();
            self.resolution = Some(self.index.resolve(&self.overlay, &evaluated_at));
        }
        let resolution = self
            .resolution
            .as_ref()
            .expect("resolution was initialized")
            .as_ref()
            .map_err(|error| *error)?;
        let reason = if self.clock_lost {
            Some("clock-unreliable")
        } else if self.expired {
            Some("overlay-expired")
        } else {
            resolution.issues.first().map(|issue| issue.code)
        };
        Ok(LegacyOperationalFreshness {
            status: if !self.clock_lost && !self.expired && resolution.valid {
                LegacyFreshnessStatus::Current
            } else {
                LegacyFreshnessStatus::Unavailable
            },
            overlay_id: resolution.overlay_id.clone(),
            reason,
        })
    }

    /// A scheduling hint only. Calling delay does not read/update lease state
    /// and its return value never authorizes guidance; call read at each use.
    pub fn delay(&self, now_wall: f64, now_monotonic: f64) -> f64 {
        let wall_remaining = self
            .valid_until
            .map(|until| until - now_wall)
            .unwrap_or(f64::NAN);
        let monotonic_remaining = self
            .deadline
            .map(|deadline| deadline - now_monotonic)
            .unwrap_or(f64::NAN);
        // Math.min propagates NaN, unlike f64::min.
        let remaining = if wall_remaining.is_nan() || monotonic_remaining.is_nan() {
            f64::NAN
        } else {
            wall_remaining.min(monotonic_remaining)
        };
        if !self.expired && !self.clock_lost && remaining > 0.0 && remaining.is_finite() {
            remaining.clamp(1.0, 1000.0)
        } else {
            1000.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::timestamp;
    use crate::{LegacyPolicyEdge, LegacyPolicyPackage};
    use serde_json::json;

    #[test]
    fn only_future_activation_recomputes_a_cached_resolution() {
        let wall = timestamp("2026-07-22T12:00:00.000Z").unwrap() as f64;
        let index = Arc::new(
            LegacyPolicyIndex::new(LegacyPolicyPackage {
                building_id: "fixture".into(),
                package_hash: "a".repeat(64),
                edges: vec![LegacyPolicyEdge {
                    id: "edge".into(),
                    source_id: "lift".into(),
                }],
            })
            .unwrap(),
        );
        let overlay = json!({
            "schemaVersion": "0.1.0", "id": "future", "buildingId": "fixture", "packageHash": "a".repeat(64),
            "validFrom": "2026-07-22T12:00:01.000Z", "validUntil": "2026-07-22T12:00:05.000Z", "closures": []
        });
        let mut lease = LegacyOperationalLease::new(index, overlay, wall, 0.0).unwrap();
        let evaluated_at = |lease: &LegacyOperationalLease| {
            lease
                .resolution
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .evaluated_at
                .clone()
        };
        assert_eq!(
            lease.read(wall, 0.0).unwrap().reason,
            Some("overlay-not-active")
        );
        let initial = evaluated_at(&lease);
        lease.read(wall + 500.0, 500.0).unwrap();
        assert_eq!(evaluated_at(&lease), initial);
        assert_eq!(
            lease.read(wall + 1000.0, 1000.0).unwrap().status,
            LegacyFreshnessStatus::Current
        );
        let active = evaluated_at(&lease);
        assert_ne!(active, initial);
        lease.read(wall + 2000.0, 2000.0).unwrap();
        assert_eq!(evaluated_at(&lease), active);
        lease.read(wall + 5000.0, 5000.0).unwrap();
        assert_eq!(evaluated_at(&lease), active);
    }
}
