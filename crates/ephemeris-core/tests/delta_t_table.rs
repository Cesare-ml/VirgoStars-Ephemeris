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

//! Tests for the measured ΔT table. Synthetic tables use the exact layout of the
//! USNO files with fictional numbers; the `#[ignore]` tests read the genuine files
//! from the directory named by `USNO_DELTAT_DIR` (nothing is bundled).

use virgostars_ephemeris_core::civil_apparent::{
    civil_to_tdb_with_policy, jd_ut_to_tdb_with_table, CivilApparentError, CivilTimeSource,
};
use virgostars_ephemeris_core::delta_t_table::{DeltaTKind, DeltaTTable, DeltaTTableError};
use virgostars_ephemeris_core::first_sky::CivilTimestamp;

const HISTORIC: &str = "  Year    TDT-UT1    error     LOD      error
             (s)      (s)      (ms)     (ms)
1950.000     29.15    0.10     1.00     1.00
1960.000     33.15    0.02     1.00     1.00
1970.000     40.18    0.001    1.00     1.00
1972.000     42.23    0.001    1.00     1.00
1972.500     42.81    0.001    1.00     1.00
1973.000     43.37    0.001    1.00     1.00
1974.000     44.48    0.001    1.00     1.00
";
const MONTHLY: &str = " 1973  2  1  43.4724
 1973  3  1  43.5648
 1973  4  1  43.6737
 1973  5  1  43.7782
 1973  6  1  43.8763
 1973  7  1  43.9562
 1973  8  1  44.0315
";
// MJD of 1973-08-01 is 41895; later rows are predictions.
const PREDS: &str = "MJD\t          Year\tTT-UT\tUT1-UTC\tError
41700.000\t1973.00\t99.00\t \t0.001
41925.000\t1973.58\t99.00\t \t0.001
42100.000\t1974.06\t44.60\t \t0.100
42300.000\t1974.61\t45.20\t \t0.300
";

fn table() -> DeltaTTable {
    DeltaTTable::from_usno(MONTHLY, HISTORIC, Some(PREDS)).unwrap()
}

#[test]
fn exact_points_midpoints_and_sources_follow_the_documented_precedence() {
    let t = table();
    // 1973-02-01 0h UT = JD 2441714.5; monthly file wins over the historic 1973.0/1974.0 rows.
    let jd_feb = 2_441_714.5;
    let v = t.delta_t(jd_feb).unwrap();
    assert_eq!(v.delta_t_seconds, 43.4724);
    assert_eq!(v.kind, DeltaTKind::Measured);
    let mid = t.delta_t(jd_feb + 14.0).unwrap(); // Feb has 28 days: halfway to 1973-03-01
    assert!((mid.delta_t_seconds - (43.4724 + 43.5648) / 2.0).abs() < 1e-12);
    // Historic uncertainties are carried and interpolated.
    let early = t.delta_t(2_433_282.5 + 0.0).unwrap(); // 1950-01-01
    assert!((early.delta_t_seconds - 29.15).abs() < 0.05);
    assert!(early.uncertainty_seconds.unwrap() > 0.05);
    // Rows of the historic file after 1973.0 are ignored (the monthly series replaces them).
    assert!((t.delta_t(2_441_760.5).unwrap().delta_t_seconds - 43.6).abs() < 0.1);
}

#[test]
fn predictions_start_after_the_last_observation_and_are_flagged() {
    let t = table();
    let last_measured = t.last_measured_jd_ut();
    assert_eq!(last_measured, 2_441_895.5); // 1973-08-01
                                            // The prediction rows at or before the last observation (value 99.00) are dropped.
    assert_eq!(t.delta_t(last_measured).unwrap().kind, DeltaTKind::Measured);
    let predicted = t.delta_t(42_300.0 + 2_400_000.5 - 10.0).unwrap();
    assert_eq!(predicted.kind, DeltaTKind::Predicted);
    assert!(
        predicted.delta_t_seconds < 50.0,
        "the 99 s rows must not appear"
    );
    assert!(predicted.uncertainty_seconds.unwrap() > 0.1);
    assert_eq!(t.last_jd_ut(), 42_300.0 + 2_400_000.5);
    // Without the predictions the table ends at the last observation.
    let no_preds = DeltaTTable::from_usno(MONTHLY, HISTORIC, None).unwrap();
    assert_eq!(no_preds.last_jd_ut(), 2_441_895.5);
    assert!(matches!(
        no_preds.delta_t(2_441_926.0),
        Err(DeltaTTableError::OutOfRange { .. })
    ));
}

#[test]
fn nothing_is_extrapolated_and_bad_input_is_rejected() {
    let t = table();
    assert!(matches!(
        t.delta_t(2_433_000.0),
        Err(DeltaTTableError::OutOfRange { .. })
    ));
    assert!(matches!(
        t.delta_t(2_500_000.0),
        Err(DeltaTTableError::OutOfRange { .. })
    ));
    assert_eq!(t.delta_t(f64::NAN), Err(DeltaTTableError::NonFinite));
    let bad_month = MONTHLY.replace("43.5648", "abc");
    assert_eq!(
        DeltaTTable::from_usno(&bad_month, HISTORIC, None),
        Err(DeltaTTableError::Parse {
            file: "deltat.data",
            line: 2
        })
    );
    let reversed = " 1973  3  1  43.5648\n 1973  2  1  43.4724\n";
    assert!(matches!(
        DeltaTTable::from_usno(reversed, HISTORIC, None),
        Err(DeltaTTableError::NotIncreasing {
            file: "deltat.data"
        })
    ));
    let gap_monthly = " 1990  2  1  56.9\n 1990  3  1  57.0\n";
    assert_eq!(
        DeltaTTable::from_usno(gap_monthly, HISTORIC, None),
        Err(DeltaTTableError::Gap)
    );
    assert!(matches!(
        DeltaTTable::from_usno(MONTHLY, "1950.0 abc 0.1 1 1\n", None),
        Err(DeltaTTableError::Parse {
            file: "historic_deltat.data",
            ..
        })
    ));
}

#[test]
fn ut_entry_reports_measured_and_predicted_values_and_refuses_outside_the_table() {
    let t = table();
    let (tdb, source) = jd_ut_to_tdb_with_table(2_441_714.5, &t).unwrap();
    assert!(matches!(
        source,
        CivilTimeSource::DeltaTTable { predicted: false, delta_t_seconds, .. } if delta_t_seconds == 43.4724
    ));
    assert!(
        (tdb.0 - 2_441_714.5 - 43.4724 / 86_400.0).abs() * 86_400.0 < 0.002,
        "TDB-TT term is below 2 ms"
    );
    let (_, predicted) = jd_ut_to_tdb_with_table(2_442_200.0, &t).unwrap();
    assert!(matches!(
        predicted,
        CivilTimeSource::DeltaTTable {
            predicted: true,
            uncertainty_seconds: Some(_),
            ..
        }
    ));
    assert!(matches!(
        jd_ut_to_tdb_with_table(2_415_000.0, &t),
        Err(CivilApparentError::UnsupportedInstant)
    ));
    assert!(matches!(
        jd_ut_to_tdb_with_table(2_500_000.0, &t),
        Err(CivilApparentError::DeltaTTable(_))
    ));
}

fn read(name: &str) -> String {
    let dir = std::env::var("USNO_DELTAT_DIR").expect("USNO_DELTAT_DIR is not set");
    std::fs::read_to_string(std::path::Path::new(&dir).join(name)).expect("USNO file")
}

#[test]
#[ignore = "requires the USNO deltat files in USNO_DELTAT_DIR"]
fn genuine_files_parse_join_and_agree_with_the_leap_second_table() {
    let t = DeltaTTable::from_usno(
        &read("deltat.data"),
        &read("historic_deltat.data"),
        Some(&read("deltat.preds")),
    )
    .unwrap();
    // Anchors of public record: ΔT(2000-01-01) = 63.83 s; the table's first monthly value.
    assert!((t.delta_t(2_451_544.5).unwrap().delta_t_seconds - 63.83).abs() < 0.02);
    assert!(t.last_measured_jd_ut() > 2_461_000.0);
    // Continuity across the historic/monthly join (1972.5 -> 1973-02-01).
    let before = t.delta_t(2_441_500.0).unwrap().delta_t_seconds;
    let after = t.delta_t(2_441_720.0).unwrap().delta_t_seconds;
    assert!(after > before && after - before < 1.0);
    // UT1-UTC = (TT-UTC) - ΔT must stay within the +/-0.9 s of the UTC definition: an
    // independent check of the table against the IERS leap-second table.
    let (mut worst, mut count) = (0.0_f64, 0);
    for year in 1973..=2026 {
        for month in 1..=12u8 {
            if year == 1973 && month < 2 || year == 2026 && month > 10 {
                continue;
            }
            let civil = CivilTimestamp {
                year,
                month,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0.0,
                utc_offset_minutes: 0,
            };
            let (tdb, _) = civil_to_tdb_with_policy(civil).unwrap();
            let g =
                (357.5291092 + 35_999.0502909 * ((tdb.0 - 2_451_545.0) / 36_525.0)).to_radians();
            let tt = tdb.0 - 0.001657 * g.sin() / 86_400.0;
            let utc = civil_jd(year, month);
            let delta_t = t.delta_t(utc).unwrap().delta_t_seconds;
            let ut1_minus_utc = (tt - utc) * 86_400.0 - delta_t;
            worst = worst.max(ut1_minus_utc.abs());
            count += 1;
        }
    }
    assert!(count > 600);
    assert!(worst < 0.9, "|UT1-UTC| reached {worst} s");
}

/// JD of 0h on the first of the month (proleptic Gregorian).
fn civil_jd(year: i32, month: u8) -> f64 {
    let (y, m) = if month <= 2 {
        (year - 1, i32::from(month) + 12)
    } else {
        (year, i32::from(month))
    };
    let a = y / 100;
    let b = 2 - a + a / 4;
    (365.25 * f64::from(y + 4716)).floor()
        + (30.6001 * f64::from(m + 1)).floor()
        + 1.0
        + f64::from(b)
        - 1524.5
}
