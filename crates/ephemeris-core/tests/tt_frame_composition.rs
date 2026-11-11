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

//! Offline mathematical checks of TT-day IAU-style matrix product
//! derivatives fed into a TDB-day rotating-frame state vector.
//! Synthetic rotations only: NOT a complete IAU precession/nutation model.

use std::f64::consts::FRAC_PI_2;

use virgostars_ephemeris_core::coordinates::project_fixed_axes;
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};
use virgostars_ephemeris_core::rotating_frame::{RotatingFrame, RotatingFrameError, TtPerTdbDay};

type Matrix3 = [[f64; 3]; 3];

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e} expected={expected:.16e} tolerance={tolerance:.2e}"
    );
}

fn identity() -> Matrix3 {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

fn z(angle: f64, rate_per_tt_day: f64) -> (Matrix3, Matrix3) {
    let (sin, cos) = angle.sin_cos();
    (
        [[cos, sin, 0.0], [-sin, cos, 0.0], [0.0, 0.0, 1.0]],
        [
            [-rate_per_tt_day * sin, rate_per_tt_day * cos, 0.0],
            [-rate_per_tt_day * cos, -rate_per_tt_day * sin, 0.0],
            [0.0; 3],
        ],
    )
}

fn x(angle: f64, rate_per_tt_day: f64) -> (Matrix3, Matrix3) {
    let (sin, cos) = angle.sin_cos();
    (
        [[1.0, 0.0, 0.0], [0.0, cos, sin], [0.0, -sin, cos]],
        [
            [0.0; 3],
            [0.0, -rate_per_tt_day * sin, rate_per_tt_day * cos],
            [0.0, -rate_per_tt_day * cos, -rate_per_tt_day * sin],
        ],
    )
}

fn state(r: [f64; 3], v: [f64; 3]) -> StateVector {
    StateVector {
        target: Body::Venus,
        center: Body::Earth,
        epoch: JulianDateTdb(2_451_545.0),
        position_km: r,
        velocity_km_per_day: v,
    }
}

fn mat_vec(m: Matrix3, r: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| m[i][0] * r[0] + m[i][1] * r[1] + m[i][2] * r[2])
}

#[test]
fn tt_per_tdb_day_requires_an_explicit_finite_near_unit_rate() {
    for invalid in [0.0, -1.0, 86_400.0, f64::INFINITY, f64::NAN, 0.998, 1.002] {
        assert_eq!(
            TtPerTdbDay::new(invalid),
            Err(RotatingFrameError::InvalidTimeScaleRate)
        );
    }
    for accepted in [0.999, 1.0, 1.000_000_1, 1.001] {
        assert_eq!(TtPerTdbDay::new(accepted).unwrap().value(), accepted);
    }
}

#[test]
fn unit_ratio_agrees_with_existing_native_tdb_rotation() {
    let (rotation, tt_rate) = z(0.3, 0.01);
    let from_tt =
        RotatingFrame::from_tt_day_rate(rotation, tt_rate, TtPerTdbDay::new(1.0).unwrap()).unwrap();
    let original = RotatingFrame::new(rotation, tt_rate).unwrap();
    let source = state([1.0e7, 5.0e6, -2.0e6], [-3_000.0, 600.0, 200.0]);
    assert_eq!(
        from_tt.rotate_state(&source).unwrap(),
        original.rotate_state(&source).unwrap()
    );
}

#[test]
fn nonunity_dttt_dt_tdb_changes_apparent_rotation_rate() {
    // A stationary ICRF source at +X rotates through theta(TT).
    // The time derivative must be multiplied by supplied dTT/dTDB.
    let tt_rate = 0.1;
    let ratio = 1.000_2;
    let (rotation, derivative_tt) = z(0.0, tt_rate);
    let frame =
        RotatingFrame::from_tt_day_rate(rotation, derivative_tt, TtPerTdbDay::new(ratio).unwrap())
            .unwrap();
    let output = frame
        .rotate_state(&state([100_000.0, 0.0, 0.0], [0.0; 3]))
        .unwrap();
    near(
        output.velocity_km_per_day[1],
        -100_000.0 * tt_rate * ratio,
        1e-10,
    );
    let sph = project_fixed_axes(&output, 0.0).unwrap();
    near(sph.longitude_rate_rad_per_day, -tt_rate * ratio, 1e-15);
}

#[test]
fn noncommuting_rotation_order_is_true_equator_nutation_after_precession() {
    // At +90° Z precession, the +X source maps to -Y.
    // +90° X nutation then maps -Y to +Z. Reversed order would not.
    let (precession, p_rate) = z(FRAC_PI_2, 0.0);
    let (nutation, n_rate) = x(FRAC_PI_2, 0.0);
    let frame = RotatingFrame::from_tt_precession_nutation(
        precession,
        p_rate,
        nutation,
        n_rate,
        TtPerTdbDay::new(1.0).unwrap(),
    )
    .unwrap();
    let result = frame
        .rotate_state(&state([1.0, 0.0, 0.0], [0.0; 3]))
        .unwrap();
    near(result.position_km[0], 0.0, 1e-15);
    near(result.position_km[1], 0.0, 1e-15);
    near(result.position_km[2], 1.0, 1e-15);
}

#[test]
fn composite_matches_sequential_rotations_of_position_and_rate() {
    let ratio = TtPerTdbDay::new(1.000_05).unwrap();
    let (precession, p_rate) = z(-0.3, 0.025);
    let (nutation, n_rate) = x(0.1, -0.014);
    let p = RotatingFrame::from_tt_day_rate(precession, p_rate, ratio).unwrap();
    let n = RotatingFrame::from_tt_day_rate(nutation, n_rate, ratio).unwrap();
    let combined =
        RotatingFrame::from_tt_precession_nutation(precession, p_rate, nutation, n_rate, ratio)
            .unwrap();
    let input = state([2.5e7, -6.0e7, 1.5e7], [11_000.0, -5_000.0, 900.0]);
    let stepwise = n.rotate_state(&p.rotate_state(&input).unwrap()).unwrap();
    let one_call = combined.rotate_state(&input).unwrap();
    assert_eq!(one_call.target, Body::Venus);
    assert_eq!(one_call.center, Body::Earth);
    assert_eq!(one_call.epoch, input.epoch);
    for i in 0..3 {
        near(one_call.position_km[i], stepwise.position_km[i], 1e-7);
        near(
            one_call.velocity_km_per_day[i],
            stepwise.velocity_km_per_day[i],
            1e-7,
        );
    }
}

#[test]
fn independent_finite_differences_of_tt_dependent_date_frames_match_all_rates() {
    // Synthetic clock relation:
    // TT_elapsed = TDB_elapsed + eps*sin(TDB_elapsed).
    // Its instantaneous rate is dTT/dTDB = 1 + eps*cos(TDB).
    // This is mathematical test data and NOT an astronomical TT/TDB model.
    let eps = 0.000_2_f64;
    let t0 = 0.5_f64;
    let h = 0.000_1_f64;
    let p0 = 0.3_f64;
    let n0 = -0.2_f64;
    let p_speed = 0.025_f64;
    let n_speed = -0.012_f64;
    let src0 = [1.4e7, 7.0e6, -1.3e7];
    let src_v = [30_000.0, -11_000.0, 2_500.0];
    let tt = |t: f64| t + eps * t.sin();
    let at = |t: f64| {
        let elapsed_tt = tt(t);
        let (p, _) = z(p0 + p_speed * elapsed_tt, p_speed);
        let (n, _) = x(n0 + n_speed * elapsed_tt, n_speed);
        let inertial = std::array::from_fn(|i| src0[i] + src_v[i] * (t - t0));
        mat_vec(n, mat_vec(p, inertial))
    };
    let factor = TtPerTdbDay::new(1.0 + eps * t0.cos()).unwrap();
    let (p, pdot) = z(p0 + p_speed * tt(t0), p_speed);
    let (n, ndot) = x(n0 + n_speed * tt(t0), n_speed);
    let frame = RotatingFrame::from_tt_precession_nutation(p, pdot, n, ndot, factor).unwrap();
    let output = frame.rotate_state(&state(src0, src_v)).unwrap();
    let before = at(t0 - h);
    let after = at(t0 + h);
    let at_current = at(t0);
    for i in 0..3 {
        near(output.position_km[i], at_current[i], 1e-7);
        near(
            output.velocity_km_per_day[i],
            (after[i] - before[i]) / (2.0 * h),
            5e-4,
        );
    }
    let source_range = project_fixed_axes(&output, 0.0).unwrap();
    let sph = |pos: [f64; 3]| {
        let state = state(pos, [0.0; 3]);
        project_fixed_axes(&state, 0.0).unwrap()
    };
    let before_sph = sph(before);
    let after_sph = sph(after);
    let delta_lon = (after_sph.longitude_rad - before_sph.longitude_rad + std::f64::consts::PI)
        .rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI;
    near(
        source_range.longitude_rate_rad_per_day,
        delta_lon / (2.0 * h),
        5e-10,
    );
    near(
        source_range.latitude_rate_rad_per_day,
        (after_sph.latitude_rad - before_sph.latitude_rad) / (2.0 * h),
        5e-10,
    );
    near(
        source_range.distance_rate_km_per_day,
        (after_sph.distance_km - before_sph.distance_km) / (2.0 * h),
        5e-4,
    );
}

#[test]
fn rejects_invalid_component_rotation_or_tt_rate_instead_of_masking_it() {
    let rate = TtPerTdbDay::new(1.0).unwrap();
    let invalid = [[1.0, 0.0, 0.0], [0.0, 1.01, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(
        RotatingFrame::from_tt_precession_nutation(
            invalid,
            [[0.0; 3]; 3],
            identity(),
            [[0.0; 3]; 3],
            rate,
        ),
        Err(RotatingFrameError::InvalidRotation)
    );
    assert_eq!(
        RotatingFrame::from_tt_precession_nutation(
            identity(),
            [[0.0; 3]; 3],
            invalid,
            [[0.0; 3]; 3],
            rate,
        ),
        Err(RotatingFrameError::InvalidRotation)
    );
    let bad_rate = [[0.01, 0.0, 0.0], [0.0; 3], [0.0; 3]];
    assert_eq!(
        RotatingFrame::from_tt_precession_nutation(
            identity(),
            bad_rate,
            identity(),
            [[0.0; 3]; 3],
            rate,
        ),
        Err(RotatingFrameError::InconsistentDerivative)
    );
    assert_eq!(
        RotatingFrame::from_tt_day_rate(identity(), [[f64::NAN; 3]; 3], rate),
        Err(RotatingFrameError::NonFinite)
    );
}

#[test]
fn tt_scale_bridge_preserves_a_station_and_retrograde_sign_change() {
    // Input source moves prograde 0.01 rad/TDB day, but the TT-based
    // frame rotates 0.02 rad/TT day. With nonunity dTT/dTDB
    // the physical moving-axis longitude rate is negative.
    let source = state([100_000.0, 0.0, 0.0], [0.0, 1_000.0, 0.0]);
    let (p, p_rate) = z(0.0, 0.02);
    let result = RotatingFrame::from_tt_precession_nutation(
        p,
        p_rate,
        identity(),
        [[0.0; 3]; 3],
        TtPerTdbDay::new(1.000_2).unwrap(),
    )
    .unwrap()
    .rotate_state(&source)
    .unwrap();
    let sky = project_fixed_axes(&result, 0.0).unwrap();
    near(sky.longitude_rate_rad_per_day, 0.01 - 0.02 * 1.000_2, 1e-15);
    near(sky.distance_rate_km_per_day, 0.0, 1e-14);
}
