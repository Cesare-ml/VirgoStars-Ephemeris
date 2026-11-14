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

#[test]
fn high_order_even_basis_has_zero_midpoint_slope() {
    // T_(2k)(0) = (-1)^k and T'_(2k)(0) = 0.
    // This complements the odd-degree midpoint test and guards both parity classes.
    for degree in [0_usize, 2, 4, 16, 32, 64, 98] {
        let mut coefficients = vec![0.0; degree + 1];
        coefficients[degree] = 1.0;
        let sample = evaluate_chebyshev(&coefficients, 0.0).unwrap();
        let expected = if (degree / 2) % 2 == 0 { 1.0 } else { -1.0 };
        assert_eq!(sample.value, expected, "degree {degree}");
        assert_eq!(sample.derivative_dx, 0.0, "degree {degree}");
    }
}

#[test]
fn sparse_high_order_series_matches_independent_endpoint_sums() {
    // Mixed degrees detect accumulation errors missed by isolated basis fixtures.
    let terms = [(0_usize, 3.0), (1, -2.0), (2, 5.0), (17, -4.0), (99, 7.0)];
    let mut coefficients = vec![0.0; 100];
    for (degree, coefficient) in terms {
        coefficients[degree] = coefficient;
    }

    for x in [-1.0, 1.0] {
        let expected_value: f64 = terms
            .iter()
            .map(|&(degree, coefficient)| {
                if x == -1.0 && degree % 2 == 1 {
                    -coefficient
                } else {
                    coefficient
                }
            })
            .sum();
        let expected_derivative: f64 = terms
            .iter()
            .map(|&(degree, coefficient)| {
                let sign = if x == -1.0 && degree % 2 == 0 {
                    -1.0
                } else {
                    1.0
                };
                coefficient * sign * (degree * degree) as f64
            })
            .sum();
        let actual = evaluate_chebyshev(&coefficients, x).unwrap();
        assert_eq!(actual.value, expected_value, "value at x={x}");
        assert_eq!(
            actual.derivative_dx, expected_derivative,
            "derivative at x={x}"
        );
    }
}

#[test]
fn high_order_interior_values_and_slopes_match_trigonometric_oracle() {
    // Independent identity: T_n(cos(theta)) = cos(n*theta),
    // dT_n/dx = n*sin(n*theta)/sin(theta).
    // Interior points exercise Clenshaw beyond endpoint and midpoint identities.
    for x in [-0.75_f64, -0.5, 0.25, 0.5, 0.75] {
        let theta = x.acos();
        for degree in [1_usize, 2, 7, 17, 32, 65, 99] {
            let mut coefficients = vec![0.0; degree + 1];
            coefficients[degree] = 1.0;
            let actual = evaluate_chebyshev(&coefficients, x).unwrap();
            let angle = degree as f64 * theta;
            let expected_value = angle.cos();
            let expected_slope = degree as f64 * angle.sin() / theta.sin();
            assert!(
                (actual.value - expected_value).abs() <= 1e-10,
                "degree {degree}, x={x}, value: {} vs {expected_value}",
                actual.value
            );
            assert!(
                (actual.derivative_dx - expected_slope).abs() <= 1e-8,
                "degree {degree}, x={x}, slope: {} vs {expected_slope}",
                actual.derivative_dx
            );
        }
    }
}

#[test]
fn mixed_high_order_interior_series_matches_trigonometric_oracle() {
    // Unlike single-basis checks, this exercises Clenshaw accumulation and
    // derivative accumulation simultaneously at non-symmetric interior epochs.
    // The independent oracle uses T_n(cos(theta)) = cos(n*theta).
    let terms = [(0_usize, 0.75), (1, -1.25), (7, 0.5), (32, -0.125), (99, 0.0625)];
    let mut coefficients = vec![0.0; 100];
    for &(degree, coefficient) in &terms {
        coefficients[degree] = coefficient;
    }

    for x in [-0.875_f64, -0.375, 0.125, 0.625, 0.875] {
        let theta = x.acos();
        let expected_value: f64 = terms
            .iter()
            .map(|&(degree, coefficient)| coefficient * (degree as f64 * theta).cos())
            .sum();
        let expected_slope: f64 = terms
            .iter()
            .map(|&(degree, coefficient)| {
                coefficient * degree as f64 * (degree as f64 * theta).sin() / theta.sin()
            })
            .sum();
        let actual = evaluate_chebyshev(&coefficients, x).unwrap();
        assert!(
            (actual.value - expected_value).abs() <= 1e-10,
            "x={x}: value {} vs {expected_value}",
            actual.value
        );
        assert!(
            (actual.derivative_dx - expected_slope).abs() <= 1e-8,
            "x={x}: derivative {} vs {expected_slope}",
            actual.derivative_dx
        );
    }
}
