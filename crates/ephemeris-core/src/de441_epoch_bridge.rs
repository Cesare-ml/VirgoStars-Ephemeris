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

//! Explicit precision guard for a split TDB Julian date passed to classic DE441.
//!
//! This converts no time scales and does not increase the precision of the
//! existing DE441 reader: that reader consumes a single `f64` Julian date.
//! It makes the quantization of a caller's split date measurable and optional
//! to reject. For fine-grained timeline navigation, keep using split dates
//! until the actual reader accepts them natively.

use std::error::Error;
use std::fmt;

use crate::calendar::{CalendarError, JulianDate, TimeScale};
use crate::de441::JulianDateTdb;
use crate::time::SECONDS_PER_DAY;

/// Error when preparing an already-TDB split epoch for classic DE441.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EpochBridgeError {
    /// The caller must provide a nonnegative, finite error budget in seconds.
    InvalidErrorBudget,
    /// Only TDB input is valid; this helper never converts UTC, TT, TAI or UT1.
    WrongTimeScale,
    /// The split date cannot be represented as valid floating-point parts.
    InvalidJulianDate(CalendarError),
    /// The requested precision cannot survive conversion to one f64 JD.
    PrecisionLoss {
        error_seconds: f64,
        max_error_seconds: f64,
    },
}

impl fmt::Display for EpochBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidErrorBudget => {
                write!(f, "DE441 epoch error budget must be finite and nonnegative")
            }
            Self::WrongTimeScale => write!(f, "DE441 epoch input must already be TDB"),
            Self::InvalidJulianDate(error) => write!(f, "invalid split Julian date: {error}"),
            Self::PrecisionLoss {
                error_seconds,
                max_error_seconds,
            } => write!(
                f,
                "DE441 f64 Julian date loses {error_seconds} seconds, exceeding the {max_error_seconds} second error budget"
            ),
        }
    }
}

impl Error for EpochBridgeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Self::InvalidJulianDate(error) = self {
            Some(error)
        } else {
            None
        }
    }
}

/// A legacy single-f64 DE441 epoch and its measured input-rounding error.
///
/// The error measures only the split-JD-to-f64 conversion. It is **not** a
/// prediction of planetary-position error or a validated astronomical bound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckedDe441Epoch {
    pub epoch: JulianDateTdb,
    pub quantization_error_seconds: f64,
}

/// Convert a split Julian date to the existing classic-DE441 epoch, failing
/// closed when the input representation loses more than the caller allows.
///
/// The input must already be TDB. The positive/zero precision budget is
/// expressed in SI seconds per conventional 86,400-second TDB day; no UTC
/// leap-second or TDB/TT conversion is performed.
///
/// The subtraction `(rounded - day_part) - subday_part` avoids measuring
/// rounding by first subtracting two entire multi-million-day Julian dates.
pub fn checked_de441_epoch(
    split: JulianDate,
    max_error_seconds: f64,
) -> Result<CheckedDe441Epoch, EpochBridgeError> {
    if !max_error_seconds.is_finite() || max_error_seconds < 0.0 {
        return Err(EpochBridgeError::InvalidErrorBudget);
    }
    if split.scale() != TimeScale::Tdb {
        return Err(EpochBridgeError::WrongTimeScale);
    }
    let (day_part, subday_part) = split
        .parts()
        .map_err(EpochBridgeError::InvalidJulianDate)?;
    let rounded = day_part + subday_part;
    if !rounded.is_finite() {
        return Err(EpochBridgeError::InvalidJulianDate(
            CalendarError::UnrepresentableTime,
        ));
    }
    let error_seconds = ((rounded - day_part) - subday_part).abs() * SECONDS_PER_DAY;
    if !error_seconds.is_finite() {
        return Err(EpochBridgeError::InvalidJulianDate(
            CalendarError::UnrepresentableTime,
        ));
    }
    if error_seconds > max_error_seconds {
        return Err(EpochBridgeError::PrecisionLoss {
            error_seconds,
            max_error_seconds,
        });
    }
    Ok(CheckedDe441Epoch {
        epoch: JulianDateTdb(rounded),
        quantization_error_seconds: error_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::CalendarError;

    #[test]
    fn exact_j2000_noon_requires_no_precision_budget() {
        let split = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap();
        let result = checked_de441_epoch(split, 0.0).unwrap();
        assert_eq!(result.epoch, JulianDateTdb(2_451_545.0));
        assert_eq!(result.quantization_error_seconds, 0.0);
    }

    #[test]
    fn quarter_second_at_j2000_measures_loss_and_checks_budget() {
        let split =
            JulianDate::new(2_451_545, 0.5 + 0.25 / SECONDS_PER_DAY, TimeScale::Tdb).unwrap();
        let relaxed = checked_de441_epoch(split, 0.001).unwrap();
        assert!(relaxed.quantization_error_seconds > 0.000_001);
        assert!(relaxed.quantization_error_seconds < 0.000_050);
        assert_eq!(relaxed.epoch, split.to_de441_epoch().unwrap());

        assert!(matches!(
            checked_de441_epoch(split, 0.000_001),
            Err(EpochBridgeError::PrecisionLoss {
                error_seconds,
                max_error_seconds
            }) if error_seconds > 0.000_001 && max_error_seconds == 0.000_001
        ));
    }

    #[test]
    fn subday_fraction_lost_at_midnight_is_rejected() {
        let split = JulianDate::new(2_460_000, 0.000_000_000_1, TimeScale::Tdb).unwrap();
        assert!(matches!(
            checked_de441_epoch(split, 0.000_001),
            Err(EpochBridgeError::PrecisionLoss { .. })
        ));
        assert!(checked_de441_epoch(split, 0.001).is_ok());
    }

    #[test]
    fn exact_historical_epoch_needs_no_extra_tolerance() {
        let split = JulianDate::new(-1_000_000, 0.5, TimeScale::Tdb).unwrap();
        assert_eq!(
            checked_de441_epoch(split, 0.0).unwrap().epoch,
            JulianDateTdb(-1_000_000.0)
        );
    }

    #[test]
    fn rejects_wrong_scale_without_implicitly_converting() {
        for scale in [TimeScale::Tai, TimeScale::Tt, TimeScale::Ut1] {
            let split = JulianDate::new(2_451_545, 0.5, scale).unwrap();
            assert_eq!(
                checked_de441_epoch(split, 1.0),
                Err(EpochBridgeError::WrongTimeScale)
            );
        }
    }

    #[test]
    fn rejects_invalid_budgets_and_out_of_range_split_day() {
        let split = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap();
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            assert_eq!(
                checked_de441_epoch(split, invalid),
                Err(EpochBridgeError::InvalidErrorBudget)
            );
        }
        let huge = JulianDate::new(1_i64 << 52, 0.5, TimeScale::Tdb).unwrap();
        assert_eq!(
            checked_de441_epoch(huge, 1.0),
            Err(EpochBridgeError::InvalidJulianDate(
                CalendarError::YearOutOfRange
            ))
        );
    }
}
