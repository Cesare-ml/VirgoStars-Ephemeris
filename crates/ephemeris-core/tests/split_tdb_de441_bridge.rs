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

//! End-to-end TDB ET -> split JD -> legacy classic-DE441 state regressions.
//! The in-memory kernel and orbital state are synthetic. These tests do not
//! establish numerical agreement with JPL/Horizons or real-kernel accuracy.

use std::io::Cursor;

use virgostars_ephemeris_core::calendar::{CalendarError, JulianDate, TimeScale};
use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};
use virgostars_ephemeris_core::time_split::split_julian_day_tdb_from_et;
use virgostars_ephemeris_core::{TdbSecondsPastJ2000, J2000_JD_TDB, SECONDS_PER_DAY};

const RECORD_BYTES: usize = 3000;

fn write_f64(data: &mut [u8], offset: usize, value: f64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(data: &mut [u8], offset: usize, value: i32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// A single two-day classic-DE441 synthetic record centered on J2000.
///
/// Mercury relative to the solar-system barycenter has XYZ polynomials:
/// X = 100 + 2*u, Y = 200 - 4*u, Z = -50 (km).
/// Here u = JD(TDB) - 2451545, so the velocities are [2, -4, 0] km/day.
fn fixture() -> Vec<u8> {
    let mut data = vec![0_u8; 3 * RECORD_BYTES];
    let title = b"JPL Planetary Ephemeris DE441";
    data[..title.len()].copy_from_slice(title);
    write_f64(&mut data, 2652, J2000_JD_TDB - 1.0);
    write_f64(&mut data, 2660, J2000_JD_TDB + 1.0);
    write_f64(&mut data, 2668, 2.0);
    write_f64(&mut data, 2688, 81.0);
    write_i32(&mut data, 2840, 441);

    // Mercury coefficients start at 1-based word 3, T0 + T1 per axis.
    write_i32(&mut data, 2696, 3);
    write_i32(&mut data, 2700, 2);
    write_i32(&mut data, 2704, 1);
    // A non-overlapping, unused libration pointer sets the 3000-byte record size.
    write_i32(&mut data, 2844, 373);
    write_i32(&mut data, 2848, 1);
    write_i32(&mut data, 2852, 1);

    let base = RECORD_BYTES * 2;
    write_f64(&mut data, base, J2000_JD_TDB - 1.0);
    write_f64(&mut data, base + 8, J2000_JD_TDB + 1.0);
    for (axis, c0, c1) in [(0, 100.0, 2.0), (1, 200.0, -4.0), (2, -50.0, 0.0)] {
        write_f64(&mut data, base + (2 + axis * 2) * 8, c0);
        write_f64(&mut data, base + (3 + axis * 2) * 8, c1);
    }
    data
}

fn legacy_epoch(seconds: f64) -> JulianDateTdb {
    let et = TdbSecondsPastJ2000::new(seconds).unwrap();
    let split = split_julian_day_tdb_from_et(et).unwrap();
    assert_eq!(split.scale(), TimeScale::Tdb);
    split.to_de441_epoch().unwrap()
}

#[test]
fn synthetic_de441_states_and_speeds_match_explicit_tdb_offsets() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();

    for seconds in [-86_400.0, -43_200.0, 0.0, 1.0, 43_200.0, 86_400.0] {
        let epoch = legacy_epoch(seconds);
        let state = reader
            .state(Body::Mercury, Body::SolarSystemBarycenter, epoch)
            .unwrap();
        let normalized = seconds / SECONDS_PER_DAY;
        let expected = [
            100.0 + 2.0 * normalized,
            200.0 - 4.0 * normalized,
            -50.0,
        ];
        for (actual, expected) in state.position_km.iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 5.0e-9,
                "ET seconds {seconds}: expected {expected} km, got {actual} km"
            );
        }
        assert_eq!(state.velocity_km_per_day, [2.0, -4.0, 0.0]);
        assert_eq!(state.epoch, epoch);
    }
}

#[test]
fn classic_reader_cannot_resolve_microseconds_after_f64_jd_collapse() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    let reference = reader
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(J2000_JD_TDB),
        )
        .unwrap();

    for microseconds in [-1.0e-6, 1.0e-6] {
        let split = split_julian_day_tdb_from_et(
            TdbSecondsPastJ2000::new(microseconds).unwrap(),
        )
        .unwrap();
        assert_ne!(split.midnight_fraction(), 0.5);
        let rounded = split.to_de441_epoch().unwrap();
        assert_eq!(rounded, JulianDateTdb(J2000_JD_TDB));
        let state = reader
            .state(Body::Mercury, Body::SolarSystemBarycenter, rounded)
            .unwrap();
        assert_eq!(state, reference);
    }
}

#[test]
fn rejects_non_tdb_input_and_out_of_coverage_legacy_epochs() {
    let wrong_scale = JulianDate::new(2_451_545, 0.5, TimeScale::Tt).unwrap();
    assert_eq!(
        wrong_scale.to_de441_epoch(),
        Err(CalendarError::WrongTimeScale)
    );

    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    assert!(matches!(
        reader.state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            legacy_epoch(2.0 * SECONDS_PER_DAY)
        ),
        Err(De441Error::OutsideCoverage { .. })
    ));
}
