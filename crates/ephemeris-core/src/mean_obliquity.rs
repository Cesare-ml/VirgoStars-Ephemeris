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

//! IAU 2006 mean obliquity of the ecliptic, and its TT-day derivative.
//!
//! Standalone polynomial angle, not an ICRF-to-ecliptic-of-date frame matrix,
//! complete precession/nutation, or a Swiss-style apparent position.
//! Coefficients: IERS Conventions (2010), Technical Note 36, Eq. 5.40,
//! <https://iers-conventions.obspm.fr/content/tn36.pdf>.

use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

use crate::calendar::{JulianDate, TimeScale};

const J2000_JD_TT: f64 = 2_451_545.0;
const JULIAN_CENTURY_TT_DAYS: f64 = 36_525.0;
const ARCSECOND_TO_RADIAN: f64 = PI / (180.0 * 3_600.0);

/// A conservative, locally tested engineering bound, NOT a formal IAU model
/// validity/accuracy guarantee. Extend only with scientific validation.
const MAX_ABS_CENTURIES_FROM_J2000: f64 = 2.0;

// Polynomial coefficients in arcseconds, increasing powers of T (TT Julian
// centuries after J2000.0), from the public IERS/IAU 2006 formulation.
// This is an independent evaluation of scientific published constants,
// not a translation of any SOFA, ERFA or Swiss software implementation.
const OBLIQUITY_ARCSECONDS: [f64; 6] = [
    84_381.406,
    -46.836_769,
    -0.000_183_1,
    0.002_003_40,
    -0.000_000_576,
    -0.000_000_043_4,
];

/// Mean obliquity angle and derivative, both parameterized by TT.
///
/// The derivative is with respect to a **TT day**. It is not automatically
/// the derivative with respect to an observation-TDB day and cannot be fed
/// directly to a moving-TDB-frame state without an explicit time convention.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeanObliquity {
    pub radians: f64,
    pub rate_radians_per_tt_day: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeanObliquityError {
    /// The input must already be on the TT time scale.
    WrongTimeScale,
    /// A split Julian date is invalid for safe floating-point evaluation.
    UnrepresentableEpoch,
    /// Conservative guarded interval is J2000 ± 2 TT Julian centuries.
    OutsideTestedInterval,
}

impl fmt::Display for MeanObliquityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongTimeScale => write!(f, "IAU 2006 mean obliquity requires TT epoch"),
            Self::UnrepresentableEpoch => write!(f, "invalid or unrepresentable TT Julian date"),
            Self::OutsideTestedInterval => {
                write!(f, "TT epoch outside the locally tested ±2-century interval")
            }
        }
    }
}
impl Error for MeanObliquityError {}

/// TT Julian centuries from J2000 for the guarded IAU 2006 precession
/// family. Kept crate-private so each constituent uses identical epochs and
/// refuses wrong time scales or dates outside the tested policy window.
pub(crate) fn tt_centuries_iau2006(tt_epoch: JulianDate) -> Result<f64, MeanObliquityError> {
    if tt_epoch.scale() != TimeScale::Tt {
        return Err(MeanObliquityError::WrongTimeScale);
    }
    let (whole, fraction) = tt_epoch
        .parts()
        .map_err(|_| MeanObliquityError::UnrepresentableEpoch)?;
    let centuries = ((whole - J2000_JD_TT) + fraction) / JULIAN_CENTURY_TT_DAYS;
    if !centuries.is_finite() {
        return Err(MeanObliquityError::UnrepresentableEpoch);
    }
    if centuries.abs() > MAX_ABS_CENTURIES_FROM_J2000 {
        return Err(MeanObliquityError::OutsideTestedInterval);
    }
    Ok(centuries)
}

/// Evaluate IAU 2006 mean obliquity and its exact polynomial derivative.
///
/// TT Julian centuries T = (JD_TT - 2451545.0) / 36525.0.
/// The split JD is used *before* subtraction to retain fractional-day
/// precision. No leap-second, UTC, TT↔TDB or UT1 conversion is attempted.
///
/// The derivative is obtained analytically from the same polynomial using
/// simultaneous Horner recurrences. This scalar angle alone must not be
/// mistaken for a fully specified ecliptic-of-date orientation in the ICRF.
pub fn mean_obliquity_iau2006(tt_epoch: JulianDate) -> Result<MeanObliquity, MeanObliquityError> {
    let centuries = tt_centuries_iau2006(tt_epoch)?;

    // Paired Horner recurrences: derivative is with respect to centuries.
    let mut angle_arcsec = OBLIQUITY_ARCSECONDS[5];
    let mut rate_arcsec_per_century = 0.0;
    for coefficient in OBLIQUITY_ARCSECONDS[..5].iter().rev() {
        rate_arcsec_per_century = rate_arcsec_per_century * centuries + angle_arcsec;
        angle_arcsec = angle_arcsec * centuries + *coefficient;
    }

    let result = MeanObliquity {
        radians: angle_arcsec * ARCSECOND_TO_RADIAN,
        rate_radians_per_tt_day: rate_arcsec_per_century * ARCSECOND_TO_RADIAN
            / JULIAN_CENTURY_TT_DAYS,
    };
    if !result.radians.is_finite() || !result.rate_radians_per_tt_day.is_finite() {
        return Err(MeanObliquityError::UnrepresentableEpoch);
    }
    Ok(result)
}
