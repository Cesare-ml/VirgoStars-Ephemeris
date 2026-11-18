// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Added 2026-11-18: opt-in evaluation of caller-supplied published ΔT splines.
//! Normalized piecewise cubic TT−UT1 model; no coefficients are bundled.
//! This evaluates a caller-supplied table but does not authenticate its source,
//! infer an uncertainty, extrapolate, interpret civil time, or activate a policy.
//! The 2021 Morrison et al. S5 table is a candidate with a documented published
//! range; source terms and scientific limitations remain separate from the code.

use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeltaTSplineSegment {
    pub start_year: f64,
    pub end_year: f64,
    /// Seconds, for a0 + a1*t + a2*t² + a3*t³, t normalized to the segment.
    pub coefficients: [f64; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeltaTSplineError {
    InvalidSegment,
    GapOrOverlap,
    Discontinuous,
    OutOfRange,
    NonFinite,
}
impl fmt::Display for DeltaTSplineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Delta T spline: {self:?}")
    }
}
impl Error for DeltaTSplineError {}

/// Published coefficients are rounded; accept at most 5 ms at shared knots.
/// This numerical integrity gate is not an uncertainty or physical tolerance.
pub const MAX_KNOT_JUMP_SECONDS: f64 = 0.005;

#[derive(Clone, Debug)]
pub struct DeltaTSpline {
    segments: Vec<DeltaTSplineSegment>,
}
impl DeltaTSpline {
    pub fn new(segments: Vec<DeltaTSplineSegment>) -> Result<Self, DeltaTSplineError> {
        if segments.is_empty() {
            return Err(DeltaTSplineError::InvalidSegment);
        }
        for segment in &segments {
            if !segment.start_year.is_finite()
                || !segment.end_year.is_finite()
                || !(segment.end_year - segment.start_year).is_finite()
                || segment.start_year >= segment.end_year
                || !segment.coefficients.iter().all(|x| x.is_finite())
            {
                return Err(DeltaTSplineError::InvalidSegment);
            }
        }
        for pair in segments.windows(2) {
            if pair[0].end_year != pair[1].start_year {
                return Err(DeltaTSplineError::GapOrOverlap);
            }
            let left = pair[0].coefficients.iter().sum::<f64>();
            if !left.is_finite() || (left - pair[1].coefficients[0]).abs() > MAX_KNOT_JUMP_SECONDS {
                return Err(DeltaTSplineError::Discontinuous);
            }
        }
        Ok(Self { segments })
    }
    /// Evaluate seconds at caller-provided decimal UT1 year; left-inclusive
    /// segments, final endpoint included. Historical calendars are not inferred.
    pub fn seconds_at(&self, year: f64) -> Result<f64, DeltaTSplineError> {
        if !year.is_finite() {
            return Err(DeltaTSplineError::NonFinite);
        }
        if year < self.segments[0].start_year || year > self.segments.last().unwrap().end_year {
            return Err(DeltaTSplineError::OutOfRange);
        }
        let index = self
            .segments
            .partition_point(|s| s.end_year <= year)
            .min(self.segments.len() - 1);
        let segment = &self.segments[index];
        let t = (year - segment.start_year) / (segment.end_year - segment.start_year);
        let [a, b, c, d] = segment.coefficients;
        let result = ((d * t + c) * t + b) * t + a;
        if !result.is_finite() {
            return Err(DeltaTSplineError::InvalidSegment);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn segment(start: f64, end: f64, c: [f64; 4]) -> DeltaTSplineSegment {
        DeltaTSplineSegment {
            start_year: start,
            end_year: end,
            coefficients: c,
        }
    }
    #[test]
    fn normalized_polynomial_and_knots_are_explicit() {
        let spline = DeltaTSpline::new(vec![
            segment(0.0, 2.0, [1.0, 2.0, 0.0, 0.0]),
            segment(2.0, 4.0, [3.001, 1.0, 0.0, 0.0]),
        ])
        .unwrap();
        assert_eq!(spline.seconds_at(1.0).unwrap(), 2.0);
        assert_eq!(spline.seconds_at(2.0).unwrap(), 3.001);
        assert_eq!(spline.seconds_at(4.0).unwrap(), 4.0009999999999994);
        assert_eq!(spline.seconds_at(-0.1), Err(DeltaTSplineError::OutOfRange));
        assert_eq!(
            spline.seconds_at(f64::NAN),
            Err(DeltaTSplineError::NonFinite)
        );
    }
    #[test]
    fn rejects_gaps_jumps_and_bad_coefficients() {
        let a = segment(0.0, 1.0, [1.0, 0.0, 0.0, 0.0]);
        assert!(matches!(
            DeltaTSpline::new(vec![a, segment(2.0, 3.0, [1.0, 0.0, 0.0, 0.0])]),
            Err(DeltaTSplineError::GapOrOverlap)
        ));
        assert!(matches!(
            DeltaTSpline::new(vec![a, segment(1.0, 2.0, [1.01, 0.0, 0.0, 0.0])]),
            Err(DeltaTSplineError::Discontinuous)
        ));
        assert!(matches!(
            DeltaTSpline::new(vec![segment(0.0, 1.0, [f64::INFINITY, 0.0, 0.0, 0.0])]),
            Err(DeltaTSplineError::InvalidSegment)
        ));
    }
}
