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

//! Minimal typed TDB/ET epoch representation.
//!
//! This module deliberately does **not** convert UTC, UT1, TAI, or TT to TDB.
//! Such conversions require explicit time-scale models and, for UTC, leap
//! second handling. Passing a UTC Julian day as TDB would be a silent error.

use crate::calendar::{CalendarError, JulianDate, TimeScale};
use crate::error::{EphemerisError, Result};

/// Julian date corresponding to the J2000 reference epoch on the TDB scale.
pub const J2000_JD_TDB: f64 = 2_451_545.0;
pub const SECONDS_PER_DAY: f64 = 86_400.0;

/// Conventional offset of Terrestrial Time (TT) from International Atomic Time (TAI).
///
/// TT is realized in practice using TAI + 32.184 SI seconds. This is not
/// the UTC leap-second offset, nor a conversion between TT and TDB.
pub const TT_MINUS_TAI_SECONDS: f64 = 32.184;

/// Errors converting between the TAI and TT labels of a split Julian date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicTimeError {
    /// The input was not expressed on the scale required by the conversion.
    WrongInputScale {
        expected: TimeScale,
        actual: TimeScale,
    },
    /// A Julian-date part is invalid or exceeds the representable range.
    InvalidJulianDate(CalendarError),
}

impl std::fmt::Display for AtomicTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongInputScale { expected, actual } => {
                write!(f, "expected {expected:?} input, received {actual:?}")
            }
            Self::InvalidJulianDate(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for AtomicTimeError {}

impl From<CalendarError> for AtomicTimeError {
    fn from(error: CalendarError) -> Self {
        Self::InvalidJulianDate(error)
    }
}

fn offset_atomic_date(
    epoch: JulianDate,
    input: TimeScale,
    output: TimeScale,
    offset_seconds: f64,
) -> std::result::Result<JulianDate, AtomicTimeError> {
    if epoch.scale() != input {
        return Err(AtomicTimeError::WrongInputScale {
            expected: input,
            actual: epoch.scale(),
        });
    }
    // Reject integral day numbers that cannot be represented exactly as f64.
    epoch.parts()?;

    // Keep the integral day separate from sub-day seconds to avoid an
    // avoidable precision loss on an ordinary single-f64 Julian date.
    let shifted_fraction = epoch.midnight_fraction() + offset_seconds / SECONDS_PER_DAY;
    let rollover: i64 = if shifted_fraction >= 1.0 {
        1
    } else if shifted_fraction < 0.0 {
        -1
    } else {
        0
    };
    let day =
        epoch
            .day_number()
            .checked_add(rollover)
            .ok_or(AtomicTimeError::InvalidJulianDate(
                CalendarError::YearOutOfRange,
            ))?;
    let shifted = JulianDate::new(day, shifted_fraction - rollover as f64, output)?;
    shifted.parts()?;
    Ok(shifted)
}

/// Convert a split Julian date in TAI to TT using the conventional offset.
///
/// TT = TAI + 32.184 seconds. This does not convert UTC to TAI and
/// does not calculate TT↔TDB. The date's scale must be `TimeScale::Tai`.
pub fn tai_to_tt(epoch: JulianDate) -> std::result::Result<JulianDate, AtomicTimeError> {
    offset_atomic_date(epoch, TimeScale::Tai, TimeScale::Tt, TT_MINUS_TAI_SECONDS)
}

/// Convert a split Julian date in TT back to TAI.
///
/// TAI = TT - 32.184 seconds. Input must explicitly be `TimeScale::Tt`.
pub fn tt_to_tai(epoch: JulianDate) -> std::result::Result<JulianDate, AtomicTimeError> {
    offset_atomic_date(epoch, TimeScale::Tt, TimeScale::Tai, -TT_MINUS_TAI_SECONDS)
}

/// SPK-style ephemeris time: TDB seconds past J2000.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct TdbSecondsPastJ2000(f64);

impl TdbSecondsPastJ2000 {
    pub fn new(seconds: f64) -> Result<Self> {
        if !seconds.is_finite() {
            return Err(EphemerisError::NonFinite {
                field: "TDB seconds",
            });
        }
        Ok(Self(seconds))
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

/// Julian day explicitly on the TDB time scale.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct JulianDayTdb(f64);

impl JulianDayTdb {
    pub fn new(julian_day: f64) -> Result<Self> {
        if !julian_day.is_finite() {
            return Err(EphemerisError::NonFinite {
                field: "TDB Julian day",
            });
        }
        Ok(Self(julian_day))
    }

    pub fn value(self) -> f64 {
        self.0
    }

    /// Simple JD(TDB) -> seconds past J2000(TDB) conversion.
    ///
    /// A single f64 Julian day loses sub-millisecond resolution at large
    /// epochs; a two-part JD representation is planned for precision work.
    pub fn to_tdb_seconds_past_j2000(self) -> Result<TdbSecondsPastJ2000> {
        let seconds = (self.0 - J2000_JD_TDB) * SECONDS_PER_DAY;
        if !seconds.is_finite() {
            return Err(EphemerisError::ArithmeticOverflow);
        }
        TdbSecondsPastJ2000::new(seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j2000_is_zero_et() {
        let et = JulianDayTdb::new(J2000_JD_TDB)
            .unwrap()
            .to_tdb_seconds_past_j2000()
            .unwrap();
        assert_eq!(et.value(), 0.0);
    }

    #[test]
    fn half_day_is_43200_seconds() {
        let et = JulianDayTdb::new(J2000_JD_TDB + 0.5)
            .unwrap()
            .to_tdb_seconds_past_j2000()
            .unwrap();
        assert_eq!(et.value(), 43_200.0);
    }

    #[test]
    fn tt_j2000_noon_is_tai_115927816() {
        // 2000-01-01 12:00:00 TT = 11:59:27.816 TAI.
        let tt_noon = JulianDate::new(2_451_545, 0.5, TimeScale::Tt).unwrap();
        let tai = tt_to_tai(tt_noon).unwrap();
        assert_eq!(tai.scale(), TimeScale::Tai);
        assert_eq!(tai.day_number(), 2_451_545);
        let tai_seconds = tai.midnight_fraction() * SECONDS_PER_DAY;
        assert!((tai_seconds - (12.0 * 3600.0 - TT_MINUS_TAI_SECONDS)).abs() < 1e-9);

        let recovered = tai_to_tt(tai).unwrap();
        assert_eq!(recovered.scale(), TimeScale::Tt);
        assert_eq!(recovered.day_number(), tt_noon.day_number());
        assert!(
            ((recovered.midnight_fraction() - tt_noon.midnight_fraction()) * SECONDS_PER_DAY).abs()
                < 1e-9
        );
    }

    #[test]
    fn addition_rolls_over_midnight_and_subtraction_rolls_back() {
        let tai = JulianDate::new(2_451_545, 1.0 - 10.0 / SECONDS_PER_DAY, TimeScale::Tai).unwrap();
        let tt = tai_to_tt(tai).unwrap();
        assert_eq!(tt.day_number(), 2_451_546);
        assert_eq!(tt.scale(), TimeScale::Tt);
        assert!((tt.midnight_fraction() * SECONDS_PER_DAY - 22.184).abs() < 1e-8);
        let tai_again = tt_to_tai(tt).unwrap();
        assert_eq!(tai_again.day_number(), tai.day_number());
        assert!(
            ((tai_again.midnight_fraction() - tai.midnight_fraction()) * SECONDS_PER_DAY).abs()
                < 1e-8
        );

        let early_tt = JulianDate::new(2_451_545, 10.0 / SECONDS_PER_DAY, TimeScale::Tt).unwrap();
        let previous_day_tai = tt_to_tai(early_tt).unwrap();
        assert_eq!(previous_day_tai.day_number(), 2_451_544);
        assert!(
            ((1.0 - previous_day_tai.midnight_fraction()) * SECONDS_PER_DAY - 22.184).abs() < 1e-8
        );
    }

    #[test]
    fn refuses_wrong_scale_in_both_directions() {
        for scale in [TimeScale::Tdb, TimeScale::Ut1, TimeScale::Tt] {
            let date = JulianDate::new(2_451_545, 0.5, scale).unwrap();
            assert_eq!(
                tai_to_tt(date),
                Err(AtomicTimeError::WrongInputScale {
                    expected: TimeScale::Tai,
                    actual: scale,
                })
            );
        }
        let tai = JulianDate::new(2_451_545, 0.5, TimeScale::Tai).unwrap();
        assert_eq!(
            tt_to_tai(tai),
            Err(AtomicTimeError::WrongInputScale {
                expected: TimeScale::Tt,
                actual: TimeScale::Tai,
            })
        );
    }

    #[test]
    fn rejects_unrepresentable_split_days_and_rollover() {
        let beyond_range = JulianDate::new(1_i64 << 52, 0.5, TimeScale::Tai).unwrap();
        assert_eq!(
            tai_to_tt(beyond_range),
            Err(AtomicTimeError::InvalidJulianDate(
                CalendarError::YearOutOfRange
            ))
        );

        let last_exact_day =
            JulianDate::new(1_i64 << 51, 1.0 - 1.0 / SECONDS_PER_DAY, TimeScale::Tai).unwrap();
        assert_eq!(
            tai_to_tt(last_exact_day),
            Err(AtomicTimeError::InvalidJulianDate(
                CalendarError::YearOutOfRange
            ))
        );
    }

    #[test]
    fn rejects_nonfinite_time() {
        assert!(JulianDayTdb::new(f64::NAN).is_err());
        assert!(TdbSecondsPastJ2000::new(f64::INFINITY).is_err());
    }
}
