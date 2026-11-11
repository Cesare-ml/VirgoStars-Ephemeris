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

//! IAU 2006 Greenwich Mean Sidereal Time (GMST) correction to Earth Rotation Angle.
//!
//! The ERA must be computed **separately from UT1**; the polynomial uses **TT**.
//! This module does not calculate ERA or perform any time-scale conversion.
//! It is not the Greenwich *apparent* sidereal time (GAST) or a house axis.
//!
//! Scientific source: IERS Conventions (2010), TN36, Section 5.6,
//! Equation 5.32 (Capitaine, Wallace & Chapront, IAU 2006).
//! https://iers-conventions.obspm.fr/conventions_versions.php
//!
//! Created 2026-11-11; independent original implementation using published
//! polynomial coefficients, not code from another astronomical library.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;

use crate::calendar::{CalendarError, JulianDate, TimeScale};

const J2000_NOON_DAY_NUMBER: i64 = 2_451_545;
const TT_DAYS_PER_JULIAN_CENTURY: f64 = 36_525.0;
const ARCSEC_TO_RAD: f64 = TAU / (360.0 * 3_600.0);

/// IAU 2006 precession-in-right-ascension correction from ERA to GMST.
///
/// `offset_rad` is signed and must be *added* to ERA (then wrapped to
/// [0, 2π)). The derivative is with respect to **TT Julian days** only;
/// ERA's UT1 derivative is a distinct physical/time-scale quantity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GmstCorrection {
    pub offset_rad: f64,
    pub offset_rate_rad_per_tt_day: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GmstError {
    RequiresTt(TimeScale),
    InvalidEpoch(CalendarError),
    InvalidEarthRotationAngle,
    NonFiniteCorrection,
}

impl fmt::Display for GmstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequiresTt(actual) => {
                write!(f, "IAU 2006 GMST correction requires TT, got {actual:?}")
            }
            Self::InvalidEpoch(error) => write!(f, "invalid TT epoch for GMST: {error}"),
            Self::InvalidEarthRotationAngle => {
                write!(f, "ERA must be a finite angle in [0, 2pi) radians")
            }
            Self::NonFiniteCorrection => write!(f, "IAU 2006 GMST correction is non-finite"),
        }
    }
}
impl Error for GmstError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidEpoch(error) => Some(error),
            _ => None,
        }
    }
}

/// Evaluate the IERS 2010 Eq. 5.32 polynomial and its TT-day derivative.
///
/// `tt_epoch` must already be expressed in TT. A split Julian date is
/// retained to avoid losing sub-millisecond information by prematurely
/// combining it into a conventional single-f64 JD.
///
/// Validating a `TimeScale::Tt` tag does not establish the physical source
/// of TT; civil UTC must be converted elsewhere by an explicit policy.
/// No nutation, equation of the equinoxes/origins, or UT1 conversion applies.
pub fn iau2006_gmst_correction(tt_epoch: JulianDate) -> Result<GmstCorrection, GmstError> {
    if tt_epoch.scale() != TimeScale::Tt {
        return Err(GmstError::RequiresTt(tt_epoch.scale()));
    }
    tt_epoch.parts().map_err(GmstError::InvalidEpoch)?;

    let tt_days_since_j2000 = (tt_epoch.day_number() - J2000_NOON_DAY_NUMBER) as f64
        + (tt_epoch.midnight_fraction() - 0.5);
    let t = tt_days_since_j2000 / TT_DAYS_PER_JULIAN_CENTURY;
    // IERS TN36 Equation (5.32), polynomial in TT Julian centuries, arcsec.
    let offset_arcsec = 0.014_506
        + t * (4_612.156_534
            + t * (1.391_581_7
                + t * (-0.000_000_44 + t * (-0.000_029_956 - 0.000_000_036_8 * t))));
    let derivative_arcsec_per_century = 4_612.156_534
        + t * (2.0 * 1.391_581_7
            + t * (-3.0 * 0.000_000_44
                + t * (-4.0 * 0.000_029_956 - 5.0 * 0.000_000_036_8 * t)));
    let offset_rad = offset_arcsec * ARCSEC_TO_RAD;
    let offset_rate_rad_per_tt_day =
        derivative_arcsec_per_century * ARCSEC_TO_RAD / TT_DAYS_PER_JULIAN_CENTURY;
    if !offset_rad.is_finite() || !offset_rate_rad_per_tt_day.is_finite() {
        return Err(GmstError::NonFiniteCorrection);
    }
    Ok(GmstCorrection {
        offset_rad,
        offset_rate_rad_per_tt_day,
    })
}

/// Combine an already-calculated IAU 2000 Earth Rotation Angle and TT
/// correction into IAU 2006 **mean** Greenwich sidereal time, [0, 2π).
///
/// ERA is an explicit input so the core's independent ERA PR #75 can be
/// reviewed and integrated separately. The caller must supply an IAU
/// 2000-compatible ERA from UT1; this function cannot verify its provenance.
///
/// This is *not* apparent sidereal time, local sidereal time, ARMC, or a
/// replacement for the Swiss house API.
pub fn iau2006_gmst_from_era(
    era_rad: f64,
    tt_epoch: JulianDate,
) -> Result<f64, GmstError> {
    if !era_rad.is_finite() || !(0.0..TAU).contains(&era_rad) {
        return Err(GmstError::InvalidEarthRotationAngle);
    }
    let correction = iau2006_gmst_correction(tt_epoch)?;
    Ok((era_rad + correction.offset_rad).rem_euclid(TAU))
}
