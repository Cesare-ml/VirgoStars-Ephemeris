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

use virgostars_ephemeris_core::calendar::{Calendar, JulianDate, TimeScale};

// Swiss's swe_revjul returns a calendar date and decimal hour for an input JD.
// This tests the existing Rust calendar API, NOT an implemented Swiss-like
// binding or any conversion between UTC, UT1, TT and TDB.
#[test]
fn backend_swe_revjul_matches_recorded_swiss_reference() {
    const INPUT: &str = include_str!("fixtures/swiss_backend_revjul.tsv");
    // Single-f64 Julian dates near modern epochs lose roughly 20 microseconds
    // of sub-day precision. The budget below is 1.08 milliseconds.
    const MAX_DECIMAL_HOUR_ERROR: f64 = 3e-7;

    let mut checked = 0;
    for (line_index, line) in INPUT.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('|').collect();
        assert_eq!(fields.len(), 7, "invalid fixture row {}", line_index + 1);
        let calendar = match fields[1] {
            "G" => Calendar::ProlepticGregorian,
            "J" => Calendar::ProlepticJulian,
            _ => panic!("unrecognized calendar in fixture row {}", line_index + 1),
        };

        let jd: f64 = fields[2].parse().unwrap();
        let year: i32 = fields[3].parse().unwrap();
        let month: u8 = fields[4].parse().unwrap();
        let day: u8 = fields[5].parse().unwrap();
        let expected_hour: f64 = fields[6].parse().unwrap();
        assert!(jd.is_finite() && expected_hour.is_finite());

        let actual = JulianDate::from_f64(jd, TimeScale::Ut1)
            .unwrap()
            .to_calendar(calendar)
            .unwrap();
        let actual_hour =
            f64::from(actual.hour) + f64::from(actual.minute) / 60.0 + actual.second / 3600.0;

        assert_eq!(
            (actual.year, actual.month, actual.day),
            (year, month, day),
            "{} calendar day differs from Swiss reference",
            fields[0]
        );
        assert_eq!(
            actual.scale,
            TimeScale::Ut1,
            "{} input JD time scale was changed",
            fields[0]
        );
        assert!(
            (actual_hour - expected_hour).abs() <= MAX_DECIMAL_HOUR_ERROR,
            "{} hour difference: Rust {:.12}, Swiss {:.12}",
            fields[0],
            actual_hour,
            expected_hour
        );
        // Verify the inverse conversion against the same original scalar JD,
        // not only against the expected civil fields. This also catches
        // fractional-day carry and negative-JD boundary regressions.
        let roundtrip = actual.to_julian_date(calendar).unwrap();
        let (whole, fraction) = roundtrip.parts().unwrap();
        let reconstructed = whole + fraction;
        assert!(
            (reconstructed - jd).abs() <= 1e-9,
            "{} JD roundtrip mismatch: original {:.12}, reconstructed {:.12}",
            fields[0],
            jd,
            reconstructed
        );
        assert_eq!(roundtrip.scale(), TimeScale::Ut1);
        checked += 1;
    }
    assert_eq!(checked, 15, "reverse-Julian reference cases were dropped");
}
