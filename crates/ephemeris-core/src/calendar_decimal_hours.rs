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

//! Strict decimal-hour calendar/JD interface for future native and language bindings.
//!
//! This is **calendar arithmetic only**, not a Swiss Ephemeris adapter:
//! no UTC->UT1 or TT/TDB conversion, time zones, leap seconds, ephemerides,
//! implicit calendar changeover or historical Delta-T policy.
//!
//! Created 2026-11-11: independent API adapting the existing Rust calendar core.
//! No numerical method or data from Swiss Ephemeris is incorporated.

use crate::calendar::{Calendar, CalendarDateTime, CalendarError, JulianDate, TimeScale};

/// Proleptic calendar date with decimal hours measured since local midnight
/// on the caller's already-declared uniform time scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecimalHourDateTime {
    /// Astronomical year numbering: 0 = 1 BCE.
    pub year: i32,
    pub month: u8,
    pub day: u8,
    /// Decimal hours in [0, 24). No implicit carry to the next day.
    pub decimal_hour: f64,
    /// This is a label, not a request to convert between time scales.
    pub scale: TimeScale,
}

/// Convert a strict date plus decimal hours into a precision-preserving split
/// Julian day, using the caller-selected proleptic calendar.
///
/// This is intended as a building block for a future `swe_julday`-shaped
/// facade. Contrary to potentially permissive external APIs, invalid dates,
/// negative hours and hours >= 24 are rejected explicitly. The time-scale
/// label is preserved without performing UTC-to-UT1, TT or TDB conversion.
pub fn to_split_julian_date(
    input: DecimalHourDateTime,
    calendar: Calendar,
) -> Result<JulianDate, CalendarError> {
    if !input.decimal_hour.is_finite() || !(0.0..24.0).contains(&input.decimal_hour) {
        return Err(CalendarError::InvalidTime);
    }

    // Reuse canonical calendar validation and civil-date/JDN arithmetic.
    // Avoid adding the fractional hour to a large single-f64 Julian date.
    let midnight = CalendarDateTime {
        year: input.year,
        month: input.month,
        day: input.day,
        hour: 0,
        minute: 0,
        second: 0.0,
        scale: input.scale,
    }
    .to_julian_date(calendar)?;
    JulianDate::new(
        midnight.day_number(),
        input.decimal_hour / 24.0,
        input.scale,
    )
}

/// Convert a split Julian day to the caller-selected proleptic calendar and
/// decimal hour. No time-scale conversion is performed.
pub fn from_split_julian_date(
    epoch: JulianDate,
    calendar: Calendar,
) -> Result<DecimalHourDateTime, CalendarError> {
    let civil = epoch.to_calendar(calendar)?;
    let decimal_hour = epoch.midnight_fraction() * 24.0;
    if !decimal_hour.is_finite() || !(0.0..24.0).contains(&decimal_hour) {
        return Err(CalendarError::UnrepresentableTime);
    }
    Ok(DecimalHourDateTime {
        year: civil.year,
        month: civil.month,
        day: civil.day,
        decimal_hour,
        scale: epoch.scale(),
    })
}

/// Return a single-f64 Julian day for legacy-style callers.
///
/// The single number necessarily loses submillisecond resolution around
/// modern dates; prefer `to_split_julian_date` where possible. This function
/// does not provide exact Swiss binding ABI, defaults or calendar leniency.
pub fn julday_f64(input: DecimalHourDateTime, calendar: Calendar) -> Result<f64, CalendarError> {
    let epoch = to_split_julian_date(input, calendar)?;
    let (integer, fraction) = epoch.parts()?;
    Ok(integer + fraction)
}

/// Reverse a legacy single-f64 Julian day to a strict calendar/decimal-hour
/// result, retaining the supplied time-scale label.
pub fn revjul_f64(
    jd: f64,
    calendar: Calendar,
    scale: TimeScale,
) -> Result<DecimalHourDateTime, CalendarError> {
    from_split_julian_date(JulianDate::from_f64(jd, scale)?, calendar)
}
