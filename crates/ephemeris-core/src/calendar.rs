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

//! Proleptic Gregorian/Julian calendars and split Julian dates.
//!
//! This is calendar arithmetic on a specified time scale, not a conversion
//! between time scales. UTC and leap seconds are deliberately not supported.

use std::error::Error;
use std::fmt;

use crate::de441::JulianDateTdb;
use crate::time::{TdbSecondsPastJ2000, J2000_JD_TDB};

pub const MIN_YEAR: i32 = -20_000;
pub const MAX_YEAR: i32 = 20_000;
const SECONDS_PER_DAY: f64 = 86_400.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Calendar {
    ProlepticGregorian,
    ProlepticJulian,
}

/// The caller must supply the scale; this module never converts between scales.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeScale {
    Tai,
    Tt,
    Tdb,
    Ut1,
}

/// Astronomical year numbering: year 0 is 1 BCE, year -1 is 2 BCE.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalendarDateTime {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: f64,
    pub scale: TimeScale,
}

/// Julian day number at noon plus a fraction measured from preceding midnight.
/// The conventional Julian date is `day_number - 0.5 + midnight_fraction`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JulianDate {
    day_number: i64,
    midnight_fraction: f64,
    scale: TimeScale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarError {
    YearOutOfRange,
    InvalidDate,
    InvalidTime,
    InvalidFraction,
    UnrepresentableTime,
    WrongTimeScale,
}

impl fmt::Display for CalendarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::YearOutOfRange => write!(f, "calendar date outside supported year range"),
            Self::InvalidDate => write!(f, "invalid calendar date"),
            Self::InvalidTime => write!(f, "invalid time of day; UTC leap seconds are unsupported"),
            Self::InvalidFraction => write!(f, "invalid Julian date fraction"),
            Self::UnrepresentableTime => write!(
                f,
                "time cannot be represented by the Julian date split at f64 precision"
            ),
            Self::WrongTimeScale => write!(f, "DE441 requires an epoch already expressed in TDB"),
        }
    }
}
impl Error for CalendarError {}

fn leap(calendar: Calendar, year: i32) -> bool {
    year.rem_euclid(4) == 0
        && (calendar == Calendar::ProlepticJulian
            || year.rem_euclid(100) != 0
            || year.rem_euclid(400) == 0)
}

fn month_days(calendar: Calendar, year: i32, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(if leap(calendar, year) { 29 } else { 28 }),
        _ => None,
    }
}

// Days before January 1 of the given astronomical year relative to year 1.
fn first_day_number(calendar: Calendar, year: i32) -> i64 {
    let y = i64::from(year) - 1;
    let mut days = 365 * y + y.div_euclid(4);
    let epoch = match calendar {
        Calendar::ProlepticGregorian => {
            days += -y.div_euclid(100) + y.div_euclid(400);
            1_721_426
        }
        Calendar::ProlepticJulian => 1_721_424,
    };
    epoch + days
}

impl CalendarDateTime {
    /// Interpret fields in the selected proleptic calendar; no 1582 cutover is implicit.
    pub fn to_julian_date(self, calendar: Calendar) -> Result<JulianDate, CalendarError> {
        if !(MIN_YEAR..=MAX_YEAR).contains(&self.year) {
            return Err(CalendarError::YearOutOfRange);
        }
        let month_length =
            month_days(calendar, self.year, self.month).ok_or(CalendarError::InvalidDate)?;
        if self.day == 0 || self.day > month_length {
            return Err(CalendarError::InvalidDate);
        }
        if self.hour > 23
            || self.minute > 59
            || !self.second.is_finite()
            || !(0.0..60.0).contains(&self.second)
        {
            return Err(CalendarError::InvalidTime);
        }
        let mut jdn = first_day_number(calendar, self.year);
        for month in 1..self.month {
            jdn += i64::from(month_days(calendar, self.year, month).expect("validated month"));
        }
        jdn += i64::from(self.day) - 1;
        let seconds = f64::from(self.hour) * 3600.0 + f64::from(self.minute) * 60.0 + self.second;
        let fraction = seconds / SECONDS_PER_DAY;
        if fraction >= 1.0 {
            return Err(CalendarError::UnrepresentableTime);
        }
        Ok(JulianDate {
            day_number: jdn,
            midnight_fraction: fraction,
            scale: self.scale,
        })
    }
}

impl JulianDate {
    /// Convert a conventional single-f64 JD without changing its time scale.
    /// Precision already lost in the input cannot be recovered.
    pub fn from_f64(julian_day: f64, scale: TimeScale) -> Result<Self, CalendarError> {
        if !julian_day.is_finite() {
            return Err(CalendarError::InvalidFraction);
        }
        let whole = julian_day.floor();
        if whole.abs() > (1u64 << 51) as f64 {
            return Err(CalendarError::YearOutOfRange);
        }
        let fraction = julian_day - whole;
        let (day_number, midnight_fraction) = if fraction >= 0.5 {
            (whole as i64 + 1, fraction - 0.5)
        } else {
            (whole as i64, fraction + 0.5)
        };
        if midnight_fraction >= 1.0 {
            return Err(CalendarError::UnrepresentableTime);
        }
        let result = Self::new(day_number, midnight_fraction, scale)?;
        let (first, second) = result.parts()?;
        if first + second != julian_day {
            return Err(CalendarError::UnrepresentableTime);
        }
        Ok(result)
    }

    /// Construct from an integral noon day number and fraction since midnight.
    pub fn new(
        day_number: i64,
        midnight_fraction: f64,
        scale: TimeScale,
    ) -> Result<Self, CalendarError> {
        if !midnight_fraction.is_finite() || !(0.0..1.0).contains(&midnight_fraction) {
            return Err(CalendarError::InvalidFraction);
        }
        Ok(Self {
            day_number,
            midnight_fraction,
            scale,
        })
    }

    pub fn day_number(self) -> i64 {
        self.day_number
    }
    pub fn midnight_fraction(self) -> f64 {
        self.midnight_fraction
    }
    pub fn scale(self) -> TimeScale {
        self.scale
    }

    /// Two f64 parts whose sum is the conventional JD. Retain both for precision.
    pub fn parts(self) -> Result<(f64, f64), CalendarError> {
        // Exact representation of the integer and half-day is needed.
        if self.day_number.unsigned_abs() > (1u64 << 51) {
            return Err(CalendarError::YearOutOfRange);
        }
        Ok((self.day_number as f64 - 0.5, self.midnight_fraction))
    }

    /// Express an already-TDB split Julian date as TDB seconds past J2000.
    ///
    /// Unlike converting to a single `f64` JD first, this keeps the integral
    /// day offset and sub-day fraction separate until after multiplication by
    /// 86,400 seconds/day. This avoids the avoidable microsecond-scale
    /// quantization caused by collapsing modern Julian dates first.
    ///
    /// No UTC, TAI, TT or UT1 conversion is performed. The result is still a
    /// single `f64` second count, whose resolution decreases far from J2000.
    /// The time scale must already be TDB.
    pub fn to_tdb_seconds_past_j2000(self) -> Result<TdbSecondsPastJ2000, CalendarError> {
        if self.scale != TimeScale::Tdb {
            return Err(CalendarError::WrongTimeScale);
        }
        let (midnight_jd, midnight_fraction) = self.parts()?;
        let integral_seconds = (midnight_jd - J2000_JD_TDB) * SECONDS_PER_DAY;
        let fractional_seconds = midnight_fraction * SECONDS_PER_DAY;
        TdbSecondsPastJ2000::new(integral_seconds + fractional_seconds)
            .map_err(|_| CalendarError::UnrepresentableTime)
    }

    /// Convert an already-TDB date to the DE441 reader's single-f64 epoch.
    /// Adding the parts loses sub-millisecond precision near modern dates.
    /// No time-scale conversion is performed.
    pub fn to_de441_epoch(self) -> Result<JulianDateTdb, CalendarError> {
        if self.scale != TimeScale::Tdb {
            return Err(CalendarError::WrongTimeScale);
        }
        let (first, second) = self.parts()?;
        Ok(JulianDateTdb(first + second))
    }

    /// Convert to the selected proleptic calendar without changing the time scale.
    pub fn to_calendar(self, calendar: Calendar) -> Result<CalendarDateTime, CalendarError> {
        if self.day_number < first_day_number(calendar, MIN_YEAR)
            || self.day_number >= first_day_number(calendar, MAX_YEAR + 1)
        {
            return Err(CalendarError::YearOutOfRange);
        }
        let (mut lo, mut hi) = (MIN_YEAR, MAX_YEAR + 1);
        while lo + 1 < hi {
            let mid = lo + (hi - lo) / 2;
            if first_day_number(calendar, mid) <= self.day_number {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let mut day_of_year = self.day_number - first_day_number(calendar, lo);
        let mut month = 1;
        while day_of_year
            >= i64::from(month_days(calendar, lo, month).expect("valid calendar month"))
        {
            day_of_year -=
                i64::from(month_days(calendar, lo, month).expect("valid calendar month"));
            month += 1;
        }
        let seconds = self.midnight_fraction * SECONDS_PER_DAY;
        if seconds >= SECONDS_PER_DAY {
            return Err(CalendarError::UnrepresentableTime);
        }
        let hour = (seconds / 3600.0).floor() as u8;
        let remaining = seconds - f64::from(hour) * 3600.0;
        let minute = (remaining / 60.0).floor() as u8;
        let second = remaining - f64::from(minute) * 60.0;
        Ok(CalendarDateTime {
            year: lo,
            month,
            day: (day_of_year + 1) as u8,
            hour,
            minute,
            second,
            scale: self.scale,
        })
    }
}


#[cfg(test)]
mod tdb_seconds_tests {
    use super::*;

    #[test]
    fn j2000_noon_is_zero_and_following_midnight_is_half_day() {
        let j2000 = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap();
        assert_eq!(j2000.to_tdb_seconds_past_j2000().unwrap().value(), 0.0);

        let midnight = JulianDate::new(2_451_546, 0.0, TimeScale::Tdb).unwrap();
        assert_eq!(
            midnight.to_tdb_seconds_past_j2000().unwrap().value(),
            43_200.0
        );
    }

    #[test]
    fn split_date_preserves_subsecond_detail_lost_by_single_julian_day() {
        let epoch =
            JulianDate::new(2_451_545, 0.5 + 0.25 / SECONDS_PER_DAY, TimeScale::Tdb).unwrap();
        let accurate = epoch.to_tdb_seconds_past_j2000().unwrap().value();
        assert!((accurate - 0.25).abs() < 1e-9);

        // This is only a demonstration of the lossy f64-JD conversion, not
        // an alternate astronomical algorithm or time-scale transformation.
        let (day, fraction) = epoch.parts().unwrap();
        let collapsed = (day + fraction - J2000_JD_TDB) * SECONDS_PER_DAY;
        assert!((collapsed - 0.25).abs() > 1e-6);
    }

    #[test]
    fn modern_epoch_keeps_day_offset_and_fraction_signs() {
        let modern = JulianDate::new(2_460_000, 0.625, TimeScale::Tdb).unwrap();
        let expected = (2_460_000.0 - J2000_JD_TDB) * SECONDS_PER_DAY
            + 0.125 * SECONDS_PER_DAY;
        let et = modern.to_tdb_seconds_past_j2000().unwrap().value();
        assert!((et - expected).abs() < 1e-6);
        assert!(et > 0.0);

        let before = JulianDate::new(2_451_544, 0.5, TimeScale::Tdb).unwrap();
        assert_eq!(
            before.to_tdb_seconds_past_j2000().unwrap().value(),
            -SECONDS_PER_DAY
        );
    }

    #[test]
    fn rejects_non_tdb_scales_and_unrepresentable_days() {
        for scale in [TimeScale::Tai, TimeScale::Tt, TimeScale::Ut1] {
            let epoch = JulianDate::new(2_451_545, 0.5, scale).unwrap();
            assert_eq!(
                epoch.to_tdb_seconds_past_j2000(),
                Err(CalendarError::WrongTimeScale)
            );
        }
        let out_of_range = JulianDate::new(1_i64 << 52, 0.5, TimeScale::Tdb).unwrap();
        assert_eq!(
            out_of_range.to_tdb_seconds_past_j2000(),
            Err(CalendarError::YearOutOfRange)
        );
    }
}
