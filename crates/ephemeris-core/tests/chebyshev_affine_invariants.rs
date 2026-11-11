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

// Modification record: 2026-11-11 — independent affine-series regression tests.
// These fixtures verify mathematical consistency, not JPL numerical accuracy.

use virgostars_ephemeris_core::evaluate_chebyshev;

#[test]
fn constant_offset_changes_value_but_not_derivative() {
    let coefficients = [0.125, -2.0, 0.75, -0.375, 0.0625];
    let offset = 17.25;
    let mut shifted = coefficients;
    shifted[0] += offset;

    for x in [-1.0, -0.75, -0.125, 0.0, 0.625, 1.0] {
        let original = evaluate_chebyshev(&coefficients, x).unwrap();
        let translated = evaluate_chebyshev(&shifted, x).unwrap();
        assert!((translated.value - original.value - offset).abs() < 1e-12);
        assert!((translated.derivative_dx - original.derivative_dx).abs() < 1e-12);
    }
}

#[test]
fn coefficient_scaling_scales_both_value_and_derivative() {
    let coefficients = [0.125, -2.0, 0.75, -0.375, 0.0625];
    let factor = -3.5;
    let scaled = coefficients.map(|value| factor * value);

    for x in [-1.0, -0.75, -0.125, 0.0, 0.625, 1.0] {
        let original = evaluate_chebyshev(&coefficients, x).unwrap();
        let transformed = evaluate_chebyshev(&scaled, x).unwrap();
        assert!((transformed.value - factor * original.value).abs() < 1e-12);
        assert!((transformed.derivative_dx - factor * original.derivative_dx).abs() < 1e-12);
    }
}
