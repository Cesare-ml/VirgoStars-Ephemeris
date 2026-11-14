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

//! Known historical Julian-day anchors for the canonical Rust calendar.
//!
//! Modification (2026-11-14): add independent calendar-era, leap-year and
//! negative-Julian-day regressions. These are calendar identities, not NASA
//! ephemeris-vector comparisons or implicit UTC/TT/TDB conversions.

use virgostars_ephemeris_core::calendar::{
    Calendar, CalendarDateTime, CalendarError, JulianDate, TimeScale,
};

fn day(year: i32, month: u8, day: u8, hour: u8, scale: TimeScale) -> CalendarDateTime {
    CalendarDateTime {
        year,
        month,
        day,
        hour,
        minute: 0,
        second: 0.0,
        scale,
    }
}

#[test]
fn j2000_noon_is_exact_tdb_anchor_without_scale_conversion() {
    let date = day(2000, 1, 1, 12, TimeScale::Tdb)
        .to_julian_date(Calendar::ProlepticGregorian)
        .unwrap();
    assert_eq!(date.parts().unwrap(), (2_451_544.5, 0.5));
    assert_eq!(date.to_de441_epoch().unwrap().0, 2_451_545.0);
    assert_eq!(
        date.to_calendar(Calendar::ProlepticGregorian).unwrap(),
        day(2000, 1, 1, 12, TimeScale::Tdb)
    );
}

#[test]
fn 1582_proleptic_calendar_choices_are_explicit_not_a_hidden_cutover() {
    // Known identical JD: 1582-10-15 Gregorian = 1582-10-05 Julian.
    // Each independently represents JD 2299160.5 at midnight.
    let gregorian = day(1582, 10, 15, 0, TimeScale::Tt)
        .to_julian_date(Calendar::ProlepticGregorian)
        .unwrap();
    let julian = day(1582, 10, 5, 0, TimeScale::Tt)
        .to_julian_date(Calendar::ProlepticJulian)
        .unwrap();
    assert_eq!(gregorian.parts().unwrap(), (2_299_160.5, 0.0));
    assert_eq!(julian.parts().unwrap(), gregorian.parts().unwrap());

    // Proleptic Gregorian October 5 remains valid; it is not skipped.
    let preceding = day(1582, 10, 4, 0, TimeScale::Tt)
        .to_julian_date(Calendar::ProlepticGregorian)
        .unwrap();
    assert_eq!(gregorian.day_number() - preceding.day_number(), 11);
    assert_eq!(
        gregorian.to_calendar(Calendar::ProlepticJulian).unwrap(),
        day(1582, 10, 5, 0, TimeScale::Tt)
    );
}

#[test]
fn julian_day_zero_roundtrips_through_both_ancient_calendars() {
    // JD zero is noon on Julian -4712-01-01, or Gregorian -4713-11-24;
    // year numbering is astronomical (year 0 denotes 1 BCE).
    let julian = day(-4712, 1, 1, 12, TimeScale::Tdb)
        .to_julian_date(Calendar::ProlepticJulian)
        .unwrap();
    let gregorian = day(-4713, 11, 24, 12, TimeScale::Tdb)
        .to_julian_date(Calendar::ProlepticGregorian)
        .unwrap();
    for date in [julian, gregorian] {
        assert_eq!(date.parts().unwrap(), (-0.5, 0.5));
        assert_eq!(date.to_de441_epoch().unwrap().0, 0.0);
    }
    let split = JulianDate::from_f64(0.0, TimeScale::Tdb).unwrap();
    assert_eq!(
        split.to_calendar(Calendar::ProlepticJulian).unwrap(),
        day(-4712, 1, 1, 12, TimeScale::Tdb)
    );
    assert_eq!(
        split.to_calendar(Calendar::ProlepticGregorian).unwrap(),
        day(-4713, 11, 24, 12, TimeScale::Tdb)
    );
}

#[test]
fn century_rules_and_astronomical_year_zero_are_calendar_specific() {
    // Julian centuries remain leap years; Gregorian centuries require /400.
    let feb_1900 = day(1900, 2, 29, 0, TimeScale::Ut1);
    assert_eq!(
        feb_1900
            .to_julian_date(Calendar::ProlepticGregorian)
            .unwrap_err(),
        CalendarError::InvalidDate
    );
    assert!(feb_1900.to_julian_date(Calendar::ProlepticJulian).is_ok());
    assert!(day(2000, 2, 29, 0, TimeScale::Tt)
        .to_julian_date(Calendar::ProlepticGregorian)
        .is_ok());
    assert_eq!(
        day(2100, 2, 29, 0, TimeScale::Tt)
            .to_julian_date(Calendar::ProlepticGregorian)
            .unwrap_err(),
        CalendarError::InvalidDate
    );

    for calendar in [Calendar::ProlepticGregorian, Calendar::ProlepticJulian] {
        let leap = day(0, 2, 29, 0, TimeScale::Tdb)
            .to_julian_date(calendar)
            .unwrap();
        let following = day(0, 3, 1, 0, TimeScale::Tdb)
            .to_julian_date(calendar)
            .unwrap();
        assert_eq!(following.day_number() - leap.day_number(), 1);
        assert_eq!(
            leap.to_calendar(calendar).unwrap(),
            day(0, 2, 29, 0, TimeScale::Tdb)
        );
        assert_eq!(
            day(-1, 2, 29, 0, TimeScale::Tdb)
                .to_julian_date(calendar)
                .unwrap_err(),
            CalendarError::InvalidDate
        );
    }
}

#[test]
fn calendar_arithmetic_never_implicitly_accepts_utc_leap_seconds_or_tdb() {
    let mut invalid = day(2016, 12, 31, 23, TimeScale::Tt);
    invalid.minute = 59;
    invalid.second = 60.0;
    assert_eq!(
        invalid
            .to_julian_date(Calendar::ProlepticGregorian)
            .unwrap_err(),
        CalendarError::InvalidTime
    );

    let tt = day(2000, 1, 1, 12, TimeScale::Tt)
        .to_julian_date(Calendar::ProlepticGregorian)
        .unwrap();
    assert_eq!(
        tt.to_de441_epoch().unwrap_err(),
        CalendarError::WrongTimeScale
    );
}
