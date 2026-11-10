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

use virgostars_ephemeris_core::calendar::{Calendar, CalendarDateTime, TimeScale};

// Swiss Ephemeris is a reference oracle only: this test does not link to it.
// This is calendar-to-JD arithmetic, NOT UTC->UT1, Delta-T or TDB conversion.
#[test]
fn backend_swe_julday_matches_recorded_swiss_reference() {
    const INPUT: &str = include_str!("fixtures/swiss_backend_julday.tsv");
    const MAX_ABSOLUTE_ERROR_DAYS: f64 = 1e-9; // 86.4 microseconds (single-f64 JD).

    let mut checked = 0;
    for (line_index, line) in INPUT.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('|').collect();
        assert_eq!(fields.len(), 8, "invalid fixture row {}", line_index + 1);

        let calendar = match fields[1] {
            "G" => Calendar::ProlepticGregorian,
            "J" => Calendar::ProlepticJulian,
            _ => panic!("unrecognized calendar at row {}", line_index + 1),
        };
        let date = CalendarDateTime {
            year: fields[2].parse().unwrap(),
            month: fields[3].parse().unwrap(),
            day: fields[4].parse().unwrap(),
            hour: fields[5].parse().unwrap(),
            minute: fields[6].parse().unwrap(),
            second: 0.0,
            // The label represents the JD convention; no conversion is done.
            scale: TimeScale::Ut1,
        };
        let expected: f64 = fields[7].parse().unwrap();
        assert!(expected.is_finite(), "bad oracle value: {}", fields[0]);
        let jd = date.to_julian_date(calendar).unwrap();
        let (whole, fraction) = jd.parts().unwrap();
        let actual = whole + fraction;
        let error = (actual - expected).abs();

        assert!(
            error <= MAX_ABSOLUTE_ERROR_DAYS,
            "{}: {:?} got {:.12}, Swiss {:.12}, |difference| {:.12} JD",
            fields[0],
            date,
            actual,
            expected,
            error
        );
        assert_eq!(jd.scale(), TimeScale::Ut1);
        checked += 1;
    }
    assert_eq!(checked, 11, "fixture rows were unexpectedly dropped");
}
