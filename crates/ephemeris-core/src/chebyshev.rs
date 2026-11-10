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

//! Chebyshev series evaluation and derivative by extended Clenshaw recurrence.
//!
//! A coefficient slice `[c0, c1, ... cn]` represents
//! `c0*T0(x) + c1*T1(x) + ... + cn*Tn(x)`, with **no implicit halving of c0**.
//! The caller supplies normalized x in [-1, 1]. For SPK type-2 records,
//! d/dt = (d/dx) / radius_seconds.

use crate::error::{EphemerisError, Result};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChebyshevSample {
    pub value: f64,
    /// Derivative with respect to normalized coordinate x (NOT seconds).
    pub derivative_dx: f64,
}

/// Evaluate a Chebyshev series and its exact polynomial derivative.
///
/// Rejects out-of-domain x rather than extrapolating SPK record coefficients.
/// Endpoints -1 and +1 are allowed.
pub fn evaluate_chebyshev(coefficients: &[f64], x: f64) -> Result<ChebyshevSample> {
    if coefficients.is_empty() {
        return Err(EphemerisError::EmptyCoefficients);
    }
    if !x.is_finite() {
        return Err(EphemerisError::NonFinite {
            field: "normalized Chebyshev coordinate",
        });
    }
    if coefficients.iter().any(|c| !c.is_finite()) {
        return Err(EphemerisError::NonFinite {
            field: "Chebyshev coefficient",
        });
    }
    if !(-1.0..=1.0).contains(&x) {
        return Err(EphemerisError::OutsideRecord { normalized_time: x });
    }

    // For k=n..1:
    //   b_k  = c_k + 2*x*b_(k+1) - b_(k+2)
    //   db_k = 2*b_(k+1) + 2*x*db_(k+1) - db_(k+2)
    // Sum = c0 + x*b1 - b2; derivative = b1 + x*db1 - db2.
    let (mut b1, mut b2) = (0.0_f64, 0.0_f64);
    let (mut db1, mut db2) = (0.0_f64, 0.0_f64);

    for &c in coefficients.iter().skip(1).rev() {
        let b = c + 2.0 * x * b1 - b2;
        let db = 2.0 * b1 + 2.0 * x * db1 - db2;
        b2 = b1;
        b1 = b;
        db2 = db1;
        db1 = db;
    }

    let value = coefficients[0] + x * b1 - b2;
    let derivative_dx = b1 + x * db1 - db2;
    if !value.is_finite() || !derivative_dx.is_finite() {
        return Err(EphemerisError::ArithmeticOverflow);
    }

    Ok(ChebyshevSample {
        value,
        derivative_dx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn constant_series() {
        let s = evaluate_chebyshev(&[12.5], -1.0).unwrap();
        near(s.value, 12.5);
        near(s.derivative_dx, 0.0);
    }

    #[test]
    fn chebyshev_t2_and_derivative() {
        // T2(x) = 2*x*x - 1; derivative = 4*x.
        let s = evaluate_chebyshev(&[0.0, 0.0, 1.0], 0.5).unwrap();
        near(s.value, -0.5);
        near(s.derivative_dx, 2.0);
    }

    #[test]
    fn cubic_series_and_derivative() {
        // T0 + 2*T1 + 3*T2 + 4*T3
        // = -2 - 10*x + 6*x^2 + 16*x^3
        // derivative = -10 + 12*x + 48*x^2.
        let x = 0.25;
        let s = evaluate_chebyshev(&[1.0, 2.0, 3.0, 4.0], x).unwrap();
        near(s.value, -2.0 - 10.0 * x + 6.0 * x * x + 16.0 * x * x * x);
        near(s.derivative_dx, -10.0 + 12.0 * x + 48.0 * x * x);
    }

    #[test]
    fn evaluates_both_endpoints() {
        let c = [1.0, 2.0, 3.0, 4.0];
        near(evaluate_chebyshev(&c, 1.0).unwrap().value, 10.0);
        near(evaluate_chebyshev(&c, -1.0).unwrap().value, -2.0);
    }

    #[test]
    fn rejects_invalid_inputs_and_extrapolation() {
        assert_eq!(
            evaluate_chebyshev(&[], 0.0),
            Err(EphemerisError::EmptyCoefficients)
        );
        assert!(evaluate_chebyshev(&[1.0, f64::NAN], 0.0).is_err());
        assert!(evaluate_chebyshev(&[1.0], 1.0001).is_err());
        assert!(evaluate_chebyshev(&[1.0], f64::NAN).is_err());
    }
}
