// EXHIBIT A. Common Public Attribution License Version 1.0.
//
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
//
// Created 2026-11-11: deterministic split-Julian-date coordinate-second tests.

//! Calendar-only stepping regressions. No UTC/leap-second or JPL numerical parity.

use virgostars_ephemeris_core::calendar::{
    Calendar, CalendarDateTime, CalendarError, JulianDate, TimeScale,
};

fn date(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: f64) -> CalendarDateTime {
    CalendarDateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        scale: TimeScale::Tt,
    }
}

#[test]
fn forward_and_backward_steps_cross_midnight_without_changing_time_scale() {
    let start = date(2026, 11, 11, 23, 59, 59.75)
        .to_julian_date(Calendar::ProlepticGregorian)
        .unwrap();
    let next = start.checked_add_seconds(0.5).unwrap();
    let civil = next.to_calendar(Calendar::ProlepticGregorian).unwrap();
    assert_eq!(
        (
            civil.year,
            civil.month,
            civil.day,
            civil.hour,
            civil.minute,
            civil.scale
        ),
        (2026, 11, 12, 0, 0, TimeScale::Tt)
    );
    assert!((civil.second - 0.25).abs() < 1.0e-8);

    let previous = next.checked_add_seconds(-0.5).unwrap();
    let back = previous.to_calendar(Calendar::ProlepticGregorian).unwrap();
    assert_eq!(
        (back.year, back.month, back.day, back.hour, back.minute),
        (2026, 11, 11, 23, 59)
    );
    assert!((back.second - 59.75).abs() < 1.0e-8);
}

#[test]
fn negative_and_whole_day_offsets_follow_calendar_leap_rules() {
    let start = date(2000, 1, 1, 0, 0, 0.0)
        .to_julian_date(Calendar::ProlepticGregorian)
        .unwrap();
    let prior = start.checked_add_seconds(-86_400.0).unwrap();
    let civil = prior.to_calendar(Calendar::ProlepticGregorian).unwrap();
    assert_eq!((civil.year, civil.month, civil.day), (1999, 12, 31));

    let after_leap_year = start.checked_add_seconds(366.0 * 86_400.0).unwrap();
    let civil = after_leap_year
        .to_calendar(Calendar::ProlepticGregorian)
        .unwrap();
    assert_eq!((civil.year, civil.month, civil.day), (2001, 1, 1));

    assert_eq!(start.checked_add_seconds(0.0).unwrap(), start);
}

#[test]
fn subsecond_step_uses_split_date_instead_of_collapsing_large_julian_day() {
    let j2000 = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap();
    let advanced = j2000.checked_add_seconds(0.000_001).unwrap();
    let et = advanced.to_tdb_seconds_past_j2000().unwrap().value();
    assert!((et - 0.000_001).abs() < 1.0e-9);
    assert_eq!(advanced.scale(), TimeScale::Tdb);
    assert_eq!(advanced.day_number(), j2000.day_number());
}

#[test]
fn incremental_one_second_steps_remain_close_to_one_hour() {
    let mut epoch = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap();
    for _ in 0..3600 {
        epoch = epoch.checked_add_seconds(1.0).unwrap();
    }
    let elapsed = epoch.to_tdb_seconds_past_j2000().unwrap().value();
    assert!((elapsed - 3600.0).abs() < 1.0e-6);
}

#[test]
fn rejects_nonfinite_and_unrepresentable_steps() {
    let base = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap();
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            base.checked_add_seconds(invalid),
            Err(CalendarError::InvalidTime)
        );
    }

    let too_large = ((1_u64 << 52) as f64) * 86_400.0;
    assert_eq!(
        base.checked_add_seconds(too_large),
        Err(CalendarError::YearOutOfRange)
    );

    let limit = JulianDate::new(1_i64 << 51, 0.5, TimeScale::Tdb).unwrap();
    assert_eq!(
        limit.checked_add_seconds(86_400.0),
        Err(CalendarError::YearOutOfRange)
    );
}
