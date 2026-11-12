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

//! Explicit EOP CSV polar motion + approximate TIO s′(TT) assembly.
//!
//! A caller-supplied **UTC** MJD and fraction select externally sourced
//! CelesTrak daily xp/yp samples; only the O/P quality-protected
//! linear interpolation is used. A **TT** epoch is derived from the
//! same UTC instant using the EOP row's integer TAI−UTC (DAT) plus
//! the definitional 32.184 seconds TT−TAI, **only for modern dates
//! and normal days on which DAT does not change**.
//!
//! This module fails closed across detected leap-second boundaries.
//! It neither parses leap-second instants nor invents UT1 from DUT1.
//! It delegates s′(TT) and polar W to the separately validated modules,
//! preserving their documented approximation and EOP limitations.
//!
//! CelesTrak field reference:
//! <https://celestrak.org/SpaceData/EOP-format.php>
//! Original VirgoStars integration. 2026-11-12.

use std::error::Error;
use std::fmt;

use crate::calendar::{CalendarError, JulianDate, TimeScale};
use crate::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopCsvError, EopPolarQuality, EopPredictionPolicy,
};
use crate::tio_polar_motion::{
    itrs_to_tirs_polar_motion_with_approximate_sp00, ApproximateTioPolarMotion,
    ApproximateTioPolarMotionError,
};

const MJD_ZERO_DAY_NUMBER: i64 = 2_400_001;
const FIRST_INTEGER_UTC_LEAP_DAY_MJD: i64 = 41_317; // 1972-01-01
const TT_MINUS_TAI_SECONDS: f64 = 32.184;
const SECONDS_PER_DAY: f64 = 86_400.0;

/// A strict numerical sanity range, NOT a guarantee of authentic
/// leap-second data or a forecast of future TAI−UTC steps.
const MIN_VALID_DAT_SECONDS: i32 = 10;
const MAX_VALID_DAT_SECONDS: i32 = 100;

/// Offline, explicitly quality-tagged Earth polar orientation.
///
/// `tt_epoch` comes from the supplied UTC day/fraction and a checked
/// integer DAT (TAI−UTC) in the *same EOP snapshot*; it is not inferred
/// from a system clock. The source label is retained for auditing but
/// cannot prove its own authenticity.
#[derive(Clone, Debug, PartialEq)]
pub struct EopSp00PolarOrientation {
    pub source_label: String,
    pub mjd_utc: i64,
    pub fraction_of_utc_day: f64,
    pub quality: EopPolarQuality,
    pub tai_minus_utc_seconds: i32,
    pub tt_epoch: JulianDate,
    pub xp_rad: f64,
    pub yp_rad: f64,
    pub approximate_polar: ApproximateTioPolarMotion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EopSp00PolarError {
    Eop(EopCsvError),
    /// Pre-1972 TAI−UTC used fractional-second adjustments; this
    /// date-only integer DAT bridge is intentionally unsupported.
    BeforeModernUtc,
    /// DAT is outside a deliberately conservative numerical range.
    UnsupportedTaiUtcOffset,
    /// The following 0h UTC record has a different DAT; this civil
    /// day may contain a leap second. Do not assume 86400 seconds.
    LeapSecondBoundary,
    JulianDayOverflow,
    TtEpoch(CalendarError),
    Polar(ApproximateTioPolarMotionError),
}

impl fmt::Display for EopSp00PolarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eop(error) => write!(f, "EOP polar CSV: {error}"),
            Self::BeforeModernUtc => write!(f, "integer-DAT UTC bridge requires 1972 or later"),
            Self::UnsupportedTaiUtcOffset => {
                write!(f, "unsupported TAI-minus-UTC daily offset")
            }
            Self::LeapSecondBoundary => {
                write!(f, "UTC day has a DAT change; leap-second handling is required")
            }
            Self::JulianDayOverflow => write!(f, "unrepresentable UTC MJD Julian day"),
            Self::TtEpoch(error) => write!(f, "could not form split TT epoch: {error}"),
            Self::Polar(error) => write!(f, "SP00 polar-motion composition: {error}"),
        }
    }
}

impl Error for EopSp00PolarError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Eop(error) => Some(error),
            Self::TtEpoch(error) => Some(error),
            Self::Polar(error) => Some(error),
            _ => None,
        }
    }
}

/// Assemble observed (or explicitly authorized predicted) pole
/// orientation from a supplied CSV catalog at one UTC date/fraction.
///
/// 1. Interpolate xp/yp in **radians** between adjacent 0h UTC EOP
///    records. Forecast use requires `AllowPredicted`.
/// 2. Read the **daily** integer DAT (TAI−UTC) recorded at 0h UTC.
///    If a fractional-day interpolation crosses a change to the next
///    record's DAT, reject the query: leap-second days need a full
///    UTC-second-of-day interpretation and must not be flattened.
/// 3. For the *normal 86400-SI-second UTC day*, form the split TT
///    epoch with `TT−UTC=DAT+32.184 seconds`, preserving fractional
///    precision and carrying across midnight when necessary.
/// 4. Call the **opted-in approximate** s′(TT) and ITRS→TIRS W
///    assembler with the interpolated polar measurements.
///
/// This is intentionally NOT a full IERS realization: the catalogue
/// does not authenticate an IERS data release, does not check that
/// DUT1/TAI−UTC fields match an external leap-second schedule, and
/// does not compute UT1, CIP, CIO, derivatives of observed pole motion
/// or a topocentric DE441 direction. Validate observation epoch and
/// data provenance separately before high-accuracy astronomy.
///
/// `fraction_of_utc_day` refers to a normal UTC day's **SI seconds
/// divided by 86400**, not a floating point TT/UT1 Julian date.
pub fn approximate_itrs_to_tirs_from_eop_utc(
    catalog: &CelestrakEopPolarCatalog,
    mjd_utc: i64,
    fraction_of_utc_day: f64,
    prediction_policy: EopPredictionPolicy,
) -> Result<EopSp00PolarOrientation, EopSp00PolarError> {
    let interp = catalog
        .interpolate(mjd_utc, fraction_of_utc_day, prediction_policy)
        .map_err(EopSp00PolarError::Eop)?;

    if mjd_utc < FIRST_INTEGER_UTC_LEAP_DAY_MJD {
        return Err(EopSp00PolarError::BeforeModernUtc);
    }

    let samples = catalog.samples();
    let index = samples
        .binary_search_by_key(&mjd_utc, |r| r.mjd_utc)
        .map_err(|_| EopSp00PolarError::Eop(EopCsvError::DayOutsideCatalog))?;
    let this_day = samples[index];
    let dat = this_day.tai_minus_utc_seconds_at_0h;
    if !(MIN_VALID_DAT_SECONDS..=MAX_VALID_DAT_SECONDS).contains(&dat) {
        return Err(EopSp00PolarError::UnsupportedTaiUtcOffset);
    }
    if fraction_of_utc_day != 0.0 {
        // Catalog::interpolate has already required exactly one
        // adjacent day and authorized its O/P quality. Still check
        // the DAT transition *before* treating UTC as a 86400s day.
        let next = samples
            .get(index + 1)
            .ok_or(EopSp00PolarError::Eop(EopCsvError::MissingFollowingDay))?;
        if next.tai_minus_utc_seconds_at_0h != dat {
            return Err(EopSp00PolarError::LeapSecondBoundary);
        }
    }

    let jdn_utc_midnight = mjd_utc
        .checked_add(MJD_ZERO_DAY_NUMBER)
        .ok_or(EopSp00PolarError::JulianDayOverflow)?;
    let tt_day_fraction = fraction_of_utc_day
        + (f64::from(dat) + TT_MINUS_TAI_SECONDS) / SECONDS_PER_DAY;
    let (day_number, midnight_fraction) = if tt_day_fraction >= 1.0 {
        (
            jdn_utc_midnight
                .checked_add(1)
                .ok_or(EopSp00PolarError::JulianDayOverflow)?,
            tt_day_fraction - 1.0,
        )
    } else {
        (jdn_utc_midnight, tt_day_fraction)
    };
    let tt_epoch = JulianDate::new(day_number, midnight_fraction, TimeScale::Tt)
        .map_err(EopSp00PolarError::TtEpoch)?;
    let approximate_polar = itrs_to_tirs_polar_motion_with_approximate_sp00(
        tt_epoch,
        interp.xp_rad,
        interp.yp_rad,
    )
    .map_err(EopSp00PolarError::Polar)?;

    Ok(EopSp00PolarOrientation {
        source_label: catalog.source_label.clone(),
        mjd_utc,
        fraction_of_utc_day,
        quality: interp.quality,
        tai_minus_utc_seconds: dat,
        tt_epoch,
        xp_rad: interp.xp_rad,
        yp_rad: interp.yp_rad,
        approximate_polar,
    })
}
