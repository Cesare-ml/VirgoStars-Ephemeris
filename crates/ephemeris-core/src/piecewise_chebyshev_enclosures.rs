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

//! Piecewise target–observer Chebyshev longitude enclosure with explicit
//! boundary-jump budgets. Each segment has an independently checked mathematical
//! rate bound from overlapping polynomial records. Adjacent intervals must
//! meet exactly in TDB seconds; no resampling or extrapolation is permitted.
//!
//! IMPORTANT: observed floating-point boundary mismatch is only a diagnostic.
//! The supplied maximum jump is an UNVERIFIED caller assertion, not proof of
//! continuity or a NASA/JPL numerical error guarantee.
//!
//! Original VirgoStars work added 2026-11-12, CPAL-1.0.

use core::fmt;

use crate::chebyshev::evaluate_chebyshev;
use crate::overlapping_chebyshev_longitude_bounds::{
    bound_overlapping_relative_chebyshev_longitude_rate,
    IndependentPlanarChebyshevRecord, OverlappingRecordError,
};

/// One closed TDB subinterval supplied in strictly chronological order.
/// The first segment MUST set `max_boundary_jump_deg` to None. Later
/// segments MUST give an explicitly asserted upper angular jump (0..=180)
/// at the boundary between the previous and current mathematical records.
#[derive(Clone, Copy, Debug)]
pub struct PiecewiseRelativeChebyshevSegment<'a> {
    pub target: IndependentPlanarChebyshevRecord<'a>,
    pub observer: IndependentPlanarChebyshevRecord<'a>,
    pub start_tdb_seconds: f64,
    pub end_tdb_seconds: f64,
    pub max_boundary_jump_deg: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PiecewiseEnclosureCoverage {
    /// All requested subintervals were mathematically covered, but the
    /// transition jump bounds are caller assertions and never independently
    /// certified by the present module.
    CallerAssertedBoundaryJumpsUnverified,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PiecewiseSegmentSummary {
    pub start_tdb_seconds: f64,
    pub end_tdb_seconds: f64,
    pub max_abs_rate_deg_per_tdb_day: f64,
    pub maximum_angular_travel_deg: f64,
    pub max_boundary_jump_deg: Option<f64>,
    /// Computed using finite-precision endpoint evaluation, for diagnostics
    /// only. This is NOT the certified jump upper bound.
    pub observed_boundary_jump_deg: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PiecewiseLongitudeEnclosure {
    /// Forward circular arc start in [0,360).
    pub start_deg: f64,
    /// Forward arc span in [0,360]. A full circle begins at zero.
    pub span_deg: f64,
    /// Reference at the start of the first mathematical segment.
    pub reference_longitude_deg: f64,
    /// Upper bound for travel from the reference including all asserted jumps.
    pub total_declared_variation_deg: f64,
    pub start_tdb_seconds: f64,
    pub end_tdb_seconds: f64,
    pub target_naif_id: i32,
    pub observer_naif_id: i32,
    pub common_origin_naif_id: i32,
    pub coverage: PiecewiseEnclosureCoverage,
    pub segments: Vec<PiecewiseSegmentSummary>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PiecewiseEnclosureError {
    EmptySegments,
    TooManySegments,
    InvalidReferenceError,
    InvalidPiece(OverlappingRecordError),
    DiscontinuousTimeCoverage,
    InconsistentBodyOrReferenceFrame,
    FirstPieceMustNotSpecifyJump,
    MissingBoundaryJumpLimit,
    InvalidBoundaryJumpLimit,
    ObservedBoundaryExceedsClaimedLimit,
    UndefinedBoundaryLongitude,
    NonFiniteVariation,
}
impl fmt::Display for PiecewiseEnclosureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySegments => write!(f, "at least one interval is required"),
            Self::TooManySegments => write!(f, "at most 256 segments are allowed"),
            Self::InvalidReferenceError => write!(f, "reference error must be finite and nonnegative"),
            Self::InvalidPiece(e) => write!(f, "invalid overlapping pair: {e}"),
            Self::DiscontinuousTimeCoverage => write!(f, "TDB pieces have a gap or overlap"),
            Self::InconsistentBodyOrReferenceFrame => write!(f, "body/origin/frame/units changed"),
            Self::FirstPieceMustNotSpecifyJump => write!(f, "first piece has no preceding boundary"),
            Self::MissingBoundaryJumpLimit => write!(f, "each later piece requires a declared jump limit"),
            Self::InvalidBoundaryJumpLimit => write!(f, "jump limit must be finite and in [0,180] degrees"),
            Self::ObservedBoundaryExceedsClaimedLimit => write!(f, "observed jump contradicts caller's upper bound"),
            Self::UndefinedBoundaryLongitude => write!(f, "finite nonzero relative XY direction required"),
            Self::NonFiniteVariation => write!(f, "piecewise variation cannot be represented"),
        }
    }
}
impl std::error::Error for PiecewiseEnclosureError {}

fn same_identity(
    first: &PiecewiseRelativeChebyshevSegment<'_>,
    current: &PiecewiseRelativeChebyshevSegment<'_>,
) -> bool {
    first.target.target_naif_id == current.target.target_naif_id
        && first.observer.target_naif_id == current.observer.target_naif_id
        && first.target.common_origin_naif_id == current.target.common_origin_naif_id
        && first.observer.common_origin_naif_id == current.observer.common_origin_naif_id
        && first.target.frame == current.target.frame
        && first.observer.frame == current.observer.frame
        && first.target.length_unit == current.target.length_unit
        && first.observer.length_unit == current.observer.length_unit
}

/// A mathematical endpoint calculation. No guarantee is made that the
/// floating-point evaluation fully covers uncertainty in external kernel
/// coefficients, coordinates, or corrections.
fn relative_longitude(
    piece: &PiecewiseRelativeChebyshevSegment<'_>,
    tdb_seconds: f64,
) -> Result<f64, PiecewiseEnclosureError> {
    let axis = |coefficients: &[f64], center: f64, radius: f64| {
        evaluate_chebyshev(coefficients, (tdb_seconds - center) / radius)
            .map(|sample| sample.value)
            .map_err(|_| PiecewiseEnclosureError::UndefinedBoundaryLongitude)
    };
    let tx = axis(piece.target.x_coefficients, piece.target.midpoint_tdb_seconds, piece.target.radius_tdb_seconds)?;
    let ty = axis(piece.target.y_coefficients, piece.target.midpoint_tdb_seconds, piece.target.radius_tdb_seconds)?;
    let ox = axis(piece.observer.x_coefficients, piece.observer.midpoint_tdb_seconds, piece.observer.radius_tdb_seconds)?;
    let oy = axis(piece.observer.y_coefficients, piece.observer.midpoint_tdb_seconds, piece.observer.radius_tdb_seconds)?;
    let x = tx - ox;
    let y = ty - oy;
    if !x.is_finite() || !y.is_finite() || !x.hypot(y).is_finite() || x.hypot(y) == 0.0 {
        return Err(PiecewiseEnclosureError::UndefinedBoundaryLongitude);
    }
    let degrees = y.atan2(x).to_degrees().rem_euclid(360.0);
    if !degrees.is_finite() {
        return Err(PiecewiseEnclosureError::UndefinedBoundaryLongitude);
    }
    Ok(degrees)
}

fn circular_separation_deg(a: f64, b: f64) -> f64 {
    let diff = (b - a).rem_euclid(360.0);
    diff.min(360.0 - diff)
}

/// Produce a circular arc containing the entire **mathematical piecewise path**
/// conditional on every segment's conservative coefficient bound and every
/// caller-asserted discontinuity limit at segment transitions.
///
/// Segment rates are integrated over their exact duration in uniform TDB days.
/// Each transition adds `max_boundary_jump_deg` to the total variation, so a
/// discontinuity is NOT disguised as a finite derivative or silently ignored.
/// The measured jump is checked only to catch visibly contradicted claims.
/// The supplied limit is not mathematically proven here.
///
/// The returned enclosure is always marked as UNVERIFIED. Never pass the
/// accumulated rate/jump cost off as a continuous global speed bound: actual
/// derivatives may be undefined at segment boundaries. This method does not
/// convert ICRF to apparent ecliptic-of-date or validate DE441 kernels.
pub fn enclose_piecewise_relative_chebyshev_longitude(
    pieces: &[PiecewiseRelativeChebyshevSegment<'_>],
    reference_error_deg: f64,
) -> Result<PiecewiseLongitudeEnclosure, PiecewiseEnclosureError> {
    if pieces.is_empty() {
        return Err(PiecewiseEnclosureError::EmptySegments);
    }
    if pieces.len() > 256 {
        return Err(PiecewiseEnclosureError::TooManySegments);
    }
    if !reference_error_deg.is_finite() || reference_error_deg < 0.0 {
        return Err(PiecewiseEnclosureError::InvalidReferenceError);
    }
    let first = &pieces[0];
    if first.max_boundary_jump_deg.is_some() {
        return Err(PiecewiseEnclosureError::FirstPieceMustNotSpecifyJump);
    }
    let mut summaries = Vec::with_capacity(pieces.len());
    let mut total_variation = 0.0_f64;

    for (index, piece) in pieces.iter().enumerate() {
        if !same_identity(first, piece) {
            return Err(PiecewiseEnclosureError::InconsistentBodyOrReferenceFrame);
        }
        let bound = bound_overlapping_relative_chebyshev_longitude_rate(
            piece.target, piece.observer,
            piece.start_tdb_seconds, piece.end_tdb_seconds,
        ).map_err(PiecewiseEnclosureError::InvalidPiece)?;

        let observed_jump = if index == 0 {
            None
        } else {
            let preceding = &pieces[index - 1];
            if piece.start_tdb_seconds != preceding.end_tdb_seconds {
                return Err(PiecewiseEnclosureError::DiscontinuousTimeCoverage);
            }
            let declared = piece
                .max_boundary_jump_deg
                .ok_or(PiecewiseEnclosureError::MissingBoundaryJumpLimit)?;
            if !declared.is_finite() || !(0.0..=180.0).contains(&declared) {
                return Err(PiecewiseEnclosureError::InvalidBoundaryJumpLimit);
            }
            let before = relative_longitude(preceding, piece.start_tdb_seconds)?;
            let after = relative_longitude(piece, piece.start_tdb_seconds)?;
            let observed = circular_separation_deg(before, after);
            if observed > declared {
                return Err(PiecewiseEnclosureError::ObservedBoundaryExceedsClaimedLimit);
            }
            total_variation += declared;
            Some(observed)
        };
        let duration_days = (piece.end_tdb_seconds - piece.start_tdb_seconds) / 86_400.0;
        if !duration_days.is_finite() || duration_days <= 0.0 {
            return Err(PiecewiseEnclosureError::NonFiniteVariation);
        }
        let travel = bound.max_abs_rate_deg_per_tdb_day * duration_days;
        total_variation += travel;
        // A nonfinite total is non-informative; for a finite physical arc
        // we conservatively saturate to 360 degrees after validating inputs.
        if total_variation.is_nan() || travel.is_nan() {
            return Err(PiecewiseEnclosureError::NonFiniteVariation);
        }
        summaries.push(PiecewiseSegmentSummary {
            start_tdb_seconds: piece.start_tdb_seconds,
            end_tdb_seconds: piece.end_tdb_seconds,
            max_abs_rate_deg_per_tdb_day: bound.max_abs_rate_deg_per_tdb_day,
            maximum_angular_travel_deg: travel,
            max_boundary_jump_deg: piece.max_boundary_jump_deg,
            observed_boundary_jump_deg: observed_jump,
        });
    }
    let reference = relative_longitude(first, first.start_tdb_seconds)?;
    let total = total_variation + reference_error_deg;
    // A small pad guards subsequent floating-point interval construction.
    // It is not a validated error bound for upstream astronomical polynomials.
    let padding = 64.0 * f64::EPSILON * (360.0 + total);
    let radius = total + padding;
    let (start_deg, span_deg) = if !radius.is_finite() || radius >= 180.0 {
        (0.0, 360.0)
    } else {
        ((reference - radius).rem_euclid(360.0), (2.0 * radius).min(360.0))
    };
    Ok(PiecewiseLongitudeEnclosure {
        start_deg, span_deg, reference_longitude_deg: reference,
        total_declared_variation_deg: total_variation,
        start_tdb_seconds: first.start_tdb_seconds,
        end_tdb_seconds: pieces[pieces.len() - 1].end_tdb_seconds,
        target_naif_id: first.target.target_naif_id,
        observer_naif_id: first.observer.target_naif_id,
        common_origin_naif_id: first.target.common_origin_naif_id,
        coverage: PiecewiseEnclosureCoverage::CallerAssertedBoundaryJumpsUnverified,
        segments: summaries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlapping_chebyshev_longitude_bounds::IndependentPlanarChebyshevRecord;
    use crate::relative_chebyshev_longitude_bounds::{FixedCartesianFrame,CartesianLengthUnit};

    fn record<'a>(id: i32, midpoint: f64, radius: f64, x: &'a [f64], y: &'a [f64])
        -> IndependentPlanarChebyshevRecord<'a> {
        IndependentPlanarChebyshevRecord {
            target_naif_id: id,
            common_origin_naif_id: 0,
            frame: FixedCartesianFrame::IcrfJ2000,
            length_unit: CartesianLengthUnit::Kilometer,
            midpoint_tdb_seconds: midpoint,
            radius_tdb_seconds: radius,
            x_coefficients: x,
            y_coefficients: y,
        }
    }
    fn piece<'a>(
        target: IndependentPlanarChebyshevRecord<'a>,
        observer: IndependentPlanarChebyshevRecord<'a>,
        start: f64, end: f64, jump: Option<f64>,
    ) -> PiecewiseRelativeChebyshevSegment<'a> {
        PiecewiseRelativeChebyshevSegment {
            target,observer,start_tdb_seconds:start,
            end_tdb_seconds:end,max_boundary_jump_deg:jump,
        }
    }
    fn contains(enclosure: &PiecewiseLongitudeEnclosure, longitude_deg: f64) -> bool {
        (longitude_deg - enclosure.start_deg).rem_euclid(360.0) <= enclosure.span_deg
    }

    #[test]
    fn two_contiguous_mathematical_records_produce_one_conditional_enclosure() {
        let target_a = record(499,43_200.0,43_200.0,&[100.0],&[1.0]);
        let target_b = record(499,129_600.0,43_200.0,&[100.0],&[1.0]);
        let earth_a = record(399,43_200.0,43_200.0,&[90.0],&[0.0]);
        let earth_b = record(399,129_600.0,43_200.0,&[90.0],&[0.0]);
        let parts = [
            piece(target_a,earth_a,0.0,86_400.0,None),
            piece(target_b,earth_b,86_400.0,172_800.0,Some(0.0)),
        ];
        let out = enclose_piecewise_relative_chebyshev_longitude(&parts,0.0).unwrap();
        assert_eq!(out.segments.len(),2);
        assert_eq!(out.start_tdb_seconds,0.0);
        assert_eq!(out.end_tdb_seconds,172_800.0);
        assert_eq!(out.coverage,PiecewiseEnclosureCoverage::CallerAssertedBoundaryJumpsUnverified);
        assert_eq!(out.segments[1].observed_boundary_jump_deg,Some(0.0));
        assert!(contains(&out,1.0_f64.atan2(10.0).to_degrees()));
    }

    #[test]
    fn explicit_nonzero_jump_limit_is_added_to_path_variation() {
        let target_a=record(499,43_200.0,43_200.0,&[100.0],&[0.0]);
        let target_b=record(499,129_600.0,43_200.0,&[100.0],&[1.0]);
        let earth_a=record(399,43_200.0,43_200.0,&[90.0],&[0.0]);
        let earth_b=record(399,129_600.0,43_200.0,&[90.0],&[0.0]);
        let parts=[
            piece(target_a,earth_a,0.0,86_400.0,None),
            piece(target_b,earth_b,86_400.0,172_800.0,Some(6.0)),
        ];
        let out=enclose_piecewise_relative_chebyshev_longitude(&parts,0.0).unwrap();
        assert_eq!(out.total_declared_variation_deg,6.0);
        assert!(out.segments[1].observed_boundary_jump_deg.unwrap()>5.0);
        assert!(contains(&out,0.0));
        assert!(contains(&out,1.0_f64.atan2(10.0).to_degrees()));
        assert_eq!(out.coverage,PiecewiseEnclosureCoverage::CallerAssertedBoundaryJumpsUnverified);
    }

    #[test]
    fn understated_boundary_limit_cannot_be_hidden_by_a_finite_speed() {
        let a=piece(record(499,43_200.0,43_200.0,&[100.0],&[0.0]),
            record(399,43_200.0,43_200.0,&[90.0],&[0.0]),0.0,86_400.0,None);
        let b=piece(record(499,129_600.0,43_200.0,&[100.0],&[1.0]),
            record(399,129_600.0,43_200.0,&[90.0],&[0.0]),86_400.0,172_800.0,Some(1.0));
        assert_eq!(enclose_piecewise_relative_chebyshev_longitude(&[a,b],0.0),
            Err(PiecewiseEnclosureError::ObservedBoundaryExceedsClaimedLimit));
    }

    #[test]
    fn gaps_and_overlaps_are_both_rejected() {
        let a=piece(record(499,43_200.0,43_200.0,&[100.0],&[0.0]),
            record(399,43_200.0,43_200.0,&[90.0],&[0.0]),0.0,86_400.0,None);
        let b=piece(record(499,129_600.0,60_000.0,&[100.0],&[0.0]),
            record(399,129_600.0,60_000.0,&[90.0],&[0.0]),86_401.0,172_800.0,Some(0.0));
        assert_eq!(enclose_piecewise_relative_chebyshev_longitude(&[a,b],0.0),
            Err(PiecewiseEnclosureError::DiscontinuousTimeCoverage));
        let c=piece(b.target,b.observer,86_399.0,172_800.0,Some(0.0));
        assert_eq!(enclose_piecewise_relative_chebyshev_longitude(&[a,c],0.0),
            Err(PiecewiseEnclosureError::DiscontinuousTimeCoverage));
    }

    #[test]
    fn all_slices_are_checked_even_when_the_result_is_a_full_circle() {
        let a=piece(record(499,43_200.0,43_200.0,&[100.0,0.0],&[0.0,50.0]),
            record(399,43_200.0,43_200.0,&[90.0,0.0],&[0.0,0.0]),0.0,86_400.0,None);
        let b=piece(record(499,129_600.0,43_200.0,&[100.0],&[0.0]),
            record(399,129_600.0,43_200.0,&[90.0],&[0.0]),86_400.0,172_800.0,Some(180.0));
        let out=enclose_piecewise_relative_chebyshev_longitude(&[a,b],0.0).unwrap();
        assert_eq!(out.span_deg,360.0);
        assert_eq!(out.start_deg,0.0);
    }

    #[test]
    fn first_piece_or_later_piece_jump_contracts_are_explicit() {
        let a=piece(record(499,43_200.0,43_200.0,&[100.0],&[0.0]),
            record(399,43_200.0,43_200.0,&[90.0],&[0.0]),0.0,86_400.0,Some(0.0));
        assert_eq!(enclose_piecewise_relative_chebyshev_longitude(&[a],0.0),
            Err(PiecewiseEnclosureError::FirstPieceMustNotSpecifyJump));
        let first=piece(a.target,a.observer,0.0,86_400.0,None);
        let later=piece(record(499,129_600.0,43_200.0,&[100.0],&[0.0]),
            record(399,129_600.0,43_200.0,&[90.0],&[0.0]),86_400.0,172_800.0,None);
        assert_eq!(enclose_piecewise_relative_chebyshev_longitude(&[first,later],0.0),
            Err(PiecewiseEnclosureError::MissingBoundaryJumpLimit));
    }

    #[test]
    fn all_body_ids_and_reference_frame_must_remain_consistent() {
        let a=piece(record(499,43_200.0,43_200.0,&[100.0],&[0.0]),
            record(399,43_200.0,43_200.0,&[90.0],&[0.0]),0.0,86_400.0,None);
        let b=piece(record(499,129_600.0,43_200.0,&[100.0],&[0.0]),
            record(398,129_600.0,43_200.0,&[90.0],&[0.0]),86_400.0,172_800.0,Some(0.0));
        assert_eq!(enclose_piecewise_relative_chebyshev_longitude(&[a,b],0.0),
            Err(PiecewiseEnclosureError::InconsistentBodyOrReferenceFrame));
    }

    #[test]
    fn invalid_reference_error_and_empty_input_fail_closed() {
        assert_eq!(enclose_piecewise_relative_chebyshev_longitude(&[],0.0),
            Err(PiecewiseEnclosureError::EmptySegments));
        let a=piece(record(499,43_200.0,43_200.0,&[100.0],&[0.0]),
            record(399,43_200.0,43_200.0,&[90.0],&[0.0]),0.0,86_400.0,None);
        for invalid in [f64::NAN, f64::INFINITY, -1.0] {
            assert_eq!(enclose_piecewise_relative_chebyshev_longitude(&[a],invalid),
                Err(PiecewiseEnclosureError::InvalidReferenceError));
        }
    }

    #[test]
    fn coefficient_singularity_cannot_be_hidden_by_other_valid_segments() {
        let a=piece(record(499,43_200.0,43_200.0,&[100.0],&[0.0]),
            record(399,43_200.0,43_200.0,&[90.0],&[0.0]),0.0,86_400.0,None);
        let b=piece(record(499,129_600.0,43_200.0,&[90.0],&[0.0]),
            record(399,129_600.0,43_200.0,&[90.0],&[0.0]),86_400.0,172_800.0,Some(0.0));
        assert_eq!(enclose_piecewise_relative_chebyshev_longitude(&[a,b],0.0),
            Err(PiecewiseEnclosureError::InvalidPiece(OverlappingRecordError::CannotExcludeRelativeOrigin)));
    }

    #[test]
    fn exact_single_piece_returns_claimed_and_padded_point_arc() {
        let a=piece(record(499,43_200.0,43_200.0,&[100.0],&[0.0]),
            record(399,43_200.0,43_200.0,&[90.0],&[0.0]),0.0,86_400.0,None);
        let out=enclose_piecewise_relative_chebyshev_longitude(&[a],0.0).unwrap();
        assert!(out.span_deg > 0.0); // finite numerical pad, no physical-certainty claim.
        assert!(out.span_deg < 1.0e-8);
        assert_eq!(out.total_declared_variation_deg,0.0);
        assert_eq!(out.target_naif_id,499);
    }
}
