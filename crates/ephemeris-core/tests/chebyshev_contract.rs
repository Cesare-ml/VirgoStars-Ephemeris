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

//! Deterministic public-API regression checks for Chebyshev interpolation.
//!
//! These fixtures test mathematical identities and unit conversion only.
//! They are not a validation against real NASA/JPL kernels or Horizons.

use ephemeris_core::{
    evaluate_chebyshev, ChebyshevPositionRecord, EphemerisError, TdbSecondsPastJ2000,
};

fn assert_near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual} (tolerance {tolerance})"
    );
}

#[test]
fn high_degree_chebyshev_endpoint_derivatives_follow_n_squared() {
    // Exact identities T_n(+1)=1, T_n(-1)=(-1)^n,
    // T'_n(+1)=n^2, T'_n(-1)=(-1)^(n-1)*n^2.
    // Degree 20 exercises the Clenshaw derivative recurrence beyond T2/T3.
    for degree in 0..=20_usize {
        let mut coefficients = vec![0.0; degree + 1];
        coefficients[degree] = 1.0;

        let plus = evaluate_chebyshev(&coefficients, 1.0).unwrap();
        let minus = evaluate_chebyshev(&coefficients, -1.0).unwrap();
        let n_squared = (degree * degree) as f64;

        assert_near(plus.value, 1.0, 1.0e-12);
        assert_near(plus.derivative_dx, n_squared, 1.0e-10);
        assert_near(
            minus.value,
            if degree % 2 == 0 { 1.0 } else { -1.0 },
            1.0e-12,
        );
        assert_near(
            minus.derivative_dx,
            if degree % 2 == 0 {
                -n_squared
            } else {
                n_squared
            },
            1.0e-10,
        );
    }
}

#[test]
fn type_two_velocity_uses_tdb_seconds_not_days() {
    // X(t)=100 + 5*T1((t-1000)/100) km: derivative 5/100 km/s.
    let record = ChebyshevPositionRecord::new(
        TdbSecondsPastJ2000::new(1_000.0).unwrap(),
        100.0,
        [vec![100.0, 5.0], vec![0.0], vec![0.0]],
    );
    // Equal-length coefficient arrays are required, including zero axes.
    assert_eq!(record.unwrap_err(), EphemerisError::MismatchedCoefficientLengths);

    let record = ChebyshevPositionRecord::new(
        TdbSecondsPastJ2000::new(1_000.0).unwrap(),
        100.0,
        [vec![100.0, 5.0], vec![0.0, 0.0], vec![0.0, 0.0]],
    )
    .unwrap();
    for (epoch, expected_x) in [(900.0, 95.0), (1_000.0, 100.0), (1_100.0, 105.0)] {
        let state = record
            .state_at(TdbSecondsPastJ2000::new(epoch).unwrap())
            .unwrap();
        assert_near(state.position_km.x, expected_x, 1.0e-12);
        assert_near(state.velocity_km_per_second.x, 0.05, 1.0e-12);
        assert_near(state.velocity_km_per_second.y, 0.0, 1.0e-12);
        assert_near(state.velocity_km_per_second.z, 0.0, 1.0e-12);
    }
}

#[test]
fn invalid_chebyshev_domain_and_nonfinite_coefficients_are_explicit() {
    assert_eq!(
        evaluate_chebyshev(&[1.0], -1.000_001),
        Err(EphemerisError::OutsideRecord {
            normalized_time: -1.000_001
        })
    );
    assert_eq!(
        evaluate_chebyshev(&[1.0], 1.000_001),
        Err(EphemerisError::OutsideRecord {
            normalized_time: 1.000_001
        })
    );
    assert_eq!(
        evaluate_chebyshev(&[f64::INFINITY], 0.0),
        Err(EphemerisError::NonFinite {
            field: "Chebyshev coefficient"
        })
    );
}
