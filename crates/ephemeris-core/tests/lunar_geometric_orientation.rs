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

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};
use virgostars_ephemeris_core::geometry::AngularSeparationError;
use virgostars_ephemeris_core::lunar_phase::{
    geometric_lunar_orientation, LunarEclipticHalf, LunarIlluminationError,
};

fn state(target: Body, center: Body, epoch: f64, position_km: [f64; 3]) -> StateVector {
    StateVector {
        target,
        center,
        epoch: JulianDateTdb(epoch),
        position_km,
        velocity_km_per_day: [0.0; 3],
    }
}

fn pair(sun_position: [f64; 3], moon_position: [f64; 3]) -> (StateVector, StateVector) {
    (
        state(Body::Sun, Body::Earth, 2_451_545.0, sun_position),
        state(Body::Moon, Body::Earth, 2_451_545.0, moon_position),
    )
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:?}, expected={expected:?}, tolerance={tolerance:?}"
    );
}

#[test]
fn quarter_triangle_distinguishes_phase_angle_elongation_and_waxing_half() {
    let (sun, moon) = pair([1.0, 1.0, 0.0], [1.0, 0.0, 0.0]);
    let result = geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]).unwrap();
    assert_eq!(result.illumination.epoch, sun.epoch);
    close(result.illumination.phase_angle_rad, FRAC_PI_2, 1e-15);
    close(result.illumination.illuminated_fraction, 0.5, 1e-15);
    close(result.geocentric_elongation_rad, FRAC_PI_4, 1e-15);
    close(
        result.signed_ecliptic_longitude_difference_rad,
        -FRAC_PI_4,
        1e-15,
    );
    assert_eq!(result.longitude_half, LunarEclipticHalf::WaningLongitudeHalf);

    let (sun, moon) = pair([1.0, -1.0, 0.0], [1.0, 0.0, 0.0]);
    let mirror = geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]).unwrap();
    close(
        mirror.signed_ecliptic_longitude_difference_rad,
        FRAC_PI_4,
        1e-15,
    );
    assert_eq!(mirror.longitude_half, LunarEclipticHalf::WaxingLongitudeHalf);
    close(mirror.illumination.illuminated_fraction, 0.5, 1e-15);
}

#[test]
fn exact_alignment_and_opposition_have_no_waxing_or_waning_half() {
    let (sun, moon) = pair([2.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    let new = geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]).unwrap();
    assert_eq!(new.longitude_half, LunarEclipticHalf::ConjunctionOrOpposition);
    close(new.signed_ecliptic_longitude_difference_rad, 0.0, 0.0);
    close(new.geocentric_elongation_rad, 0.0, 0.0);
    close(new.illumination.illuminated_fraction, 0.0, 0.0);

    let (sun, moon) = pair([2.0, 0.0, 0.0], [-1.0, 0.0, 0.0]);
    let full = geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]).unwrap();
    assert_eq!(full.longitude_half, LunarEclipticHalf::ConjunctionOrOpposition);
    close(full.signed_ecliptic_longitude_difference_rad, PI, 0.0);
    close(full.geocentric_elongation_rad, PI, 0.0);
    close(full.illumination.illuminated_fraction, 1.0, 0.0);
}

#[test]
fn three_dimensional_elongation_differs_from_projected_longitude_difference() {
    let (sun, moon) = pair([1.0, 0.0, 2.0], [1.0, 1.0, 0.0]);
    let result = geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]).unwrap();
    close(
        result.signed_ecliptic_longitude_difference_rad,
        FRAC_PI_4,
        1e-15,
    );
    assert!(
        (result.geocentric_elongation_rad - FRAC_PI_4).abs() > 0.1,
        "3-D elongation must preserve latitude rather than use projected longitude"
    );
    assert_eq!(result.longitude_half, LunarEclipticHalf::WaxingLongitudeHalf);
}

#[test]
fn reference_pole_positive_scale_and_orientation_have_explicit_semantics() {
    let (sun, moon) = pair([2.0, 0.0, 0.0], [1.0, 1.0, 0.0]);
    let reference = geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]).unwrap();
    for length in [f64::MIN_POSITIVE, f64::MAX] {
        let scaled = geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, length]).unwrap();
        assert_eq!(scaled, reference);
    }
    assert_eq!(reference.longitude_half, LunarEclipticHalf::WaxingLongitudeHalf);
    let reversed = geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, -1.0]).unwrap();
    assert_eq!(reversed.longitude_half, LunarEclipticHalf::WaningLongitudeHalf);
    close(
        reversed.signed_ecliptic_longitude_difference_rad,
        -reference.signed_ecliptic_longitude_difference_rad,
        1e-15,
    );
    close(reversed.geocentric_elongation_rad, reference.geocentric_elongation_rad, 0.0);
}

#[test]
fn signed_angle_retains_side_near_opposition_without_naming_phase_event() {
    let (sun, moon) = pair([2.0, 0.0, 0.0], [-1.0, 1e-9, 0.0]);
    let before = geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]).unwrap();
    assert_eq!(before.longitude_half, LunarEclipticHalf::WaxingLongitudeHalf);
    assert!(before.signed_ecliptic_longitude_difference_rad > PI - 1e-8);

    let (sun, moon) = pair([2.0, 0.0, 0.0], [-1.0, -1e-9, 0.0]);
    let after = geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]).unwrap();
    assert_eq!(after.longitude_half, LunarEclipticHalf::WaningLongitudeHalf);
    assert!(after.signed_ecliptic_longitude_difference_rad < -PI + 1e-8);
}

#[test]
fn rejects_missing_nonfinite_reference_pole_and_undefined_projection() {
    let (sun, moon) = pair([2.0, 0.0, 0.0], [1.0, 1.0, 0.0]);
    assert_eq!(
        geometric_lunar_orientation(&sun, &moon, [0.0; 3]),
        Err(LunarIlluminationError::UndefinedReferencePole)
    );
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            geometric_lunar_orientation(&sun, &moon, [invalid, 0.0, 1.0]),
            Err(LunarIlluminationError::NonFiniteInput)
        );
    }

    let (sun, moon) = pair([0.0, 0.0, 2.0], [1.0, 0.0, 0.0]);
    assert_eq!(
        geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]),
        Err(LunarIlluminationError::UndefinedEclipticProjection)
    );
    let (sun, moon) = pair([2.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
    assert_eq!(
        geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]),
        Err(LunarIlluminationError::UndefinedEclipticProjection)
    );
}

#[test]
fn same_state_validation_errors_remain_shared_with_illumination() {
    let (mut sun, mut moon) = pair([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    sun.center = Body::SolarSystemBarycenter;
    assert_eq!(
        geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]),
        Err(LunarIlluminationError::WrongTargetOrCenter)
    );
    sun.center = Body::Earth;
    moon.epoch = JulianDateTdb(sun.epoch.0 + 1.0);
    assert_eq!(
        geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]),
        Err(LunarIlluminationError::MismatchedEpoch)
    );
    moon.epoch = sun.epoch;
    moon.position_km = sun.position_km;
    assert_eq!(
        geometric_lunar_orientation(&sun, &moon, [0.0, 0.0, 1.0]),
        Err(LunarIlluminationError::Geometry(
            AngularSeparationError::UndefinedDirection
        ))
    );
}
