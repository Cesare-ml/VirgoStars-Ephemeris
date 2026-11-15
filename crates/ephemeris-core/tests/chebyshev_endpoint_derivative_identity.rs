// SPDX-License-Identifier: CPAL-1.0
// Copyright (c) 2026 Cesare-ml (VirgoStars project).
// See LICENSE and NOTICE for the CPAL-1.0 Exhibit A and attribution requirements.

//! Independent endpoint identities for Chebyshev polynomials.
//! T_n(1) = 1, T_n(-1) = (-1)^n;
//! T'_n(1) = n², T'_n(-1) = (-1)^(n-1) n².
//! These exact integer identities check the derivative recurrence without a
//! second implementation of Clenshaw or an external kernel fixture.

use virgostars_ephemeris_core::evaluate_chebyshev;

#[test]
fn basis_endpoint_values_and_derivatives() {
    for degree in 0_usize..=32 {
        let mut coefficients = vec![0.0; degree + 1];
        coefficients[degree] = 1.0;
        let plus = evaluate_chebyshev(&coefficients, 1.0).unwrap();
        let minus = evaluate_chebyshev(&coefficients, -1.0).unwrap();
        let parity = if degree % 2 == 0 { 1.0 } else { -1.0 };
        let squared = (degree * degree) as f64;
        assert_eq!(plus.value, 1.0, "degree={degree}, x=1");
        assert_eq!(minus.value, parity, "degree={degree}, x=-1");
        assert_eq!(plus.derivative_dx, squared, "degree={degree}, x=1");
        assert_eq!(minus.derivative_dx, -parity * squared, "degree={degree}, x=-1");
    }
}
