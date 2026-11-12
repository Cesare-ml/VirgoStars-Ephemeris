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

//! Global *mathematical* relative-longitude speed bound for two overlapping,
//! differently parameterized planar Chebyshev records in fixed ICRF axes.
//!
//! Each polynomial is evaluated only in its own Chebyshev domain. Neither
//! resampling nor coefficient alignment is required, and neither is performed.
//! This is NOT a certified JPL/DE441 observer state, apparent longitude,
//! multi-record stitcher, or proof of numeric floating-point containment.
//!
//! Original VirgoStars contribution, 2026-11-12. License: CPAL-1.0.

use core::f64::consts::PI;
use core::fmt;

use crate::relative_chebyshev_longitude_bounds::{
    CartesianLengthUnit, FixedCartesianFrame,
};

/// Full single Chebyshev record in the same fixed ICRF/Cartesian reference
/// system as the other record. No implicit coordinate or origin conversion.
#[derive(Clone, Copy, Debug)]
pub struct IndependentPlanarChebyshevRecord<'a> {
    pub target_naif_id: i32,
    pub common_origin_naif_id: i32,
    pub frame: FixedCartesianFrame,
    pub length_unit: CartesianLengthUnit,
    pub midpoint_tdb_seconds: f64,
    pub radius_tdb_seconds: f64,
    /// Chebyshev coefficients including unhalved c0; degrees may differ
    /// between the two records, but x and y degrees within one must match.
    pub x_coefficients: &'a [f64],
    pub y_coefficients: &'a [f64],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlappingRecordCoverage {
    /// Full requested strict subinterval of the intersection of two
    /// mathematical records; no claim about adjacent records or real kernels.
    RequestedOverlapOfTwoPolynomials,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlappingRelativeLongitudeRateBound {
    /// Supremum bound for absolute angular rate in degrees per uniform TDB day.
    pub max_abs_rate_deg_per_tdb_day: f64,
    pub min_relative_planar_distance_km: f64,
    pub max_relative_planar_speed_km_per_tdb_day: f64,
    pub start_tdb_seconds: f64,
    pub end_tdb_seconds: f64,
    pub target_naif_id: i32,
    pub observer_naif_id: i32,
    pub common_origin_naif_id: i32,
    pub coverage: OverlappingRecordCoverage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlappingRecordError {
    InvalidPolynomialRecord,
    NonFiniteCoefficient,
    DifferentAxisDegrees,
    UnsupportedDegree,
    InvalidRecordEpoch,
    IncompatibleOrigin,
    DuplicateBody,
    InvalidRequestedInterval,
    OutsideCommonInterval,
    CannotExcludeRelativeOrigin,
    BoundOverflow,
}

impl fmt::Display for OverlappingRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolynomialRecord => write!(f, "Chebyshev records require coefficients"),
            Self::NonFiniteCoefficient => write!(f, "Chebyshev coefficients must be finite"),
            Self::DifferentAxisDegrees => write!(f, "x/y degrees within a record must match"),
            Self::UnsupportedDegree => write!(f, "Chebyshev record degree exceeds 4095"),
            Self::InvalidRecordEpoch => write!(f, "record epoch/radius is invalid or unresolvable"),
            Self::IncompatibleOrigin => write!(f, "common Cartesian origins/frames/units differ"),
            Self::DuplicateBody => write!(f, "target and observer must be distinct"),
            Self::InvalidRequestedInterval => write!(f, "requested TDB interval must be finite and positive"),
            Self::OutsideCommonInterval => write!(f, "requested interval must fit both record domains"),
            Self::CannotExcludeRelativeOrigin => write!(f, "cannot bound angular speed near planar origin"),
            Self::BoundOverflow => write!(f, "non-finite numeric bound; no precision fallback"),
        }
    }
}
impl std::error::Error for OverlappingRecordError {}

fn validate<'a>(r: &IndependentPlanarChebyshevRecord<'a>) -> Result<(f64,f64),OverlappingRecordError> {
    if r.x_coefficients.is_empty() || r.y_coefficients.is_empty() {
        return Err(OverlappingRecordError::InvalidPolynomialRecord);
    }
    if r.x_coefficients.len() != r.y_coefficients.len() {
        return Err(OverlappingRecordError::DifferentAxisDegrees);
    }
    if r.x_coefficients.len() > 4096 {
        return Err(OverlappingRecordError::UnsupportedDegree);
    }
    if r.x_coefficients.iter().chain(r.y_coefficients.iter()).any(|c| !c.is_finite()) {
        return Err(OverlappingRecordError::NonFiniteCoefficient);
    }
    let center=r.midpoint_tdb_seconds;
    let radius=r.radius_tdb_seconds;
    if !center.is_finite() || !radius.is_finite() || radius<=0.0 {
        return Err(OverlappingRecordError::InvalidRecordEpoch);
    }
    let start=center-radius;
    let end=center+radius;
    if !start.is_finite() || !end.is_finite() || start>=center || end<=center {
        return Err(OverlappingRecordError::InvalidRecordEpoch);
    }
    Ok((start,end))
}

/// Bounds for one Cartesian component across a pair of *independent* records.
///
/// |T_k|<=1 and |T'_k|<=k² on each record's respective normalized domain.
/// Thus target and observer position tails and derivative magnitude sums
/// remain bounded on the intersection, without synchronized normalized time.
fn component_bounds(
    target:&[f64],observer:&[f64],
    target_radius_seconds:f64,observer_radius_seconds:f64,guard:f64,
) -> Result<(f64,f64),OverlappingRecordError> {
    let reference=target[0]-observer[0];
    let constant_scale=target[0].abs()+observer[0].abs();
    if !reference.is_finite() || !constant_scale.is_finite() {
        return Err(OverlappingRecordError::BoundOverflow);
    }
    let mut tail=0.0_f64;
    let mut normalized_target_rate=0.0_f64;
    let mut normalized_observer_rate=0.0_f64;
    for (k,c) in target.iter().enumerate().skip(1) {
        let ck=c.abs();
        let weight=(k as f64)*(k as f64);
        tail+=ck;
        normalized_target_rate+=ck*weight;
        if !tail.is_finite() || !normalized_target_rate.is_finite() {
            return Err(OverlappingRecordError::BoundOverflow);
        }
    }
    for (k,c) in observer.iter().enumerate().skip(1) {
        let ck=c.abs();
        let weight=(k as f64)*(k as f64);
        tail+=ck;
        normalized_observer_rate+=ck*weight;
        if !tail.is_finite() || !normalized_observer_rate.is_finite() {
            return Err(OverlappingRecordError::BoundOverflow);
        }
    }
    let margin=16.0*f64::EPSILON*(constant_scale+reference.abs());
    let lower=(reference.abs()/guard - tail*guard - margin).max(0.0);
    let seconds_rate=(normalized_target_rate/target_radius_seconds
        + normalized_observer_rate/observer_radius_seconds)*guard;
    // Explicit 86400 seconds per uniform TDB day; no UTC leap-second model.
    let day_rate=seconds_rate*86_400.0*guard;
    if !margin.is_finite() || !lower.is_finite()
        || !seconds_rate.is_finite() || !day_rate.is_finite() {
        return Err(OverlappingRecordError::BoundOverflow);
    }
    Ok((lower,day_rate))
}

/// Absolute angular rate bound across ANY explicitly requested strict
/// subinterval contained in the overlap of two independent Chebyshev records.
///
/// Unlike `bound_aligned_relative_chebyshev_longitude_rate`, the records may
/// have distinct center times, radii and degrees. It is essential that both
/// polynomials use one common physical origin and fixed coordinate system.
/// This method makes NO continuity assertion across a change of record:
/// a multi-record result requires independent boundary/coverage evidence.
///
/// A nonzero relative distance is proven conservatively using the separate
/// c0 terms and absolute tail bounds. If that cannot be proven, fail closed.
/// Rounding guards are heuristic, not a formal directed-rounding proof.
pub fn bound_overlapping_relative_chebyshev_longitude_rate(
    target: IndependentPlanarChebyshevRecord<'_>,
    observer: IndependentPlanarChebyshevRecord<'_>,
    start_tdb_seconds:f64,
    end_tdb_seconds:f64,
) -> Result<OverlappingRelativeLongitudeRateBound,OverlappingRecordError> {
    let (ts,te)=validate(&target)?;
    let (os,oe)=validate(&observer)?;
    if target.target_naif_id==observer.target_naif_id {
        return Err(OverlappingRecordError::DuplicateBody);
    }
    if target.common_origin_naif_id!=observer.common_origin_naif_id ||
        target.frame!=observer.frame || target.length_unit!=observer.length_unit {
        return Err(OverlappingRecordError::IncompatibleOrigin);
    }
    if !start_tdb_seconds.is_finite() || !end_tdb_seconds.is_finite() ||
        start_tdb_seconds>=end_tdb_seconds {
        return Err(OverlappingRecordError::InvalidRequestedInterval);
    }
    if start_tdb_seconds<ts.max(os) || end_tdb_seconds>te.min(oe) {
        return Err(OverlappingRecordError::OutsideCommonInterval);
    }
    let degree=target.x_coefficients.len().max(observer.x_coefficients.len()) as f64;
    let guard=1.0+64.0*f64::EPSILON*(degree+4.0);
    if !guard.is_finite() {
        return Err(OverlappingRecordError::BoundOverflow);
    }
    let (x_min,x_rate)=component_bounds(
        target.x_coefficients, observer.x_coefficients,
        target.radius_tdb_seconds, observer.radius_tdb_seconds,guard,
    )?;
    let (y_min,y_rate)=component_bounds(
        target.y_coefficients, observer.y_coefficients,
        target.radius_tdb_seconds, observer.radius_tdb_seconds,guard,
    )?;
    let min_distance=x_min.hypot(y_min)/guard;
    if !min_distance.is_finite() {
        return Err(OverlappingRecordError::BoundOverflow);
    }
    if min_distance<=0.0 {
        return Err(OverlappingRecordError::CannotExcludeRelativeOrigin);
    }
    let planar_speed=x_rate.hypot(y_rate)*guard;
    let angular_rate=(planar_speed/min_distance)*(180.0/PI)*guard;
    if !planar_speed.is_finite() || !angular_rate.is_finite() {
        return Err(OverlappingRecordError::BoundOverflow);
    }
    Ok(OverlappingRelativeLongitudeRateBound {
        max_abs_rate_deg_per_tdb_day:angular_rate,
        min_relative_planar_distance_km:min_distance,
        max_relative_planar_speed_km_per_tdb_day:planar_speed,
        start_tdb_seconds,end_tdb_seconds,
        target_naif_id:target.target_naif_id,
        observer_naif_id:observer.target_naif_id,
        common_origin_naif_id:target.common_origin_naif_id,
        coverage:OverlappingRecordCoverage::RequestedOverlapOfTwoPolynomials,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chebyshev::evaluate_chebyshev;
    use crate::longitude_enclosures::{
        enclose_longitude_from_claim,LongitudeRateBoundClaim,UniformTimeScale,
    };

    fn record<'a>(id:i32,mid:f64,rad:f64,x:&'a [f64],y:&'a [f64])
        -> IndependentPlanarChebyshevRecord<'a> {
        IndependentPlanarChebyshevRecord{
            target_naif_id:id,common_origin_naif_id:0,
            frame:FixedCartesianFrame::IcrfJ2000,
            length_unit:CartesianLengthUnit::Kilometer,
            midpoint_tdb_seconds:mid,radius_tdb_seconds:rad,
            x_coefficients:x,y_coefficients:y,
        }
    }
    fn actual_rate(
        target:IndependentPlanarChebyshevRecord<'_>,
        observer:IndependentPlanarChebyshevRecord<'_>,t:f64,
    ) -> f64 {
        fn axis(c:&[f64],mid:f64,rad:f64,t:f64)->(f64,f64) {
            let s=evaluate_chebyshev(c,(t-mid)/rad).unwrap();
            (s.value,s.derivative_dx/rad)
        }
        let (tx,tvx)=axis(target.x_coefficients,target.midpoint_tdb_seconds,target.radius_tdb_seconds,t);
        let (ty,tvy)=axis(target.y_coefficients,target.midpoint_tdb_seconds,target.radius_tdb_seconds,t);
        let (ox,ovx)=axis(observer.x_coefficients,observer.midpoint_tdb_seconds,observer.radius_tdb_seconds,t);
        let (oy,ovy)=axis(observer.y_coefficients,observer.midpoint_tdb_seconds,observer.radius_tdb_seconds,t);
        let x=tx-ox;
        let y=ty-oy;
        let vx=tvx-ovx;
        let vy=tvy-ovy;
        ((x*vy-y*vx)/(x*x+y*y)*86_400.0).to_degrees()
    }

    #[test]
    fn different_centers_and_radii_are_allowed_inside_intersection() {
        let target=record(499,0.0,86_400.0,&[100.0,0.0],&[0.0,1.0]);
        let earth=record(399,43_200.0,86_400.0,&[90.0,0.0],&[0.0,0.0]);
        let result=bound_overlapping_relative_chebyshev_longitude_rate(target,earth,0.0,86_400.0).unwrap();
        assert_eq!(result.start_tdb_seconds,0.0);
        assert_eq!(result.end_tdb_seconds,86_400.0);
        assert!(result.min_relative_planar_distance_km>0.0);
        for step in 0..=100 {
            let t=86_400.0*f64::from(step)/100.0;
            assert!(actual_rate(target,earth,t).abs()<=result.max_abs_rate_deg_per_tdb_day);
        }
    }

    #[test]
    fn observer_motion_is_included_at_its_own_normalized_time() {
        let target=record(499,0.0,86_400.0,&[100.0,0.0],&[0.0,1.0]);
        let earth=record(399,21_600.0,43_200.0,&[90.0,0.0],&[0.0,0.5]);
        let result=bound_overlapping_relative_chebyshev_longitude_rate(target,earth,0.0,43_200.0).unwrap();
        for step in 0..=200 {
            let t=43_200.0*f64::from(step)/200.0;
            assert!(actual_rate(target,earth,t).abs()<=result.max_abs_rate_deg_per_tdb_day);
        }
    }

    #[test]
    fn station_and_retrograde_are_covered_without_sampling_the_rate_to_derive_bound() {
        let target=record(499,0.0,86_400.0,&[100.0,0.0,0.0],&[0.0,0.0,0.5]);
        let earth=record(399,21_600.0,43_200.0,&[90.0],&[0.0]);
        let result=bound_overlapping_relative_chebyshev_longitude_rate(target,earth,-10_800.0,54_000.0).unwrap();
        assert!(actual_rate(target,earth,-10_800.0)<0.0);
        assert_eq!(actual_rate(target,earth,0.0),0.0);
        assert!(actual_rate(target,earth,54_000.0)>0.0);
        for step in 0..=200 {
            let t=-10_800.0+64_800.0*f64::from(step)/200.0;
            assert!(actual_rate(target,earth,t).abs()<=result.max_abs_rate_deg_per_tdb_day);
        }
    }

    #[test]
    fn different_polynomial_degrees_do_not_require_reparameterization() {
        let target=record(499,0.0,86_400.0,&[100.0,0.0,0.0],&[0.0,0.0,0.5]);
        let earth=record(399,43_200.0,86_400.0,&[90.0],&[0.0]);
        let result=bound_overlapping_relative_chebyshev_longitude_rate(target,earth,0.0,86_400.0).unwrap();
        assert!(result.max_abs_rate_deg_per_tdb_day>0.0);
    }

    #[test]
    fn disjoint_domains_and_out_of_overlap_requests_fail_closed() {
        let target=record(499,0.0,86_400.0,&[100.0],&[0.0]);
        let far=record(399,259_200.0,86_400.0,&[90.0],&[0.0]);
        assert_eq!(
            bound_overlapping_relative_chebyshev_longitude_rate(target,far,0.0,86_400.0),
            Err(OverlappingRecordError::OutsideCommonInterval)
        );
        let partial=record(399,43_200.0,86_400.0,&[90.0],&[0.0]);
        assert_eq!(
            bound_overlapping_relative_chebyshev_longitude_rate(target,partial,-86_400.0,0.0),
            Err(OverlappingRecordError::OutsideCommonInterval)
        );
    }

    #[test]
    fn interval_endpoints_must_be_ordered_and_finite() {
        let target=record(499,0.0,86_400.0,&[100.0],&[0.0]);
        let earth=record(399,0.0,86_400.0,&[90.0],&[0.0]);
        for (start,end) in [(1.0,1.0),(2.0,1.0),(f64::NAN,1.0),(0.0,f64::INFINITY)] {
            assert_eq!(
                bound_overlapping_relative_chebyshev_longitude_rate(target,earth,start,end),
                Err(OverlappingRecordError::InvalidRequestedInterval)
            );
        }
    }

    #[test]
    fn rejects_incompatible_origin_and_identical_body() {
        let target=record(499,0.0,86_400.0,&[100.0],&[0.0]);
        let mut earth=record(399,0.0,86_400.0,&[90.0],&[0.0]);
        earth.common_origin_naif_id=10;
        assert_eq!(
            bound_overlapping_relative_chebyshev_longitude_rate(target,earth,0.0,1.0),
            Err(OverlappingRecordError::IncompatibleOrigin)
        );
        earth.common_origin_naif_id=0;
        earth.target_naif_id=499;
        assert_eq!(
            bound_overlapping_relative_chebyshev_longitude_rate(target,earth,0.0,1.0),
            Err(OverlappingRecordError::DuplicateBody)
        );
    }

    #[test]
    fn malformed_records_and_nonfinite_coefficients_are_rejected() {
        let target=record(499,0.0,86_400.0,&[100.0],&[0.0]);
        let bad=record(399,0.0,86_400.0,&[],&[]);
        assert_eq!(
            bound_overlapping_relative_chebyshev_longitude_rate(target,bad,0.0,1.0),
            Err(OverlappingRecordError::InvalidPolynomialRecord)
        );
        let bad=record(399,0.0,86_400.0,&[90.0],&[0.0,1.0]);
        assert_eq!(
            bound_overlapping_relative_chebyshev_longitude_rate(target,bad,0.0,1.0),
            Err(OverlappingRecordError::DifferentAxisDegrees)
        );
        let bad=record(399,0.0,86_400.0,&[90.0],&[f64::NAN]);
        assert_eq!(
            bound_overlapping_relative_chebyshev_longitude_rate(target,bad,0.0,1.0),
            Err(OverlappingRecordError::NonFiniteCoefficient)
        );
        let bad=record(399,f64::MAX,86_400.0,&[90.0],&[0.0]);
        assert_eq!(
            bound_overlapping_relative_chebyshev_longitude_rate(target,bad,0.0,1.0),
            Err(OverlappingRecordError::InvalidRecordEpoch)
        );
        let bad=record(399,0.0,0.0,&[90.0],&[0.0]);
        assert_eq!(
            bound_overlapping_relative_chebyshev_longitude_rate(target,bad,0.0,1.0),
            Err(OverlappingRecordError::InvalidRecordEpoch)
        );
    }

    #[test]
    fn planar_singularity_and_coefficient_overflow_do_not_produce_fake_rates() {
        let target=record(499,0.0,86_400.0,&[100.0,1.0],&[1.0,1.0]);
        let earth=record(399,0.0,86_400.0,&[100.0],&[1.0]);
        assert_eq!(
            bound_overlapping_relative_chebyshev_longitude_rate(target,earth,0.0,1.0),
            Err(OverlappingRecordError::CannotExcludeRelativeOrigin)
        );
        let target=record(499,0.0,86_400.0,&[1.0e308],&[0.0]);
        let earth=record(399,0.0,86_400.0,&[1.0e308],&[0.0]);
        assert_eq!(
            bound_overlapping_relative_chebyshev_longitude_rate(target,earth,0.0,1.0),
            Err(OverlappingRecordError::BoundOverflow)
        );
    }

    #[test]
    fn overlap_rate_can_feed_prior_conditional_circular_enclosure() {
        let target=record(499,0.0,86_400.0,&[100.0,0.0],&[0.0,1.0]);
        let earth=record(399,43_200.0,86_400.0,&[90.0,0.0],&[0.0,0.0]);
        let result=bound_overlapping_relative_chebyshev_longitude_rate(target,earth,0.0,86_400.0).unwrap();
        let arc=enclose_longitude_from_claim(LongitudeRateBoundClaim{
            reference_longitude_deg:0.0,
            start_offset_days:0.0,end_offset_days:1.0,
            max_abs_rate_deg_per_day:result.max_abs_rate_deg_per_tdb_day,
            reference_error_deg:0.0,time_scale:UniformTimeScale::Tdb,
            asserts_full_interval_rate_bound:true,
        }).unwrap();
        for t in [0.0,21_600.0,43_200.0,64_800.0,86_400.0] {
            let tau=t/86_400.0;
            let longitude=tau.atan2(10.0).to_degrees().rem_euclid(360.0);
            assert!((longitude-arc.start_deg).rem_euclid(360.0)<=arc.span_deg+1.0e-9);
        }
    }

    #[test]
    fn result_remains_explicitly_mathematical_not_astronomical() {
        let target=record(499,0.0,86_400.0,&[100.0],&[0.0]);
        let earth=record(399,43_200.0,86_400.0,&[90.0],&[0.0]);
        let b=bound_overlapping_relative_chebyshev_longitude_rate(target,earth,0.0,1.0).unwrap();
        assert_eq!(b.coverage,OverlappingRecordCoverage::RequestedOverlapOfTwoPolynomials);
        assert_eq!(b.max_abs_rate_deg_per_tdb_day,0.0);
        assert_eq!(b.target_naif_id,499);
        assert_eq!(b.observer_naif_id,399);
    }
}
