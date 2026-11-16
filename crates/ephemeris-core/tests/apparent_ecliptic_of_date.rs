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

//! Contract tests for the geocentric apparent ecliptic-of-date stage.
//!
//! The non-ignored tests use synthetic geometry and verify mathematics only.
//! The `#[ignore]` tests read a caller-supplied classic DE441 file named by
//! the `DE441_PATH` environment variable; they check internal consistency
//! (rates against exact-step centered differences) and do NOT embed or
//! reproduce any third-party reference output.

use virgostars_ephemeris_core::aberration_state::{
    AberratedGeocentricState, ObserverAccelerationSource,
};
use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    apparent_ecliptic_of_date_from_aberrated_state, apparent_to_degrees_au,
    ecliptic_of_date_rotation_with_rate, geocentric_apparent_six_fields,
    geocentric_apparent_six_fields_many, tdb_to_tt_jd, ApparentError, MATRIX_RATE_HALF_STEP_DAYS,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

const J2000: f64 = 2_451_545.0;
const AU_KM: f64 = 149_597_870.7;

fn state(position_km: [f64; 3], velocity: [f64; 3], tdb: f64) -> AberratedGeocentricState {
    AberratedGeocentricState {
        target: Body::MarsBarycenter,
        observation_epoch: JulianDateTdb(tdb),
        emission_epoch: JulianDateTdb(tdb - 1.0e-3),
        light_time_seconds: 86.4,
        light_time_rate: 0.0,
        observer_acceleration_source: ObserverAccelerationSource::CallerProvided,
        position_km,
        velocity_km_per_tdb_day: velocity,
    }
}

fn det(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

#[test]
fn rotation_is_proper_orthogonal_and_rate_is_a_rotation_generator() {
    for tt in [2_415_100.5, J2000, 2_460_000.25, 2_488_000.0] {
        let (m, dm) = ecliptic_of_date_rotation_with_rate(tt).unwrap();
        assert!((det(m) - 1.0).abs() < 1e-14);
        // M * M^T = I and dM * M^T must be antisymmetric (so |v| is preserved).
        let mut skew_max: f64 = 0.0;
        for i in 0..3 {
            for j in 0..3 {
                let id: f64 = (0..3).map(|k| m[i][k] * m[j][k]).sum();
                assert!((id - if i == j { 1.0 } else { 0.0 }).abs() < 1e-14);
                let w: f64 = (0..3).map(|k| dm[i][k] * m[j][k]).sum();
                let wt: f64 = (0..3).map(|k| dm[j][k] * m[i][k]).sum();
                skew_max = skew_max.max((w + wt).abs());
            }
        }
        // The symmetric part is O(h^2) truncation, far below the signal (~1e-7/day).
        assert!(skew_max < 1e-11, "skew residual {skew_max}");
    }
}

#[test]
fn a_fixed_icrf_direction_drifts_at_the_precession_rate_only() {
    // A stationary ICRF point on the +X axis: ecliptic longitude grows by
    // general precession (about 50.29 arcsec/yr = 0.1377 arcsec/day) plus a
    // bounded nutation rate (< ~0.1 arcsec/day); latitude rate stays tiny.
    let r = 1.0e9_f64;
    let s = state([r, 0.0, 0.0], [0.0, 0.0, 0.0], J2000);
    let a = apparent_ecliptic_of_date_from_aberrated_state(&s, false).unwrap();
    let six = apparent_to_degrees_au(&a).unwrap();
    let lon_rate_arcsec_per_day = six.fields.longitude_speed_deg_per_tdb_day * 3600.0;
    assert!(
        (lon_rate_arcsec_per_day - 0.1377).abs() < 0.1,
        "{lon_rate_arcsec_per_day}"
    );
    // Radial speed of a fixed point is zero up to the O(h^2) symmetric part of dM/dt.
    assert!(six.fields.distance_speed_au_per_tdb_day.abs() < 1e-9);
    assert!((six.fields.distance_au - r / AU_KM).abs() < 1e-12);
    assert_eq!(six.fields.target, Body::MarsBarycenter);
    assert_eq!(six.fields.center, Body::Earth);
    // The ICRF +X axis sits within frame bias of the mean equinox, so its true
    // longitude is minus the nutation in longitude (about -14 arcsec at J2000;
    // |dpsi| never exceeds ~17.2 arcsec = 0.0048 degree).
    let lon = six.fields.longitude_deg;
    let signed = if lon > 180.0 { lon - 360.0 } else { lon };
    assert!(signed < 0.0 && signed > -0.0048, "{lon}");
}

#[test]
fn velocity_is_rotated_with_the_frame_not_merely_copied() {
    let moving = state(
        [1.5e8, 2.0e7, -1.0e6],
        [10.0e6, 2.0e6, 3.0e5],
        J2000 + 5000.25,
    );
    let a = apparent_ecliptic_of_date_from_aberrated_state(&moving, false).unwrap();
    let still = state([1.5e8, 2.0e7, -1.0e6], [0.0; 3], J2000 + 5000.25);
    let b = apparent_ecliptic_of_date_from_aberrated_state(&still, false).unwrap();
    let delta = a.spherical.distance_rate_km_per_day - b.spherical.distance_rate_km_per_day;
    let speed = (1.0e7_f64.powi(2) + 2.0e6_f64.powi(2) + 3.0e5_f64.powi(2)).sqrt();
    assert!(delta.abs() <= speed + 1.0, "{delta} {speed}");
    assert!(delta.abs() > 1.0e5, "velocity must contribute: {delta}");
}

#[test]
fn tdb_to_tt_is_bounded_by_the_periodic_term() {
    for k in 0..400 {
        let tdb = J2000 + k as f64 * 7.3;
        let diff_s = (tdb - tdb_to_tt_jd(tdb).unwrap()) * 86_400.0;
        assert!(diff_s.abs() <= 0.001_657_01, "{diff_s}");
    }
    assert!(matches!(
        tdb_to_tt_jd(f64::NAN),
        Err(ApparentError::InvalidIntermediate)
    ));
}

#[test]
fn rejects_earth_target_bad_metadata_and_unvalidated_epochs() {
    let mut earth = state([1.0e8, 0.0, 0.0], [0.0; 3], J2000);
    earth.target = Body::Earth;
    assert!(matches!(
        apparent_ecliptic_of_date_from_aberrated_state(&earth, false),
        Err(ApparentError::InvalidIntermediate)
    ));
    let mut late = state([1.0e8, 0.0, 0.0], [0.0; 3], J2000);
    late.emission_epoch = JulianDateTdb(J2000 + 1.0);
    assert!(matches!(
        apparent_ecliptic_of_date_from_aberrated_state(&late, false),
        Err(ApparentError::InvalidIntermediate)
    ));
    // IAU 2000B is only validated for 1900-2100 TT: fail closed outside it.
    for tdb in [2_415_000.0, 2_488_100.0, 1.0e6, 3.0e6] {
        let s = state([1.0e8, 0.0, 0.0], [0.0; 3], tdb);
        assert!(matches!(
            apparent_ecliptic_of_date_from_aberrated_state(&s, false),
            Err(ApparentError::Orientation(_))
        ));
    }
    let nan = state([f64::NAN, 0.0, 0.0], [0.0; 3], J2000);
    assert!(apparent_ecliptic_of_date_from_aberrated_state(&nan, false).is_err());
}

#[test]
fn half_step_constant_is_documented_value() {
    assert_eq!(MATRIX_RATE_HALF_STEP_DAYS, 0.01);
}

fn real_kernel() -> De441<std::fs::File> {
    let path = std::env::var_os("DE441_PATH").expect("DE441_PATH is not set");
    De441::open(path).expect("DE441_PATH must be a readable classic DE441 file")
}

fn wrap_deg(d: f64) -> f64 {
    (d + 540.0).rem_euclid(360.0) - 180.0
}

const ALL_TARGETS: [Body; 10] = [
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

/// Run with `DE441_PATH=<classic de441.eph> cargo test -- --ignored`.
#[test]
#[ignore = "requires a legally obtained classic DE441 file in DE441_PATH"]
fn real_de441_rates_match_exact_step_centered_differences() {
    let mut eph = real_kernel();
    // Powers of two make jd +/- h exact in binary floating point.
    let h = 1.0_f64 / 256.0;
    let mut jd = 2_415_100.5;
    while jd < 2_488_000.0 {
        let centre =
            geocentric_apparent_six_fields_many(&mut eph, &ALL_TARGETS, JulianDateTdb(jd)).unwrap();
        let plus =
            geocentric_apparent_six_fields_many(&mut eph, &ALL_TARGETS, JulianDateTdb(jd + h))
                .unwrap();
        let minus =
            geocentric_apparent_six_fields_many(&mut eph, &ALL_TARGETS, JulianDateTdb(jd - h))
                .unwrap();
        for (i, target) in ALL_TARGETS.iter().enumerate() {
            let c = &centre[i].fields;
            let lon =
                wrap_deg(plus[i].fields.longitude_deg - minus[i].fields.longitude_deg) / (2.0 * h);
            let lat = (plus[i].fields.latitude_deg - minus[i].fields.latitude_deg) / (2.0 * h);
            let dist = (plus[i].fields.distance_au - minus[i].fields.distance_au) / (2.0 * h);
            // Centered-difference truncation scales with h^2 times the third
            // derivative; the Moon's is the largest, so it gets a looser bound.
            let (lon_tol, lat_tol) = if *target == Body::Moon {
                (2.0e-4, 1.0e-4)
            } else {
                (2.0e-5, 2.0e-5)
            };
            assert!(
                (c.longitude_speed_deg_per_tdb_day - lon).abs() < lon_tol,
                "{target:?} jd {jd} lon rate {} vs {lon}",
                c.longitude_speed_deg_per_tdb_day
            );
            assert!(
                (c.latitude_speed_deg_per_tdb_day - lat).abs() < lat_tol,
                "{target:?} jd {jd} lat rate {} vs {lat}",
                c.latitude_speed_deg_per_tdb_day
            );
            assert!(
                (c.distance_speed_au_per_tdb_day - dist).abs() * AU_KM < 5.0,
                "{target:?} jd {jd} distance rate"
            );
        }
        jd += 1_234.5;
    }
}

#[test]
#[ignore = "requires a legally obtained classic DE441 file in DE441_PATH"]
fn real_de441_j2000_sun_and_moon_are_near_published_apparent_longitudes() {
    let mut eph = real_kernel();
    let sun = geocentric_apparent_six_fields(&mut eph, Body::Sun, JulianDateTdb(J2000)).unwrap();
    let moon = geocentric_apparent_six_fields(&mut eph, Body::Moon, JulianDateTdb(J2000)).unwrap();
    // Order-of-magnitude almanac values (~1e-3 degree), NOT a numerical
    // parity gate: Sun 280.368 deg, Moon 223.315 deg at J2000.0.
    assert!((sun.fields.longitude_deg - 280.368).abs() < 1.0e-3);
    assert!((moon.fields.longitude_deg - 223.315).abs() < 2.0e-3);
    assert!(sun.fields.distance_au > 0.9833 && sun.fields.distance_au < 0.9834);
    assert!(moon.light_time_seconds > 1.2 && moon.light_time_seconds < 1.4);
}

#[test]
#[ignore = "requires a legally obtained classic DE441 file in DE441_PATH"]
fn real_de441_moon_light_time_converges_across_1900_2100() {
    // Regression: the Moon's one-way delay (~1.3 s) used to alternate between
    // two adjacent representable JDs and report DidNotConverge.
    let mut eph = real_kernel();
    let mut jd = 2_415_100.5;
    while jd < 2_488_000.0 {
        geocentric_apparent_six_fields(&mut eph, Body::Moon, JulianDateTdb(jd)).unwrap();
        jd += 367.3;
    }
}
