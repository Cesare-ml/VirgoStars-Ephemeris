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

//! Offline parser and guarded interpolation of externally supplied EOP
//! polar-motion values from the documented **CelesTrak EOP CSV** format.
//!
//! Header: DATE,MJD,X,Y,UT1-UTC,LOD,DPSI,DEPS,DX,DY,DAT,DATA_TYPE
//! X/Y are arcseconds at 0h UTC; MJD is the integer UTC day number;
//! O means observed and P means predicted. Conversion to radians is
//! explicit. Predictions require an explicit opt-in at query time.
//!
//! No download, network dependency, model extrapolation or implicit
//! UTC->UT1/TT conversion. UT1-UTC and DAT are retained as *daily
//! metadata only*, not interpolated across potential leap seconds.
//!
//! Format and independent historical sample: CelesTrak EOP docs
//! <https://celestrak.org/SpaceData/EOP-format.php>
//! Original VirgoStars parser and interpolation, 2026-11-12.

use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

use crate::calendar::{Calendar, CalendarDateTime, TimeScale};

const HEADER: &str = "DATE,MJD,X,Y,UT1-UTC,LOD,DPSI,DEPS,DX,DY,DAT,DATA_TYPE";
const RADIANS_PER_ARCSECOND: f64 = PI / (180.0 * 3_600.0);
/// Matches the polar-motion mathematical input guard; not an IERS physical bound.
const MAX_POLE_ABS_RAD: f64 = 0.01;
const MJD_ZERO_JULIAN_DAY_NUMBER: i64 = 2_400_001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EopRecordType {
    Observed,
    Predicted,
}

/// Explicit policy. ObservedOnly is the default for the query API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EopPredictionPolicy {
    ObservedOnly,
    AllowPredicted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EopPolarQuality {
    Observed,
    IncludesPrediction,
}

/// One validated daily row, sampled at **0h UTC** (MJD integer).
/// No assertion is made that its dataset is authentic or updated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DailyEopPolarMotion {
    pub mjd_utc: i64,
    pub xp_rad: f64,
    pub yp_rad: f64,
    pub ut1_minus_utc_seconds_at_0h: f64,
    pub tai_minus_utc_seconds_at_0h: i32,
    pub record_type: EopRecordType,
}

/// Linear xp/yp interpolation between consecutive **UTC-labeled** days.
///
/// This is NOT a TT-based sample, nor a UT1 conversion. The caller
/// supplies the UTC day fraction from an independently valid UTC
/// civil-time/leap-second interpretation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InterpolatedEopPolarMotion {
    pub mjd_utc: i64,
    pub fraction_of_utc_day: f64,
    pub xp_rad: f64,
    pub yp_rad: f64,
    pub quality: EopPolarQuality,
}

/// Validated sorted observations/predictions from an externally provided
/// snapshot. The nonempty source label is for audit, NOT authenticity.
#[derive(Clone, Debug, PartialEq)]
pub struct CelestrakEopPolarCatalog {
    pub source_label: String,
    samples: Vec<DailyEopPolarMotion>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EopCsvError {
    MissingSourceLabel,
    InvalidHeader,
    EmptyCatalog,
    MalformedRow { line: usize },
    InvalidPolarCoordinate { line: usize },
    DateMjdMismatch { line: usize },
    UnsortedOrDuplicateDays { line: usize },
    InvalidUtcFraction,
    DayOutsideCatalog,
    MissingFollowingDay,
    PredictionNotAuthorized,
}

impl fmt::Display for EopCsvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSourceLabel => write!(f, "EOP data require a nonempty source label"),
            Self::InvalidHeader => write!(f, "unknown EOP CSV header or column order"),
            Self::EmptyCatalog => write!(f, "EOP CSV has no daily rows"),
            Self::MalformedRow { line } => write!(f, "malformed EOP CSV row at line {line}"),
            Self::InvalidPolarCoordinate { line } => {
                write!(f, "nonfinite or implausible EOP pole angle at line {line}")
            }
            Self::DateMjdMismatch { line } => {
                write!(f, "UTC Gregorian date and MJD disagree at line {line}")
            }
            Self::UnsortedOrDuplicateDays { line } => {
                write!(f, "EOP days are not strictly increasing at line {line}")
            }
            Self::InvalidUtcFraction => {
                write!(f, "UTC day fraction must be finite in [0,1)")
            }
            Self::DayOutsideCatalog => write!(f, "requested UTC MJD is not in the EOP catalog"),
            Self::MissingFollowingDay => {
                write!(f, "linear interpolation needs the next consecutive UTC day")
            }
            Self::PredictionNotAuthorized => {
                write!(
                    f,
                    "EOP predicted data require explicit AllowPredicted policy"
                )
            }
        }
    }
}

impl Error for EopCsvError {}

fn parse_utc_date_jdn(date: &str) -> Option<i64> {
    let b = date.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    if b.iter()
        .enumerate()
        .any(|(i, c)| i != 4 && i != 7 && !c.is_ascii_digit())
    {
        return None;
    }
    let year: i32 = date[0..4].parse().ok()?;
    let month: u8 = date[5..7].parse().ok()?;
    let day: u8 = date[8..10].parse().ok()?;
    // Reuse existing Gregorian calendar validation/ordinal arithmetic.
    // This TimeScale tag is ONLY a dummy for date-only arithmetic:
    // no claim UTC=UT1 or UT1 leap-second behavior is made.
    let civil_ordinal = CalendarDateTime {
        year,
        month,
        day,
        hour: 0,
        minute: 0,
        second: 0.0,
        scale: TimeScale::Ut1,
    }
    .to_julian_date(Calendar::ProlepticGregorian)
    .ok()?;
    Some(civil_ordinal.day_number())
}

fn parse_row(line: &str, line_number: usize) -> Result<DailyEopPolarMotion, EopCsvError> {
    let invalid = || EopCsvError::MalformedRow { line: line_number };
    let fields: Vec<_> = line.split(',').map(str::trim).collect();
    if fields.len() != 12 || fields.iter().any(|f| f.is_empty()) {
        return Err(invalid());
    }
    let jdn = parse_utc_date_jdn(fields[0]).ok_or_else(invalid)?;
    let mjd_utc: i64 = fields[1].parse().map_err(|_| invalid())?;
    if jdn - MJD_ZERO_JULIAN_DAY_NUMBER != mjd_utc {
        return Err(EopCsvError::DateMjdMismatch { line: line_number });
    }

    let numeric = |index: usize| -> Result<f64, EopCsvError> {
        let value = fields[index].parse::<f64>().map_err(|_| invalid())?;
        if !value.is_finite() {
            return Err(invalid());
        }
        Ok(value)
    };
    let xp_rad = numeric(2)? * RADIANS_PER_ARCSECOND;
    let yp_rad = numeric(3)? * RADIANS_PER_ARCSECOND;
    if !xp_rad.is_finite()
        || !yp_rad.is_finite()
        || xp_rad.abs() > MAX_POLE_ABS_RAD
        || yp_rad.abs() > MAX_POLE_ABS_RAD
    {
        return Err(EopCsvError::InvalidPolarCoordinate { line: line_number });
    }
    let ut1_minus_utc_seconds_at_0h = numeric(4)?;
    // Validate all documented numeric fields, even if unused in this
    // first polar-motion-only API. They cannot silently hold NaN.
    for index in 5..=9 {
        let _ = numeric(index)?;
    }
    let tai_minus_utc_seconds_at_0h = fields[10].parse().map_err(|_| invalid())?;
    let record_type = match fields[11] {
        "O" => EopRecordType::Observed,
        "P" => EopRecordType::Predicted,
        _ => return Err(invalid()),
    };
    Ok(DailyEopPolarMotion {
        mjd_utc,
        xp_rad,
        yp_rad,
        ut1_minus_utc_seconds_at_0h,
        tai_minus_utc_seconds_at_0h,
        record_type,
    })
}

impl CelestrakEopPolarCatalog {
    /// Parse CelesTrak's documented EOP CSV **already supplied by the caller**.
    ///
    /// Accepts a UTF-8 BOM, CRLF and blank lines. Requires the exact
    /// named 12-column CSV header, finite numeric values, valid
    /// Gregorian dates consistent with the UTC MJD, strictly increasing
    /// sample days, and O/P record types. It does NOT verify a network
    /// source, snapshot signature, freshness, IERS realization, or
    /// scientific consistency of UT1-UTC / leap-second metadata.
    pub fn parse_csv(source_label: &str, csv: &str) -> Result<Self, EopCsvError> {
        if source_label.trim().is_empty() {
            return Err(EopCsvError::MissingSourceLabel);
        }
        let mut lines = csv.trim_start_matches('\u{feff}').lines().enumerate();
        let (first_line, first_text) = lines
            .find(|(_, line)| !line.trim().is_empty())
            .ok_or(EopCsvError::InvalidHeader)?;
        if first_text.trim() != HEADER {
            return Err(EopCsvError::InvalidHeader);
        }
        let mut samples: Vec<DailyEopPolarMotion> = Vec::new();
        for (index, line) in lines {
            if line.trim().is_empty() {
                continue;
            }
            let parsed = parse_row(line, index + 1)?;
            if samples
                .last()
                .is_some_and(|last| parsed.mjd_utc <= last.mjd_utc)
            {
                return Err(EopCsvError::UnsortedOrDuplicateDays { line: index + 1 });
            }
            samples.push(parsed);
        }
        if samples.is_empty() {
            return Err(EopCsvError::EmptyCatalog);
        }
        // first_line is preserved in enumeration offsets, ensuring
        // reported row numbers refer to the actual uploaded CSV.
        let _ = first_line;
        Ok(Self {
            source_label: source_label.to_owned(),
            samples,
        })
    }

    pub fn samples(&self) -> &[DailyEopPolarMotion] {
        &self.samples
    }

    /// Interpolate only between adjacent 0h UTC EOP samples, never
    /// extrapolate. Predicted records are rejected by default.
    ///
    /// `fraction_of_utc_day` must be a caller-interpreted fraction
    /// of the actual UTC civil day. This function does not compute
    /// day lengths, leap seconds or UT1/TT transformations; it also
    /// does not interpolate the UT1-UTC field across leap seconds.
    pub fn interpolate_observed(
        &self,
        mjd_utc: i64,
        fraction_of_utc_day: f64,
    ) -> Result<InterpolatedEopPolarMotion, EopCsvError> {
        self.interpolate(
            mjd_utc,
            fraction_of_utc_day,
            EopPredictionPolicy::ObservedOnly,
        )
    }

    /// Explicitly configurable prediction policy; use only if an
    /// application authorizes the error budget and prediction source.
    pub fn interpolate(
        &self,
        mjd_utc: i64,
        fraction_of_utc_day: f64,
        prediction_policy: EopPredictionPolicy,
    ) -> Result<InterpolatedEopPolarMotion, EopCsvError> {
        if !fraction_of_utc_day.is_finite() || !(0.0..1.0).contains(&fraction_of_utc_day) {
            return Err(EopCsvError::InvalidUtcFraction);
        }
        let index = self
            .samples
            .binary_search_by_key(&mjd_utc, |row| row.mjd_utc)
            .map_err(|_| EopCsvError::DayOutsideCatalog)?;
        let lower = self.samples[index];
        let upper = if fraction_of_utc_day == 0.0 {
            lower
        } else {
            let next = self
                .samples
                .get(index + 1)
                .ok_or(EopCsvError::MissingFollowingDay)?;
            if next.mjd_utc
                != mjd_utc
                    .checked_add(1)
                    .ok_or(EopCsvError::MissingFollowingDay)?
            {
                return Err(EopCsvError::MissingFollowingDay);
            }
            *next
        };
        let quality = if lower.record_type == EopRecordType::Observed
            && upper.record_type == EopRecordType::Observed
        {
            EopPolarQuality::Observed
        } else {
            EopPolarQuality::IncludesPrediction
        };
        if quality == EopPolarQuality::IncludesPrediction
            && prediction_policy == EopPredictionPolicy::ObservedOnly
        {
            return Err(EopCsvError::PredictionNotAuthorized);
        }
        let xp_rad = (upper.xp_rad - lower.xp_rad).mul_add(fraction_of_utc_day, lower.xp_rad);
        let yp_rad = (upper.yp_rad - lower.yp_rad).mul_add(fraction_of_utc_day, lower.yp_rad);
        Ok(InterpolatedEopPolarMotion {
            mjd_utc,
            fraction_of_utc_day,
            xp_rad,
            yp_rad,
            quality,
        })
    }
}
