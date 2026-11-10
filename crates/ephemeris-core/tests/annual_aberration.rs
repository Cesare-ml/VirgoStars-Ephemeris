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

use std::f64::consts::{FRAC_1_SQRT_2, PI};
use virgostars_ephemeris_core::aberration::{
    annual_aberration_icrf, AberrationError, SPEED_OF_LIGHT_KM_PER_DAY,
};

fn near(value: f64, reference: f64, error: f64) {
    assert!(
        (value - reference).abs() <= error,
        "actual={value:.16e}, reference={reference:.16e}, tolerance={error:.3e}"
    );
}

fn magnitude(vector: [f64; 3]) -> f64 {
    vector[0].hypot(vector[1]).hypot(vector[2])
}

#[test]
fn no_observer_motion_keeps_unit_source_direction() {
    let direction = annual_aberration_icrf([3.0, 4.0, 0.0], [0.0; 3]).unwrap();
    near(direction[0], 0.6, 1e-15);
    near(direction[1], 0.8, 1e-15);
    near(direction[2], 0.0, 1e-15);
    near(magnitude(direction), 1.0, 1e-15);
}

#[test]
fn transverse_lorentz_boost_matches_exact_one_dimensional_aberration() {
    // Analytical special-relativistic reference: for n=(0,1,0) and
    // observer velocity +beta in X, n'=(beta,sqrt(1-beta²),0).
    let speed = 0.1;
    let direction = annual_aberration_icrf(
        [0.0, 500_000_000.0, 0.0],
        [speed * SPEED_OF_LIGHT_KM_PER_DAY, 0.0, 0.0],
    )
    .unwrap();
    near(direction[0], speed, 1e-15);
    near(direction[1], (1.0_f64 - speed * speed).sqrt(), 1e-15);
    near(direction[2], 0.0, 1e-15);
    near(magnitude(direction), 1.0, 1e-15);
}

#[test]
fn oblique_lorentz_boost_matches_independent_angular_formula_and_inverts() {
    // Independent 1D Lorentz-boost relation (see docs/ANNUAL_ABERRATION.md):
    // cos(theta')=(cos(theta)+beta)/(1+beta*cos(theta)).
    // sin(theta')=sqrt(1-beta²)*sin(theta)/(1+beta*cos(theta)).
    let speed = 0.2;
    let original = [FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0];
    let observer_velocity = [speed * SPEED_OF_LIGHT_KM_PER_DAY, 0.0, 0.0];
    let transformed = annual_aberration_icrf(original, observer_velocity).unwrap();
    let denominator = 1.0 + speed * FRAC_1_SQRT_2;
    near(transformed[0], (FRAC_1_SQRT_2 + speed) / denominator, 1e-15);
    near(
        transformed[1],
        FRAC_1_SQRT_2 * (1.0 - speed * speed).sqrt() / denominator,
        1e-15,
    );

    // An equal and opposite observer boost restores the original direction.
    let restored = annual_aberration_icrf(transformed, [-observer_velocity[0], 0.0, 0.0]).unwrap();
    for axis in 0..3 {
        near(restored[axis], original[axis], 1e-15);
    }
}

#[test]
fn observer_motion_parallel_to_source_has_no_deflection() {
    let speed = 0.3 * SPEED_OF_LIGHT_KM_PER_DAY;
    for source in [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]] {
        let actual = annual_aberration_icrf(source, [speed, 0.0, 0.0]).unwrap();
        for axis in 0..3 {
            near(actual[axis], source[axis], 1e-15);
        }
    }
}

#[test]
fn orbital_speed_yields_expected_aberration_scale_without_calendar_or_jpl() {
    // Independent check on the scale: a 29.78 km/s transverse Earth speed
    // produces asin(v/c) ~= 20.49 arcseconds (special relativity).
    const KM_PER_SECOND: f64 = 29.78;
    let direction =
        annual_aberration_icrf([0.0, 1.0, 0.0], [KM_PER_SECOND * 86_400.0, 0.0, 0.0]).unwrap();
    let shift_arcseconds = direction[0].atan2(direction[1]) * 180.0 / PI * 3600.0;
    near(shift_arcseconds, 20.489_394_500_179_557, 1e-9);
}

#[test]
fn arbitrary_nonzero_range_does_not_change_an_aberrated_direction() {
    let velocity = [0.0, 24.0 * 86_400.0, 15.0 * 86_400.0];
    let near_source = annual_aberration_icrf([4.0, 5.0, -2.0], velocity).unwrap();
    let far_source = annual_aberration_icrf([4.0e9, 5.0e9, -2.0e9], velocity).unwrap();
    for axis in 0..3 {
        near(near_source[axis], far_source[axis], 1e-15);
    }
    near(magnitude(near_source), 1.0, 1e-15);
}

#[test]
fn high_but_sub_luminal_speed_stays_finite_and_normalized() {
    let observer_velocity = [0.999_99 * SPEED_OF_LIGHT_KM_PER_DAY, 0.0, 0.0];
    let result = annual_aberration_icrf([0.0, 1.0, 0.0], observer_velocity).unwrap();
    near(result[0], 0.999_99, 1e-13);
    near(magnitude(result), 1.0, 1e-14);
}

#[test]
fn rejects_undefined_or_nonfinite_source_or_velocity() {
    assert_eq!(
        annual_aberration_icrf([0.0; 3], [0.0; 3]),
        Err(AberrationError::UndefinedDirection),
    );
    assert_eq!(
        annual_aberration_icrf([f64::NAN, 0.0, 0.0], [0.0; 3]),
        Err(AberrationError::NonFiniteInput),
    );
    assert_eq!(
        annual_aberration_icrf([1.0, 0.0, 0.0], [0.0, f64::INFINITY, 0.0]),
        Err(AberrationError::NonFiniteInput),
    );
    assert_eq!(
        annual_aberration_icrf([f64::MAX, f64::MAX, 0.0], [0.0; 3]),
        Err(AberrationError::ArithmeticOverflow),
    );
}

#[test]
fn rejects_speed_at_or_above_c_instead_of_silent_fallback() {
    for speed in [SPEED_OF_LIGHT_KM_PER_DAY, 1.01 * SPEED_OF_LIGHT_KM_PER_DAY] {
        assert_eq!(
            annual_aberration_icrf([0.0, 1.0, 0.0], [speed, 0.0, 0.0]),
            Err(AberrationError::SuperluminalObserver),
        );
    }
    let component = SPEED_OF_LIGHT_KM_PER_DAY * 0.8;
    assert_eq!(
        annual_aberration_icrf([0.0, 1.0, 0.0], [component, component, 0.0]),
        Err(AberrationError::SuperluminalObserver),
    );
}
