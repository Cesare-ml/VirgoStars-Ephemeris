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

//! Synthetic civil→TDB→DE441→light-time→annual-aberration batch tests.
//! These fixtures are invented coefficients, NOT real NASA/JPL vectors.

use std::io::Cursor;

use virgostars_ephemeris_core::aberrated_spherical::{
    geocentric_aberrated_fixed_six_fields_many, AberratedSphericalError,
};
use virgostars_ephemeris_core::aberration_state::{
    AberratedStateError, ObserverAccelerationSource,
};
use virgostars_ephemeris_core::civil_aberrated::{
    geocentric_aberrated_fixed_six_fields_many_from_civil, CivilAberratedError,
};
use virgostars_ephemeris_core::coordinates::ProjectionError;
use virgostars_ephemeris_core::de441::{Body, De441, De441Error};
use virgostars_ephemeris_core::first_sky::{CivilTimestamp, SkyError};
use virgostars_ephemeris_core::light_time::LightTimeError;

const RECORD_SIZE: usize = 3_000;
const FIRST_JD: f64 = 2_461_344.0;
const LAST_JD: f64 = 2_461_376.0;

fn put_f64(bytes: &mut [u8], at: usize, value: f64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut [u8], at: usize, value: i32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn series(bytes: &mut [u8], first_word: usize, coords: [[f64; 3]; 3]) {
    for (axis, coeff) in coords.into_iter().enumerate() {
        for (term, value) in coeff.into_iter().enumerate() {
            let word = first_word - 1 + axis * 3 + term;
            put_f64(bytes, RECORD_SIZE * 2 + word * 8, value);
        }
    }
}

fn fictional_de441() -> De441<Cursor<Vec<u8>>> {
    let mut bytes = vec![0_u8; RECORD_SIZE * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    put_f64(&mut bytes, 2652, FIRST_JD);
    put_f64(&mut bytes, 2660, LAST_JD);
    put_f64(&mut bytes, 2668, 32.0);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);
    for (index, first, coeff) in [
        (0usize, 3, 3),
        (2, 12, 3),
        (9, 21, 3),
        (12, 373, 1),
    ] {
        let at = if index == 12 { 2844 } else { 2696 + index * 12 };
        put_i32(&mut bytes, at, first);
        put_i32(&mut bytes, at + 4, coeff);
        put_i32(&mut bytes, at + 8, 1);
    }
    put_f64(&mut bytes, RECORD_SIZE * 2, FIRST_JD);
    put_f64(&mut bytes, RECORD_SIZE * 2 + 8, LAST_JD);

    // Three-dimensional quadratic Mercury/EMB/Moon data; Earth is
    // reconstructed through the mass-weighted EMB–geocentric Moon chain.
    // c1 = 16 * km/TDB-day velocity at the record midpoint, c2 causes
    // nonzero analytically differentiated observer acceleration.
    series(
        &mut bytes,
        3,
        [
            [200_000_000.0, 16.0 * 90_000.0, 10_000.0],
            [70_000_000.0, 16.0 * -1_300_000.0, -30_000.0],
            [15_000_000.0, 16.0 * 250_000.0, 5_000.0],
        ],
    );
    series(
        &mut bytes,
        12,
        [
            [1_000_000.0, 16.0 * 50_000.0, 5_000.0],
            [300_000.0, 16.0 * 1_100_000.0, 200_000.0],
            [0.0, 16.0 * 200_000.0, 1_000.0],
        ],
    );
    series(
        &mut bytes,
        21,
        [
            [10_000.0, 16.0 * 50.0, 20.0],
            [20_000.0, 16.0 * 150.0, 10.0],
            [1_000.0, 16.0 * 10.0, 1.0],
        ],
    );
    De441::new(Cursor::new(bytes)).unwrap()
}

fn timestamp(value: &str) -> CivilTimestamp {
    CivilTimestamp::parse(value).unwrap()
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e}, expected={expected:.16e}, tol={tolerance:.2e}"
    );
}

#[test]
fn civil_offsets_produce_same_complete_aberrated_multi_body_results() {
    let mut eph = fictional_de441();
    let targets = [Body::Moon, Body::Mercury, Body::Moon];
    let local = geocentric_aberrated_fixed_six_fields_many_from_civil(
        &mut eph,
        &targets,
        timestamp("2026-11-10T13:30:00+01:00"),
        0.0,
    )
    .unwrap();
    let utc = geocentric_aberrated_fixed_six_fields_many_from_civil(
        &mut eph,
        &targets,
        timestamp("2026-11-10T12:30:00Z"),
        0.0,
    )
    .unwrap();
    assert_eq!(local, utc);
    assert_eq!(local.results.len(), 3);
    assert_eq!(local.results[0], local.results[2]);

    let direct =
        geocentric_aberrated_fixed_six_fields_many(&mut eph, &targets, utc.reception_epoch, 0.0)
            .unwrap();
    assert_eq!(utc.results, direct);
    for (result, target) in utc.results.iter().zip(targets) {
        assert_eq!(result.fields.target, target);
        assert_eq!(result.fields.center, Body::Earth);
        assert_eq!(result.fields.epoch, utc.reception_epoch);
        assert!(result.emission_epoch.0 < utc.reception_epoch.0);
        assert_eq!(
            result.observer_acceleration_source,
            ObserverAccelerationSource::AnalyticDe441Chebyshev
        );
        assert!(result.fields.distance_au > 0.0);
    }
}

#[test]
fn civil_aberration_longitude_latitude_and_range_rates_match_time_differences() {
    let mut eph = fictional_de441();
    let targets = [Body::Mercury, Body::Moon];
    let sample = |reader: &mut De441<Cursor<Vec<u8>>>, label: &str| {
        geocentric_aberrated_fixed_six_fields_many_from_civil(
            reader,
            &targets,
            timestamp(label),
            0.0,
        )
        .unwrap()
    };
    let a = sample(&mut eph, "2026-11-10T12:28:00Z");
    let m = sample(&mut eph, "2026-11-10T12:30:00Z");
    let b = sample(&mut eph, "2026-11-10T12:32:00Z");
    let dt = b.reception_epoch.0 - a.reception_epoch.0;
    assert!(dt > 0.0);
    for index in 0..targets.len() {
        let before = &a.results[index].fields;
        let middle = &m.results[index].fields;
        let after = &b.results[index].fields;
        let signed_degrees =
            (after.longitude_deg - before.longitude_deg + 540.0).rem_euclid(360.0) - 180.0;
        near(
            middle.longitude_speed_deg_per_tdb_day,
            signed_degrees / dt,
            1.0e-4,
        );
        near(
            middle.latitude_speed_deg_per_tdb_day,
            (after.latitude_deg - before.latitude_deg) / dt,
            1.0e-4,
        );
        near(
            middle.distance_speed_au_per_tdb_day,
            (after.distance_au - before.distance_au) / dt,
            1.0e-6,
        );
    }
    assert!(m.results[0].fields.longitude_speed_deg_per_tdb_day < 0.0);
}

#[test]
fn invalid_civil_utc_and_missing_target_series_have_separate_error_provenance() {
    let mut eph = fictional_de441();
    let invalid = geocentric_aberrated_fixed_six_fields_many_from_civil(
        &mut eph,
        &[Body::Mercury],
        timestamp("1969-01-01T00:00:00Z"),
        0.0,
    );
    assert!(matches!(
        invalid,
        Err(CivilAberratedError::CivilTime(SkyError::UnsupportedUtc))
    ));

    let missing = geocentric_aberrated_fixed_six_fields_many_from_civil(
        &mut eph,
        &[Body::Mercury, Body::Sun],
        timestamp("2026-11-10T12:30:00Z"),
        0.0,
    );
    assert!(matches!(
        missing,
        Err(CivilAberratedError::Aberrated(
            AberratedSphericalError::Aberration(
                AberratedStateError::LightTime(LightTimeError::Ephemeris(
                    De441Error::MissingSeries(_)
                ))
            )
        ))
    ));

    // A failed batch does not leave the reader unusable.
    assert!(geocentric_aberrated_fixed_six_fields_many_from_civil(
        &mut eph,
        &[Body::Moon],
        timestamp("2026-11-10T12:30:00Z"),
        0.0
    )
    .is_ok());
}

#[test]
fn empty_batch_retains_tdb_epoch_and_rejects_nonfinite_tilt() {
    let mut eph = fictional_de441();
    let civil = timestamp("2026-11-10T12:30:00Z");
    let valid =
        geocentric_aberrated_fixed_six_fields_many_from_civil(&mut eph, &[], civil, 0.0)
            .unwrap();
    assert!(valid.results.is_empty());
    assert_eq!(
        valid.reception_epoch,
        civil.to_approximate_tdb_julian_date().unwrap()
    );

    assert!(matches!(
        geocentric_aberrated_fixed_six_fields_many_from_civil(
            &mut eph,
            &[],
            civil,
            f64::NAN
        ),
        Err(CivilAberratedError::Aberrated(
            AberratedSphericalError::Projection(ProjectionError::NonFinite)
        ))
    ));
}
