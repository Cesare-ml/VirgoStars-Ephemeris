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

//! Heliocentric and equatorial output of the apparent pipeline.
//! The `#[ignore]` tests need a classic DE441 file in `DE441_PATH`.

use virgostars_ephemeris_core::aberration_state::{
    AberratedGeocentricState, ObserverAccelerationSource,
};
use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    apparent_from_aberrated_state_in_frame, applied_corrections,
    geocentric_apparent_six_fields_many_with_options, not_applied_corrections, ApparentCenter,
    ApparentError, ApparentFrame, ApparentOptions,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::earth_orientation::{
    ecliptic_nutation, matrix_vector_multiply, true_ecliptic_of_date_matrix,
};

const J2000: f64 = 2_451_545.0;

fn wrap(d: f64) -> f64 {
    (d + 540.0).rem_euclid(360.0) - 180.0
}

#[test]
fn defaults_are_the_geocentric_ecliptic_set_and_helio_lists_its_corrections() {
    let d = ApparentOptions::default();
    assert!(d.solar_light_deflection);
    assert_eq!(d.frame, ApparentFrame::TrueEclipticOfDate);
    assert_eq!(d.center, ApparentCenter::Geocentric);
    let helio = ApparentOptions {
        center: ApparentCenter::Heliocentric,
        frame: ApparentFrame::TrueEquatorOfDate,
        ..d
    };
    let applied = applied_corrections(helio);
    assert!(applied
        .iter()
        .any(|c| c.contains("heliocentric light-time")));
    assert!(!applied
        .iter()
        .any(|c| c.contains("aberration") || c.contains("deflection")));
    assert!(applied.iter().any(|c| c.contains("true equator of date")));
    assert!(not_applied_corrections(helio)
        .iter()
        .any(|c| c.contains("not defined for a Sun-centered")));
}

#[test]
fn the_equatorial_frame_is_the_ecliptic_frame_rotated_by_the_true_obliquity() {
    let tdb = J2000 + 5000.25;
    let state = AberratedGeocentricState {
        target: Body::MarsBarycenter,
        observation_epoch: JulianDateTdb(tdb),
        emission_epoch: JulianDateTdb(tdb - 1.0e-3),
        light_time_seconds: 86.4,
        light_time_rate: 0.0,
        observer_acceleration_source: ObserverAccelerationSource::CallerProvided,
        position_km: [1.5e8, 4.0e7, -3.0e6],
        velocity_km_per_tdb_day: [1.0e6, -2.0e6, 3.0e5],
    };
    let ecl =
        apparent_from_aberrated_state_in_frame(&state, ApparentFrame::TrueEclipticOfDate, false)
            .unwrap();
    let equ =
        apparent_from_aberrated_state_in_frame(&state, ApparentFrame::TrueEquatorOfDate, false)
            .unwrap();
    assert_eq!(ecl.frame, ApparentFrame::TrueEclipticOfDate);
    assert_eq!(equ.frame, ApparentFrame::TrueEquatorOfDate);
    let eps = ecliptic_nutation(ecl.tt_jd).unwrap().true_obliquity_rad;
    let (lon, lat) = (ecl.spherical.longitude_rad, ecl.spherical.latitude_rad);
    let x = lat.cos() * lon.cos();
    let y = lat.cos() * lon.sin() * eps.cos() - lat.sin() * eps.sin();
    let z = lat.cos() * lon.sin() * eps.sin() + lat.sin() * eps.cos();
    let ra = y.atan2(x).rem_euclid(std::f64::consts::TAU);
    let dec = z.atan2(x.hypot(y));
    assert!(
        (ra - equ.spherical.longitude_rad).abs() < 1e-12,
        "{ra} vs {}",
        equ.spherical.longitude_rad
    );
    assert!((dec - equ.spherical.latitude_rad).abs() < 1e-12);
    assert!((ecl.spherical.distance_km - equ.spherical.distance_km).abs() < 1e-3);
}

fn kernel() -> De441<std::fs::File> {
    De441::open(std::env::var_os("DE441_PATH").expect("DE441_PATH is not set")).unwrap()
}

#[test]
#[ignore = "requires a legally obtained classic DE441 file in DE441_PATH"]
fn real_de441_heliocentric_position_follows_the_stated_light_time_definition() {
    let mut eph = kernel();
    let c = 299_792.458;
    let ssb = Body::SolarSystemBarycenter;
    for tdb in [2_415_100.5, 2_440_000.25, J2000, 2_459_000.125] {
        for target in [
            Body::Venus,
            Body::MarsBarycenter,
            Body::JupiterBarycenter,
            Body::Earth,
            Body::Moon,
        ] {
            let sun = eph
                .state(Body::Sun, ssb, JulianDateTdb(tdb))
                .unwrap()
                .position_km;
            // The photon leaves the target at te and arrives at the Sun's centre at tdb.
            let mut te = tdb;
            for _ in 0..30 {
                let p = eph
                    .state(target, ssb, JulianDateTdb(te))
                    .unwrap()
                    .position_km;
                let range =
                    ((p[0] - sun[0]).powi(2) + (p[1] - sun[1]).powi(2) + (p[2] - sun[2]).powi(2))
                        .sqrt();
                te = tdb - range / c / 86_400.0;
            }
            let p = eph
                .state(target, ssb, JulianDateTdb(te))
                .unwrap()
                .position_km;
            let v = [p[0] - sun[0], p[1] - sun[1], p[2] - sun[2]];
            let tt = tdb; // the matrix is evaluated at TT; the difference is < 2 ms
            let m = true_ecliptic_of_date_matrix(tt).unwrap();
            let r = matrix_vector_multiply(m, v);
            let lon = r[1].atan2(r[0]).to_degrees().rem_euclid(360.0);
            let lat = r[2].atan2(r[0].hypot(r[1])).to_degrees();
            let got = geocentric_apparent_six_fields_many_with_options(
                &mut eph,
                &[target],
                JulianDateTdb(tdb),
                ApparentOptions {
                    center: ApparentCenter::Heliocentric,
                    ..ApparentOptions::default()
                },
            )
            .unwrap()[0]
                .fields;
            assert_eq!(got.center, Body::Sun);
            // TT versus TDB changes the matrix by < 1e-8 degree.
            assert!(
                wrap(got.longitude_deg - lon).abs() < 1e-7,
                "{target:?} {tdb}: {} vs {lon}",
                got.longitude_deg
            );
            assert!((got.latitude_deg - lat).abs() < 1e-7, "{target:?} {tdb}");
        }
    }
}

#[test]
#[ignore = "requires a legally obtained classic DE441 file in DE441_PATH"]
fn real_de441_heliocentric_and_equatorial_rates_match_exact_step_centered_differences() {
    let mut eph = kernel();
    let h = 1.0_f64 / 256.0;
    let targets = [
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::MarsBarycenter,
        Body::JupiterBarycenter,
        Body::PlutoBarycenter,
        Body::Earth,
    ];
    for (center, frame) in [
        (
            ApparentCenter::Heliocentric,
            ApparentFrame::TrueEclipticOfDate,
        ),
        (
            ApparentCenter::Heliocentric,
            ApparentFrame::TrueEquatorOfDate,
        ),
        (ApparentCenter::Geocentric, ApparentFrame::TrueEquatorOfDate),
    ] {
        let options = ApparentOptions {
            center,
            frame,
            ..ApparentOptions::default()
        };
        let list: Vec<Body> = if center == ApparentCenter::Geocentric {
            targets
                .iter()
                .copied()
                .filter(|b| *b != Body::Earth)
                .collect()
        } else {
            targets.to_vec()
        };
        let mut jd = 2_415_100.5;
        while jd < 2_487_000.0 {
            let c = geocentric_apparent_six_fields_many_with_options(
                &mut eph,
                &list,
                JulianDateTdb(jd),
                options,
            )
            .unwrap();
            let p = geocentric_apparent_six_fields_many_with_options(
                &mut eph,
                &list,
                JulianDateTdb(jd + h),
                options,
            )
            .unwrap();
            let m = geocentric_apparent_six_fields_many_with_options(
                &mut eph,
                &list,
                JulianDateTdb(jd - h),
                options,
            )
            .unwrap();
            for (i, target) in list.iter().enumerate() {
                let lon = wrap(p[i].fields.longitude_deg - m[i].fields.longitude_deg) / (2.0 * h);
                let lat = (p[i].fields.latitude_deg - m[i].fields.latitude_deg) / (2.0 * h);
                let tol = if *target == Body::Moon {
                    (3.0e-4, 2.0e-4)
                } else {
                    (2.0e-5, 2.0e-5)
                };
                assert!(
                    (c[i].fields.longitude_speed_deg_per_tdb_day - lon).abs() < tol.0,
                    "{center:?} {frame:?} {target:?} {jd}"
                );
                assert!(
                    (c[i].fields.latitude_speed_deg_per_tdb_day - lat).abs() < tol.1,
                    "{center:?} {frame:?} {target:?} {jd}"
                );
            }
            jd += 2_345.6;
        }
    }
}

#[test]
#[ignore = "requires a legally obtained classic DE441 file in DE441_PATH"]
fn real_de441_rejects_the_sun_from_the_sun_and_the_geocentric_earth() {
    let mut eph = kernel();
    let helio = ApparentOptions {
        center: ApparentCenter::Heliocentric,
        ..ApparentOptions::default()
    };
    assert!(matches!(
        geocentric_apparent_six_fields_many_with_options(
            &mut eph,
            &[Body::Sun],
            JulianDateTdb(J2000),
            helio
        ),
        Err(ApparentError::Aberration(_))
    ));
    assert!(geocentric_apparent_six_fields_many_with_options(
        &mut eph,
        &[Body::Earth],
        JulianDateTdb(J2000),
        ApparentOptions::default()
    )
    .is_err());
    assert!(geocentric_apparent_six_fields_many_with_options(
        &mut eph,
        &[Body::Earth],
        JulianDateTdb(J2000),
        helio
    )
    .is_ok());
}
