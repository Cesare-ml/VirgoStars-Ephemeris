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

//! IAU 2000 Earth Rotation Angle (ERA), using an already-UT1 split Julian date.
//!
//! This is the Earth rotation angle relative to the celestial/terrestrial
//! intermediate origins, **not** Greenwich sidereal time, GAST, a complete
//! terrestrial-to-celestial frame transformation, or a timezone conversion.
//!
//! Original implementation from IAU Resolution B1.8, note 3, and IERS
//! Conventions (2010), Chapter 5, equations 5.14-5.15:
//! <https://iau-a3.gitlab.io/res.html>
//! <https://iers-conventions.obspm.fr/conventions_material.php>
//!
//! Created 2026-11-11. No third-party source implementation is copied.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;

use crate::calendar::{CalendarError, JulianDate, TimeScale};

/// Constant term in turns at J2000.0 (UT1).
const ERA_TURNS_AT_J2000: f64 = 0.779_057_273_264_0;
/// Excess turns per UT1 day beyond one full turn per day.
const ERA_EXCESS_TURNS_PER_UT1_DAY: f64 = 0.002_737_811_911_354_48;
const J2000_NOON_DAY_NUMBER: i64 = 2_451_545;

/// Earth rotation angle and its constant IAU 2000 UT1-time derivative.
///
/// `angle_rad` is normalized to [0, 2π). The angular rate is the
/// derivative with respect to the **input UT1 day**, not TT or TDB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EarthRotation {
    pub angle_rad: f64,
    pub rate_rad_per_ut1_day: f64,
}

/// A strict ERA input failure, distinguishing a mislabeled time scale from
/// unrepresentable split-JD arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EarthRotationError {
    RequiresUt1(TimeScale),
    InvalidDate(CalendarError),
}

impl fmt::Display for EarthRotationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequiresUt1(actual) => {
                write!(f, "Earth rotation angle requires a UT1 Julian date, got {actual:?}")
            }
            Self::InvalidDate(error) => write!(f, "invalid Earth rotation epoch: {error}"),
        }
    }
}
impl Error for EarthRotationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDate(error) => Some(error),
            Self::RequiresUt1(_) => None,
        }
    }
}

/// Compute IAU 2000 ERA (in radians) with its rate for an explicitly UT1 date.
///
/// The epoch must **already be expressed in UT1**. This routine has no UTC,
/// EOP, leap-second, Delta-T, TT or TDB conversion and cannot validate the
/// physical origin of its caller-supplied UT1 label.
///
/// IERS Eq. 5.15 splits the linear rotation into integral turns and a small
/// fractional correction:
///
/// `ERA/(2π) = 0.7790572732640 + frac(JD_UT1 - J2000)
///                + 0.00273781191135448 * (JD_UT1 - J2000) (mod 1)`.
///
/// The integer `day_number` and sub-day fraction stay separate until the
/// small correction is evaluated, avoiding the precision loss of collapsing
/// a modern Julian date into one f64 first.
pub fn earth_rotation_angle(
    epoch: JulianDate,
) -> Result<EarthRotation, EarthRotationError> {
    if epoch.scale() != TimeScale::Ut1 {
        return Err(EarthRotationError::RequiresUt1(epoch.scale()));
    }
    // Validate the representability bound used by the canonical split-JD API.
    epoch.parts().map_err(EarthRotationError::InvalidDate)?;

    let whole_days = epoch.day_number() - J2000_NOON_DAY_NUMBER;
    // Because midnight_fraction ranges over [0, 1), J2000 noon is at +0.5.
    // Dropping whole days is exact modulo one rotation per UT1 day.
    let fractional_days = epoch.midnight_fraction() - 0.5;
    let days_since_j2000 = whole_days as f64 + fractional_days;
    let turns = (ERA_TURNS_AT_J2000
        + fractional_days
        + ERA_EXCESS_TURNS_PER_UT1_DAY * days_since_j2000)
        .rem_euclid(1.0);
    let angle_rad = TAU * turns;
    let rate_rad_per_ut1_day = TAU * (1.0 + ERA_EXCESS_TURNS_PER_UT1_DAY);

    Ok(EarthRotation {
        angle_rad,
        rate_rad_per_ut1_day,
    })
}
