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
//! Contract tests for first-order solar light deflection.
//!
//! The analytic checks follow the closed form for a source at infinity:
//! deflection = (2GM/c²/E) · cot(ψ/2) at solar elongation ψ, directed away
//! from the Sun. The `#[ignore]` tests need a classic DE441 file in
//! `DE441_PATH` and assert only physical bounds, not third-party outputs.

use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    geocentric_apparent_six_fields_many_with_options, ApparentOptions,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::light_deflection::{
    solar_deflection_delta, DeflectionError, SUN_SCHWARZSCHILD_RADIUS_AU,
};

fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Unit vectors for an observer at 1 AU from the Sun and a source at
/// infinity (q = p) seen at solar elongation `psi`.
fn geometry(psi: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
    // e: Sun -> observer = +X, so the observer->Sun direction is -X.
    let e = [1.0, 0.0, 0.0];
    let p = [-psi.cos(), psi.sin(), 0.0];
    (p, p, e)
}

#[test]
fn matches_the_closed_form_cot_half_elongation() {
    for degrees in [10.0_f64, 30.0, 60.0, 90.0, 120.0, 170.0] {
        let psi = degrees.to_radians();
        let (p, q, e) = geometry(psi);
        let delta = solar_deflection_delta(p, q, e, 1.0).unwrap();
        let expected = SUN_SCHWARZSCHILD_RADIUS_AU / (psi / 2.0).tan();
        assert!(
            (norm(delta) - expected).abs() <= 1e-12 * expected,
            "{degrees} deg: {} vs {expected}",
            norm(delta)
        );
        // Perpendicular to the ray and pushing the source away from the Sun.
        assert!(dot(delta, p).abs() < 1e-18);
        let toward_sun = [-e[0], -e[1], -e[2]];
        assert!(dot(delta, toward_sun) < 0.0, "{degrees} deg");
    }
}

#[test]
fn ninety_degrees_at_one_au_is_four_milliarcseconds() {
    let (p, q, e) = geometry(std::f64::consts::FRAC_PI_2);
    let arcsec = norm(solar_deflection_delta(p, q, e, 1.0).unwrap()) * 206_264.806_247;
    assert!((arcsec - 0.004_072).abs() < 1.0e-5, "{arcsec}");
}

#[test]
fn deflection_scales_inversely_with_sun_observer_distance() {
    let (p, q, e) = geometry(1.0);
    let near = norm(solar_deflection_delta(p, q, e, 0.98).unwrap());
    let far = norm(solar_deflection_delta(p, q, e, 1.02).unwrap());
    assert!((near * 0.98 - far * 1.02).abs() < 1e-18);
}

#[test]
fn vanishes_for_sources_in_front_of_or_behind_the_ray_axis() {
    // Source on the observer's side of the Sun along the Sun-observer line.
    let e = [1.0, 0.0, 0.0];
    let in_front = solar_deflection_delta([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], e, 1.0).unwrap();
    assert!(norm(in_front) < 1e-30);
    // Exactly opposite the Sun: no bending (sin psi = 0).
    let opposite = solar_deflection_delta([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], e, 1.0).unwrap();
    assert!(norm(opposite) < 1e-30);
    // Source exactly behind the Sun: finite thanks to the guard, never NaN.
    let behind = solar_deflection_delta([-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], e, 1.0).unwrap();
    assert!(behind.iter().all(|x| x.is_finite()));
}

#[test]
fn rejects_non_positive_or_non_finite_sun_distance() {
    let (p, q, e) = geometry(1.0);
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            solar_deflection_delta(p, q, e, bad),
            Err(DeflectionError::InvalidGeometry)
        ));
    }
}

#[test]
#[ignore = "requires a legally obtained classic DE441 file in DE441_PATH"]
fn real_de441_deflection_is_bounded_and_sun_is_unchanged() {
    let path = std::env::var_os("DE441_PATH").expect("DE441_PATH is not set");
    let mut eph = De441::open(path).unwrap();
    let targets = [
        Body::Sun,
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::MarsBarycenter,
        Body::JupiterBarycenter,
        Body::SaturnBarycenter,
        Body::UranusBarycenter,
        Body::NeptuneBarycenter,
        Body::PlutoBarycenter,
    ];
    let off = ApparentOptions {
        solar_light_deflection: false,
    };
    let mut jd = 2_415_100.5;
    while jd < 2_488_000.0 {
        let on = geocentric_apparent_six_fields_many_with_options(
            &mut eph,
            &targets,
            JulianDateTdb(jd),
            ApparentOptions::default(),
        )
        .unwrap();
        let without = geocentric_apparent_six_fields_many_with_options(
            &mut eph,
            &targets,
            JulianDateTdb(jd),
            off,
        )
        .unwrap();
        for (i, target) in targets.iter().enumerate() {
            assert!(on[i].solar_deflection_applied && !without[i].solar_deflection_applied);
            let dlon = ((on[i].fields.longitude_deg - without[i].fields.longitude_deg + 540.0)
                .rem_euclid(360.0)
                - 180.0)
                * 3600.0;
            let dlat = (on[i].fields.latitude_deg - without[i].fields.latitude_deg) * 3600.0;
            let total = dlon.hypot(dlat);
            // Sun-limb deflection is 1.75 arcsec; 2 arcsec is a hard upper bound here.
            assert!(total < 2.0, "{target:?} jd {jd}: {total} arcsec");
            if matches!(target, Body::Sun | Body::Moon) {
                assert!(total < 1.0e-3, "{target:?} jd {jd}: {total} arcsec");
            }
            // Deflection changes direction only; the range is untouched.
            assert!(
                (on[i].fields.distance_au - without[i].fields.distance_au).abs() < 1e-12,
                "{target:?}"
            );
        }
        jd += 367.3;
    }
}
