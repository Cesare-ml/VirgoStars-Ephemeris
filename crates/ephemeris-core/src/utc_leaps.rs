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


//! Explicit post-1972 UTC leap-second day schedule.
//!
//! This module does **not** convert a UTC clock reading into TAI/TT/TDB.
//! It only models the duration and TAI-UTC offset of named Gregorian UTC days
//! using a caller-provided, independently sourced transition schedule.
//! Nothing is downloaded, updated automatically or extrapolated beyond the
//! declared coverage interval.

use std::error::Error;
use std::fmt;

use crate::calendar::{Calendar, CalendarDateTime, CalendarError, JulianDate, TimeScale};

/// A Gregorian UTC calendar day (not a uniform-scale Julian date).
///
/// The stored integer is the Julian day number of the civil day at noon,
/// matching the integer day number used by the split \`calendar::JulianDate\`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtcDay {
    julian_day_number: i64,
}

/// A leap-second schedule error. None of these conditions can be repaired
/// by silently extrapolating or assuming UTC has 86,400 seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UtcLeapError {
    InvalidGregorianDate,
    /// The integer leap-second model is not valid for pre-1972 UTC.
    Before1972,
    InvalidCoverage,
    InvalidInitialOffset,
    /// Duplicated, unordered, out-of-coverage or non-month-boundary transition.
    InvalidTransition,
    /// A leap event must change TAI-UTC by exactly +1 or -1 second.
    InvalidOffsetStep,
    OutsideCoverage,
}

impl fmt::Display for UtcLeapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGregorianDate => write!(f, "invalid proleptic Gregorian UTC date"),
            Self::Before1972 => write!(f, "integer UTC leap-second schedule begins in 1972"),
            Self::InvalidCoverage => write!(f, "invalid UTC day coverage interval"),
            Self::InvalidInitialOffset => write!(f, "invalid initial TAI minus UTC offset"),
            Self::InvalidTransition => write!(f, "invalid UTC leap-second transition date/order"),
            Self::InvalidOffsetStep => write!(f, "TAI minus UTC must change by exactly one second"),
            Self::OutsideCoverage => write!(f, "UTC day lies outside approved schedule coverage"),
        }
    }
}

impl Error for UtcLeapError {}

impl UtcDay {
    /// Validate a Gregorian UTC day in the post-1972 leap-second regime.
    ///
    /// This uses the existing *calendar-only* conversion to a Julian day
    /// number. The internal dummy TAI scale label is never exposed and never
    /// interpreted as converting between UTC and TAI.
    pub fn from_gregorian(year: i32, month: u8, day: u8) -> Result<Self, UtcLeapError> {
        if year < 1972 {
            return Err(UtcLeapError::Before1972);
        }
        let midnight = CalendarDateTime {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0.0,
            scale: TimeScale::Tai,
        }
        .to_julian_date(Calendar::ProlepticGregorian)
        .map_err(|_| UtcLeapError::InvalidGregorianDate)?;
        Ok(Self {
            julian_day_number: midnight.day_number(),
        })
    }

    /// Integer Julian day number corresponding to the named Gregorian day.
    pub fn julian_day_number(self) -> i64 {
        self.julian_day_number
    }

    fn starts_month(self) -> Result<bool, UtcLeapError> {
        let day = JulianDate::new(self.julian_day_number, 0.0, TimeScale::Tai)
            .map_err(|_| UtcLeapError::InvalidGregorianDate)?;
        let date = day
            .to_calendar(Calendar::ProlepticGregorian)
            .map_err(|_| UtcLeapError::InvalidGregorianDate)?;
        Ok(date.day == 1)
    }
}

/// Transition effective at UTC 00:00 on a Gregorian day.
///
/// A positive leap second happens at 23:59:60 on the **preceding** day.
/// At the effective midnight, TAI-UTC increases by one second. This type
/// deliberately cannot be treated as a TDB/TT Julian date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UtcLeapTransition {
    pub effective_utc_day: UtcDay,
    pub tai_minus_utc_seconds: i32,
}

/// Explicit, non-extrapolating schedule of post-1972 leap-second UTC days.
///
/// Input transitions must be provided in ascending effective-date order.
/// The final covered day is inclusive; a transition on the following day
/// may describe the leap second at the final covered midnight boundary.
#[derive(Clone, Debug)]
pub struct UtcLeapSchedule {
    first_day: UtcDay,
    last_day: UtcDay,
    initial_tai_minus_utc_seconds: i32,
    transitions: Vec<UtcLeapTransition>,
}

impl UtcLeapSchedule {
    pub fn new(
        first_day: UtcDay,
        last_day: UtcDay,
        initial_tai_minus_utc_seconds: i32,
        transitions: Vec<UtcLeapTransition>,
    ) -> Result<Self, UtcLeapError> {
        if first_day > last_day {
            return Err(UtcLeapError::InvalidCoverage);
        }
        if initial_tai_minus_utc_seconds < 0 {
            return Err(UtcLeapError::InvalidInitialOffset);
        }
        let last_boundary = last_day
            .julian_day_number
            .checked_add(1)
            .ok_or(UtcLeapError::InvalidCoverage)?;
        let mut previous_day = first_day.julian_day_number;
        let mut previous_offset = initial_tai_minus_utc_seconds;
        for step in &transitions {
            let effective = step.effective_utc_day.julian_day_number;
            if effective <= previous_day || effective > last_boundary {
                return Err(UtcLeapError::InvalidTransition);
            }
            if !step.effective_utc_day.starts_month()? {
                return Err(UtcLeapError::InvalidTransition);
            }
            let change = i64::from(step.tai_minus_utc_seconds) - i64::from(previous_offset);
            if step.tai_minus_utc_seconds < 0 || change.abs() != 1 {
                return Err(UtcLeapError::InvalidOffsetStep);
            }
            previous_day = effective;
            previous_offset = step.tai_minus_utc_seconds;
        }
        Ok(Self {
            first_day,
            last_day,
            initial_tai_minus_utc_seconds,
            transitions,
        })
    }

    fn offset_at_boundary(&self, day_number: i64) -> i32 {
        let count = self
            .transitions
            .partition_point(|step| step.effective_utc_day.julian_day_number <= day_number);
        if count == 0 {
            self.initial_tai_minus_utc_seconds
        } else {
            self.transitions[count - 1].tai_minus_utc_seconds
        }
    }

    /// Return TAI-UTC in integer SI seconds **at 00:00 UTC of this day**.
    pub fn tai_minus_utc_at_day_start(&self, day: UtcDay) -> Result<i32, UtcLeapError> {
        if day < self.first_day || day > self.last_day {
            return Err(UtcLeapError::OutsideCoverage);
        }
        Ok(self.offset_at_boundary(day.julian_day_number))
    }

    /// Return 86,399, 86,400 or 86,401 SI seconds for a UTC civil day.
    ///
    /// This describes the actual named UTC civil day, including any positive
    /// leap second (23:59:60) or hypothetical negative leap second. It does
    /// not declare that a UTC Julian date is a uniform-duration time scale.
    pub fn utc_day_length_seconds(&self, day: UtcDay) -> Result<u32, UtcLeapError> {
        let before = self.tai_minus_utc_at_day_start(day)?;
        let after = self.offset_at_boundary(day.julian_day_number + 1);
        let length = 86_400 + after - before;
        Ok(length as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(year: i32, month: u8, day: u8) -> UtcDay {
        UtcDay::from_gregorian(year, month, day).unwrap()
    }

    #[test]
    fn positive_2016_leap_second_has_86401_seconds_only_on_december_31() {
        // Demonstrative transition from the official IERS history. The
        // schedule explicitly covers only the named test dates.
        let schedule = UtcLeapSchedule::new(
            d(2016, 12, 30),
            d(2017, 1, 2),
            36,
            vec![UtcLeapTransition {
                effective_utc_day: d(2017, 1, 1),
                tai_minus_utc_seconds: 37,
            }],
        )
        .unwrap();
        assert_eq!(schedule.utc_day_length_seconds(d(2016, 12, 30)), Ok(86_400));
        assert_eq!(schedule.utc_day_length_seconds(d(2016, 12, 31)), Ok(86_401));
        assert_eq!(schedule.utc_day_length_seconds(d(2017, 1, 1)), Ok(86_400));
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(2016, 12, 31)), Ok(36));
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(2017, 1, 1)), Ok(37));
        assert_eq!(
            schedule.tai_minus_utc_at_day_start(d(2017, 1, 3)),
            Err(UtcLeapError::OutsideCoverage)
        );
    }

    #[test]
    fn hypothetical_negative_leap_day_is_86399_seconds() {
        // No negative leap second had occurred as of the dated source.
        let schedule = UtcLeapSchedule::new(
            d(2020, 6, 29),
            d(2020, 7, 2),
            37,
            vec![UtcLeapTransition {
                effective_utc_day: d(2020, 7, 1),
                tai_minus_utc_seconds: 36,
            }],
        )
        .unwrap();
        assert_eq!(schedule.utc_day_length_seconds(d(2020, 6, 30)), Ok(86_399));
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(2020, 7, 1)), Ok(36));
    }

    #[test]
    fn validates_transition_order_step_month_boundary_and_coverage() {
        let first = d(2016, 12, 30);
        let last = d(2017, 1, 3);
        let positive = UtcLeapTransition {
            effective_utc_day: d(2017, 1, 1),
            tai_minus_utc_seconds: 37,
        };
        assert!(UtcLeapSchedule::new(last, first, 36, vec![]).is_err());
        assert!(UtcLeapSchedule::new(first, last, -1, vec![]).is_err());
        assert_eq!(
            UtcLeapSchedule::new(first, last, 36, vec![positive, positive]).unwrap_err(),
            UtcLeapError::InvalidTransition
        );
        let jump = UtcLeapTransition { tai_minus_utc_seconds: 38, ..positive };
        assert_eq!(
            UtcLeapSchedule::new(first, last, 36, vec![jump]).unwrap_err(),
            UtcLeapError::InvalidOffsetStep
        );
        let middle_of_month = UtcLeapTransition {
            effective_utc_day: d(2017, 1, 2),
            tai_minus_utc_seconds: 37,
        };
        assert_eq!(
            UtcLeapSchedule::new(first, last, 36, vec![middle_of_month]).unwrap_err(),
            UtcLeapError::InvalidTransition
        );
        let beyond = UtcLeapTransition {
            effective_utc_day: d(2017, 2, 1),
            tai_minus_utc_seconds: 37,
        };
        assert_eq!(
            UtcLeapSchedule::new(first, last, 36, vec![beyond]).unwrap_err(),
            UtcLeapError::InvalidTransition
        );
    }

    #[test]
    fn rejects_legacy_dates_and_invalid_gregorian_days() {
        assert_eq!(UtcDay::from_gregorian(1971, 12, 31), Err(UtcLeapError::Before1972));
        assert_eq!(
            UtcDay::from_gregorian(2019, 2, 29),
            Err(UtcLeapError::InvalidGregorianDate)
        );
        assert!(UtcDay::from_gregorian(2020, 2, 29).is_ok());
    }

    #[test]
    fn final_boundary_transition_and_no_extrapolation_are_explicit() {
        let last = d(2016, 12, 31);
        let schedule = UtcLeapSchedule::new(
            last,
            last,
            36,
            vec![UtcLeapTransition {
                effective_utc_day: d(2017, 1, 1),
                tai_minus_utc_seconds: 37,
            }],
        )
        .unwrap();
        assert_eq!(schedule.utc_day_length_seconds(last), Ok(86_401));
        assert_eq!(
            schedule.utc_day_length_seconds(d(2017, 1, 1)),
            Err(UtcLeapError::OutsideCoverage)
        );
    }
}
