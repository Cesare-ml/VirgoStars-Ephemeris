// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0.
//
// The Original Developer is Cesare-ml (VirgoStars project). Contributors retain applicable rights in their own contributions under the terms of this License.
// SPDX-License-Identifier: CPAL-1.0

//! Deterministic high-order Chebyshev endpoint checks for the canonical Rust core.
//! These are mathematical fixtures, not NASA/JPL ephemeris accuracy claims.

use virgostars_ephemeris_core::evaluate_chebyshev;

#[test]
fn high_order_basis_endpoint_values_and_derivatives() {
    // T_n(1) = 1, T_n(-1) = (-1)^n;
    // T'_n(1) = n^2, T'_n(-1) = (-1)^(n-1) n^2.
    // Checking a single nonzero coefficient also detects unintended c0 halving.
    for degree in [0_usize, 1, 2, 3, 8, 16, 32, 64, 99] {
        let mut coefficients = vec![0.0; degree + 1];
        coefficients[degree] = 1.0;
        let at_start = evaluate_chebyshev(&coefficients, -1.0).unwrap();
        let at_end = evaluate_chebyshev(&coefficients, 1.0).unwrap();
        let n_squared = (degree * degree) as f64;
        let start_value = if degree % 2 == 0 { 1.0 } else { -1.0 };
        let start_derivative = if degree == 0 {
            0.0
        } else if degree % 2 == 0 {
            -n_squared
        } else {
            n_squared
        };
        assert_eq!(at_start.value, start_value, "degree {degree}, x=-1");
        assert_eq!(at_end.value, 1.0, "degree {degree}, x=1");
        assert_eq!(
            at_start.derivative_dx, start_derivative,
            "degree {degree}, derivative at x=-1"
        );
        assert_eq!(
            at_end.derivative_dx, n_squared,
            "degree {degree}, derivative at x=1"
        );
    }
}

#[test]
fn high_order_basis_has_expected_midpoint_parity() {
    // T_(2k+1)(0) = 0 and T'_(2k)(0) = 0.
    // T'_(2k+1)(0) = (-1)^k (2k+1).
    for degree in [1_usize, 3, 5, 17, 33, 65, 99] {
        let mut coefficients = vec![0.0; degree + 1];
        coefficients[degree] = 1.0;
        let sample = evaluate_chebyshev(&coefficients, 0.0).unwrap();
        assert_eq!(sample.value, 0.0, "degree {degree}");
        let expected = if ((degree - 1) / 2) % 2 == 0 {
            degree as f64
        } else {
            -(degree as f64)
        };
        assert_eq!(sample.derivative_dx, expected, "degree {degree}");
    }
}
