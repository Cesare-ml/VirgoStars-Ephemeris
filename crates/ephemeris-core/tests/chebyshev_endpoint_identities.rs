// Independent polynomial identities for normalized Chebyshev record boundaries.
// Licensed under CPAL-1.0; see LICENSE and NOTICE.
use virgostars_ephemeris_core::evaluate_chebyshev;

#[test]
fn basis_endpoint_derivatives_follow_closed_form() {
    // T_n(1)=1, T_n(-1)=(-1)^n; T'_n(1)=n^2 and
    // T'_n(-1)=(-1)^(n-1)*n^2 (also valid for n=0).
    for degree in 0..=32_usize {
        let mut coefficients = vec![0.0; degree + 1];
        coefficients[degree] = 1.0;
        let plus = evaluate_chebyshev(&coefficients, 1.0).unwrap();
        let minus = evaluate_chebyshev(&coefficients, -1.0).unwrap();
        let parity = if degree % 2 == 0 { 1.0 } else { -1.0 };
        let derivative = (degree * degree) as f64;
        assert_eq!(plus.value, 1.0, "degree {degree}");
        assert_eq!(minus.value, parity, "degree {degree}");
        assert_eq!(plus.derivative_dx, derivative, "degree {degree}");
        assert_eq!(minus.derivative_dx, -parity * derivative, "degree {degree}");
    }
}

#[test]
fn reflected_coefficients_preserve_values_and_reverse_derivatives() {
    let coefficients = [1.25, -2.0, 0.75, 3.5, -1.0, 0.125];
    let reflected: Vec<f64> = coefficients
        .iter()
        .enumerate()
        .map(|(i, c)| if i % 2 == 0 { *c } else { -*c })
        .collect();
    for x in [-1.0, -0.75, -0.25, 0.0, 0.5, 1.0] {
        let a = evaluate_chebyshev(&coefficients, -x).unwrap();
        let b = evaluate_chebyshev(&reflected, x).unwrap();
        assert!((a.value - b.value).abs() < 1e-12, "x={x}");
        assert!((a.derivative_dx + b.derivative_dx).abs() < 1e-12, "x={x}");
    }
}
