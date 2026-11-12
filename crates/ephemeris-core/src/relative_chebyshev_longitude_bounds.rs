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

//! Conservative *mathematical* fixed-frame relative longitude-rate bounds.
//!
//! Target and observer are separate planar Chebyshev coordinate polynomials
//! referring to one common origin, axes and identical TDB record interval.
//! No observer geometry is invented here; the reference-center metadata must
//! match. Real DE441 segment chains/frame corrections are not implemented.
//!
//! Original VirgoStars contribution, 2026-11-12, CPAL-1.0.

use core::f64::consts::PI;
use core::fmt;

use crate::longitude_enclosures::UniformTimeScale;

/// Intentionally only ICRF fixed equatorial Cartesian axes. Never apparent of date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedCartesianFrame {
    IcrfJ2000,
}

/// A validated caller must provide positions in the same physical units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CartesianLengthUnit {
    Kilometer,
}

/// Describes an entire *single* position Chebyshev record, not sample positions.
///
/// Coefficients follow `sum(c[k] * T_k(tau))`, without halving c0.
/// `tau=(tdb_seconds-midpoint_tdb_seconds)/radius_tdb_seconds`.
/// No frames, time scales or kernels are converted by this struct.
#[derive(Clone, Copy, Debug)]
pub struct AlignedPlanarChebyshevRecord<'a> {
    pub target_naif_id: i32,
    pub common_origin_naif_id: i32,
    pub frame: FixedCartesianFrame,
    pub length_unit: CartesianLengthUnit,
    pub midpoint_tdb_seconds: f64,
    pub radius_tdb_seconds: f64,
    pub x_coefficients: &'a [f64],
    pub y_coefficients: &'a [f64],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelativeChebyshevLongitudeRateBound {
    /// Absolute maximum rate of atan2(Y_target-Y_observer, X_target-X_observer).
    pub max_abs_rate_deg_per_tdb_day: f64,
    /// Lower bound on the planar target-observer distance in kilometers.
    pub min_relative_planar_distance_km: f64,
    /// Upper bound of the planar relative speed in kilometers/TDB day.
    pub max_relative_planar_speed_km_per_tdb_day: f64,
    pub target_naif_id: i32,
    pub observer_naif_id: i32,
    pub common_origin_naif_id: i32,
    pub midpoint_tdb_seconds: f64,
    pub radius_tdb_seconds: f64,
    pub time_scale: UniformTimeScale,
    pub coverage: RelativeRateBoundCoverage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeRateBoundCoverage {
    /// Algebraic coefficient inequality over the complete SAME polynomial record.
    /// Does not certify the coefficients against official ephemeris data.
    FullAlignedMathematicalRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeRateBoundError {
    MissingCoefficients,
    InconsistentRecordAxisLengths,
    ExcessivePolynomialDegree,
    NonFiniteCoefficients,
    InvalidTdbRecord,
    MisalignedRecordEpochs,
    IncompatibleCommonOrigin,
    DuplicateBodyId,
    BoundArithmeticOverflow,
    CannotExcludeRelativePlanarOrigin,
}

impl fmt::Display for RelativeRateBoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCoefficients => write!(f, "planar X/Y coefficient slices must not be empty"),
            Self::InconsistentRecordAxisLengths => {
                write!(f, "within each record, X/Y polynomial lengths must match")
            }
            Self::ExcessivePolynomialDegree => write!(f, "excessive Chebyshev record degree"),
            Self::NonFiniteCoefficients => write!(f, "non-finite Chebyshev coefficient"),
            Self::InvalidTdbRecord => write!(f, "TDB record center/radius is invalid or unresolvable"),
            Self::MisalignedRecordEpochs => {
                write!(f, "target and observer Chebyshev records must have identical TDB domains")
            }
            Self::IncompatibleCommonOrigin => {
                write!(f, "target and observer must have identical known Cartesian origin and units")
            }
            Self::DuplicateBodyId => write!(f, "target and observer must be different bodies"),
            Self::BoundArithmeticOverflow => {
                write!(f, "conservative relative bound arithmetic overflowed")
            }
            Self::CannotExcludeRelativePlanarOrigin => {
                write!(f, "relative longitude may be undefined: planar origin not excluded")
            }
        }
    }
}
impl std::error::Error for RelativeRateBoundError {}

fn validate_record(record: &AlignedPlanarChebyshevRecord<'_>) -> Result<(), RelativeRateBoundError> {
    let len = record.x_coefficients.len();
    if len == 0 || record.y_coefficients.is_empty() {
        return Err(RelativeRateBoundError::MissingCoefficients);
    }
    if len != record.y_coefficients.len() {
        return Err(RelativeRateBoundError::InconsistentRecordAxisLengths);
    }
    if len > 4096 {
        return Err(RelativeRateBoundError::ExcessivePolynomialDegree);
    }
    if record
        .x_coefficients
        .iter()
        .chain(record.y_coefficients.iter())
        .any(|value| !value.is_finite())
    {
        return Err(RelativeRateBoundError::NonFiniteCoefficients);
    }
    let mid = record.midpoint_tdb_seconds;
    let radius = record.radius_tdb_seconds;
    if !mid.is_finite() || !radius.is_finite() || radius <= 0.0
        || !(mid - radius).is_finite() || !(mid + radius).is_finite()
        || mid - radius == mid || mid + radius == mid
    {
        return Err(RelativeRateBoundError::InvalidTdbRecord);
    }
    Ok(())
}

/// Analytically bound the angular speed of a target relative to an observer
/// over the *entire aligned Chebyshev record* in fixed ICRF Cartesian axes.
///
/// The records MUST have the same `midpoint_tdb_seconds` and
/// `radius_tdb_seconds`; no interpolation, resampling, or polynomial
/// reparameterization is performed. They may have different degrees.
///
/// For u in X and Y, write the exact mathematical relative polynomial as
/// `sum((target[k] - observer[k]) T_k)`. With |T_k|<=1 and
/// |d T_k/d tau|<=k^2:
/// `|u| >= |t0-o0| - sum_{k>=1} (|tk| + |ok|)`
/// `|du/dt| <= sum_{k>=1} k^2(|tk| + |ok|) / radius_days`.
///
/// We use the component lower bounds to exclude the relative planar origin,
/// then `|d lambda/dt|<=hypot(u'_x,u'_y)/hypot(u_x,u_y)`.
///
/// This is deliberately conservative; it can refuse valid trajectories.
/// A finite-precision guard includes coefficient-subtraction cancellation
/// around the constant term, but this is not a formal directed-rounding
/// proof or a DE441 coefficient-uncertainty certification. Real JPL data,
/// origin chains, apparent corrections and reference-frame derivatives
/// require separate independently validated stages.
pub fn bound_aligned_relative_chebyshev_longitude_rate(
    target: AlignedPlanarChebyshevRecord<'_>,
    observer: AlignedPlanarChebyshevRecord<'_>,
) -> Result<RelativeChebyshevLongitudeRateBound, RelativeRateBoundError> {
    validate_record(&target)?;
    validate_record(&observer)?;
    if target.target_naif_id == observer.target_naif_id {
        return Err(RelativeRateBoundError::DuplicateBodyId);
    }
    if target.common_origin_naif_id != observer.common_origin_naif_id
        || target.frame != observer.frame || target.length_unit != observer.length_unit
    {
        return Err(RelativeRateBoundError::IncompatibleCommonOrigin);
    }
    if target.midpoint_tdb_seconds != observer.midpoint_tdb_seconds
        || target.radius_tdb_seconds != observer.radius_tdb_seconds
    {
        return Err(RelativeRateBoundError::MisalignedRecordEpochs);
    }

    // Explicitly account for coefficient magnitude/cancellation, plus
    // polynomial summation rounding. It is intentionally not exact interval
    // arithmetic; do not use this to claim a machine-certified JPL bound.
    let max_len = target.x_coefficients.len().max(observer.x_coefficients.len());
    let guard = 1.0 + 64.0 * f64::EPSILON * (max_len as f64 + 4.0);
    if !guard.is_finite() {
        return Err(RelativeRateBoundError::BoundArithmeticOverflow);
    }

    fn axis_bounds(
        target: &[f64],
        observer: &[f64],
        max_len: usize,
        guard: f64,
    ) -> Result<(f64, f64), RelativeRateBoundError> {
        let constant_delta = target[0] - observer[0];
        let base_magnitudes = target[0].abs() + observer[0].abs();
        if !constant_delta.is_finite() || !base_magnitudes.is_finite() {
            return Err(RelativeRateBoundError::BoundArithmeticOverflow);
        }

        let mut tail_sum = 0.0_f64;
        let mut derivative_sum = 0.0_f64;
        for k in 1..max_len {
            let tc = target.get(k).copied().unwrap_or(0.0);
            let oc = observer.get(k).copied().unwrap_or(0.0);
            let magnitude = tc.abs() + oc.abs();
            let weight = (k as f64) * (k as f64);
            tail_sum += magnitude;
            derivative_sum += weight * magnitude;
            if !tail_sum.is_finite() || !derivative_sum.is_finite() {
                return Err(RelativeRateBoundError::BoundArithmeticOverflow);
            }
        }

        let rounding_margin = 16.0 * f64::EPSILON
            * (base_magnitudes + constant_delta.abs());
        let tail_upper = tail_sum * guard;
        let derivative_upper = derivative_sum * guard;
        if !rounding_margin.is_finite() || !tail_upper.is_finite()
            || !derivative_upper.is_finite()
        {
            return Err(RelativeRateBoundError::BoundArithmeticOverflow);
        }
        let center_lower = constant_delta.abs() / guard;
        let min_component = (center_lower - rounding_margin - tail_upper).max(0.0);
        Ok((min_component, derivative_upper))
    }

    let (min_x, dx_normalized) = axis_bounds(
        target.x_coefficients, observer.x_coefficients, max_len, guard,
    )?;
    let (min_y, dy_normalized) = axis_bounds(
        target.y_coefficients, observer.y_coefficients, max_len, guard,
    )?;
    let min_r = min_x.hypot(min_y) / guard;
    if !min_r.is_finite() {
        return Err(RelativeRateBoundError::BoundArithmeticOverflow);
    }
    if min_r <= 0.0 {
        return Err(RelativeRateBoundError::CannotExcludeRelativePlanarOrigin);
    }

    // The Chebyshev domain radius is in TDB seconds. The numerator uses
    // normalized-coordinate derivatives, so seconds-to-days is explicit.
    const SECONDS_PER_TDB_DAY: f64 = 86_400.0;
    let radius_days = target.radius_tdb_seconds / SECONDS_PER_TDB_DAY;
    if radius_days <= 0.0 {
        return Err(RelativeRateBoundError::BoundArithmeticOverflow);
    }
    let speed = (dx_normalized.hypot(dy_normalized) / radius_days) * guard;
    let angular_rate = (speed / min_r) * (180.0 / PI) * guard;
    if !speed.is_finite() || !angular_rate.is_finite() {
        return Err(RelativeRateBoundError::BoundArithmeticOverflow);
    }
    Ok(RelativeChebyshevLongitudeRateBound {
        max_abs_rate_deg_per_tdb_day: angular_rate,
        min_relative_planar_distance_km: min_r,
        max_relative_planar_speed_km_per_tdb_day: speed,
        target_naif_id: target.target_naif_id,
        observer_naif_id: observer.target_naif_id,
        common_origin_naif_id: target.common_origin_naif_id,
        midpoint_tdb_seconds: target.midpoint_tdb_seconds,
        radius_tdb_seconds: target.radius_tdb_seconds,
        time_scale: UniformTimeScale::Tdb,
        coverage: RelativeRateBoundCoverage::FullAlignedMathematicalRecord,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chebyshev::evaluate_chebyshev;
    use crate::longitude_enclosures::{
        enclose_longitude_from_claim, LongitudeRateBoundClaim,
    };

    fn base<'a>(
        body: i32, x: &'a [f64], y: &'a [f64],
    ) -> AlignedPlanarChebyshevRecord<'a> {
        AlignedPlanarChebyshevRecord {
            target_naif_id:body,
            common_origin_naif_id:0,
            frame:FixedCartesianFrame::IcrfJ2000,
            length_unit:CartesianLengthUnit::Kilometer,
            midpoint_tdb_seconds:0.0,
            radius_tdb_seconds:86_400.0,
            x_coefficients:x,y_coefficients:y,
        }
    }
    fn sample_relative(
        target: AlignedPlanarChebyshevRecord<'_>,
        observer: AlignedPlanarChebyshevRecord<'_>,
        tau:f64,
    ) -> (f64,f64,f64,f64) {
        let tx=evaluate_chebyshev(target.x_coefficients,tau).unwrap();
        let ty=evaluate_chebyshev(target.y_coefficients,tau).unwrap();
        let ox=evaluate_chebyshev(observer.x_coefficients,tau).unwrap();
        let oy=evaluate_chebyshev(observer.y_coefficients,tau).unwrap();
        (tx.value-ox.value,ty.value-oy.value,
         tx.derivative_dx-ox.derivative_dx,ty.derivative_dx-oy.derivative_dx)
    }
    fn assert_all_samples_bounded(
        target:AlignedPlanarChebyshevRecord<'_>,
        observer:AlignedPlanarChebyshevRecord<'_>,
        upper:f64,
    ){
        for j in -200..=200 {
            let tau=f64::from(j)/200.0;
            let (x,y,dx,dy)=sample_relative(target,observer,tau);
            let rate=((x*dy-y*dx)/(x*x+y*y)).to_degrees();
            assert!(rate.abs()<=upper*(1.0+1.0e-12),
                "rate {} exceeded bound {} at {}",rate,upper,tau);
        }
    }

    #[test]
    fn common_origin_subtraction_has_correct_synthetic_rate() {
        let target=base(499,&[100.0,0.0],&[0.0,1.0]);
        let earth=base(399,&[90.0,0.0],&[0.0,0.0]);
        let rate=bound_aligned_relative_chebyshev_longitude_rate(target,earth).unwrap();
        assert!(rate.max_abs_rate_deg_per_tdb_day>5.0);
        assert!(rate.min_relative_planar_distance_km>0.0);
        assert_eq!(rate.target_naif_id,499);
        assert_eq!(rate.observer_naif_id,399);
        assert_eq!(rate.common_origin_naif_id,0);
        assert_eq!(rate.time_scale,UniformTimeScale::Tdb);
        assert_all_samples_bounded(target,earth,rate.max_abs_rate_deg_per_tdb_day);
    }

    #[test]
    fn uneven_polynomial_degrees_are_zero_padded_algebraically() {
        let target=base(499,&[100.0,0.0,0.0],&[0.0,0.0,0.5]);
        let earth=base(399,&[90.0],&[0.0]);
        let rate=bound_aligned_relative_chebyshev_longitude_rate(target,earth).unwrap();
        assert_all_samples_bounded(target,earth,rate.max_abs_rate_deg_per_tdb_day);
    }

    #[test]
    fn station_and_retrograde_reversal_in_relative_trajectory() {
        // Relative Y(tau)=0.5*T2(tau), derivative changes sign at tau=0.
        let target=base(499,&[100.0,0.0,0.0],&[3.0,0.0,0.5]);
        let earth=base(399,&[90.0],&[3.0]);
        let result=bound_aligned_relative_chebyshev_longitude_rate(target,earth).unwrap();
        let (_,_,_,d_center)=sample_relative(target,earth,0.0);
        assert_eq!(d_center,0.0);
        assert_all_samples_bounded(target,earth,result.max_abs_rate_deg_per_tdb_day);
    }

    #[test]
    fn relative_fixed_frame_bound_feeds_conditional_circular_enclosure() {
        let target=base(499,&[100.0,0.0],&[0.0,1.0]);
        let earth=base(399,&[90.0,0.0],&[0.0,0.0]);
        let bound=bound_aligned_relative_chebyshev_longitude_rate(target,earth).unwrap();
        let arc=enclose_longitude_from_claim(LongitudeRateBoundClaim{
            reference_longitude_deg:0.0,
            start_offset_days:-1.0,
            end_offset_days:1.0,
            max_abs_rate_deg_per_day:bound.max_abs_rate_deg_per_tdb_day,
            reference_error_deg:0.0,
            time_scale:UniformTimeScale::Tdb,
            asserts_full_interval_rate_bound:true,
        }).unwrap();
        for tau in [-1.0,-0.5,0.0,0.5,1.0] {
            let (x,y,_,_)=sample_relative(target,earth,tau);
            let longitude=y.atan2(x).to_degrees().rem_euclid(360.0);
            assert!((longitude-arc.start_deg).rem_euclid(360.0)<=arc.span_deg+1.0e-9);
        }
    }

    #[test]
    fn mismatched_time_windows_are_rejected_not_interpolated() {
        let target=base(499,&[100.0],&[1.0]);
        let mut earth=base(399,&[90.0],&[0.0]);
        earth.midpoint_tdb_seconds=1.0;
        assert_eq!(
            bound_aligned_relative_chebyshev_longitude_rate(target,earth),
            Err(RelativeRateBoundError::MisalignedRecordEpochs)
        );
        earth.midpoint_tdb_seconds=0.0;
        earth.radius_tdb_seconds=43_200.0;
        assert_eq!(
            bound_aligned_relative_chebyshev_longitude_rate(target,earth),
            Err(RelativeRateBoundError::MisalignedRecordEpochs)
        );
    }

    #[test]
    fn unknown_or_incompatible_origin_and_body_are_rejected() {
        let target=base(499,&[100.0],&[1.0]);
        let mut earth=base(399,&[90.0],&[0.0]);
        earth.common_origin_naif_id=10;
        assert_eq!(bound_aligned_relative_chebyshev_longitude_rate(target,earth),
            Err(RelativeRateBoundError::IncompatibleCommonOrigin));
        earth.common_origin_naif_id=0;
        earth.target_naif_id=499;
        assert_eq!(bound_aligned_relative_chebyshev_longitude_rate(target,earth),
            Err(RelativeRateBoundError::DuplicateBodyId));
    }

    #[test]
    fn refuses_planar_origin_singularity_and_conservative_unknown() {
        let target=base(499,&[100.0,1.0],&[1.0,1.0]);
        let earth=base(399,&[100.0],&[1.0]);
        assert_eq!(bound_aligned_relative_chebyshev_longitude_rate(target,earth),
            Err(RelativeRateBoundError::CannotExcludeRelativePlanarOrigin));
    }

    #[test]
    fn malformed_coefficients_and_tdb_record_fail() {
        let target=base(499,&[100.0],&[1.0]);
        let empty=base(399,&[],&[]);
        assert_eq!(bound_aligned_relative_chebyshev_longitude_rate(target,empty),
            Err(RelativeRateBoundError::MissingCoefficients));
        let bad=base(399,&[90.0],&[0.0,1.0]);
        assert_eq!(bound_aligned_relative_chebyshev_longitude_rate(target,bad),
            Err(RelativeRateBoundError::InconsistentRecordAxisLengths));
        let bad=base(399,&[f64::NAN],&[0.0]);
        assert_eq!(bound_aligned_relative_chebyshev_longitude_rate(target,bad),
            Err(RelativeRateBoundError::NonFiniteCoefficients));
        let mut bad=base(399,&[90.0],&[0.0]);
        bad.radius_tdb_seconds=0.0;
        assert_eq!(bound_aligned_relative_chebyshev_longitude_rate(target,bad),
            Err(RelativeRateBoundError::InvalidTdbRecord));
        bad.radius_tdb_seconds=86_400.0;
        bad.midpoint_tdb_seconds=f64::MAX;
        assert_eq!(bound_aligned_relative_chebyshev_longitude_rate(target,bad),
            Err(RelativeRateBoundError::InvalidTdbRecord));
    }

    #[test]
    fn catastrophic_cancellation_refuses_finite_relative_radius() {
        let target=base(499,&[1.0e308,0.0],&[0.0,0.0]);
        let observer=base(399,&[1.0e308,0.0],&[0.0,0.0]);
        assert_eq!(bound_aligned_relative_chebyshev_longitude_rate(target,observer),
            Err(RelativeRateBoundError::BoundArithmeticOverflow));
    }

    #[test]
    fn output_metadata_is_appropriately_conditional() {
        let target=base(499,&[100.0],&[1.0]);
        let earth=base(399,&[90.0],&[0.0]);
        let result=bound_aligned_relative_chebyshev_longitude_rate(target,earth).unwrap();
        assert_eq!(result.coverage,RelativeRateBoundCoverage::FullAlignedMathematicalRecord);
        assert_eq!(result.radius_tdb_seconds,86_400.0);
        assert_eq!(result.max_relative_planar_speed_km_per_tdb_day,0.0);
        assert_eq!(result.max_abs_rate_deg_per_tdb_day,0.0);
    }
}
