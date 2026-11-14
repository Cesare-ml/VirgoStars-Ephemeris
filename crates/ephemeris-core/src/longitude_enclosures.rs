// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

//! Conservative circular longitude enclosure from a caller-declared speed bound.
//!
//! This module does NOT derive the speed bound from DE441, verify astronomical
//! coverage, compute a frame transform, or certify continuous data. It is a
//! mathematical building block for a future Rust ephemeris interval producer.
//!
//! For all t in [start, end], if |d(longitude)/dt| <= L in a fixed declared
//! frame/time scale, the circular distance from the reference instant cannot
//! exceed L * max(reference - start, end - reference). This holds across
//! direct motion, retrograde motion, and stationary reversals, provided the
//! *global* bound L truly covers the interval (including every station).
//!
//! Added 2026-11-12: original VirgoStars continuous-rate envelope primitive.

use std::error::Error;
use std::fmt;

/// Time scale for the supplied offsets and the claimed angular speed.
///
/// UTC is intentionally excluded: civil-day length and leap seconds must be
/// resolved by a separate validated time conversion before using this math.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniformTimeScale {
    Tdb,
    Tt,
}

/// Data *asserted* by the caller, not proof that a real ephemeris satisfies it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LongitudeRateBoundClaim {
    /// Reference longitude in canonical [0, 360) degrees.
    pub reference_longitude_deg: f64,
    /// Start of uncertainty interval relative to reference, in days, <= 0.
    pub start_offset_days: f64,
    /// End of uncertainty interval relative to reference, in days, >= 0.
    pub end_offset_days: f64,
    /// Claimed upper bound on |d(longitude)/dt| everywhere in the interval.
    pub max_abs_rate_deg_per_day: f64,
    /// Conservative absolute error in the reference longitude, in degrees.
    pub reference_error_deg: f64,
    pub time_scale: UniformTimeScale,
    /// Must be true only after the upstream provider has established a
    /// continuous bound *including* all retrograde and stationary passages.
    pub asserts_full_interval_rate_bound: bool,
}

/// Circumferential arc measured forward from start, with span in [0, 360].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClaimedLongitudeEnclosure {
    pub start_deg: f64,
    pub span_deg: f64,
    pub time_scale: UniformTimeScale,
    /// This is a caller assertion, NEVER independently validated evidence.
    pub coverage: EnclosureCoverage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnclosureCoverage {
    CallerAssertedRateBound,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LongitudeEnclosureError {
    NonFiniteInput,
    NonCanonicalLongitude,
    InvalidTimeOffsets,
    InvalidRateOrErrorBound,
    FullIntervalBoundNotAsserted,
}

impl fmt::Display for LongitudeEnclosureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "longitude envelope inputs must be finite"),
            Self::NonCanonicalLongitude => {
                write!(f, "reference longitude must be within [0, 360) degrees")
            }
            Self::InvalidTimeOffsets => {
                write!(f, "start offset must be <= 0 and end offset must be >= 0")
            }
            Self::InvalidRateOrErrorBound => {
                write!(f, "speed and reference error limits must be nonnegative")
            }
            Self::FullIntervalBoundNotAsserted => write!(
                f,
                "caller must explicitly assert full-interval coverage including stations"
            ),
        }
    }
}
impl Error for LongitudeEnclosureError {}

/// Produce an arc guaranteed *conditionally* to cover the claimed motion.
///
/// The assumption is a global absolute angular speed bound in the given
/// continuous time scale and SAME longitude frame throughout. Time samples
/// or an instantaneous speed at the representative date are insufficient.
///
/// Numerical inputs are finite and validated. In finite-precision arithmetic,
/// a small extra outward pad is applied to the computed bound; arithmetic
/// overflow conservatively returns the full circle instead of a false arc.
/// If the exact mathematical bound is zero, the point arc is retained so that
/// known-birth-time zero-span contracts remain representable.
///
/// IMPORTANT: this calculation neither derives nor verifies L. A future
/// scientifically validated Rust provider must supply a proven bound and
/// retain provenance; do not label this output astronomy-certified.
pub fn enclose_longitude_from_claim(
    claim: LongitudeRateBoundClaim,
) -> Result<ClaimedLongitudeEnclosure, LongitudeEnclosureError> {
    let numbers = [
        claim.reference_longitude_deg,
        claim.start_offset_days,
        claim.end_offset_days,
        claim.max_abs_rate_deg_per_day,
        claim.reference_error_deg,
    ];
    if numbers.iter().any(|x| !x.is_finite()) {
        return Err(LongitudeEnclosureError::NonFiniteInput);
    }
    if claim.reference_longitude_deg < 0.0 || claim.reference_longitude_deg >= 360.0 {
        return Err(LongitudeEnclosureError::NonCanonicalLongitude);
    }
    if claim.start_offset_days > 0.0 || claim.end_offset_days < 0.0 {
        return Err(LongitudeEnclosureError::InvalidTimeOffsets);
    }
    if claim.max_abs_rate_deg_per_day < 0.0 || claim.reference_error_deg < 0.0 {
        return Err(LongitudeEnclosureError::InvalidRateOrErrorBound);
    }
    if !claim.asserts_full_interval_rate_bound {
        return Err(LongitudeEnclosureError::FullIntervalBoundNotAsserted);
    }

    let max_distance_days = (-claim.start_offset_days).max(claim.end_offset_days);
    let raw_radius = claim.max_abs_rate_deg_per_day * max_distance_days
        + claim.reference_error_deg;

    let wrap = |start_deg: f64, span_deg: f64| ClaimedLongitudeEnclosure {
        start_deg,
        span_deg,
        time_scale: claim.time_scale,
        coverage: EnclosureCoverage::CallerAssertedRateBound,
    };

    // A saturated angular enclosure is deliberately non-informative.
    if !raw_radius.is_finite() || raw_radius >= 180.0 {
        return Ok(wrap(0.0, 360.0));
    }
    if raw_radius == 0.0 {
        return Ok(wrap(claim.reference_longitude_deg, 0.0));
    }

    // Conservative guard for floating-point multiplication, summation,
    // subtraction and modulo near both 0/360 and the envelope boundary.
    // It is a numerical pad, NOT a substitute for astronomical uncertainty.
    let pad = 64.0 * f64::EPSILON * (360.0 + raw_radius);
    let radius = raw_radius + pad;
    if !radius.is_finite() || radius >= 180.0 {
        return Ok(wrap(0.0, 360.0));
    }
    let start = (claim.reference_longitude_deg - radius).rem_euclid(360.0);
    let span = (2.0 * radius).min(360.0);
    Ok(wrap(start, span))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim() -> LongitudeRateBoundClaim {
        LongitudeRateBoundClaim {
            reference_longitude_deg: 359.0,
            start_offset_days: -1.0,
            end_offset_days: 1.0,
            max_abs_rate_deg_per_day: 3.0,
            reference_error_deg: 0.0,
            time_scale: UniformTimeScale::Tdb,
            asserts_full_interval_rate_bound: true,
        }
    }

    fn contains(arc: ClaimedLongitudeEnclosure, longitude_deg: f64) -> bool {
        let distance = (longitude_deg - arc.start_deg).rem_euclid(360.0);
        distance <= arc.span_deg + 1.0e-10
    }

    #[test]
    fn wrap_across_zero_encloses_both_direct_and_retrograde_motion() {
        let arc = enclose_longitude_from_claim(claim()).unwrap();
        assert!(arc.start_deg > 355.0);
        assert!(arc.span_deg > 6.0);
        for longitude in [356.0, 359.0, 0.0, 1.0, 2.0] {
            assert!(contains(arc, longitude), "{longitude} must be covered");
        }
        assert!(!contains(arc, 180.0));
    }

    #[test]
    fn stationary_reversal_is_enclosed_by_one_global_speed_bound() {
        // Synthetic longitude(t) = 20 + 3 t^2, t in [-1,1].
        // Its rate is 6t degrees/day, including a station at t=0.
        let arc = enclose_longitude_from_claim(LongitudeRateBoundClaim {
            reference_longitude_deg: 20.0,
            max_abs_rate_deg_per_day: 6.0,
            ..claim()
        }).unwrap();
        for step in -100..=100 {
            let t = f64::from(step) / 100.0;
            assert!(contains(arc, 20.0 + 3.0 * t * t));
        }
    }

    #[test]
    fn direct_and_retrograde_linear_trajectories_fit() {
        let arc = enclose_longitude_from_claim(claim()).unwrap();
        for step in -100..=100 {
            let t = f64::from(step) / 100.0;
            assert!(contains(arc, (359.0 + 3.0 * t).rem_euclid(360.0)));
            assert!(contains(arc, (359.0 - 3.0 * t).rem_euclid(360.0)));
        }
    }

    #[test]
    fn exact_known_instant_produces_point_arc() {
        let arc = enclose_longitude_from_claim(LongitudeRateBoundClaim {
            start_offset_days: 0.0,
            end_offset_days: 0.0,
            reference_error_deg: 0.0,
            ..claim()
        }).unwrap();
        assert_eq!(arc.start_deg, 359.0);
        assert_eq!(arc.span_deg, 0.0);
    }

    #[test]
    fn asymmetric_interval_uses_farther_time_endpoint() {
        let arc = enclose_longitude_from_claim(LongitudeRateBoundClaim {
            reference_longitude_deg: 10.0,
            start_offset_days: -2.0,
            end_offset_days: 0.5,
            ..claim()
        }).unwrap();
        assert!(contains(arc, 4.0));
        assert!(contains(arc, 16.0));
        assert!(!contains(arc, 200.0));
    }

    #[test]
    fn reference_error_is_included_in_the_arc() {
        let arc = enclose_longitude_from_claim(LongitudeRateBoundClaim {
            start_offset_days: 0.0,
            end_offset_days: 0.0,
            max_abs_rate_deg_per_day: 0.0,
            reference_error_deg: 0.5,
            ..claim()
        }).unwrap();
        assert!(contains(arc, 358.5));
        assert!(contains(arc, 359.5));
    }

    #[test]
    fn unconstrained_motion_saturates_to_full_circle() {
        for speed in [181.0, f64::MAX] {
            let arc = enclose_longitude_from_claim(LongitudeRateBoundClaim {
                max_abs_rate_deg_per_day: speed,
                ..claim()
            }).unwrap();
            assert_eq!(arc.start_deg, 0.0);
            assert_eq!(arc.span_deg, 360.0);
        }
    }

    #[test]
    fn invalid_claims_fail_closed() {
        let bad = [
            (LongitudeRateBoundClaim {
                reference_longitude_deg: f64::NAN,
                ..claim()
            }, LongitudeEnclosureError::NonFiniteInput),
            (LongitudeRateBoundClaim {
                reference_longitude_deg: 360.0,
                ..claim()
            }, LongitudeEnclosureError::NonCanonicalLongitude),
            (LongitudeRateBoundClaim {
                start_offset_days: 1.0,
                ..claim()
            }, LongitudeEnclosureError::InvalidTimeOffsets),
            (LongitudeRateBoundClaim {
                end_offset_days: -1.0,
                ..claim()
            }, LongitudeEnclosureError::InvalidTimeOffsets),
            (LongitudeRateBoundClaim {
                max_abs_rate_deg_per_day: -1.0,
                ..claim()
            }, LongitudeEnclosureError::InvalidRateOrErrorBound),
            (LongitudeRateBoundClaim {
                reference_error_deg: -0.1,
                ..claim()
            }, LongitudeEnclosureError::InvalidRateOrErrorBound),
            (LongitudeRateBoundClaim {
                asserts_full_interval_rate_bound: false,
                ..claim()
            }, LongitudeEnclosureError::FullIntervalBoundNotAsserted),
        ];
        for (bad_claim, expected_error) in bad {
            assert_eq!(enclose_longitude_from_claim(bad_claim), Err(expected_error));
        }
    }

    #[test]
    fn zero_rate_even_over_long_duration_returns_point_when_claimed() {
        let arc = enclose_longitude_from_claim(LongitudeRateBoundClaim {
            start_offset_days: -20.0,
            end_offset_days: 20.0,
            max_abs_rate_deg_per_day: 0.0,
            reference_error_deg: 0.0,
            ..claim()
        }).unwrap();
        assert_eq!(arc.start_deg, 359.0);
        assert_eq!(arc.span_deg, 0.0);
    }

    #[test]
    fn full_circle_cover_is_explicitly_conditional_not_verified() {
        let arc = enclose_longitude_from_claim(LongitudeRateBoundClaim {
            reference_longitude_deg: 0.0,
            max_abs_rate_deg_per_day: 180.0,
            time_scale: UniformTimeScale::Tt,
            ..claim()
        }).unwrap();
        assert_eq!(arc.coverage, EnclosureCoverage::CallerAssertedRateBound);
        assert_eq!(arc.time_scale, UniformTimeScale::Tt);
        assert_eq!(arc.span_deg, 360.0);
    }
}
