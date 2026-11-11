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

//! Original synthetic math checks of the **ecliptic-of-date rotation**
//! built from externally supplied equator P/N and true obliquity.
//! These do not evaluate IAU coefficients or validate Swiss positions.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use virgostars_ephemeris_core::coordinates::project_fixed_axes;
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};
use virgostars_ephemeris_core::rotating_frame::{
    RotatingFrame, RotatingFrameError, TtPerTdbDay,
};

type M = [[f64; 3]; 3];
const ZERO: M = [[0.0; 3]; 3];

fn identity() -> M {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

fn x(angle: f64, rate: f64) -> (M, M) {
    let (s, c) = angle.sin_cos();
    (
        [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]],
        [[0.0; 3], [0.0, -rate * s, rate * c], [0.0, -rate * c, -rate * s]],
    )
}

fn z(angle: f64, rate: f64) -> (M, M) {
    let (s, c) = angle.sin_cos();
    (
        [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]],
        [[-rate * s, rate * c, 0.0], [-rate * c, -rate * s, 0.0], [0.0; 3]],
    )
}

fn mul_vec(m: M, r: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| m[i][0] * r[0] + m[i][1] * r[1] + m[i][2] * r[2])
}

fn source(r: [f64; 3], v: [f64; 3]) -> StateVector {
    StateVector {
        target: Body::Mercury,
        center: Body::Earth,
        epoch: JulianDateTdb(2_451_545.0),
        position_km: r,
        velocity_km_per_day: v,
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e} expected={expected:.16e} tol={tolerance:.2e}"
    );
}

#[test]
fn positive_true_obliquity_tilts_equatorial_y_toward_negative_ecliptic_latitude() {
    let eps = 0.4_f64;
    let frame = RotatingFrame::from_tt_true_ecliptic(
        identity(), ZERO, identity(), ZERO, eps, 0.0, TtPerTdbDay::new(1.0).unwrap(),
    )
    .unwrap();
    let moved = frame.rotate_state(&source([0.0, 1.0, 0.0], [0.0; 3])).unwrap();
    near(moved.position_km[0], 0.0, 0.0);
    near(moved.position_km[1], eps.cos(), 1e-15);
    near(moved.position_km[2], -eps.sin(), 1e-15);
    let spherical = project_fixed_axes(&moved, 0.0).unwrap();
    near(spherical.longitude_rad, FRAC_PI_2, 1e-15);
    near(spherical.latitude_rad, -eps, 1e-15);
}

#[test]
fn evolving_true_obliquity_changes_latitude_rate_even_for_static_source() {
    let eps = 0.4_f64;
    let epsdot = 0.006_f64;
    let factor = 1.000_2_f64;
    let frame = RotatingFrame::from_tt_true_ecliptic(
        identity(), ZERO, identity(), ZERO, eps, epsdot, TtPerTdbDay::new(factor).unwrap(),
    )
    .unwrap();
    let moved = frame.rotate_state(&source([0.0, 1.0e7, 0.0], [0.0; 3])).unwrap();
    near(moved.velocity_km_per_day[1], -1.0e7 * eps.sin() * epsdot * factor, 1e-8);
    near(moved.velocity_km_per_day[2], -1.0e7 * eps.cos() * epsdot * factor, 1e-8);
    let sky = project_fixed_axes(&moved, 0.0).unwrap();
    near(sky.longitude_rate_rad_per_day, 0.0, 1e-15);
    near(sky.latitude_rate_rad_per_day, -epsdot * factor, 1e-15);
    near(sky.distance_rate_km_per_day, 0.0, 1e-8);
}

#[test]
fn noncommuting_product_is_ecliptic_after_nutation_after_precession() {
    // P maps +X to -Y; N maps -Y to +Z; positive E maps +Z
    // to +Y and +Z (not the result of reversing their order).
    let (p, _) = z(FRAC_PI_2, 0.0);
    let (n, _) = x(FRAC_PI_2, 0.0);
    let eps = 0.3_f64;
    let frame = RotatingFrame::from_tt_true_ecliptic(
        p, ZERO, n, ZERO, eps, 0.0, TtPerTdbDay::new(1.0).unwrap(),
    )
    .unwrap();
    let result = frame.rotate_state(&source([1.0, 0.0, 0.0], [0.0; 3])).unwrap();
    near(result.position_km[0], 0.0, 1e-15);
    near(result.position_km[1], eps.sin(), 1e-15);
    near(result.position_km[2], eps.cos(), 1e-15);
}

#[test]
fn triple_composition_matches_sequential_transform_of_positions_and_speeds() {
    let clock = TtPerTdbDay::new(1.000_05).unwrap();
    let (p, pdot) = z(-0.2, 0.025);
    let (n, ndot) = x(0.1, -0.014);
    let (e, edot) = x(0.4, 0.005);
    let composed = RotatingFrame::from_tt_true_ecliptic(
        p, pdot, n, ndot, 0.4, 0.005, clock,
    )
    .unwrap();
    let stage_p = RotatingFrame::from_tt_day_rate(p, pdot, clock).unwrap();
    let stage_n = RotatingFrame::from_tt_day_rate(n, ndot, clock).unwrap();
    let stage_e = RotatingFrame::from_tt_day_rate(e, edot, clock).unwrap();
    let input = source([2.5e7, -6.0e7, 1.5e7], [11_000.0, -5_000.0, 900.0]);
    let reference = stage_e
        .rotate_state(&stage_n.rotate_state(&stage_p.rotate_state(&input).unwrap()).unwrap())
        .unwrap();
    let result = composed.rotate_state(&input).unwrap();
    assert_eq!(result.target, input.target);
    assert_eq!(result.center, input.center);
    assert_eq!(result.epoch, input.epoch);
    for i in 0..3 {
        near(result.position_km[i], reference.position_km[i], 1e-7);
        near(result.velocity_km_per_day[i], reference.velocity_km_per_day[i], 1e-6);
    }
}

#[test]
fn independently_differentiated_three_time_varying_rotations_match_six_rates() {
    // Entirely synthetic clock relation, NOT an IAU time scale:
    // TT(t) = t + eps*sin(t). The clock factor is 1+eps*cos(t).
    let eps_clock = 0.000_2_f64;
    let t0 = 0.5_f64;
    let h = 0.000_1_f64;
    let p0 = 0.3_f64;
    let n0 = -0.15_f64;
    let e0 = 0.4_f64;
    let p_rate = 0.025_f64;
    let n_rate = -0.012_f64;
    let e_rate = 0.006_f64;
    let initial = [1.4e7, 7.0e6, -1.3e7];
    let initial_rate = [30_000.0, -11_000.0, 2_500.0];
    let tt = |t: f64| t + eps_clock * t.sin();
    let at = |t: f64| {
        let elapsed_tt = tt(t);
        let (p, _) = z(p0 + p_rate * elapsed_tt, p_rate);
        let (n, _) = x(n0 + n_rate * elapsed_tt, n_rate);
        let (e, _) = x(e0 + e_rate * elapsed_tt, e_rate);
        let inertial = std::array::from_fn(|i| initial[i] + initial_rate[i] * (t - t0));
        mul_vec(e, mul_vec(n, mul_vec(p, inertial)))
    };

    let (p, pdot) = z(p0 + p_rate * tt(t0), p_rate);
    let (n, ndot) = x(n0 + n_rate * tt(t0), n_rate);
    let frame = RotatingFrame::from_tt_true_ecliptic(
        p, pdot, n, ndot, e0 + e_rate * tt(t0), e_rate,
        TtPerTdbDay::new(1.0 + eps_clock * t0.cos()).unwrap(),
    )
    .unwrap();
    let result = frame.rotate_state(&source(initial, initial_rate)).unwrap();
    let before = at(t0 - h);
    let after = at(t0 + h);
    let now = at(t0);
    for i in 0..3 {
        near(result.position_km[i], now[i], 1e-7);
        near(result.velocity_km_per_day[i], (after[i] - before[i]) / (2.0 * h), 5e-4);
    }
    let projected = project_fixed_axes(&result, 0.0).unwrap();
    let sky = |position: [f64; 3]| {
        project_fixed_axes(&source(position, [0.0; 3]), 0.0).unwrap()
    };
    let left = sky(before);
    let right = sky(after);
    let lon_step = (right.longitude_rad - left.longitude_rad + PI).rem_euclid(TAU) - PI;
    near(projected.longitude_rate_rad_per_day, lon_step / (2.0 * h), 5e-10);
    near(
        projected.latitude_rate_rad_per_day,
        (right.latitude_rad - left.latitude_rad) / (2.0 * h),
        5e-10,
    );
    near(
        projected.distance_rate_km_per_day,
        (right.distance_km - left.distance_km) / (2.0 * h),
        5e-4,
    );
}

#[test]
fn true_obliquity_must_be_finite_physical_and_its_rate_finite() {
    let rate = TtPerTdbDay::new(1.0).unwrap();
    for invalid in [0.0, -0.1, FRAC_PI_2, PI, f64::NAN, f64::INFINITY] {
        assert_eq!(
            RotatingFrame::from_tt_true_ecliptic(
                identity(), ZERO, identity(), ZERO, invalid, 0.0, rate,
            ),
            Err(RotatingFrameError::InvalidTrueObliquity)
        );
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            RotatingFrame::from_tt_true_ecliptic(
                identity(), ZERO, identity(), ZERO, 0.4, invalid, rate,
            ),
            Err(RotatingFrameError::InvalidTrueObliquity)
        );
    }
}

#[test]
fn rejects_bad_precession_and_nutation_even_with_valid_ecliptic_tilt() {
    let rate = TtPerTdbDay::new(1.0).unwrap();
    let invalid = [[1.0, 0.0, 0.0], [0.0, 1.1, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(
        RotatingFrame::from_tt_true_ecliptic(
            invalid, ZERO, identity(), ZERO, 0.4, 0.0, rate,
        ),
        Err(RotatingFrameError::InvalidRotation)
    );
    assert_eq!(
        RotatingFrame::from_tt_true_ecliptic(
            identity(), ZERO, invalid, ZERO, 0.4, 0.0, rate,
        ),
        Err(RotatingFrameError::InvalidRotation)
    );
    let bad_rate = [[0.01, 0.0, 0.0], [0.0; 3], [0.0; 3]];
    assert_eq!(
        RotatingFrame::from_tt_true_ecliptic(
            identity(), bad_rate, identity(), ZERO, 0.4, 0.0, rate,
        ),
        Err(RotatingFrameError::InconsistentDerivative)
    );
}
