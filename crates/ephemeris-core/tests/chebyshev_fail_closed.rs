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

// Modification record: 2026-11-11 — deterministic fail-closed Chebyshev regressions.

use virgostars_ephemeris_core::{evaluate_chebyshev, EphemerisError};

#[test]
fn rejects_nonfinite_coefficients_even_when_they_would_not_contribute() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for coefficients in [[invalid, 0.0], [0.0, invalid]] {
            assert_eq!(
                evaluate_chebyshev(&coefficients, 0.0),
                Err(EphemerisError::NonFinite {
                    field: "Chebyshev coefficient"
                })
            );
        }
    }
}

#[test]
fn rejects_nonfinite_normalized_epochs_and_out_of_record_endpoints() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            evaluate_chebyshev(&[1.0], invalid),
            Err(EphemerisError::NonFinite {
                field: "normalized Chebyshev coordinate"
            })
        );
    }
    for outside in [-1.000_001, 1.000_001] {
        assert_eq!(
            evaluate_chebyshev(&[1.0], outside),
            Err(EphemerisError::OutsideRecord {
                normalized_time: outside
            })
        );
    }
}

#[test]
fn rejects_finite_inputs_that_overflow_value_or_derivative() {
    assert_eq!(
        evaluate_chebyshev(&[f64::MAX, f64::MAX], 1.0),
        Err(EphemerisError::ArithmeticOverflow)
    );
    // At x=1, T2=1 and T2'=4: the value remains finite but
    // the derivative cannot be represented in f64.
    assert_eq!(
        evaluate_chebyshev(&[0.0, 0.0, f64::MAX], 1.0),
        Err(EphemerisError::ArithmeticOverflow)
    );
}

#[test]
fn accepts_exact_record_endpoints_without_extrapolation() {
    for x in [-1.0, 1.0] {
        let result = evaluate_chebyshev(&[7.0], x).unwrap();
        assert_eq!(result.value, 7.0);
        assert_eq!(result.derivative_dx, 0.0);
    }
}
