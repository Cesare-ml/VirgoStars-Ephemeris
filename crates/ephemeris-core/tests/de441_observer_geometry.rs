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

//! Synthetic classic-DE441 Earth-center and independent ERFA spherical
//! reference tests. These are **not** real NASA/JPL kernel parity tests.
//! Original VirgoStars tests added 2026-11-12.

use std::f64::consts::{FRAC_PI_2, TAU};
use std::io::Cursor;

use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};
use virgostars_ephemeris_core::de441_observer_geometry::{
    de441_geometric_observer_sightline_icrf, De441IcrfObserverError,
};

const RECORD_BYTES: usize = 3000;

fn put_f64(bytes: &mut [u8], at: usize, value: f64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut [u8], at: usize, value: i32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

// One 32-day synthetic classic-DE441 record, plus mandatory header
// records. Moon geocentric is [100,-50,25] km, chosen to expose the
// separately published SOFA/ERFA t_p2s Cartesian/spherical test.
// Mercury and the Earth-Moon barycenter are unrelated synthetic series.
fn fixture() -> Vec<u8> {
    let mut bytes = vec![0u8; 3 * RECORD_BYTES];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    put_f64(&mut bytes, 2652, 1000.0);
    put_f64(&mut bytes, 2660, 1032.0);
    put_f64(&mut bytes, 2668, 32.0);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);

    // Moon [3..6), Mercury [6..9), EMB [9..12) and a harmless
    // last pointer [373..374) fixing the 3000-byte record width.
    for (index, first) in [(9, 3), (0, 6), (2, 9), (12, 373)] {
        let at = if index == 12 { 2844 } else { 2696 + index * 12 };
        put_i32(&mut bytes, at, first);
        put_i32(&mut bytes, at + 4, 1);
        put_i32(&mut bytes, at + 8, 1);
    }
    put_f64(&mut bytes, 2 * RECORD_BYTES, 1000.0);
    put_f64(&mut bytes, 2 * RECORD_BYTES + 8, 1032.0);
    for (i, value) in [100.0, -50.0, 25.0].into_iter().enumerate() {
        put_f64(&mut bytes, 2 * RECORD_BYTES + (2 + i) * 8, value);
    }
    for (i, value) in [2000.0, 3100.0, 5000.0].into_iter().enumerate() {
        put_f64(&mut bytes, 2 * RECORD_BYTES + (5 + i) * 8, value);
    }
    for (i, value) in [200.0, -100.0, 50.0].into_iter().enumerate() {
        put_f64(&mut bytes, 2 * RECORD_BYTES + (8 + i) * 8, value);
    }
    bytes
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, observed {actual} (tolerance {tolerance})"
    );
}

fn near_vec(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (a, b) in actual.into_iter().zip(expected) {
        near(a, b, tolerance);
    }
}

#[test]
fn synthetic_de441_lunar_vector_matches_independent_erfa_p2s_spherical_oracle() {
    // Published SOFA/ERFA t_p2s: p=[100,-50,25],
    // theta=-0.4636476090008061162 rad,
    // phi=0.2199879773954594463 rad,
    // range=114.5643923738960002 input units.
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    let jd = JulianDateTdb(1016.0);
    let sight = de441_geometric_observer_sightline_icrf(
        &mut eph,
        Body::Moon,
        jd,
        [0.0; 3],
    )
    .unwrap();
    assert_eq!(sight.target, Body::Moon);
    assert_eq!(sight.epoch_tdb, jd);
    near_vec(sight.target_geocentric_icrf_km, [100.0, -50.0, 25.0], 0.0);
    near_vec(sight.target_geocentric_velocity_icrf_km_per_tdb_day, [0.0; 3], 0.0);
    near_vec(sight.displacement_icrf_km, [100.0, -50.0, 25.0], 0.0);
    near(sight.range_km, 114.564_392_373_896, 1.0e-12);
    near(sight.declination_icrf_rad, 0.219_987_977_395_459_45, 1.0e-14);
    near(
        sight.right_ascension_icrf_rad.unwrap(),
        TAU - 0.463_647_609_000_806_1,
        1.0e-14,
    );
    let squared = sight.unit_direction_icrf.into_iter().map(|x| x * x).sum::<f64>();
    near(squared, 1.0, 5.0e-16);
}

#[test]
fn supplied_icrf_observer_changes_only_same_frame_displacement_and_direction() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    let sight = de441_geometric_observer_sightline_icrf(
        &mut eph,
        Body::Moon,
        JulianDateTdb(1016.0),
        [0.0, 10.0, 0.0],
    )
    .unwrap();
    near_vec(sight.observer_geocentric_icrf_km, [0.0, 10.0, 0.0], 0.0);
    near_vec(sight.displacement_icrf_km, [100.0, -60.0, 25.0], 0.0);
    near(sight.range_km, 100.0_f64.hypot(-60.0).hypot(25.0), 1.0e-14);
    near(
        sight.right_ascension_icrf_rad.unwrap(),
        (-60.0_f64).atan2(100.0).rem_euclid(TAU),
        1.0e-14,
    );
    near(
        sight.declination_icrf_rad,
        25.0_f64.atan2(100.0_f64.hypot(-60.0)),
        1.0e-14,
    );
}

#[test]
fn earth_center_relative_moon_works_without_emb_barycenter_series() {
    let mut data = fixture();
    for at in [2720, 2724, 2728] {
        put_i32(&mut data, at, 0);
    }
    let mut eph = De441::new(Cursor::new(data)).unwrap();
    let sight = de441_geometric_observer_sightline_icrf(
        &mut eph, Body::Moon, JulianDateTdb(1016.0), [0.0; 3],
    ).unwrap();
    near_vec(sight.displacement_icrf_km, [100.0, -50.0, 25.0], 0.0);
}

#[test]
fn mercury_earth_center_keeps_target_and_center_semantics() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    let epoch = JulianDateTdb(1016.0);
    let sight = de441_geometric_observer_sightline_icrf(
        &mut eph, Body::Mercury, epoch, [0.0; 3],
    ).unwrap();
    let expected = eph.state(Body::Mercury, Body::Earth, epoch).unwrap();
    near_vec(sight.target_geocentric_icrf_km, expected.position_km, 1.0e-12);
    near_vec(
        sight.target_geocentric_velocity_icrf_km_per_tdb_day,
        expected.velocity_km_per_day,
        0.0,
    );
    assert_eq!(sight.target, Body::Mercury);
}

#[test]
fn does_not_infer_an_ephemeris_for_unsupported_target_series() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    let result = de441_geometric_observer_sightline_icrf(
        &mut eph, Body::MarsBarycenter, JulianDateTdb(1016.0), [0.0; 3],
    );
    assert!(matches!(
        result,
        Err(De441IcrfObserverError::Ephemeris(De441Error::MissingSeries(_)))
    ));
}

#[test]
fn invalid_epoch_and_outside_coverage_fail_closed() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            de441_geometric_observer_sightline_icrf(
                &mut eph, Body::Moon, JulianDateTdb(bad), [0.0; 3]
            ),
            Err(De441IcrfObserverError::InvalidTdbEpoch)
        ));
    }
    assert!(matches!(
        de441_geometric_observer_sightline_icrf(
            &mut eph, Body::Moon, JulianDateTdb(1033.0), [0.0; 3]
        ),
        Err(De441IcrfObserverError::Ephemeris(
            De441Error::OutsideCoverage { .. }
        ))
    ));
}

#[test]
fn target_earth_and_bad_observer_are_explicitly_rejected() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    assert!(matches!(
        de441_geometric_observer_sightline_icrf(
            &mut eph, Body::Earth, JulianDateTdb(1016.0), [0.0; 3]
        ),
        Err(De441IcrfObserverError::EarthIsNotAValidTarget)
    ));
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            de441_geometric_observer_sightline_icrf(
                &mut eph, Body::Moon, JulianDateTdb(1016.0), [0.0, bad, 0.0]
            ),
            Err(De441IcrfObserverError::InvalidObserver)
        ));
    }
}

#[test]
fn exact_polar_direction_is_defined_but_right_ascension_is_not() {
    let mut data = fixture();
    for (i, val) in [0.0, 0.0, 25.0].into_iter().enumerate() {
        put_f64(&mut data, 2 * RECORD_BYTES + (2 + i) * 8, val);
    }
    let mut eph = De441::new(Cursor::new(data)).unwrap();
    let sight = de441_geometric_observer_sightline_icrf(
        &mut eph, Body::Moon, JulianDateTdb(1016.0), [0.0; 3],
    ).unwrap();
    assert_eq!(sight.right_ascension_icrf_rad, None);
    near(sight.declination_icrf_rad, FRAC_PI_2, 0.0);
    near_vec(sight.unit_direction_icrf, [0.0, 0.0, 1.0], 0.0);
}

#[test]
fn coincident_observer_and_overlarge_finite_observer_are_rejected() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    assert!(matches!(
        de441_geometric_observer_sightline_icrf(
            &mut eph, Body::Moon, JulianDateTdb(1016.0), [100.0, -50.0, 25.0]
        ),
        Err(De441IcrfObserverError::CoincidentTargetAndObserver)
    ));
    assert!(matches!(
        de441_geometric_observer_sightline_icrf(
            &mut eph, Body::Moon, JulianDateTdb(1016.0), [f64::MAX; 3]
        ),
        Err(De441IcrfObserverError::InvalidRange)
    ));
}
