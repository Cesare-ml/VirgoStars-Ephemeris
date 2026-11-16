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

use virgostars_ephemeris_core::evaluate_chebyshev;

/// Modification record (2026-11-11): add independent Chebyshev endpoint
/// derivative identities; no external ephemeris data or Swiss code is used.
#[test]
fn basis_endpoint_derivatives_match_closed_form() {
    // T_n(1) = 1; T_n(-1) = (-1)^n.
    // T'_n(1) = n^2; T'_n(-1) = (-1)^(n-1) * n^2 (n >= 1).
    for degree in [0_usize, 1, 2, 3, 8, 16, 32] {
        let mut coefficients = vec![0.0; degree + 1];
        coefficients[degree] = 1.0;
        let plus = evaluate_chebyshev(&coefficients, 1.0).unwrap();
        let minus = evaluate_chebyshev(&coefficients, -1.0).unwrap();
        let expected_minus = if degree % 2 == 0 { 1.0 } else { -1.0 };
        let slope = (degree * degree) as f64;
        let expected_negative_slope = if degree == 0 {
            0.0
        } else if degree % 2 == 0 {
            -slope
        } else {
            slope
        };
        let tolerance = 1e-10 * slope.max(1.0);
        assert!((plus.value - 1.0).abs() < tolerance, "degree={degree}");
        assert!((minus.value - expected_minus).abs() < tolerance, "degree={degree}");
        assert!((plus.derivative_dx - slope).abs() < tolerance, "degree={degree}");
        assert!(
            (minus.derivative_dx - expected_negative_slope).abs() < tolerance,
            "degree={degree}"
        );
    }
}
