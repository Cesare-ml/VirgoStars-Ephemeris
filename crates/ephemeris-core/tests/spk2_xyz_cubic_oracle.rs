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

//! Independent three-axis cubic polynomial oracle for an SPK type-2-style
//! in-memory position record. No binary SPK kernel is opened here.
//!
//! Created 2026-11-14: validate the actual public Rust state evaluation and
//! derivative conversion across negative TDB ET epochs, record endpoints and
//! finite differences. This is synthetic mathematics, NOT JPL/Horizons parity.

use virgostars_ephemeris_core::{ChebyshevPositionRecord, TdbSecondsPastJ2000, Vector3};

const MIDPOINT_ET: f64 = -172_800.0;
const RADIUS_SECONDS: f64 = 43_200.0;

fn et(seconds: f64) -> TdbSecondsPastJ2000 {
    TdbSecondsPastJ2000::new(seconds).unwrap()
}

fn record() -> ChebyshevPositionRecord {
    // Independent cubics in normalized u:
    // x(u) = 11 + 2u - 3u² + 0.5u³
    // y(u) = -5 + 4u + 0.25u² - 2u³
    // z(u) = 3 - u + 1.5u² + u³
    // Map via u²=(T2+T0)/2 and u³=(T3+3T1)/4.
    ChebyshevPositionRecord::new(
        et(MIDPOINT_ET),
        RADIUS_SECONDS,
        [
            vec![9.5, 2.375, -1.5, 0.125],
            vec![-4.875, 2.5, 0.125, -0.5],
            vec![3.75, -0.25, 0.75, 0.25],
        ],
    )
    .unwrap()
}

fn polynomial_oracle(u: f64) -> ([f64; 3], [f64; 3]) {
    let u2 = u * u;
    let u3 = u2 * u;
    let xyz = [
        11.0 + 2.0 * u - 3.0 * u2 + 0.5 * u3,
        -5.0 + 4.0 * u + 0.25 * u2 - 2.0 * u3,
        3.0 - u + 1.5 * u2 + u3,
    ];
    let dxyz_dt = [
        (2.0 - 6.0 * u + 1.5 * u2) / RADIUS_SECONDS,
        (4.0 + 0.5 * u - 6.0 * u2) / RADIUS_SECONDS,
        (-1.0 + 3.0 * u + 3.0 * u2) / RADIUS_SECONDS,
    ];
    (xyz, dxyz_dt)
}

fn axes(vector: Vector3) -> [f64; 3] {
    [vector.x, vector.y, vector.z]
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected:.16e}, got {actual:.16e}, tolerance {tolerance:.2e}"
    );
}

#[test]
fn three_axis_positions_and_km_per_second_rates_match_independent_cubics() {
    let record = record();
    for u in [-1.0, -0.75, -0.25, 0.0, 0.25, 0.75, 1.0] {
        let sample = record
            .state_at(et(MIDPOINT_ET + u * RADIUS_SECONDS))
            .unwrap();
        let (position, velocity) = polynomial_oracle(u);
        for axis in 0..3 {
            near(axes(sample.position_km)[axis], position[axis], 1.0e-12);
            near(
                axes(sample.velocity_km_per_second)[axis],
                velocity[axis],
                1.0e-16,
            );
        }
    }
}

#[test]
fn three_axis_velocities_agree_with_independent_centered_time_differences() {
    let record = record();
    let delta_et_seconds = 1.0;
    for u in [-0.75, -0.375, 0.0, 0.375, 0.75] {
        let seconds = MIDPOINT_ET + u * RADIUS_SECONDS;
        let previous = record.state_at(et(seconds - delta_et_seconds)).unwrap();
        let next = record.state_at(et(seconds + delta_et_seconds)).unwrap();
        let current = record.state_at(et(seconds)).unwrap();
        for axis in 0..3 {
            let slope = (axes(next.position_km)[axis] - axes(previous.position_km)[axis])
                / (2.0 * delta_et_seconds);
            near(axes(current.velocity_km_per_second)[axis], slope, 2.0e-12);
        }
    }
}

#[test]
fn both_record_endpoints_are_inclusive_but_no_extrapolation_is_permitted() {
    let record = record();
    let start = MIDPOINT_ET - RADIUS_SECONDS;
    let end = MIDPOINT_ET + RADIUS_SECONDS;
    assert!(record.state_at(et(start)).is_ok());
    assert!(record.state_at(et(end)).is_ok());
    assert!(record.state_at(et(start - 1.0)).is_err());
    assert!(record.state_at(et(end + 1.0)).is_err());
}

#[test]
fn rejects_invalid_radius_and_coefficients_before_any_state_is_returned() {
    let coefficients = [vec![1.0], vec![2.0], vec![3.0]];
    for radius in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            ChebyshevPositionRecord::new(et(MIDPOINT_ET), radius, coefficients.clone()).is_err()
        );
    }
    assert!(ChebyshevPositionRecord::new(
        et(MIDPOINT_ET),
        RADIUS_SECONDS,
        [vec![1.0], vec![f64::NEG_INFINITY], vec![3.0]],
    )
    .is_err());
}
