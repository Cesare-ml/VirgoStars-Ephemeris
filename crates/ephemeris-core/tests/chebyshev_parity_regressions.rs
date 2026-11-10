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

//! Deterministic polynomial identities for Chebyshev interpolation.
//! Modification record: 2026-11-10 — add parity, endpoint and derivative fixtures.
//! Synthetic polynomial tests do not establish real-kernel/JPL numerical parity.

use virgostars_ephemeris_core::evaluate_chebyshev;

fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-11,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn fourth_order_polynomial_and_derivative_match_closed_form() {
    // T4(x) = 8x^4 - 8x^2 + 1; T4'(x) = 32x^3 - 16x.
    for x in [-1.0_f64, -0.75, -0.25, 0.0, 0.25, 0.75, 1.0] {
        let sample = evaluate_chebyshev(&[0.0, 0.0, 0.0, 0.0, 1.0], x).unwrap();
        near(sample.value, 8.0 * x.powi(4) - 8.0 * x.powi(2) + 1.0);
        near(sample.derivative_dx, 32.0 * x.powi(3) - 16.0 * x);
    }
}

#[test]
fn even_and_odd_series_obey_parity_in_value_and_derivative() {
    let even = [2.0, 0.0, -3.0, 0.0, 0.5];
    let odd = [0.0, -1.5, 0.0, 2.0];
    for x in [0.0, 0.125, 0.5, 0.875, 1.0] {
        let ep = evaluate_chebyshev(&even, x).unwrap();
        let en = evaluate_chebyshev(&even, -x).unwrap();
        near(ep.value, en.value);
        near(ep.derivative_dx, -en.derivative_dx);

        let op = evaluate_chebyshev(&odd, x).unwrap();
        let on = evaluate_chebyshev(&odd, -x).unwrap();
        near(op.value, -on.value);
        near(op.derivative_dx, on.derivative_dx);
    }
}
