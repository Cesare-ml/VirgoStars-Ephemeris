/* EXHIBIT A. Common Public Attribution License Version 1.0.
The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

use std::io::Cursor;

use virgostars_ephemeris_core::de441::{Body, De441, De441Error};
use virgostars_ephemeris_core::first_sky::{
    calculate, CivilTimestamp, Observer, SkyApproximation, SkyError, MODEL_APPROXIMATIONS,
};

const RECORD_BYTES: usize = 3000;
const FIRST_TDB_JD: f64 = 2_461_344.5;
const LAST_TDB_JD: f64 = FIRST_TDB_JD + 32.0;

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// Deliberately fictional, fixed Cartesian ICRF barycentric coordinates in km.
// Series index 9 is a geocentric lunar offset, not a barycentric Moon.
// All eleven series used by first_sky are present; the final dummy pointer
// establishes the classic-DE441 3000-byte record shape.
fn synthetic_de441() -> Vec<u8> {
    let mut bytes = vec![0u8; RECORD_BYTES * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    put_f64(&mut bytes, 2652, FIRST_TDB_JD);
    put_f64(&mut bytes, 2660, LAST_TDB_JD);
    put_f64(&mut bytes, 2668, 32.0);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);

    let states: [[f64; 3]; 11] = [
        [110_000_000.0, 100_000_000.0, 13_000_000.0], // Mercury
        [90_000_000.0, -30_000_000.0, 8_000_000.0],   // Venus
        [150_000_000.0, 1_000_000.0, -500_000.0],     // Earth-Moon barycenter
        [220_000_000.0, 80_000_000.0, -20_000_000.0], // Mars barycenter
        [530_000_000.0, 200_000_000.0, 20_000_000.0], // Jupiter barycenter
        [1_100_000_000.0, -400_000_000.0, 100_000_000.0], // Saturn barycenter
        [2_500_000_000.0, 800_000_000.0, -100_000_000.0], // Uranus barycenter
        [4_400_000_000.0, -1_200_000_000.0, 300_000_000.0], // Neptune barycenter
        [5_000_000_000.0, 800_000_000.0, -200_000_000.0], // Pluto barycenter
        [384_400.0, 12_000.0, -8_000.0],              // Moon relative to Earth
        [0.0, 0.0, 0.0],                              // Sun
    ];
    for (index, xyz) in states.into_iter().enumerate() {
        let first = 3 + index * 3;
        let pointer_offset = 2696 + index * 12;
        put_i32(&mut bytes, pointer_offset, first as i32);
        put_i32(&mut bytes, pointer_offset + 4, 1);
        put_i32(&mut bytes, pointer_offset + 8, 1);
        for (axis, value) in xyz.into_iter().enumerate() {
            put_f64(&mut bytes, RECORD_BYTES * 2 + (first - 1 + axis) * 8, value);
        }
    }
    put_i32(&mut bytes, 2844, 373);
    put_i32(&mut bytes, 2848, 1);
    put_i32(&mut bytes, 2852, 1);
    put_f64(&mut bytes, RECORD_BYTES * 2, FIRST_TDB_JD);
    put_f64(&mut bytes, RECORD_BYTES * 2 + 8, LAST_TDB_JD);
    bytes
}

fn site() -> Observer {
    Observer {
        latitude_deg: 45.4642,
        longitude_deg_east: 9.19,
        height_m: 120.0,
    }
}

fn timestamp(value: &str) -> CivilTimestamp {
    CivilTimestamp::parse(value).unwrap()
}

fn near(a: f64, b: f64, tolerance: f64) {
    assert!(
        (a - b).abs() <= tolerance,
        "actual={a:?}, expected={b:?}, tolerance={tolerance:?}"
    );
}

#[test]
fn computes_all_ten_geometric_bodies_from_a_tiny_offline_kernel() {
    let mut ephemeris = De441::new(Cursor::new(synthetic_de441())).unwrap();
    let sky = calculate(&mut ephemeris, timestamp("2026-11-10T12:30:00Z"), site()).unwrap();

    assert_eq!(sky.tai_minus_utc_seconds, 37);
    assert_eq!(sky.observer, site());
    assert_eq!(sky.approximations, MODEL_APPROXIMATIONS);
    assert!(sky.approximations.contains(&SkyApproximation::NoAberration));
    assert_eq!(sky.positions.len(), 10);

    let expected = [
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
    for (position, body) in sky.positions.iter().zip(expected) {
        assert_eq!(position.body, body);
        assert!(position.distance_km.is_finite() && position.distance_km > 0.0);
        assert!((0.0..360.0).contains(&position.mean_ecliptic_longitude_deg));
        assert!((-90.0..=90.0).contains(&position.mean_ecliptic_latitude_deg));
        assert!((0.0..360.0).contains(&position.mean_right_ascension_deg));
        assert!((-90.0..=90.0).contains(&position.mean_declination_deg));
        assert!((-90.0..=90.0).contains(&position.altitude_deg));
        assert!(position
            .azimuth_deg
            .is_none_or(|az| (0.0..360.0).contains(&az)));
    }
    assert!((149_000_000.0..151_000_000.0).contains(&sky.positions[0].distance_km));
    assert!((350_000.0..410_000.0).contains(&sky.positions[1].distance_km));
}

#[test]
fn identical_utc_instants_produce_matching_full_sky_positions() {
    let mut ephemeris = De441::new(Cursor::new(synthetic_de441())).unwrap();
    let utc = calculate(&mut ephemeris, timestamp("2026-11-10T12:30:00Z"), site()).unwrap();
    let local = calculate(
        &mut ephemeris,
        timestamp("2026-11-10T13:30:00+01:00"),
        site(),
    )
    .unwrap();

    assert_eq!(utc.utc_day_number, local.utc_day_number);
    near(utc.utc_fraction_of_day, local.utc_fraction_of_day, 1e-14);
    near(utc.tt_julian_day, local.tt_julian_day, 1e-12);
    near(utc.tdb_julian_day, local.tdb_julian_day, 1e-12);
    for (a, b) in utc.positions.iter().zip(&local.positions) {
        assert_eq!(a.body, b.body);
        near(a.distance_km, b.distance_km, 1e-5);
        near(
            a.mean_ecliptic_longitude_deg,
            b.mean_ecliptic_longitude_deg,
            1e-9,
        );
        near(a.mean_declination_deg, b.mean_declination_deg, 1e-9);
        near(a.altitude_deg, b.altitude_deg, 1e-9);
    }
}

#[test]
fn changing_observer_changes_the_nearby_moon_projection() {
    let mut ephemeris = De441::new(Cursor::new(synthetic_de441())).unwrap();
    let time = timestamp("2026-11-10T12:30:00Z");
    let a = calculate(
        &mut ephemeris,
        time,
        Observer {
            latitude_deg: 0.0,
            longitude_deg_east: 0.0,
            height_m: 0.0,
        },
    )
    .unwrap();
    let b = calculate(
        &mut ephemeris,
        time,
        Observer {
            latitude_deg: 0.0,
            longitude_deg_east: 90.0,
            height_m: 0.0,
        },
    )
    .unwrap();
    let moon_a = a.positions[1];
    let moon_b = b.positions[1];
    assert_eq!(moon_a.body, Body::Moon);
    assert_eq!(moon_b.body, Body::Moon);
    assert!(
        (moon_a.distance_km - moon_b.distance_km).abs() > 0.1
            || (moon_a.mean_ecliptic_longitude_deg - moon_b.mean_ecliptic_longitude_deg).abs()
                > 0.001
    );
}

#[test]
fn missing_sun_series_and_outside_epoch_fail_without_partial_results() {
    let mut missing = synthetic_de441();
    for offset in [2696 + 10 * 12, 2696 + 10 * 12 + 4, 2696 + 10 * 12 + 8] {
        put_i32(&mut missing, offset, 0);
    }
    let mut missing_sun = De441::new(Cursor::new(missing)).unwrap();
    assert!(matches!(
        calculate(&mut missing_sun, timestamp("2026-11-10T12:30:00Z"), site()),
        Err(SkyError::Ephemeris(De441Error::MissingSeries(_)))
    ));

    let mut ephemeris = De441::new(Cursor::new(synthetic_de441())).unwrap();
    assert!(matches!(
        calculate(&mut ephemeris, timestamp("2026-12-20T12:30:00Z"), site()),
        Err(SkyError::Ephemeris(De441Error::OutsideCoverage { .. }))
    ));
    assert!(matches!(
        calculate(
            &mut ephemeris,
            timestamp("2026-11-10T12:30:00Z"),
            Observer {
                latitude_deg: 91.0,
                ..site()
            }
        ),
        Err(SkyError::InvalidInput(_))
    ));
}
