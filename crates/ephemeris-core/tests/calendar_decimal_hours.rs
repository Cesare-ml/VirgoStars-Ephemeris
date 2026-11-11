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

//! Synthetic and published calendar-anchor checks for decimal-hour JD APIs.
//! These checks do not certify the future Node Swiss Ephemeris adapter.
//!
//! Created 2026-11-11: decimal-hour forward/reverse conversion regressions.

use virgostars_ephemeris_core::calendar::{Calendar, CalendarError, TimeScale};
use virgostars_ephemeris_core::calendar_decimal_hours::{
    from_split_julian_date, julday_f64, revjul_f64, to_split_julian_date,
    DecimalHourDateTime,
};

fn date(
    year: i32,
    month: u8,
    day: u8,
    decimal_hour: f64,
    scale: TimeScale,
) -> DecimalHourDateTime {
    DecimalHourDateTime {
        year,
        month,
        day,
        decimal_hour,
        scale,
    }
}

fn nearly(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}; tolerance {tolerance}"
    );
}

#[test]
fn julian_date_anchors_match_known_proleptic_calendars() {
    use Calendar::{ProlepticGregorian as G, ProlepticJulian as J};
    // J2000 noon, the astronomical Julian-day origin, the Gregorian
    // reform example and modern leap/century dates. No automatic cutover.
    let cases = [
        (2000, 1, 1, 12.0, G, 2_451_545.0),
        (-4712, 1, 1, 12.0, J, 0.0),
        (1582, 10, 15, 0.0, G, 2_299_160.5),
        (1582, 10, 4, 0.0, J, 2_299_159.5),
        (2024, 2, 29, 23.75, G, 2_460_370.489_583_333_5),
        (1900, 3, 1, 0.0, G, 2_415_079.5),
        (2100, 3, 1, 0.0, G, 2_488_128.5),
    ];
    for (year, month, day, hour, calendar, expected_jd) in cases {
        let input = date(year, month, day, hour, TimeScale::Ut1);
        let actual_jd = julday_f64(input, calendar).unwrap();
        nearly(actual_jd, expected_jd, 1.0e-9);
        let reverse = revjul_f64(actual_jd, calendar, TimeScale::Ut1).unwrap();
        assert_eq!((reverse.year, reverse.month, reverse.day), (year, month, day));
        nearly(reverse.decimal_hour, hour, 1.0e-7);
        assert_eq!(reverse.scale, TimeScale::Ut1);
    }
}

#[test]
fn split_julian_day_preserves_small_decimal_hour_offsets() {
    use Calendar::ProlepticGregorian as G;
    // At a modern epoch, adding this small offset to the *whole* JD would
    // quantize it. The split representation retains the fractional part.
    let hour = 12.0 + 0.000_001 / 3600.0;
    let input = date(2026, 11, 11, hour, TimeScale::Tdb);
    let split = to_split_julian_date(input, G).unwrap();
    assert_eq!(split.scale(), TimeScale::Tdb);
    assert_eq!(split.day_number(), 2_461_356);
    let roundtrip = from_split_julian_date(split, G).unwrap();
    nearly(roundtrip.decimal_hour, hour, 1.0e-12);
    assert_eq!(roundtrip.scale, TimeScale::Tdb);
}

#[test]
fn midnight_and_negative_julian_days_preserve_calendar_dates() {
    use Calendar::{ProlepticGregorian as G, ProlepticJulian as J};
    for (jd, calendar, year, month, day, hour) in [
        (-0.5, J, -4712, 1, 1, 0.0),
        (0.0, J, -4712, 1, 1, 12.0),
        (2_440_587.5, G, 1970, 1, 1, 0.0),
        (2_451_545.5, G, 2000, 1, 2, 0.0),
    ] {
        let result = revjul_f64(jd, calendar, TimeScale::Tt).unwrap();
        assert_eq!((result.year, result.month, result.day), (year, month, day));
        nearly(result.decimal_hour, hour, 1.0e-12);
        assert_eq!(result.scale, TimeScale::Tt);
    }
}

#[test]
fn invalid_calendar_dates_and_decimal_hours_fail_explicitly() {
    use Calendar::{ProlepticGregorian as G, ProlepticJulian as J};
    for hour in [
        -0.01,
        24.0,
        25.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        assert_eq!(
            julday_f64(date(2026, 11, 11, hour, TimeScale::Ut1), G),
            Err(CalendarError::InvalidTime)
        );
    }
    assert_eq!(
        julday_f64(date(1900, 2, 29, 0.0, TimeScale::Ut1), G),
        Err(CalendarError::InvalidDate)
    );
    assert!(julday_f64(date(1900, 2, 29, 0.0, TimeScale::Ut1), J).is_ok());
    assert_eq!(
        julday_f64(date(20_001, 1, 1, 0.0, TimeScale::Ut1), G),
        Err(CalendarError::YearOutOfRange)
    );
    assert_eq!(
        revjul_f64(f64::NAN, G, TimeScale::Ut1),
        Err(CalendarError::InvalidFraction)
    );
    assert_eq!(
        revjul_f64(f64::INFINITY, G, TimeScale::Ut1),
        Err(CalendarError::InvalidFraction)
    );
}

#[test]
fn calendar_selection_is_not_a_physical_timescale_conversion() {
    let input = date(1582, 10, 15, 0.0, TimeScale::Tai);
    let gregorian = to_split_julian_date(input, Calendar::ProlepticGregorian).unwrap();
    let julian = to_split_julian_date(input, Calendar::ProlepticJulian).unwrap();
    let (ga, gb) = gregorian.parts().unwrap();
    let (ja, jb) = julian.parts().unwrap();
    nearly((ja + jb) - (ga + gb), 10.0, 1.0e-9);
    assert_eq!(gregorian.scale(), TimeScale::Tai);
    assert_eq!(julian.scale(), TimeScale::Tai);
}
