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
//! It models UTC day lengths, TAI-UTC day-start offsets and clock
//! validity using a caller-provided, independently sourced transition schedule.
//! Nothing is downloaded, updated automatically or extrapolated beyond the
//! declared coverage interval.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use crate::calendar::{Calendar, CalendarDateTime, JulianDate, TimeScale};

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
    /// Invalid hour, minute, fractional second, or leap-second placement.
    InvalidClockTime,
    /// This clock reading does not exist on the named UTC day.
    ClockTimeOutsideDay,
    /// Conversion to a normalized split TAI Julian date was not representable.
    UnrepresentableTai,
    /// Only a TAI-tagged JulianDate can be converted back to UTC.
    WrongInputScale,
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
            Self::InvalidClockTime => write!(f, "invalid UTC clock fields"),
            Self::ClockTimeOutsideDay => write!(f, "UTC clock reading does not exist on this day"),
            Self::UnrepresentableTai => {
                write!(f, "TAI Julian date cannot represent this UTC instant")
            }
            Self::WrongInputScale => write!(f, "TAI input time scale is required"),
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

/// A UTC civil-clock reading, independently of the named day.
///
/// Second 60 (including its fractional portion) is syntactically permitted
/// **only at 23:59**. It is valid for a particular date only if the explicitly
/// supplied leap-second schedule confirms a positive leap on that day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UtcClockTime {
    hour: u8,
    minute: u8,
    second: f64,
}

impl UtcClockTime {
    /// Construct a UTC clock reading without assuming a leap-second calendar.
    pub fn new(hour: u8, minute: u8, second: f64) -> Result<Self, UtcLeapError> {
        if hour > 23
            || minute > 59
            || !second.is_finite()
            || !(0.0..61.0).contains(&second)
            || (second >= 60.0 && (hour != 23 || minute != 59))
        {
            return Err(UtcLeapError::InvalidClockTime);
        }
        Ok(Self {
            hour,
            minute,
            second,
        })
    }

    pub fn hour(self) -> u8 {
        self.hour
    }

    pub fn minute(self) -> u8 {
        self.minute
    }

    pub fn second(self) -> f64 {
        self.second
    }

    fn seconds_since_midnight(self) -> f64 {
        f64::from(self.hour) * 3_600.0 + f64::from(self.minute) * 60.0 + self.second
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

    /// Validate that a UTC clock reading exists on a covered civil day.
    ///
    /// The result is elapsed SI seconds into that **named UTC day**, not a
    /// Julian date or a TAI/TT/TDB instant. A positive leap permits
    /// 23:59:60.x; a hypothetical negative leap omits 23:59:59.x.
    /// At every day's end, the next valid reading is the next day's 00:00.
    pub fn seconds_into_utc_day(
        &self,
        day: UtcDay,
        clock: UtcClockTime,
    ) -> Result<f64, UtcLeapError> {
        let day_length = f64::from(self.utc_day_length_seconds(day)?);
        let seconds = clock.seconds_since_midnight();
        if seconds >= day_length {
            return Err(UtcLeapError::ClockTimeOutsideDay);
        }
        Ok(seconds)
    }

    /// Convert a validated UTC civil day and clock into a split **TAI** JD.
    ///
    /// The caller-provided leap schedule determines the integer TAI-UTC
    /// offset at the **start of the UTC day**. Add that offset to the elapsed
    /// SI seconds within the named day, then normalize across TAI midnight.
    /// This preserves distinct instants at 23:59:59, 23:59:60 (positive
    /// leap), and the next day's 00:00:00, without treating UTC as a uniform
    /// Julian-day time scale. A hypothetical negative leap is also supported.
    ///
    /// The output is explicitly tagged `TimeScale::Tai`. It is not TT/TDB;
    /// the schedule must have been independently sourced and approved.
    pub fn utc_to_tai(&self, day: UtcDay, clock: UtcClockTime) -> Result<JulianDate, UtcLeapError> {
        let utc_elapsed = self.seconds_into_utc_day(day, clock)?;
        let offset = self.tai_minus_utc_at_day_start(day)?;
        let tai_seconds = utc_elapsed + f64::from(offset);
        let tai_day_carry = (tai_seconds / 86_400.0).floor() as i64;
        let tai_day_number = day
            .julian_day_number
            .checked_add(tai_day_carry)
            .ok_or(UtcLeapError::UnrepresentableTai)?;
        let tai_fraction = (tai_seconds - (tai_day_carry as f64) * 86_400.0) / 86_400.0;
        let tai = JulianDate::new(tai_day_number, tai_fraction, TimeScale::Tai)
            .map_err(|_| UtcLeapError::UnrepresentableTai)?;
        tai.parts().map_err(|_| UtcLeapError::UnrepresentableTai)?;
        Ok(tai)
    }

    /// Compute the TAI instant of a UTC midnight, including the exclusive
    /// boundary just after the last covered UTC day. The offset is integral
    /// SI seconds; Euclidean division keeps the split Julian date normalized
    /// even if a caller's unusual schedule offset spans multiple TAI days.
    fn tai_at_utc_midnight(&self, utc_day_number: i64) -> Result<JulianDate, UtcLeapError> {
        let offset = i64::from(self.offset_at_boundary(utc_day_number));
        let tai_day = utc_day_number
            .checked_add(offset.div_euclid(86_400))
            .ok_or(UtcLeapError::UnrepresentableTai)?;
        let fraction = (offset.rem_euclid(86_400) as f64) / 86_400.0;
        let tai = JulianDate::new(tai_day, fraction, TimeScale::Tai)
            .map_err(|_| UtcLeapError::UnrepresentableTai)?;
        tai.parts().map_err(|_| UtcLeapError::UnrepresentableTai)?;
        Ok(tai)
    }

    /// Compare two normalized TAI Julian dates without collapsing them to a
    /// single large `f64` Julian day.
    fn compare_tai(a: JulianDate, b: JulianDate) -> Ordering {
        a.day_number()
            .cmp(&b.day_number())
            .then_with(|| a.midnight_fraction().total_cmp(&b.midnight_fraction()))
    }

    /// Convert a **TAI** split Julian date to a named UTC day and clock time.
    ///
    /// Finds the UTC day whose *TAI* midnight boundaries surround the input,
    /// then recovers the elapsed SI seconds into that actual UTC day. This
    /// preserves the positive 23:59:60.x clock reading and excludes the
    /// missing final second of a hypothetical negative-leap UTC day.
    ///
    /// The conversion requires the same independently sourced, explicitly
    /// bounded leap schedule as `utc_to_tai`; it refuses UTC instants before
    /// the first covered midnight or at/after the end of the final covered
    /// day. There is no guessing or extrapolation.
    pub fn tai_to_utc(&self, tai: JulianDate) -> Result<(UtcDay, UtcClockTime), UtcLeapError> {
        if tai.scale() != TimeScale::Tai {
            return Err(UtcLeapError::WrongInputScale);
        }
        tai.parts().map_err(|_| UtcLeapError::UnrepresentableTai)?;
        let end_exclusive = self
            .last_day
            .julian_day_number
            .checked_add(1)
            .ok_or(UtcLeapError::InvalidCoverage)?;
        let first_tai = self.tai_at_utc_midnight(self.first_day.julian_day_number)?;
        let last_tai = self.tai_at_utc_midnight(end_exclusive)?;
        if Self::compare_tai(tai, first_tai) == Ordering::Less
            || Self::compare_tai(tai, last_tai) != Ordering::Less
        {
            return Err(UtcLeapError::OutsideCoverage);
        }

        // UTC midnight boundaries are monotonic because every leap changes
        // the day duration by only one SI second. Binary search is O(log N)
        // in covered days, independent of the number of leap transitions.
        let mut first = self.first_day.julian_day_number;
        let mut end = end_exclusive;
        while end - first > 1 {
            let middle = first + (end - first) / 2;
            let middle_tai = self.tai_at_utc_midnight(middle)?;
            if Self::compare_tai(tai, middle_tai) == Ordering::Less {
                end = middle;
            } else {
                first = middle;
            }
        }

        let day = UtcDay {
            julian_day_number: first,
        };
        let start_tai = self.tai_at_utc_midnight(first)?;
        let day_delta = tai.day_number() - start_tai.day_number();
        let elapsed = (day_delta as f64) * 86_400.0
            + (tai.midnight_fraction() - start_tai.midnight_fraction()) * 86_400.0;
        let day_length = f64::from(self.utc_day_length_seconds(day)?);
        if !elapsed.is_finite() || elapsed < 0.0 || elapsed >= day_length {
            return Err(UtcLeapError::UnrepresentableTai);
        }
        let clock = if elapsed >= 86_400.0 {
            // Only a positive leap day can contain seconds beyond 86400.
            UtcClockTime::new(23, 59, elapsed - 86_340.0)
        } else {
            let hour = (elapsed / 3_600.0).floor() as u8;
            let remaining = elapsed - f64::from(hour) * 3_600.0;
            let minute = (remaining / 60.0).floor() as u8;
            let second = remaining - f64::from(minute) * 60.0;
            UtcClockTime::new(hour, minute, second)
        }
        .map_err(|_| UtcLeapError::UnrepresentableTai)?;
        Ok((day, clock))
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
        let jump = UtcLeapTransition {
            tai_minus_utc_seconds: 38,
            ..positive
        };
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
        assert_eq!(
            UtcDay::from_gregorian(1971, 12, 31),
            Err(UtcLeapError::Before1972)
        );
        assert_eq!(
            UtcDay::from_gregorian(2019, 2, 29),
            Err(UtcLeapError::InvalidGregorianDate)
        );
        assert!(UtcDay::from_gregorian(2020, 2, 29).is_ok());
    }

    #[test]
    fn utc_clock_accepts_positive_leap_second_and_fraction_only_on_that_day() {
        let schedule = UtcLeapSchedule::new(
            d(2016, 12, 31),
            d(2017, 1, 1),
            36,
            vec![UtcLeapTransition {
                effective_utc_day: d(2017, 1, 1),
                tai_minus_utc_seconds: 37,
            }],
        )
        .unwrap();
        let leap = UtcClockTime::new(23, 59, 60.5).unwrap();
        assert_eq!(leap.hour(), 23);
        assert_eq!(leap.minute(), 59);
        assert_eq!(leap.second(), 60.5);
        assert_eq!(
            schedule.seconds_into_utc_day(d(2016, 12, 31), leap),
            Ok(86_400.5)
        );
        assert_eq!(
            schedule.seconds_into_utc_day(d(2017, 1, 1), leap),
            Err(UtcLeapError::ClockTimeOutsideDay)
        );
        assert_eq!(
            schedule.seconds_into_utc_day(d(2017, 1, 1), UtcClockTime::new(0, 0, 0.0).unwrap()),
            Ok(0.0)
        );
    }

    #[test]
    fn negative_leap_day_omits_235959_but_keeps_235958() {
        // Synthetic negative leap; no historical negative leap is asserted.
        let schedule = UtcLeapSchedule::new(
            d(2020, 6, 30),
            d(2020, 7, 1),
            37,
            vec![UtcLeapTransition {
                effective_utc_day: d(2020, 7, 1),
                tai_minus_utc_seconds: 36,
            }],
        )
        .unwrap();
        assert_eq!(
            schedule.seconds_into_utc_day(d(2020, 6, 30), UtcClockTime::new(23, 59, 58.5).unwrap()),
            Ok(86_398.5)
        );
        assert_eq!(
            schedule.seconds_into_utc_day(d(2020, 6, 30), UtcClockTime::new(23, 59, 59.0).unwrap()),
            Err(UtcLeapError::ClockTimeOutsideDay)
        );
        assert_eq!(
            schedule.seconds_into_utc_day(d(2020, 7, 1), UtcClockTime::new(23, 59, 59.0).unwrap()),
            Ok(86_399.0)
        );
    }

    #[test]
    fn rejects_invalid_clock_fields_and_nonfinite_seconds() {
        for (hour, minute, second) in [
            (24, 0, 0.0),
            (0, 60, 0.0),
            (0, 0, -0.001),
            (0, 0, 60.0),
            (23, 58, 60.0),
            (23, 59, 61.0),
            (23, 59, f64::NAN),
            (23, 59, f64::INFINITY),
        ] {
            assert_eq!(
                UtcClockTime::new(hour, minute, second),
                Err(UtcLeapError::InvalidClockTime)
            );
        }
        assert!(UtcClockTime::new(23, 59, 60.999).is_ok());
        assert!(UtcClockTime::new(12, 30, 59.999).is_ok());
    }

    #[test]
    fn clock_validation_requires_explicit_coverage() {
        let schedule = UtcLeapSchedule::new(d(2020, 1, 1), d(2020, 1, 1), 37, vec![]).unwrap();
        let midnight = UtcClockTime::new(0, 0, 0.0).unwrap();
        assert_eq!(
            schedule.seconds_into_utc_day(d(2019, 12, 31), midnight),
            Err(UtcLeapError::OutsideCoverage)
        );
        assert_eq!(
            schedule.seconds_into_utc_day(d(2020, 1, 2), midnight),
            Err(UtcLeapError::OutsideCoverage)
        );
    }

    #[test]
    fn utc_to_tai_distinguishes_positive_leap_second_and_next_midnight() {
        let schedule = UtcLeapSchedule::new(
            d(2016, 12, 31),
            d(2017, 1, 1),
            36,
            vec![UtcLeapTransition {
                effective_utc_day: d(2017, 1, 1),
                tai_minus_utc_seconds: 37,
            }],
        )
        .unwrap();
        let last_day = d(2016, 12, 31);
        let next_day = d(2017, 1, 1);

        let before = schedule
            .utc_to_tai(last_day, UtcClockTime::new(23, 59, 59.0).unwrap())
            .unwrap();
        let leap = schedule
            .utc_to_tai(last_day, UtcClockTime::new(23, 59, 60.0).unwrap())
            .unwrap();
        let after = schedule
            .utc_to_tai(next_day, UtcClockTime::new(0, 0, 0.0).unwrap())
            .unwrap();
        assert_eq!(before.scale(), TimeScale::Tai);
        assert_eq!(leap.scale(), TimeScale::Tai);
        assert_eq!(after.scale(), TimeScale::Tai);
        assert_eq!(before.day_number(), next_day.julian_day_number());
        assert_eq!(leap.day_number(), next_day.julian_day_number());
        assert_eq!(after.day_number(), next_day.julian_day_number());
        for (epoch, expected_seconds) in [(before, 35.0), (leap, 36.0), (after, 37.0)] {
            assert!((epoch.midnight_fraction() * 86_400.0 - expected_seconds).abs() < 1e-8);
        }
        let fraction = schedule
            .utc_to_tai(last_day, UtcClockTime::new(23, 59, 60.5).unwrap())
            .unwrap();
        assert!((fraction.midnight_fraction() * 86_400.0 - 36.5).abs() < 1e-8);
    }

    #[test]
    fn utc_to_tai_negative_leap_has_no_missing_tai_second() {
        // Synthetic negative leap: valid 23:59:58 -> 00:00:00 UTC
        // corresponds to consecutive TAI clock seconds 00:00:35 and :36.
        let schedule = UtcLeapSchedule::new(
            d(2020, 6, 30),
            d(2020, 7, 1),
            37,
            vec![UtcLeapTransition {
                effective_utc_day: d(2020, 7, 1),
                tai_minus_utc_seconds: 36,
            }],
        )
        .unwrap();
        let before = schedule
            .utc_to_tai(d(2020, 6, 30), UtcClockTime::new(23, 59, 58.0).unwrap())
            .unwrap();
        let after = schedule
            .utc_to_tai(d(2020, 7, 1), UtcClockTime::new(0, 0, 0.0).unwrap())
            .unwrap();
        assert_eq!(before.day_number(), d(2020, 7, 1).julian_day_number());
        assert_eq!(after.day_number(), d(2020, 7, 1).julian_day_number());
        assert!((before.midnight_fraction() * 86_400.0 - 35.0).abs() < 1e-8);
        assert!((after.midnight_fraction() * 86_400.0 - 36.0).abs() < 1e-8);
        assert_eq!(
            schedule.utc_to_tai(d(2020, 6, 30), UtcClockTime::new(23, 59, 59.0).unwrap()),
            Err(UtcLeapError::ClockTimeOutsideDay)
        );
    }

    #[test]
    fn utc_to_tai_normal_midday_and_tai_to_tt_composition() {
        let day = d(2017, 1, 2);
        let schedule = UtcLeapSchedule::new(day, day, 37, vec![]).unwrap();
        let tai = schedule
            .utc_to_tai(day, UtcClockTime::new(12, 0, 0.25).unwrap())
            .unwrap();
        assert_eq!(tai.day_number(), day.julian_day_number());
        assert!((tai.midnight_fraction() * 86_400.0 - 43_237.25).abs() < 1e-8);
        let tt = crate::time::tai_to_tt(tai).unwrap();
        assert_eq!(tt.scale(), TimeScale::Tt);
        assert!((tt.midnight_fraction() * 86_400.0 - 43_269.434).abs() < 1e-8);
    }

    #[test]
    fn utc_to_tai_rejects_invalid_date_specific_clock_and_coverage() {
        let day = d(2017, 1, 2);
        let schedule = UtcLeapSchedule::new(day, day, 37, vec![]).unwrap();
        assert_eq!(
            schedule.utc_to_tai(day, UtcClockTime::new(23, 59, 60.0).unwrap()),
            Err(UtcLeapError::ClockTimeOutsideDay)
        );
        assert_eq!(
            schedule.utc_to_tai(d(2017, 1, 3), UtcClockTime::new(0, 0, 0.0).unwrap()),
            Err(UtcLeapError::OutsideCoverage)
        );
    }

    #[test]
    fn tai_to_utc_roundtrips_positive_leap_and_next_midnight() {
        let last = d(2016, 12, 31);
        let next = d(2017, 1, 1);
        let schedule = UtcLeapSchedule::new(
            last,
            next,
            36,
            vec![UtcLeapTransition {
                effective_utc_day: next,
                tai_minus_utc_seconds: 37,
            }],
        )
        .unwrap();
        for (day, clock) in [
            (last, UtcClockTime::new(23, 59, 59.0).unwrap()),
            (last, UtcClockTime::new(23, 59, 60.0).unwrap()),
            (last, UtcClockTime::new(23, 59, 60.5).unwrap()),
            (next, UtcClockTime::new(0, 0, 0.0).unwrap()),
            (next, UtcClockTime::new(12, 30, 0.25).unwrap()),
        ] {
            let tai = schedule.utc_to_tai(day, clock).unwrap();
            let (recovered_day, recovered_clock) = schedule.tai_to_utc(tai).unwrap();
            assert_eq!(recovered_day, day);
            assert_eq!(recovered_clock.hour(), clock.hour());
            assert_eq!(recovered_clock.minute(), clock.minute());
            assert!((recovered_clock.second() - clock.second()).abs() < 1e-8);
        }
    }

    #[test]
    fn tai_to_utc_roundtrips_hypothetical_negative_leap() {
        let last = d(2020, 6, 30);
        let next = d(2020, 7, 1);
        let schedule = UtcLeapSchedule::new(
            last,
            next,
            37,
            vec![UtcLeapTransition {
                effective_utc_day: next,
                tai_minus_utc_seconds: 36,
            }],
        )
        .unwrap();
        for (day, clock) in [
            (last, UtcClockTime::new(23, 59, 58.0).unwrap()),
            (last, UtcClockTime::new(23, 59, 58.5).unwrap()),
            (next, UtcClockTime::new(0, 0, 0.0).unwrap()),
        ] {
            let (actual_day, actual_clock) = schedule
                .tai_to_utc(schedule.utc_to_tai(day, clock).unwrap())
                .unwrap();
            assert_eq!(actual_day, day);
            assert_eq!(actual_clock.hour(), clock.hour());
            assert_eq!(actual_clock.minute(), clock.minute());
            assert!((actual_clock.second() - clock.second()).abs() < 1e-8);
        }
    }

    #[test]
    fn tai_to_utc_rejects_wrong_scale_and_coverage_boundaries() {
        let first = d(2017, 1, 2);
        let last = d(2017, 1, 3);
        let schedule = UtcLeapSchedule::new(first, last, 37, vec![]).unwrap();
        let first_tai = schedule
            .utc_to_tai(first, UtcClockTime::new(0, 0, 0.0).unwrap())
            .unwrap();
        let one_second = JulianDate::new(
            first_tai.day_number(),
            first_tai.midnight_fraction() - 1.0 / 86_400.0,
            TimeScale::Tai,
        )
        .unwrap();
        assert_eq!(
            schedule.tai_to_utc(one_second),
            Err(UtcLeapError::OutsideCoverage)
        );
        let end_boundary = JulianDate::new(
            last.julian_day_number() + 1,
            37.0 / 86_400.0,
            TimeScale::Tai,
        )
        .unwrap();
        assert_eq!(
            schedule.tai_to_utc(end_boundary),
            Err(UtcLeapError::OutsideCoverage)
        );
        let tt = crate::time::tai_to_tt(first_tai).unwrap();
        assert_eq!(schedule.tai_to_utc(tt), Err(UtcLeapError::WrongInputScale));
    }

    #[test]
    fn tai_to_utc_roundtrips_multi_day_atomic_offset() {
        // A synthetic schedule with an unusually large, but valid, initial
        // offset verifies that the inverse does not assume TAI and UTC day
        // numbers differ by at most one.
        let day = d(2020, 1, 1);
        let schedule = UtcLeapSchedule::new(day, day, 200_000, vec![]).unwrap();
        for clock in [
            UtcClockTime::new(0, 0, 0.0).unwrap(),
            UtcClockTime::new(12, 34, 56.0).unwrap(),
            UtcClockTime::new(23, 59, 59.0).unwrap(),
        ] {
            let tai = schedule.utc_to_tai(day, clock).unwrap();
            let (back_day, back_clock) = schedule.tai_to_utc(tai).unwrap();
            assert_eq!(back_day, day);
            assert_eq!(back_clock.hour(), clock.hour());
            assert_eq!(back_clock.minute(), clock.minute());
            assert!((back_clock.second() - clock.second()).abs() < 1e-8);
        }
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
