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

//! Analytic 3-axis SPK-type-2-style Chebyshev position and velocity contract.
//!
//! Deterministic mathematical fixtures only, not JPL/SPK numerical validation.

use virgostars_ephemeris_core::{ChebyshevPositionRecord, EphemerisError, TdbSecondsPastJ2000};

fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 2.0e-12,
        "expected {expected}, received {actual}"
    );
}

/// Independent ordinary-power expansions of the supplied Chebyshev basis.
/// T2(x) = 2x² - 1 and T3(x) = 4x³ - 3x.
fn analytic_position_and_slope(x: f64) -> ([f64; 3], [f64; 3]) {
    (
        [
            6.0 - 12.0 * x - 8.0 * x * x + 20.0 * x * x * x,
            -3.0 + 3.0 * x + 4.0 * x * x - 4.0 * x * x * x,
            6.0 - 8.0 * x + 2.0 * x * x + 8.0 * x * x * x,
        ],
        [
            -12.0 - 16.0 * x + 60.0 * x * x,
            3.0 + 8.0 * x - 12.0 * x * x,
            -8.0 + 4.0 * x + 24.0 * x * x,
        ],
    )
}

fn record(radius_seconds: f64) -> ChebyshevPositionRecord {
    ChebyshevPositionRecord::new(
        TdbSecondsPastJ2000::new(5_000.0).unwrap(),
        radius_seconds,
        [
            vec![2.0, 3.0, -4.0, 5.0],
            vec![-1.0, 0.0, 2.0, -1.0],
            vec![7.0, -2.0, 1.0, 2.0],
        ],
    )
    .unwrap()
}

#[test]
fn cubic_three_axis_position_and_velocity_match_analytic_time_derivatives() {
    // dX/dET = (dX/dx)/radius_seconds. Both output units are explicit:
    // positions in km and velocities in km/second.
    let normalized_times: [f64; 7] = [-1.0, -0.8, -0.6, 0.0, 0.25, 0.8, 1.0];

    for radius_seconds in [25.0, 50.0] {
        let polynomial = record(radius_seconds);
        for x in normalized_times {
            let et = TdbSecondsPastJ2000::new(5_000.0 + x * radius_seconds).unwrap();
            let actual = polynomial.state_at(et).unwrap();
            let (expected_position, expected_slope) = analytic_position_and_slope(x);
            for axis in 0..3 {
                let position = [
                    actual.position_km.x,
                    actual.position_km.y,
                    actual.position_km.z,
                ][axis];
                let velocity = [
                    actual.velocity_km_per_second.x,
                    actual.velocity_km_per_second.y,
                    actual.velocity_km_per_second.z,
                ][axis];
                near(position, expected_position[axis]);
                near(velocity, expected_slope[axis] / radius_seconds);
            }
        }
    }
}

#[test]
fn doubling_record_duration_halves_speed_without_changing_positions() {
    let short = record(25.0);
    let long = record(50.0);
    // The same x corresponds to distinct ET epochs for different radii.
    for x in [-0.75, -0.25, 0.0, 0.5, 1.0] {
        let a = short
            .state_at(TdbSecondsPastJ2000::new(5_000.0 + 25.0 * x).unwrap())
            .unwrap();
        let b = long
            .state_at(TdbSecondsPastJ2000::new(5_000.0 + 50.0 * x).unwrap())
            .unwrap();
        for (pa, pb, va, vb) in [
            (a.position_km.x, b.position_km.x, a.velocity_km_per_second.x, b.velocity_km_per_second.x),
            (a.position_km.y, b.position_km.y, a.velocity_km_per_second.y, b.velocity_km_per_second.y),
            (a.position_km.z, b.position_km.z, a.velocity_km_per_second.z, b.velocity_km_per_second.z),
        ] {
            near(pa, pb);
            near(va, 2.0 * vb);
        }
    }
}

#[test]
fn rejects_times_outside_both_synthetic_record_radii() {
    for radius_seconds in [25.0, 50.0] {
        let polynomial = record(radius_seconds);
        for et in [5_000.0 - radius_seconds - 0.125, 5_000.0 + radius_seconds + 0.125] {
            assert!(matches!(
                polynomial.state_at(TdbSecondsPastJ2000::new(et).unwrap()),
                Err(EphemerisError::OutsideRecord { .. })
            ));
        }
    }
}
