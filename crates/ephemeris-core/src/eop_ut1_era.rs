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

//! Explicit UTC-SI-second to UT1, TT, IAU ERA and optional SP00 polar W.
//!
//! Consumes ONLY a caller-supplied offline CelesTrak EOP CSV catalog.
//! On a UTC day with a leap second, the civil-day elapsed SI-second
//! count is 86401 (positive) or 86399 (negative), not always 86400.
//!
//! Crucially, interpolate the **continuous UT1−TAI** daily quantity:
//!   (UT1−UTC) − (TAI−UTC).
//! Interpolating the raw DUT1 endpoint values across a leap second
//! would instead invent an unphysical near-one-second daily drift.
//!
//! For position-only orientation, this module delegates to the
//! separately tested IAU ERA(UT1) and approximate TIO s′(TT)/
//! ITRS→TIRS polar-motion components. It does not implement CIP/CIO,
//! authenticate IERS data, model Earth's variable rotation rate,
//! or claim full apparent planetary positions.
//!
//! References:
//! <https://celestrak.org/SpaceData/EOP-format.php>
//! <https://www.nist.gov/pml/time-and-frequency-division/time-realization/leap-seconds>
//! <https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c>
//!
//! Original VirgoStars implementation and tests, 2026-11-12.

use std::error::Error;
use std::fmt;

use crate::calendar::{CalendarError, JulianDate, TimeScale};
use crate::earth_rotation::{earth_rotation_angle, EarthRotation, EarthRotationError};
use crate::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopCsvError, EopPolarQuality, EopPredictionPolicy,
};
use crate::tio_polar_motion::{
    itrs_to_tirs_polar_motion_with_approximate_sp00, ApproximateTioPolarMotion,
    ApproximateTioPolarMotionError,
};

const MJD_ZERO_DAY_NUMBER: i64 = 2_400_001;
const FIRST_MODERN_UTC_MJD: i64 = 41_317; // 1972-01-01
const TT_MINUS_TAI_SECONDS: f64 = 32.184;
const SI_SECONDS_PER_DAY: f64 = 86_400.0;
const MIN_DAT_SECONDS: i32 = 10;
const MAX_DAT_SECONDS: i32 = 100;
const MAX_ABS_DUT1_SECONDS: f64 = 0.9;

/// Intentional sanity check for interpolating the **smooth** UT1−TAI
/// between daily points. This is NOT a formal precision guarantee.
const MAX_UT1_TAI_CHANGE_PER_DAY_SECONDS: f64 = 0.1;

/// Result at one physical UTC instant, expressed using two separate
/// *split* date epochs with unambiguous TT and UT1 scale labels.
#[derive(Clone, Debug, PartialEq)]
pub struct EopUt1EraOrientation {
    /// Identifies the source provided by the caller. Not authenticated.
    pub source_label: String,
    pub mjd_utc: i64,
    /// Actual elapsed SI seconds since this UTC day's 0h, including
    /// the additional leap second where applicable.
    pub seconds_into_utc_day: f64,
    /// 86399, 86400 or 86401 SI seconds, derived from adjacent DAT.
    pub utc_day_length_seconds: f64,
    /// Fraction of the *actual* UTC day, used for EOP interpolation.
    pub fraction_of_utc_day: f64,
    pub quality: EopPolarQuality,
    pub tai_minus_utc_seconds_at_0h: i32,
    /// Continuously interpolated UT1−TAI, seconds. This is preferable
    /// to interpolating discontinuous DUT1 across a leap second.
    pub ut1_minus_tai_seconds: f64,
    /// UT1−UTC in seconds *before the day's ending leap boundary*.
    pub ut1_minus_utc_seconds: f64,
    pub ut1_epoch: JulianDate,
    pub tt_epoch: JulianDate,
    pub earth_rotation: EarthRotation,
    pub xp_rad: f64,
    pub yp_rad: f64,
    /// Explicitly **approximate** TIO s′(TT) + ITRS→TIRS W.
    pub approximate_polar: ApproximateTioPolarMotion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EopUt1EraError {
    Eop(EopCsvError),
    BeforeModernUtc,
    UnsupportedTaiUtcOffset,
    InvalidLeapStep,
    InvalidElapsedUtcSecond,
    InvalidDut1,
    ExcessiveUt1TaiChange,
    JulianDayOverflow,
    InvalidJulianDate(CalendarError),
    EarthRotation(EarthRotationError),
    Polar(ApproximateTioPolarMotionError),
}

impl fmt::Display for EopUt1EraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eop(error) => write!(f, "EOP CSV: {error}"),
            Self::BeforeModernUtc => write!(f, "integer leap-second UTC requires 1972 or later"),
            Self::UnsupportedTaiUtcOffset => write!(f, "unsupported daily TAI−UTC value"),
            Self::InvalidLeapStep => write!(f, "adjacent EOP DAT change must be -1, 0 or +1"),
            Self::InvalidElapsedUtcSecond => {
                write!(f, "elapsed UTC SI second is outside civil day")
            }
            Self::InvalidDut1 => write!(f, "daily UT1−UTC must be finite within ±0.9 s"),
            Self::ExcessiveUt1TaiChange => {
                write!(f, "excessive daily UT1−TAI change in supplied EOP data")
            }
            Self::JulianDayOverflow => write!(f, "unrepresentable UTC MJD"),
            Self::InvalidJulianDate(error) => {
                write!(f, "invalid split UT1/TT Julian date: {error}")
            }
            Self::EarthRotation(error) => write!(f, "IAU ERA from UT1: {error}"),
            Self::Polar(error) => write!(f, "approximate SP00/polar orientation: {error}"),
        }
    }
}

impl Error for EopUt1EraError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Eop(error) => Some(error),
            Self::InvalidJulianDate(error) => Some(error),
            Self::EarthRotation(error) => Some(error),
            Self::Polar(error) => Some(error),
            _ => None,
        }
    }
}

/// Build a split Julian date from the given UTC midnight plus a signed
/// finite SI-second offset. The Julian day is always a uniform 86400
/// SI-second period for UT1 and TT; the civil UTC leap day is handled
/// *upstream* when computing the offset.
fn epoch_from_utc_midnight(
    mjd_utc: i64,
    offset_seconds: f64,
    scale: TimeScale,
) -> Result<JulianDate, EopUt1EraError> {
    let midnight_jdn = mjd_utc
        .checked_add(MJD_ZERO_DAY_NUMBER)
        .ok_or(EopUt1EraError::JulianDayOverflow)?;
    if !offset_seconds.is_finite() {
        return Err(EopUt1EraError::InvalidJulianDate(
            CalendarError::InvalidFraction,
        ));
    }
    // All valid caller offsets lie close to a single UTC day. Work
    // relative to midnight without ever collapsing to one f64 JD.
    let days = (offset_seconds / SI_SECONDS_PER_DAY).floor();
    if !(-2.0..=2.0).contains(&days) {
        return Err(EopUt1EraError::InvalidJulianDate(
            CalendarError::UnrepresentableTime,
        ));
    }
    let day_number = midnight_jdn
        .checked_add(days as i64)
        .ok_or(EopUt1EraError::JulianDayOverflow)?;
    let fraction = offset_seconds / SI_SECONDS_PER_DAY - days;
    JulianDate::new(day_number, fraction, scale).map_err(EopUt1EraError::InvalidJulianDate)
}

/// Derive UT1 and ERA from a supplied EOP CSV snapshot at an actual
/// elapsed UTC SI second, including an explicitly represented leap
/// second `seconds_into_utc_day=86400` on a positive-leap day.
///
/// `seconds_into_utc_day` is **not** a conventional UTC Julian date
/// fraction; it is a nonnegative count of physical SI seconds elapsed
/// since 0h UTC, constrained to `[0,86400+ΔDAT)`. In particular:
///
/// - ordinary day: [0,86400), last valid civil second is 86399.x;
/// - positive leap day: [0,86401), 23:59:60 is [86400,86401);
/// - negative leap day: [0,86399), no fictitious 23:59:59.
///
/// Both daily EOP rows must exist and be consecutive; `DATA_TYPE=P`
/// is rejected unless `AllowPredicted` was explicitly supplied.
/// An integer DAT change other than ±1 or 0 is refused. These
/// checks verify **internal** EOP consistency but do not authenticate
/// the dates against the authoritative IERS leap-second schedule.
///
/// Use the day fraction `f=elapsed/(86400+ΔDAT)` only for the
/// slow daily interpolation of **UT1−TAI** and `xp/yp`:
///
/// `UT1−TAI = mix(dut1_0−dat_0, dut1_1−dat_1, f)`
///
/// `UT1 JD = UTC_midnight_JD + (elapsed+DAT_0+(UT1−TAI))/86400`
/// `TT JD = UTC_midnight_JD + (elapsed+DAT_0+32.184)/86400`
///
/// This keeps UT1 and TT continuous over leap seconds; no raw DUT1
/// interpolation or leap-day-as-86400 assumption is used. Returns
/// IAU ERA from the resulting UT1 and the explicitly opted-in
/// approximate SP00 s′/polar W from TT/xp/yp, **not** a full ITRS→GCRS
/// rotation (CIP/CIO data are still required).
///
/// Remaining science gates: EOP snapshot authenticity, external leap
/// schedule, EOP realization and interpolation error, true observer
/// state, moving frame and DE441/Horizons numerical parity.
pub fn eop_ut1_era_from_utc_si_seconds(
    catalog: &CelestrakEopPolarCatalog,
    mjd_utc: i64,
    seconds_into_utc_day: f64,
    prediction_policy: EopPredictionPolicy,
) -> Result<EopUt1EraOrientation, EopUt1EraError> {
    if mjd_utc < FIRST_MODERN_UTC_MJD {
        return Err(EopUt1EraError::BeforeModernUtc);
    }
    let samples = catalog.samples();
    let index = samples
        .binary_search_by_key(&mjd_utc, |r| r.mjd_utc)
        .map_err(|_| EopUt1EraError::Eop(EopCsvError::DayOutsideCatalog))?;
    let lower = samples[index];
    let upper = samples
        .get(index + 1)
        .ok_or(EopUt1EraError::Eop(EopCsvError::MissingFollowingDay))?;
    if upper.mjd_utc
        != mjd_utc
            .checked_add(1)
            .ok_or(EopUt1EraError::JulianDayOverflow)?
    {
        return Err(EopUt1EraError::Eop(EopCsvError::MissingFollowingDay));
    }
    let dat0 = lower.tai_minus_utc_seconds_at_0h;
    let dat1 = upper.tai_minus_utc_seconds_at_0h;
    if !(MIN_DAT_SECONDS..=MAX_DAT_SECONDS).contains(&dat0)
        || !(MIN_DAT_SECONDS..=MAX_DAT_SECONDS).contains(&dat1)
    {
        return Err(EopUt1EraError::UnsupportedTaiUtcOffset);
    }
    let leap_step = dat1 - dat0;
    if !(-1..=1).contains(&leap_step) {
        return Err(EopUt1EraError::InvalidLeapStep);
    }
    let day_length = SI_SECONDS_PER_DAY + f64::from(leap_step);
    if !seconds_into_utc_day.is_finite() || !(0.0..day_length).contains(&seconds_into_utc_day) {
        return Err(EopUt1EraError::InvalidElapsedUtcSecond);
    }
    let dut10 = lower.ut1_minus_utc_seconds_at_0h;
    let dut11 = upper.ut1_minus_utc_seconds_at_0h;
    if !dut10.is_finite()
        || !dut11.is_finite()
        || dut10.abs() > MAX_ABS_DUT1_SECONDS
        || dut11.abs() > MAX_ABS_DUT1_SECONDS
    {
        return Err(EopUt1EraError::InvalidDut1);
    }
    let ut1_tai0 = dut10 - f64::from(dat0);
    let ut1_tai1 = dut11 - f64::from(dat1);
    if (ut1_tai1 - ut1_tai0).abs() > MAX_UT1_TAI_CHANGE_PER_DAY_SECONDS {
        return Err(EopUt1EraError::ExcessiveUt1TaiChange);
    }
    let fraction = seconds_into_utc_day / day_length;
    let poles = catalog
        .interpolate(mjd_utc, fraction, prediction_policy)
        .map_err(EopUt1EraError::Eop)?;
    let ut1_minus_tai_seconds = (ut1_tai1 - ut1_tai0).mul_add(fraction, ut1_tai0);
    let ut1_minus_utc_seconds = f64::from(dat0) + ut1_minus_tai_seconds;
    let ut1_epoch = epoch_from_utc_midnight(
        mjd_utc,
        seconds_into_utc_day + f64::from(dat0) + ut1_minus_tai_seconds,
        TimeScale::Ut1,
    )?;
    let tt_epoch = epoch_from_utc_midnight(
        mjd_utc,
        seconds_into_utc_day + f64::from(dat0) + TT_MINUS_TAI_SECONDS,
        TimeScale::Tt,
    )?;
    let earth_rotation = earth_rotation_angle(ut1_epoch).map_err(EopUt1EraError::EarthRotation)?;
    let approximate_polar =
        itrs_to_tirs_polar_motion_with_approximate_sp00(tt_epoch, poles.xp_rad, poles.yp_rad)
            .map_err(EopUt1EraError::Polar)?;
    Ok(EopUt1EraOrientation {
        source_label: catalog.source_label.clone(),
        mjd_utc,
        seconds_into_utc_day,
        utc_day_length_seconds: day_length,
        fraction_of_utc_day: fraction,
        quality: poles.quality,
        tai_minus_utc_seconds_at_0h: dat0,
        ut1_minus_tai_seconds,
        ut1_minus_utc_seconds,
        ut1_epoch,
        tt_epoch,
        earth_rotation,
        xp_rad: poles.xp_rad,
        yp_rad: poles.yp_rad,
        approximate_polar,
    })
}
