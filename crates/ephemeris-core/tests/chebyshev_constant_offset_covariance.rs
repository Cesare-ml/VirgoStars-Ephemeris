// The contents of this file are subject to the Common Public Attribution License Version 1.0 (CPAL-1.0).
// See LICENSE and NOTICE for the full terms and VirgoStars attribution.
// Original Developer: Cesare-ml (VirgoStars project).
//
// Modification (2026-11-15): deterministic constant-offset covariance
// regression for the public Chebyshev value-and-derivative API.

use virgostars_ephemeris_core::chebyshev::evaluate_chebyshev;

#[test]
fn constant_term_offset_changes_value_but_not_normalized_derivative() {
    // T_0(x) = 1 and dT_0/dx = 0. Adding an offset to c_0 must
    // translate the value uniformly without changing the derivative.
    // Binary fractions keep the expected differences exactly representable.
    let coefficients = [0.25, -0.5, 0.125, 0.0625, -0.03125];
    for x in [-1.0, -0.75, -0.125, 0.0, 0.375, 0.875, 1.0] {
        let baseline = evaluate_chebyshev(&coefficients, x).unwrap();
        for offset in [-32.0, -0.5, 0.0, 0.25, 64.0] {
            let mut shifted = coefficients;
            shifted[0] += offset;
            let sample = evaluate_chebyshev(&shifted, x).unwrap();
            assert!(
                (sample.value - (baseline.value + offset)).abs() <= 1e-12,
                "value offset mismatch at x={x}, offset={offset}"
            );
            assert!(
                (sample.derivative_dx - baseline.derivative_dx).abs() <= 1e-12,
                "derivative changed at x={x}, offset={offset}"
            );
        }
    }
}
