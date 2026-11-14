// EXHIBIT A. Common Public Attribution License Version 1.0.
//
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

//! Added 2026-11-14: synthetic floating-point range regressions for geometric
//! ICRF fixed-axis longitude rates (km and km/day, TDB Julian Date).
//! No external JPL/Horizons kernel or independent science validation is implied.

use virgostars_ephemeris_core::coordinates::project_fixed_axes;
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};

fn sample(position_km: [f64; 3], velocity_km_per_day: [f64; 3]) -> StateVector {
    StateVector {
        target: Body::Moon,
        center: Body::Earth,
        epoch: JulianDateTdb(2_451_545.0),
        position_km,
        velocity_km_per_day,
    }
}

fn relative_near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        actual.is_finite() && (actual - expected).abs() <= expected.abs() * tolerance,
        "expected {expected:e}, got {actual:e}"
    );
}

#[test]
fn large_finite_velocity_yields_finite_representable_angular_rate() {
    // In the original expression (ux*vy - uy*vx)/planar, the numerator
    // overflows: -sqrt(2)*1.6e308 is not finite. The quotient IS finite.
    // Analytic result: (x*vy-y*vx)/(x²+y²) = -1.6e208 rad/day.
    let state = sample([1.0e100, 1.0e100, 0.0], [1.6e308, -1.6e308, 0.0]);
    let projected = project_fixed_axes(&state, 0.0).unwrap();
    relative_near(projected.longitude_rate_rad_per_day, -1.6e208, 1.0e-14);
    relative_near(projected.distance_km, 2.0_f64.sqrt() * 1.0e100, 1.0e-14);
    assert!(projected.distance_rate_km_per_day.abs() <= 1.0e293);
    assert_eq!(projected.latitude_rate_rad_per_day, 0.0);
    assert_eq!((projected.target, projected.center, projected.epoch),
               (state.target, state.center, state.epoch));
}

#[test]
fn angular_rate_agrees_on_both_sides_of_sub_km_threshold() {
    // v is transverse to (3,4,0), so radial distance rate is zero and
    // d(longitude)/dt=(3*6-4*(-8))/(3²+4²)=2 rad/day, regardless of scale.
    for scale in [1.0e-9, 1.0, 1.0e9] {
        let state = sample(
            [3.0 * scale, 4.0 * scale, 0.0],
            [-8.0 * scale, 6.0 * scale, 0.0],
        );
        let projected = project_fixed_axes(&state, 0.0).unwrap();
        assert!((projected.longitude_rate_rad_per_day - 2.0).abs() < 1.0e-14);
        assert!(projected.distance_rate_km_per_day.abs() < 1.0e-13 * scale);
        assert_eq!(projected.latitude_rate_rad_per_day, 0.0);
    }
}
