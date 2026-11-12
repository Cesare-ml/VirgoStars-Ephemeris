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

//! Bounded secular IERS approximation for the TIO locator s′ at TT.
//!
//! s′ positions the terrestrial intermediate origin (TIO), and enters
//! the ITRS↔TIRS polar-motion matrix together with observed xp and yp.
//! Its long-term approximation is −47 microarcseconds per TT Julian
//! century since J2000.0. It is **not** a measurement or a complete
//! geophysical model: a higher-accuracy application should prefer
//! a separately validated, time-matched TIO locator.
//!
//! This module evaluates only the IERS linear conventional approximation,
//! using an explicit split-JD TT epoch. No UT1=TT assumption, EOP
//! interpolation, celestial CIO s, polar motion or rotation matrix is
//! calculated here.
//!
//! Independent oracle: SOFA/ERFA t_sp00 (2400000.5 + 52541.0 TT).
//! <https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c>
//!
//! Original VirgoStars geometry, 2026-11-12.

use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

use crate::calendar::{JulianDate, TimeScale};

/// Conventional Julian days per TT Julian century.
const JULIAN_DAYS_PER_CENTURY: f64 = 36_525.0;
/// Published conventional secular drift in microarcseconds per
/// TT Julian century, NOT microarcseconds per day.
const SECULAR_MICROARCSECONDS_PER_CENTURY: f64 = -47.0;
/// Radians per arcsecond, from the angular unit definition.
const RADIANS_PER_ARCSECOND: f64 = PI / (180.0 * 3_600.0);
/// Explicit use boundary for this *secular approximation*, not a
/// guarantee of physical accuracy throughout that interval.
const MAX_ABS_CENTURIES_FROM_J2000: f64 = 2.0;
const J2000_TT_DAY_NUMBER: i64 = 2_451_545;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TioLocatorError {
    /// The caller must supply a TT epoch, not TDB, UT1 or TAI.
    RequiresTerrestrialTime(TimeScale),
    /// Excessively large/small integral JD cannot be subtracted safely.
    InvalidEpoch,
    /// The approximate secular model is guarded to 1800–2200 (TT)
    /// instead of extrapolating unbounded centuries.
    BeyondApproximationRange,
    /// The evaluated epoch or angle cannot be represented as finite f64.
    NonFiniteResult,
}

impl fmt::Display for TioLocatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequiresTerrestrialTime(scale) => {
                write!(f, "TIO s-prime approximation requires TT, got {scale:?}")
            }
            Self::InvalidEpoch => write!(f, "unrepresentable split-Julian-date epoch"),
            Self::BeyondApproximationRange => {
                write!(
                    f,
                    "TIO s-prime secular model is limited to ±2 TT Julian centuries"
                )
            }
            Self::NonFiniteResult => write!(f, "non-finite TIO locator approximation"),
        }
    }
}

impl Error for TioLocatorError {}

/// Evaluated conventional approximation and its **TT-day** derivative.
///
/// This rate is the derivative of the *linear approximation only*.
/// It is not a measured instantaneous TIO angular velocity or a
/// replacement for EOP-derived terrestrial-origin motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApproximateTioLocator {
    /// Terrestrial Intermediate Origin locator s′ (radians).
    pub sp_rad: f64,
    /// Linear secular derivative ds′/dTT, radians per TT day.
    pub secular_rate_rad_per_tt_day: f64,
}

/// Evaluate the conventional IERS secular approximation of s′ from TT.
///
/// For `T = (JD_TT - 2451545.0) / 36525` TT Julian centuries,
///
/// `s′ = −47 × 10⁻⁶ × (π / 648000) × T` radians.
///
/// Uses the integer/fractional split of `JulianDate` to avoid
/// first collapsing a modern TT Julian date to a single f64. Strictly
/// accepts `TimeScale::Tt`; does not convert UTC, UT1, TAI or TDB.
/// An explicit ±2-century guard prevents unlimited extrapolation
/// of the approximate secular trend, but does NOT certify accuracy
/// inside that interval. The independent input xp/yp polar motion
/// and the CIO locator s are completely separate quantities.
///
/// The result may be passed *explicitly* as the TIO s′ input to
/// `polar_motion::itrs_to_tirs_polar_motion_matrix` after review,
/// but this module does not depend on the unmerged polar-motion PR.
pub fn approximate_tio_locator_sp00(
    tt_epoch: JulianDate,
) -> Result<ApproximateTioLocator, TioLocatorError> {
    if tt_epoch.scale() != TimeScale::Tt {
        return Err(TioLocatorError::RequiresTerrestrialTime(tt_epoch.scale()));
    }

    let whole_days = tt_epoch
        .day_number()
        .checked_sub(J2000_TT_DAY_NUMBER)
        .ok_or(TioLocatorError::InvalidEpoch)?;
    // Fail closed before converting an enormous i64 day count to f64.
    if whole_days.unsigned_abs() > 2 * 36_525 + 1 {
        return Err(TioLocatorError::BeyondApproximationRange);
    }
    // JulianDate day_number is the integral noon JD number, while
    // midnight_fraction counts forward from the preceding midnight:
    // conventional JD = day_number - 0.5 + midnight_fraction.
    let days_since_j2000 = whole_days as f64 + (tt_epoch.midnight_fraction() - 0.5);
    let centuries_since_j2000 = days_since_j2000 / JULIAN_DAYS_PER_CENTURY;
    if !centuries_since_j2000.is_finite() {
        return Err(TioLocatorError::NonFiniteResult);
    }
    if centuries_since_j2000.abs() > MAX_ABS_CENTURIES_FROM_J2000 {
        return Err(TioLocatorError::BeyondApproximationRange);
    }

    let radians_per_century = SECULAR_MICROARCSECONDS_PER_CENTURY * 1e-6 * RADIANS_PER_ARCSECOND;
    let sp_rad = radians_per_century * centuries_since_j2000;
    let secular_rate_rad_per_tt_day = radians_per_century / JULIAN_DAYS_PER_CENTURY;
    if !sp_rad.is_finite() || !secular_rate_rad_per_tt_day.is_finite() {
        return Err(TioLocatorError::NonFiniteResult);
    }
    Ok(ApproximateTioLocator {
        sp_rad,
        secular_rate_rad_per_tt_day,
    })
}
