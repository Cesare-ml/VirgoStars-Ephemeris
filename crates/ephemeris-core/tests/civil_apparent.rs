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

//! Contract tests for the civil-time front door and the 1900–1972 ΔT model.
//!
//! The `#[ignore]` test needs a classic DE441 file in `DE441_PATH`.

use virgostars_ephemeris_core::apparent_ecliptic_of_date::ApparentOptions;
use virgostars_ephemeris_core::civil_apparent::{
    civil_to_tdb_with_policy, geocentric_apparent_six_fields_many_from_civil, CivilApparentError,
    CivilTimeSource,
};
use virgostars_ephemeris_core::de441::{Body, De441};
use virgostars_ephemeris_core::delta_t::{
    decimal_year_from_jd, delta_t_seconds_1900_1972, DeltaTError,
};
use virgostars_ephemeris_core::first_sky::CivilTimestamp;

fn civil(year: i32, month: u8, day: u8, hour: u8, minute: u8, offset: i16) -> CivilTimestamp {
    CivilTimestamp {
        year,
        month,
        day,
        hour,
        minute,
        second: 0.0,
        utc_offset_minutes: offset,
    }
}

#[test]
fn delta_t_polynomials_are_continuous_at_their_joins() {
    for join in [1920.0_f64, 1941.0, 1961.0] {
        let before = delta_t_seconds_1900_1972(join - 1e-9).unwrap();
        let after = delta_t_seconds_1900_1972(join).unwrap();
        assert!((before - after).abs() < 0.1, "{join}: {before} vs {after}");
    }
}

#[test]
fn delta_t_joins_the_exact_leap_second_table_at_1972() {
    // TT - UTC on 1972-01-01 is exactly 32.184 s + 10 s = 42.184 s, and
    // UT1 - UTC was within 0.1 s, so the fit must be within that of it.
    let at_1972 = delta_t_seconds_1900_1972(1972.0 - 1e-9).unwrap();
    assert!((at_1972 - 42.184).abs() < 0.1, "{at_1972}");
}

#[test]
fn delta_t_matches_published_anchor_values() {
    // The fits' own anchor values and widely tabulated historical ΔT.
    assert!((delta_t_seconds_1900_1972(1900.0).unwrap() - (-2.79)).abs() < 1e-12);
    assert!((delta_t_seconds_1900_1972(1920.0).unwrap() - 21.20).abs() < 1e-12);
    assert!((delta_t_seconds_1900_1972(1950.0).unwrap() - 29.07).abs() < 1e-12);
    // Tabulated: 1960 ≈ 33.2 s, 1970 ≈ 40.2 s.
    assert!((delta_t_seconds_1900_1972(1960.0).unwrap() - 33.2).abs() < 0.5);
    assert!((delta_t_seconds_1900_1972(1970.0).unwrap() - 40.2).abs() < 0.5);
}

#[test]
fn delta_t_rejects_out_of_range_and_non_finite_years() {
    for year in [1899.999, 1972.0, 2000.0, -500.0] {
        assert_eq!(
            delta_t_seconds_1900_1972(year),
            Err(DeltaTError::OutOfRange)
        );
    }
    assert_eq!(
        delta_t_seconds_1900_1972(f64::NAN),
        Err(DeltaTError::NonFinite)
    );
    assert!((decimal_year_from_jd(2_451_544.5) - 2000.0).abs() < 1e-12);
}

#[test]
fn policy_switches_at_1972_and_reports_the_source() {
    let (after, source_after) = civil_to_tdb_with_policy(civil(1972, 1, 1, 0, 0, 0)).unwrap();
    assert_eq!(source_after, CivilTimeSource::IersLeapSeconds);
    let (before, source_before) = civil_to_tdb_with_policy(civil(1971, 12, 31, 23, 59, 0)).unwrap();
    assert!(matches!(
        source_before,
        CivilTimeSource::DeltaTPolynomial { delta_t_seconds, .. }
            if (delta_t_seconds - 42.2).abs() < 0.2
    ));
    // 60 s of UT apart; the two policies disagree by less than 0.1 s.
    let gap_seconds = (after.0 - before.0) * 86_400.0;
    assert!((gap_seconds - 60.0).abs() < 0.1, "{gap_seconds}");
}

#[test]
fn utc_offset_decides_the_policy_not_the_local_date() {
    // Local 1972-01-01 00:30 at +01:00 is still 1971-12-31 23:30 UT.
    let (_, source) = civil_to_tdb_with_policy(civil(1972, 1, 1, 0, 30, 60)).unwrap();
    assert!(matches!(source, CivilTimeSource::DeltaTPolynomial { .. }));
    let (_, source) = civil_to_tdb_with_policy(civil(1971, 12, 31, 23, 30, -60)).unwrap();
    assert_eq!(source, CivilTimeSource::IersLeapSeconds);
}

#[test]
fn historical_epochs_are_reasonable_tdb_dates() {
    // 1950-01-01 12:00 UT: JD(UT) 2433283.0, ΔT ≈ 29.07 s.
    let (tdb, source) = civil_to_tdb_with_policy(civil(1950, 1, 1, 12, 0, 0)).unwrap();
    let expected = 2_433_283.0 + 29.07 / 86_400.0;
    assert!((tdb.0 - expected).abs() * 86_400.0 < 0.01, "{}", tdb.0);
    assert!(matches!(source, CivilTimeSource::DeltaTPolynomial { .. }));
}

#[test]
fn rejects_unsupported_instants_bad_offsets_and_leap_second_labels() {
    assert!(matches!(
        civil_to_tdb_with_policy(civil(1899, 12, 31, 23, 59, 0)),
        Err(CivilApparentError::UnsupportedInstant)
    ));
    assert!(matches!(
        civil_to_tdb_with_policy(civil(2027, 6, 28, 0, 0, 0)),
        Err(CivilApparentError::UnsupportedInstant)
    ));
    assert!(matches!(
        civil_to_tdb_with_policy(civil(2000, 1, 1, 0, 0, 900)),
        Err(CivilApparentError::InvalidCivil(_))
    ));
    let mut leap = civil(1998, 12, 31, 23, 59, 0);
    leap.second = 60.0;
    assert!(matches!(
        civil_to_tdb_with_policy(leap),
        Err(CivilApparentError::Calendar(_))
    ));
    assert!(matches!(
        civil_to_tdb_with_policy(civil(2001, 2, 30, 0, 0, 0)),
        Err(CivilApparentError::Calendar(_))
    ));
}

#[test]
#[ignore = "requires a legally obtained classic DE441 file in DE441_PATH"]
fn real_de441_civil_natal_samples_are_plausible_and_report_their_source() {
    let path = std::env::var_os("DE441_PATH").expect("DE441_PATH is not set");
    let mut eph = De441::open(path).unwrap();
    let targets = [Body::Sun, Body::Moon, Body::Mercury];
    // Fictional natal instants, one per policy.
    let historical = geocentric_apparent_six_fields_many_from_civil(
        &mut eph,
        &targets,
        civil(1950, 5, 15, 12, 0, 60),
        ApparentOptions::default(),
    )
    .unwrap();
    assert!(matches!(
        historical.time_source,
        CivilTimeSource::DeltaTPolynomial { .. }
    ));
    // Sun on 15 May is near 54 degrees of tropical longitude.
    let sun = historical.results[0].fields.longitude_deg;
    assert!((sun - 54.0).abs() < 1.0, "{sun}");
    let modern = geocentric_apparent_six_fields_many_from_civil(
        &mut eph,
        &targets,
        civil(2000, 1, 1, 12, 0, 0),
        ApparentOptions::default(),
    )
    .unwrap();
    assert_eq!(modern.time_source, CivilTimeSource::IersLeapSeconds);
    let sun = modern.results[0].fields.longitude_deg;
    assert!((sun - 280.37).abs() < 0.01, "{sun}");
    assert_eq!(modern.results.len(), 3);
}
