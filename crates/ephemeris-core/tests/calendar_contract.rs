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

use virgostars_ephemeris_core::calendar::{
    Calendar, CalendarDateTime, CalendarError, JulianDate, TimeScale,
};
use virgostars_ephemeris_core::de441::JulianDateTdb;

fn date(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: f64) -> CalendarDateTime {
    CalendarDateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        scale: TimeScale::Tdb,
    }
}

#[test]
fn reference_dates_match_usno_and_erfa() {
    use Calendar::{ProlepticGregorian as G, ProlepticJulian as J};
    // USNO: JD 0 at Julian -4712-01-01 noon; Gregorian J2000 noon.
    for (d, calendar, expected) in [
        (date(-4712, 1, 1, 12, 0, 0.0), J, 0.0),
        (date(1582, 10, 4, 0, 0, 0.0), J, 2_299_159.5),
        (date(1582, 10, 15, 0, 0, 0.0), G, 2_299_160.5),
        (date(2000, 1, 1, 12, 0, 0.0), G, 2_451_545.0),
        // ERFA t_cal2jd reference: 2400000.5 + 52791.0.
        (date(2003, 6, 1, 0, 0, 0.0), G, 2_452_791.5),
    ] {
        let jd = d.to_julian_date(calendar).unwrap();
        let (a, b) = jd.parts().unwrap();
        assert_eq!(a + b, expected);
        assert_eq!(jd.to_de441_epoch().unwrap(), JulianDateTdb(expected));
        assert_eq!(jd.to_calendar(calendar).unwrap(), d);
        let from_single = JulianDate::from_f64(expected, TimeScale::Tdb).unwrap();
        assert_eq!(from_single.to_calendar(calendar).unwrap(), d);
    }
}

#[test]
fn proleptic_leap_rules_and_invalid_values_are_explicit() {
    use Calendar::{ProlepticGregorian as G, ProlepticJulian as J};
    assert_eq!(
        date(1900, 2, 29, 0, 0, 0.0).to_julian_date(G),
        Err(CalendarError::InvalidDate)
    );
    assert!(date(1900, 2, 29, 0, 0, 0.0).to_julian_date(J).is_ok());
    assert!(date(2000, 2, 29, 0, 0, 0.0).to_julian_date(G).is_ok());
    assert!(date(0, 2, 29, 0, 0, 0.0).to_julian_date(G).is_ok());
    for invalid in [
        date(2026, 0, 1, 0, 0, 0.0),
        date(2026, 2, 29, 0, 0, 0.0),
        date(2026, 4, 31, 0, 0, 0.0),
    ] {
        assert_eq!(invalid.to_julian_date(G), Err(CalendarError::InvalidDate));
    }
    for invalid in [
        date(2026, 1, 1, 24, 0, 0.0),
        date(2026, 1, 1, 0, 60, 0.0),
        date(2026, 1, 1, 0, 0, 60.0),
        date(2026, 1, 1, 0, 0, f64::NAN),
    ] {
        assert_eq!(invalid.to_julian_date(G), Err(CalendarError::InvalidTime));
    }
    assert_eq!(
        date(20_001, 1, 1, 0, 0, 0.0).to_julian_date(G),
        Err(CalendarError::YearOutOfRange)
    );
    assert_eq!(
        JulianDate::new(0, 1.0, TimeScale::Tdb),
        Err(CalendarError::InvalidFraction)
    );
    assert_eq!(
        JulianDate::new(0, f64::NAN, TimeScale::Tdb),
        Err(CalendarError::InvalidFraction)
    );
    assert_eq!(
        JulianDate::from_f64(f64::INFINITY, TimeScale::Tdb),
        Err(CalendarError::InvalidFraction)
    );
}

#[test]
fn calendar_round_trips_across_bce_and_de441_range() {
    for calendar in [Calendar::ProlepticGregorian, Calendar::ProlepticJulian] {
        for year in (-20_000..=20_000).step_by(13).chain([20_000]) {
            for month in 1..=12 {
                for day in [1, 28, 29, 30, 31] {
                    let original = date(year, month, day, 12, 34, 56.25);
                    let jd = match original.to_julian_date(calendar) {
                        Ok(jd) => jd,
                        Err(CalendarError::InvalidDate) => continue,
                        Err(other) => panic!("unexpected error: {other}"),
                    };
                    let back = jd.to_calendar(calendar).unwrap();
                    assert_eq!(
                        (
                            back.year,
                            back.month,
                            back.day,
                            back.hour,
                            back.minute,
                            back.scale
                        ),
                        (year, month, day, 12, 34, TimeScale::Tdb)
                    );
                    assert!((back.second - original.second).abs() < 1e-9);
                }
            }
        }
    }
}

#[test]
fn preserves_fraction_and_scale_without_utc_or_implicit_calendar_cutover() {
    let original = CalendarDateTime {
        scale: TimeScale::Tt,
        ..date(2000, 1, 1, 12, 0, 0.000001)
    };
    let jd = original
        .to_julian_date(Calendar::ProlepticGregorian)
        .unwrap();
    assert_eq!(jd.scale(), TimeScale::Tt);
    assert_eq!(jd.to_de441_epoch(), Err(CalendarError::WrongTimeScale));
    assert_eq!(jd.day_number(), 2_451_545);
    let (whole, fraction) = jd.parts().unwrap();
    assert_eq!(whole, 2_451_544.5);
    assert!(fraction > 0.5);
    let back = jd.to_calendar(Calendar::ProlepticGregorian).unwrap();
    assert!((back.second - 0.000001).abs() < 1e-9);
    assert_eq!(jd.to_calendar(Calendar::ProlepticJulian).unwrap().day, 19);
    let huge = JulianDate::new(i64::MAX, 0.5, TimeScale::Tai).unwrap();
    assert_eq!(huge.parts(), Err(CalendarError::YearOutOfRange));
    assert_eq!(
        huge.to_calendar(Calendar::ProlepticJulian),
        Err(CalendarError::YearOutOfRange)
    );
}

#[test]
fn single_part_julian_dates_round_trip_across_zero_and_noon() {
    for value in [-1.25, -0.75, -0.5, 0.0, 0.5, 1.25, 2_451_545.75] {
        let jd = JulianDate::from_f64(value, TimeScale::Tdb).unwrap();
        let (a, b) = jd.parts().unwrap();
        assert_eq!(a + b, value);
    }
    // The midnight-based split cannot express this tiny negative offset.
    assert_eq!(
        JulianDate::from_f64(-f64::EPSILON / 4.0, TimeScale::Tdb),
        Err(CalendarError::UnrepresentableTime)
    );
}
