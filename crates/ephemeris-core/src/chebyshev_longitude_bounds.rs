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

//! Full-record longitude-rate bounds for a two-axis Chebyshev position polynomial.
//!
//! This is a *mathematical* global bound for a FIXED Cartesian coordinate frame
//! and a single complete Chebyshev record. It neither reads JPL data nor proves
//! an apparent/ecliptic-of-date bound. The calling provider must validate actual
//! ephemeris records, observers, frame rotations, time scales and error budgets.
//!
//! Added 2026-11-12: original VirgoStars coefficient-inequality bound primitive.

use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

use crate::longitude_enclosures::UniformTimeScale;

/// The method covers the complete normalized Chebyshev domain [-1, 1].
/// This is NOT a verification of the external ephemeris or its coefficients.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoefficientBoundCoverage {
    FullMathematicalPolynomialRecord,
}

/// Bound for planar angular rate in a fixed Cartesian reference frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChebyshevLongitudeRateBound {
    /// Absolute maximum for |d(atan2(y,x))/dt|, in degrees per uniform day.
    pub max_abs_rate_deg_per_day: f64,
    /// Strict positive conservative lower bound for hypot(x,y).
    /// Has the same arbitrary linear units as the input coefficients.
    pub min_planar_radius: f64,
    /// Conservative upper bound for hypot(dx/dt,dy/dt), in input units/day.
    pub max_planar_speed_per_day: f64,
    pub degree: usize,
    pub time_scale: UniformTimeScale,
    pub coverage: CoefficientBoundCoverage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChebyshevRateBoundError {
    EmptyCoefficients,
    DifferentPolynomialDegrees,
    UnsupportedDegree,
    NonFiniteInput,
    InvalidRecordRadius,
    UnboundedArithmetic,
    CannotExcludePlanarOrigin,
}
impl fmt::Display for ChebyshevRateBoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCoefficients => write!(f, "x and y coefficient arrays must be nonempty"),
            Self::DifferentPolynomialDegrees => {
                write!(f, "x and y must have the same Chebyshev record degree")
            }
            Self::UnsupportedDegree => write!(f, "Chebyshev record degree is too large"),
            Self::NonFiniteInput => write!(f, "every Chebyshev coefficient must be finite"),
            Self::InvalidRecordRadius => write!(f, "record radius in uniform days must be positive"),
            Self::UnboundedArithmetic => {
                write!(f, "coefficient bound arithmetic overflowed; cannot guarantee a rate")
            }
            Self::CannotExcludePlanarOrigin => {
                write!(f, "coefficient intervals cannot exclude an undefined longitude")
            }
        }
    }
}
impl Error for ChebyshevRateBoundError {}

/// Produce a global angular-speed bound from **all** planar Chebyshev coefficients.
///
/// Coefficients use the convention sum(c[k] * T_k(tau)) with no c[0]/2
/// adjustment (the same convention as `crate::chebyshev`), and tau in [-1,1].
/// The record radius is given in the same **uniform time scale** used for
/// requested angular speeds, in days: tau = (t - midpoint) / radius_days.
///
/// For all tau in [-1,1], |T_k(tau)| <= 1 and |T'_k(tau)| <= k*k.
/// Consequently, for component u in {x,y}:
///
///   |u(tau)| >= max(0, |c0| - sum_{k>=1}|ck|)
///   |du/dt| <= sum_{k>=1} k*k*|ck| / radius_days.
///
/// If planar separation is bounded away from zero, then
///
///   |dlambda/dt| <= hypot(|dx/dt|_max, |dy/dt|_max) / r_xy_min.
///
/// This bound is valid across stations and changes in sign of angular speed
/// for the *mathematical polynomial*. Floating-point operations use a
/// conservative safety inflation factor, but are NOT certified outward-rounded
/// interval arithmetic. For scientific certification, validate the kernel
/// coefficients and numerical error separately. Missing positive radius
/// certification fails closed rather than silently asserting a finite speed.
///
/// This function does NOT transform coordinates, derive a geocentric state,
/// apply apparent corrections or calculate any astrological aspect.
pub fn bound_full_chebyshev_planar_longitude_rate(
    x_coefficients: &[f64],
    y_coefficients: &[f64],
    radius_days: f64,
    time_scale: UniformTimeScale,
) -> Result<ChebyshevLongitudeRateBound, ChebyshevRateBoundError> {
    if x_coefficients.is_empty() || y_coefficients.is_empty() {
        return Err(ChebyshevRateBoundError::EmptyCoefficients);
    }
    if x_coefficients.len() != y_coefficients.len() {
        return Err(ChebyshevRateBoundError::DifferentPolynomialDegrees);
    }
    // Every coefficient is visited; a very high degree also magnifies roundoff
    // and makes the simple coefficient-sum bound impractically loose.
    if x_coefficients.len() > 4096 {
        return Err(ChebyshevRateBoundError::UnsupportedDegree);
    }
    if x_coefficients
        .iter()
        .chain(y_coefficients.iter())
        .any(|c| !c.is_finite())
    {
        return Err(ChebyshevRateBoundError::NonFiniteInput);
    }
    if !radius_days.is_finite() || radius_days <= 0.0 {
        return Err(ChebyshevRateBoundError::InvalidRecordRadius);
    }

    // Inflate positive upper bounds and deflate the lower bound against
    // ordinary floating-point rounding. This is explicitly not an error
    // budget for a physical astronomical ephemeris or a formal directed
    // rounding proof. If any operation loses finite representability, fail.
    let n = (x_coefficients.len() - 1) as f64;
    let guard = 1.0 + 64.0 * f64::EPSILON * (n + 4.0);
    if !guard.is_finite() {
        return Err(ChebyshevRateBoundError::UnboundedArithmetic);
    }

    fn estimates(
        coefficients: &[f64],
        guard: f64,
    ) -> Result<(f64, f64), ChebyshevRateBoundError> {
        let mut tail = 0.0_f64;
        let mut derivative = 0.0_f64;
        for (i, coefficient) in coefficients.iter().enumerate().skip(1) {
            let magnitude = coefficient.abs();
            tail += magnitude;
            let weight = (i as f64) * (i as f64);
            derivative += magnitude * weight;
            if !tail.is_finite() || !derivative.is_finite() {
                return Err(ChebyshevRateBoundError::UnboundedArithmetic);
            }
        }
        let tail_upper = tail * guard;
        let derivative_upper = derivative * guard;
        if !tail_upper.is_finite() || !derivative_upper.is_finite() {
            return Err(ChebyshevRateBoundError::UnboundedArithmetic);
        }
        let center_lower = coefficients[0].abs() / guard;
        let coordinate_lower = (center_lower - tail_upper).max(0.0);
        Ok((coordinate_lower, derivative_upper))
    }

    let (x_lower, dx_upper_normalized) = estimates(x_coefficients, guard)?;
    let (y_lower, dy_upper_normalized) = estimates(y_coefficients, guard)?;
    let radius_lower = x_lower.hypot(y_lower) / guard;
    if !radius_lower.is_finite() {
        return Err(ChebyshevRateBoundError::UnboundedArithmetic);
    }
    if radius_lower <= 0.0 {
        return Err(ChebyshevRateBoundError::CannotExcludePlanarOrigin);
    }

    let dx_upper = (dx_upper_normalized / radius_days) * guard;
    let dy_upper = (dy_upper_normalized / radius_days) * guard;
    let velocity_upper = dx_upper.hypot(dy_upper) * guard;
    let rate_upper = (velocity_upper / radius_lower) * (180.0 / PI) * guard;
    if !dx_upper.is_finite()
        || !dy_upper.is_finite()
        || !velocity_upper.is_finite()
        || !rate_upper.is_finite()
    {
        return Err(ChebyshevRateBoundError::UnboundedArithmetic);
    }
    Ok(ChebyshevLongitudeRateBound {
        max_abs_rate_deg_per_day: rate_upper,
        min_planar_radius: radius_lower,
        max_planar_speed_per_day: velocity_upper,
        degree: x_coefficients.len() - 1,
        time_scale,
        coverage: CoefficientBoundCoverage::FullMathematicalPolynomialRecord,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chebyshev::evaluate_chebyshev;
    use crate::longitude_enclosures::{
        enclose_longitude_from_claim, LongitudeRateBoundClaim,
    };

    fn bound(x: &[f64], y: &[f64]) -> Result<ChebyshevLongitudeRateBound, ChebyshevRateBoundError> {
        bound_full_chebyshev_planar_longitude_rate(x, y, 1.0, UniformTimeScale::Tdb)
    }

    #[test]
    fn constant_nonzero_planar_position_has_zero_global_rate() {
        let result = bound(&[10.0], &[0.0]).unwrap();
        assert_eq!(result.max_abs_rate_deg_per_day, 0.0);
        assert!(result.min_planar_radius > 0.0);
        assert_eq!(result.degree, 0);
        assert_eq!(result.coverage, CoefficientBoundCoverage::FullMathematicalPolynomialRecord);
    }

    #[test]
    fn linear_y_sweep_has_global_bound_including_both_endpoints() {
        let result = bound(&[10.0, 0.0], &[0.0, 1.0]).unwrap();
        for i in -100..=100 {
            let tau = f64::from(i) / 100.0;
            let x = evaluate_chebyshev(&[10.0, 0.0], tau).unwrap();
            let y = evaluate_chebyshev(&[0.0, 1.0], tau).unwrap();
            let rate = (x.value * y.derivative_dx - y.value * x.derivative_dx)
                / (x.value * x.value + y.value * y.value);
            assert!(rate.abs().to_degrees() <= result.max_abs_rate_deg_per_day);
        }
        assert!(result.max_abs_rate_deg_per_day > 5.0);
    }

    #[test]
    fn station_and_retrograde_with_t2_synthetic_polynomial() {
        // y(tau) = tau^2 - 0.5 = 0.5*T2(tau), derivative 2*tau.
        // Angular speed reverses at the record midpoint tau=0.
        let x = [10.0, 0.0, 0.0];
        let y = [0.0, 0.0, 0.5];
        let result = bound(&x, &y).unwrap();
        for i in -200..=200 {
            let tau = f64::from(i) / 200.0;
            let a = evaluate_chebyshev(&x, tau).unwrap();
            let b = evaluate_chebyshev(&y, tau).unwrap();
            let rate = (a.value * b.derivative_dx - b.value * a.derivative_dx)
                / (a.value * a.value + b.value * b.value);
            assert!(rate.abs().to_degrees() <= result.max_abs_rate_deg_per_day);
        }
    }

    #[test]
    fn fails_closed_when_coefficient_bounds_cannot_exclude_origin() {
        assert_eq!(bound(&[0.0, 1.0], &[0.0, 0.0]), Err(
            ChebyshevRateBoundError::CannotExcludePlanarOrigin
        ));
        // Conservative false-negative: x^2 + y^2 may be nonzero, but the
        // independent coefficient intervals are too loose to prove it.
        assert_eq!(bound(&[1.0, 2.0], &[1.0, 2.0]), Err(
            ChebyshevRateBoundError::CannotExcludePlanarOrigin
        ));
    }

    #[test]
    fn rejects_invalid_input_without_rate_fallback() {
        assert_eq!(bound(&[], &[]), Err(ChebyshevRateBoundError::EmptyCoefficients));
        assert_eq!(
            bound(&[1.0], &[1.0, 2.0]),
            Err(ChebyshevRateBoundError::DifferentPolynomialDegrees)
        );
        assert_eq!(bound(&[f64::NAN], &[2.0]), Err(ChebyshevRateBoundError::NonFiniteInput));
        assert_eq!(bound(&[1.0], &[f64::INFINITY]), Err(
            ChebyshevRateBoundError::NonFiniteInput
        ));
        assert_eq!(
            bound_full_chebyshev_planar_longitude_rate(
                &[1.0], &[0.0], 0.0, UniformTimeScale::Tdb
            ),
            Err(ChebyshevRateBoundError::InvalidRecordRadius)
        );
        assert_eq!(
            bound_full_chebyshev_planar_longitude_rate(
                &[1.0], &[0.0], f64::NAN, UniformTimeScale::Tdb
            ),
            Err(ChebyshevRateBoundError::InvalidRecordRadius)
        );
    }

    #[test]
    fn rejects_nonrepresentable_derivative_and_too_many_coefficients() {
        let bad = bound(&[10.0, 0.0], &[0.0, f64::MAX]);
        assert_eq!(bad, Err(ChebyshevRateBoundError::UnboundedArithmetic));
        let huge = vec![0.0; 4097];
        assert_eq!(bound(&huge, &huge), Err(ChebyshevRateBoundError::UnsupportedDegree));
    }

    #[test]
    fn record_radius_changes_rate_in_inverse_proportion() {
        let one = bound_full_chebyshev_planar_longitude_rate(
            &[10.0, 0.0], &[0.0, 1.0], 1.0, UniformTimeScale::Tdb,
        ).unwrap();
        let two = bound_full_chebyshev_planar_longitude_rate(
            &[10.0, 0.0], &[0.0, 1.0], 2.0, UniformTimeScale::Tt,
        ).unwrap();
        assert!((two.max_abs_rate_deg_per_day / one.max_abs_rate_deg_per_day - 0.5).abs() < 1.0e-12);
        assert_eq!(two.time_scale, UniformTimeScale::Tt);
    }

    #[test]
    fn conservative_rate_can_feed_existing_declared_enclosure_helper() {
        let b = bound(&[10.0, 0.0], &[0.0, 1.0]).unwrap();
        let arc = enclose_longitude_from_claim(LongitudeRateBoundClaim {
            reference_longitude_deg: 0.0,
            start_offset_days: -1.0,
            end_offset_days: 1.0,
            max_abs_rate_deg_per_day: b.max_abs_rate_deg_per_day,
            reference_error_deg: 0.0,
            time_scale: UniformTimeScale::Tdb,
            asserts_full_interval_rate_bound: true,
        }).unwrap();
        // Two degrees-of-freedom? Only x=10 and y=tau here: both ends
        // must lie inside the circular enclosure generated from the bound.
        let contains = |longitude_deg: f64| {
            (longitude_deg - arc.start_deg).rem_euclid(360.0) <= arc.span_deg + 1.0e-10
        };
        assert!(contains(1.0_f64.atan2(10.0).to_degrees()));
        assert!(contains((-1.0_f64).atan2(10.0).to_degrees().rem_euclid(360.0)));
    }
}
