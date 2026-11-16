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

//! Explicit Gregorian/Julian leap-century and astronomical-year-zero regressions.
//!
//! Calendar arithmetic only; these tests do not convert time scales.
//! Independent proleptic-Gregorian noon JD anchors were evaluated with
//! SQLite 3.46.1 `julianday()` (https://www.sqlite.org/lang_datefunc.html).
//! The existing J2000 and 1582 anchors remain covered by calendar_boundaries.rs.
//! Added 2026-11-16 for issue #397.

use virgostars_ephemeris_core::calendar::{
    Calendar, CalendarDateTime, CalendarError, JulianDate, TimeScale,
};

fn civil(
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: f64,
    scale: TimeScale,
) -> CalendarDateTime {
    CalendarDateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        scale,
    }
}

fn at_midnight(year: i32, month: u8, day: u8, calendar: Calendar) -> JulianDate {
    civil(year, month, day, 0, 0, 0.0, TimeScale::Tt)
        .to_julian_date(calendar)
        .unwrap()
}

fn assert_consecutive_days(
    first: CalendarDateTime,
    second: CalendarDateTime,
    calendar: Calendar,
) {
    let first_jd = first.to_julian_date(calendar).unwrap();
    let second_jd = second.to_julian_date(calendar).unwrap();
    assert_eq!(second_jd.day_number(), first_jd.day_number() + 1);
    assert_eq!(first_jd.midnight_fraction(), 0.0);
    assert_eq!(second_jd.midnight_fraction(), 0.0);
    assert_eq!(first_jd.to_calendar(calendar).unwrap(), first);
    assert_eq!(second_jd.to_calendar(calendar).unwrap(), second);
}

#[test]
fn explicit_gregorian_and_julian_century_leap_dates() {
    for (year, valid) in [(1900, false), (2000, true), (2100, false)] {
        let february_29 = civil(year, 2, 29, 0, 0, 0.0, TimeScale::Tt);
        let result = february_29.to_julian_date(Calendar::ProlepticGregorian);
        if valid {
            assert_eq!(
                result.unwrap().to_calendar(Calendar::ProlepticGregorian).unwrap(),
                february_29
            );
        } else {
            assert_eq!(result, Err(CalendarError::InvalidDate));
        }

        let julian = february_29.to_julian_date(Calendar::ProlepticJulian).unwrap();
        assert_eq!(julian.to_calendar(Calendar::ProlepticJulian).unwrap(), february_29);
    }
}

#[test]
fn astronomical_year_zero_is_leap_and_year_minus_one_is_not() {
    for calendar in [Calendar::ProlepticGregorian, Calendar::ProlepticJulian] {
        let year_zero_leap_day = civil(0, 2, 29, 12, 0, 0.0, TimeScale::Tdb);
        let jd = year_zero_leap_day.to_julian_date(calendar).unwrap();
        assert_eq!(jd.to_calendar(calendar).unwrap(), year_zero_leap_day);

        let year_minus_one_leap_day = civil(-1, 2, 29, 12, 0, 0.0, TimeScale::Tdb);
        assert_eq!(
            year_minus_one_leap_day.to_julian_date(calendar),
            Err(CalendarError::InvalidDate)
        );

        let year_minus_one_last_february_day = civil(-1, 2, 28, 12, 0, 0.0, TimeScale::Tdb);
        let round_trip = year_minus_one_last_february_day
            .to_julian_date(calendar)
            .unwrap()
            .to_calendar(calendar)
            .unwrap();
        assert_eq!(round_trip, year_minus_one_last_february_day);
    }
}

#[test]
fn independent_proleptic_gregorian_jd_noon_anchors_cover_centuries_and_bce() {
    // SQLite 3.46.1 julianday(), proleptic Gregorian civil labels at noon.
    // Expected conventional JD is integral at noon; parts() keeps its split.
    for (year, month, day, expected_jd) in [
        (1900, 3, 1, 2_415_080_i64),
        (2000, 2, 29, 2_451_604_i64),
        (2100, 3, 1, 2_488_129_i64),
        (0, 2, 29, 1_721_119_i64),
        (-1, 2, 28, 1_720_753_i64),
    ] {
        let jd = civil(year, month, day, 12, 0, 0.0, TimeScale::Tt)
            .to_julian_date(Calendar::ProlepticGregorian)
            .unwrap();
        assert_eq!(jd.day_number(), expected_jd);
        assert_eq!(jd.midnight_fraction(), 0.5);
        assert_eq!(jd.parts().unwrap(), (expected_jd as f64 - 0.5, 0.5));
    }
}

#[test]
fn year_and_february_boundaries_are_consecutive_in_both_calendars() {
    for calendar in [Calendar::ProlepticGregorian, Calendar::ProlepticJulian] {
        for year in [-2, -1, 0, 1, 1899, 1999, 2099] {
            assert_consecutive_days(
                civil(year, 12, 31, 0, 0, 0.0, TimeScale::Ut1),
                civil(year + 1, 1, 1, 0, 0, 0.0, TimeScale::Ut1),
                calendar,
            );
        }

        for year in [-4, -1, 0, 1900, 2000, 2100] {
            let leap = match calendar {
                Calendar::ProlepticGregorian => year.rem_euclid(400) == 0
                    || (year.rem_euclid(4) == 0 && year.rem_euclid(100) != 0),
                Calendar::ProlepticJulian => year.rem_euclid(4) == 0,
            };
            if leap {
                assert_consecutive_days(
                    civil(year, 2, 28, 0, 0, 0.0, TimeScale::Tt),
                    civil(year, 2, 29, 0, 0, 0.0, TimeScale::Tt),
                    calendar,
                );
                assert_consecutive_days(
                    civil(year, 2, 29, 0, 0, 0.0, TimeScale::Tt),
                    civil(year, 3, 1, 0, 0, 0.0, TimeScale::Tt),
                    calendar,
                );
            } else {
                assert_consecutive_days(
                    civil(year, 2, 28, 0, 0, 0.0, TimeScale::Tt),
                    civil(year, 3, 1, 0, 0, 0.0, TimeScale::Tt),
                    calendar,
                );
            }
        }
    }
}

#[test]
fn same_conventional_jd_has_distinct_gregorian_and_julian_labels() {
    // In 2000 the proleptic calendars differ by 13 civil dates:
    // 2000-01-14 Gregorian and 2000-01-01 Julian name the same instant.
    let gregorian = at_midnight(2000, 1, 14, Calendar::ProlepticGregorian);
    let julian = at_midnight(2000, 1, 1, Calendar::ProlepticJulian);
    assert_eq!(gregorian, julian);
    assert_eq!(
        gregorian.to_calendar(Calendar::ProlepticGregorian).unwrap(),
        civil(2000, 1, 14, 0, 0, 0.0, TimeScale::Tt)
    );
    assert_eq!(
        gregorian.to_calendar(Calendar::ProlepticJulian).unwrap(),
        civil(2000, 1, 1, 0, 0, 0.0, TimeScale::Tt)
    );
}

#[test]
fn fractional_seconds_and_time_scale_survive_negative_year_round_trip() {
    let original = civil(-1, 12, 31, 23, 59, 12.25, TimeScale::Tai);
    let jd = original.to_julian_date(Calendar::ProlepticGregorian).unwrap();
    assert_eq!(jd.scale(), TimeScale::Tai);
    let restored = jd.to_calendar(Calendar::ProlepticGregorian).unwrap();
    assert_eq!(restored.year, original.year);
    assert_eq!(restored.month, original.month);
    assert_eq!(restored.day, original.day);
    assert_eq!(restored.hour, original.hour);
    assert_eq!(restored.minute, original.minute);
    assert!((restored.second - original.second).abs() < 1.0e-8);
}
