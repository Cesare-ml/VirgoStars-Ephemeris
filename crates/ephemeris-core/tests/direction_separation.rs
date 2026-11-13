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

use std::f64::consts::{FRAC_PI_2, PI};

use virgostars_ephemeris_core::geometry::{
    geometric_angular_separation_rad, AngularSeparationError,
};

fn angle(a: [f64; 3], b: [f64; 3]) -> f64 {
    geometric_angular_separation_rad(a, b).unwrap()
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:?}, expected={expected:?}, tolerance={tolerance:?}"
    );
}

#[test]
fn aligned_orthogonal_and_opposed_directions() {
    near(angle([1.0, 0.0, 0.0], [3.0, 0.0, 0.0]), 0.0, 0.0);
    near(angle([1.0, 0.0, 0.0], [0.0, -1.0, 0.0]), FRAC_PI_2, 1e-15);
    near(angle([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]), PI, 1e-15);
}

#[test]
fn resolves_small_angles_near_conjunction_and_opposition() {
    let tiny = 1e-12;
    near(angle([1.0, 0.0, 0.0], [1.0, tiny, 0.0]), tiny, 1e-20);
    near(PI - angle([1.0, 0.0, 0.0], [-1.0, tiny, 0.0]), tiny, 1e-15);
}

#[test]
fn finite_extreme_scales_do_not_overflow_or_underflow() {
    near(
        angle([f64::MAX, 0.0, 0.0], [0.0, f64::MIN_POSITIVE, 0.0]),
        FRAC_PI_2,
        1e-15,
    );
    near(
        angle([f64::from_bits(1), 0.0, 0.0], [0.0, f64::from_bits(2), 0.0]),
        FRAC_PI_2,
        1e-15,
    );
    near(
        angle([f64::MAX, 0.0, 0.0], [-f64::MAX, 0.0, 0.0]),
        PI,
        1e-15,
    );
}

#[test]
fn symmetric_and_invariant_to_independent_positive_distance_scales() {
    let first = [3.0, -4.0, 5.0];
    let second = [-2.0, 8.0, 1.0];
    let reference = angle(first, second);
    near(reference, angle(second, first), 1e-15);
    near(
        reference,
        angle(first.map(|v| v * 1e300), second.map(|v| v * 1e-300)),
        1e-15,
    );
}

#[test]
fn rejects_zero_vectors_and_nonfinite_components_without_panicking() {
    assert_eq!(
        geometric_angular_separation_rad([0.0; 3], [1.0, 0.0, 0.0]),
        Err(AngularSeparationError::UndefinedDirection)
    );
    assert_eq!(
        geometric_angular_separation_rad([1.0, 0.0, 0.0], [-0.0; 3]),
        Err(AngularSeparationError::UndefinedDirection)
    );
    assert_eq!(
        geometric_angular_separation_rad([f64::NAN, 0.0, 0.0], [1.0, 0.0, 0.0]),
        Err(AngularSeparationError::NonFiniteInput)
    );
    assert_eq!(
        geometric_angular_separation_rad([1.0, 0.0, 0.0], [0.0, f64::INFINITY, 0.0]),
        Err(AngularSeparationError::NonFiniteInput)
    );
}
