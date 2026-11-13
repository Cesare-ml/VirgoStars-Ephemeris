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

//! Preserve sub-millisecond ET detail in an existing split Julian-day model.
//!
//! The conversion is TDB -> TDB only. It does NOT calculate TT/TDB offsets,
//! leap seconds, UTC, Earth orientation or any planetary state.

use crate::calendar::{CalendarError, JulianDate, TimeScale};
use crate::time::{TdbSecondsPastJ2000, J2000_JD_TDB, SECONDS_PER_DAY};

/// Convert TDB seconds since J2000 into a split TDB Julian date.
///
/// J2000 is JD(TDB) 2451545.0 at noon, represented here as
/// `JulianDate::new(2451545, 0.5, TimeScale::Tdb)`. Integer day offsets
/// and the fractional day are calculated separately to avoid adding tiny
/// offsets directly to a multi-million-day floating-point Julian date.
///
/// Epoch input is still an f64: times smaller than its own ULP cannot be
/// recovered. The split date has a finite fraction precision too. The
/// existing classic DE441 reader still expects a single-f64 Julian day;
/// this function does not improve that reader's epoch precision.
///
/// Change (2026-11-13): additive epoch-format conversion, CPAL-1.0.
pub fn split_julian_day_tdb_from_et(
    epoch: TdbSecondsPastJ2000,
) -> Result<JulianDate, CalendarError> {
    let seconds = epoch.value();
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let remainder = seconds.rem_euclid(SECONDS_PER_DAY);

    if !days.is_finite() || !(0.0..SECONDS_PER_DAY).contains(&remainder) {
        return Err(CalendarError::UnrepresentableTime);
    }
    let base_day = J2000_JD_TDB + days;
    // Integers and half-days must be exactly representable in the split API.
    // Keep one extra day of headroom for a midnight rollover below.
    if !base_day.is_finite() || base_day.abs() >= ((1_u64 << 51) as f64 - 1.0) {
        return Err(CalendarError::YearOutOfRange);
    }

    let candidate_fraction = 0.5 + remainder / SECONDS_PER_DAY;
    let (day_number, midnight_fraction) = if candidate_fraction >= 1.0 {
        (base_day as i64 + 1, candidate_fraction - 1.0)
    } else {
        (base_day as i64, candidate_fraction)
    };
    let split = JulianDate::new(day_number, midnight_fraction, TimeScale::Tdb)?;
    split.parts()?;
    Ok(split)
}

/// Results of auditing a TDB ET-to-split-JD representation change.
///
/// The reported residual compares two binary64 representations of the *given*
/// ET input. It is not a bound on TDB modeling, clock accuracy, or ephemeris
/// position error.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuditedSplitJulianDayTdb {
    pub julian_date: JulianDate,
    pub roundtrip_error_seconds: f64,
}

/// Invalid precision budget, date representation, or detected quantization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SplitEpochAuditError {
    InvalidErrorBudget,
    InvalidJulianDate(CalendarError),
    PrecisionLoss {
        error_seconds: f64,
        max_error_seconds: f64,
    },
}

impl std::fmt::Display for SplitEpochAuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidErrorBudget => {
                write!(
                    f,
                    "ET-to-split-JD precision budget must be finite and nonnegative"
                )
            }
            Self::InvalidJulianDate(error) => write!(f, "invalid split JD: {error}"),
            Self::PrecisionLoss {
                error_seconds,
                max_error_seconds,
            } => write!(
                f,
                "ET-to-split-JD roundtrip loses {error_seconds} s, exceeding {max_error_seconds} s"
            ),
        }
    }
}

impl std::error::Error for SplitEpochAuditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidJulianDate(error) => Some(error),
            _ => None,
        }
    }
}

/// Convert an already-TDB ET epoch into a split JD and audit the roundtrip.
///
/// The budget is in TDB seconds. The check rejects a detected binary64
/// ET-to-split-to-ET residual larger than the caller's finite, nonnegative
/// tolerance. It cannot recover resolution already lost in the ET input,
/// and is NOT a rigorous forward-error bound for arbitrary huge epochs.
///
/// For a strict split-JD -> legacy-DE441 single-f64-JD budget, use the
/// independently developed `de441_epoch_bridge` API (PR #136); this helper
/// checks the *preceding* ET -> split step only.
///
/// Modified 2026-11-14: opt-in ET-to-split roundtrip precision audit.
pub fn audited_split_julian_day_tdb_from_et(
    epoch: TdbSecondsPastJ2000,
    max_roundtrip_error_seconds: f64,
) -> Result<AuditedSplitJulianDayTdb, SplitEpochAuditError> {
    if !max_roundtrip_error_seconds.is_finite() || max_roundtrip_error_seconds < 0.0 {
        return Err(SplitEpochAuditError::InvalidErrorBudget);
    }
    let julian_date =
        split_julian_day_tdb_from_et(epoch).map_err(SplitEpochAuditError::InvalidJulianDate)?;

    let days_from_j2000 = julian_date.day_number() - J2000_JD_TDB as i64;
    let subday_seconds = (julian_date.midnight_fraction() - 0.5) * SECONDS_PER_DAY;
    let reconstructed_et = (days_from_j2000 as f64).mul_add(SECONDS_PER_DAY, subday_seconds);
    let error_seconds = (reconstructed_et - epoch.value()).abs();
    if !error_seconds.is_finite() {
        return Err(SplitEpochAuditError::InvalidJulianDate(
            CalendarError::UnrepresentableTime,
        ));
    }
    if error_seconds > max_roundtrip_error_seconds {
        return Err(SplitEpochAuditError::PrecisionLoss {
            error_seconds,
            max_error_seconds: max_roundtrip_error_seconds,
        });
    }
    Ok(AuditedSplitJulianDayTdb {
        julian_date,
        roundtrip_error_seconds: error_seconds,
    })
}
