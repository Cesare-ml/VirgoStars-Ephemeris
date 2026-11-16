// The contents of this file are subject to the Common Public Attribution License Version 1.0 (CPAL-1.0).
// See LICENSE and NOTICE for the full terms and VirgoStars attribution.
//
// Modification (2026-11-14): deterministic scale-covariance regression for
// the public Chebyshev value-and-derivative API.

use virgostars_ephemeris_core::chebyshev::evaluate_chebyshev;

#[test]
fn binary_power_scaling_preserves_value_and_normalized_derivative() {
    // Scaling every coefficient by an exact power of two should scale both
    // outputs by the same factor, without changing the normalized time.
    // This tests the public contract without numerical differentiation.
    let coefficients = [0.25, -0.5, 0.125, 0.0625, -0.03125];
    for x in [-1.0, -0.75, -0.125, 0.0, 0.375, 0.875, 1.0] {
        let baseline = evaluate_chebyshev(&coefficients, x).unwrap();
        for exponent in [-40, -8, 0, 8, 40] {
            for sign in [-1.0, 1.0] {
                let scale = sign * 2.0_f64.powi(exponent);
                let scaled: Vec<f64> = coefficients.iter().map(|c| c * scale).collect();
                let sample = evaluate_chebyshev(&scaled, x).unwrap();
                let value_tolerance = 1e-12 * scale.abs() * baseline.value.abs().max(1.0);
                let derivative_tolerance =
                    1e-12 * scale.abs() * baseline.derivative_dx.abs().max(1.0);
                assert!(
                    (sample.value - baseline.value * scale).abs() <= value_tolerance,
                    "value scaling failed at x={x}, exponent={exponent}, sign={sign}"
                );
                assert!(
                    (sample.derivative_dx - baseline.derivative_dx * scale).abs()
                        <= derivative_tolerance,
                    "derivative scaling failed at x={x}, exponent={exponent}, sign={sign}"
                );
            }
        }
    }
}

#[test]
fn reversing_normalized_epoch_with_odd_coefficient_signs_preserves_series() {
    // T_n(-x) = (-1)^n T_n(x). For q_n = (-1)^n c_n,
    // Q(-x) = P(x), while Q'(-x) = -P'(x).
    // Check both values and derivatives without finite differences.
    let coefficients = [0.125, -0.75, 0.5, 0.0625, -0.25, 0.03125, 0.015625];
    let reflected: Vec<f64> = coefficients
        .iter()
        .enumerate()
        .map(|(degree, &coefficient)| {
            if degree % 2 == 0 {
                coefficient
            } else {
                -coefficient
            }
        })
        .collect();

    for x in [-1.0, -0.875, -0.5, -0.125, 0.0, 0.25, 0.75, 1.0] {
        let original = evaluate_chebyshev(&coefficients, x).unwrap();
        let mirrored = evaluate_chebyshev(&reflected, -x).unwrap();
        assert!(
            (original.value - mirrored.value).abs() <= 1e-12,
            "reflection value mismatch at x={x}"
        );
        assert!(
            (original.derivative_dx + mirrored.derivative_dx).abs() <= 1e-12,
            "reflection derivative mismatch at x={x}"
        );
    }
}
