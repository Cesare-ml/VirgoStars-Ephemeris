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

//! Calendar boundary regression tests with externally published Julian-day anchors.
//!
//! Calendar arithmetic only; no UTC/leap-second or dynamical-time conversion.

use virgostars_ephemeris_core::calendar::{
    Calendar, CalendarDateTime, CalendarError, JulianDate, TimeScale,
};

fn date(year: i32, month: u8, day: u8, hour: u8, scale: TimeScale) -> CalendarDateTime {
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
fn j2000_and_historical_gregorian_julian_day_anchors() {
    // USNO's conventional Julian Date epoch: JD 2451545.0 at
    // 2000-01-01 12:00, and Gregorian 1582-10-15 00:00 = JD 2299160.5.
    // These are calendar labels on the chosen scale, not UTC conversions.
    let j2000 = date(2000, 1, 1, 12, TimeScale::Tt)
        .to_julian_date(Calendar::ProlepticGregorian)
        .unwrap();
    assert_eq!(j2000.parts().unwrap(), (2_451_544.5, 0.5));

    let gregorian = date(1582, 10, 15, 0, TimeScale::Tt)
        .to_julian_date(Calendar::ProlepticGregorian)
        .unwrap();
    assert_eq!(gregorian.parts().unwrap(), (2_299_160.5, 0.0));

    // 1582-10-05 in the Julian calendar names the SAME midnight as
    // 1582-10-15 Gregorian. No implicit civil calendar cutover.
    let julian = date(1582, 10, 5, 0, TimeScale::Tt)
        .to_julian_date(Calendar::ProlepticJulian)
        .unwrap();
    assert_eq!(gregorian, julian);
}

#[test]
fn gregorian_cutover_is_not_implicitly_applied() {
    // A proleptic calendar includes all dates, even dates omitted in the
    // historical 1582 Catholic civil reform.
    for day in [4, 5, 10, 14, 15] {
        let original = date(1582, 10, day, 0, TimeScale::Ut1);
        let jd = original
            .to_julian_date(Calendar::ProlepticGregorian)
            .unwrap();
        assert_eq!(jd.to_calendar(Calendar::ProlepticGregorian).unwrap(), original);
    }
}

#[test]
fn century_leap_rules_are_distinct_and_reversible() {
    for year in [1600, 1700, 1800, 1900, 2000, 2100, 2400] {
        let leap_gregorian = year % 400 == 0;
        let leap_date = date(year, 2, 29, 0, TimeScale::Tdb);
        let result = leap_date.to_julian_date(Calendar::ProlepticGregorian);
        if leap_gregorian {
            let jd = result.unwrap();
            assert_eq!(jd.to_calendar(Calendar::ProlepticGregorian).unwrap(), leap_date);
        } else {
            assert_eq!(result, Err(CalendarError::InvalidDate));
        }
        let jd_julian = leap_date.to_julian_date(Calendar::ProlepticJulian).unwrap();
        assert_eq!(jd_julian.to_calendar(Calendar::ProlepticJulian).unwrap(), leap_date);
    }
}

#[test]
fn astronomical_year_zero_and_negative_years_round_trip() {
    for calendar in [Calendar::ProlepticGregorian, Calendar::ProlepticJulian] {
        for year in [-100, -4, -1, 0, 1, 4] {
            let original = date(year, 3, 1, 12, TimeScale::Tai);
            let jd = original.to_julian_date(calendar).unwrap();
            assert_eq!(jd.to_calendar(calendar).unwrap(), original);
        }
    }
}

#[test]
fn scale_labels_survive_round_trip_without_conversions() {
    for scale in [TimeScale::Tai, TimeScale::Tt, TimeScale::Tdb, TimeScale::Ut1] {
        let original = date(2026, 11, 10, 12, scale);
        let jd = original
            .to_julian_date(Calendar::ProlepticGregorian)
            .unwrap();
        assert_eq!(jd.scale(), scale);
        assert_eq!(jd.to_calendar(Calendar::ProlepticGregorian).unwrap(), original);
    }
}

#[test]
fn invalid_calendar_fields_and_fractional_julian_dates_are_rejected() {
    let mut invalid = date(2001, 2, 29, 0, TimeScale::Tt);
    assert_eq!(
        invalid.to_julian_date(Calendar::ProlepticGregorian),
        Err(CalendarError::InvalidDate)
    );
    invalid = date(2000, 1, 1, 24, TimeScale::Tt);
    assert_eq!(
        invalid.to_julian_date(Calendar::ProlepticGregorian),
        Err(CalendarError::InvalidTime)
    );
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            JulianDate::new(2_451_545, bad, TimeScale::Tt),
            Err(CalendarError::InvalidFraction)
        );
    }
    for bad in [-0.01, 1.0] {
        assert_eq!(
            JulianDate::new(2_451_545, bad, TimeScale::Tt),
            Err(CalendarError::InvalidFraction)
        );
    }
}
