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

//! Experimental geometrical ecliptic horizon and meridian intersections.
//! Added 2026-11-14. House cusps belong to astrological-functions.
//! No UTC-to-UT1 conversion or polar-motion correction is performed.

use crate::earth_orientation::{ecliptic_nutation, gast_iau2006, OrientationError};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HouseAngles {
    pub armc_deg: f64,
    pub ascendant_deg: f64,
    pub midheaven_deg: f64,
    pub true_obliquity_deg: f64,
}

#[derive(Debug)]
pub enum HouseAnglesError {
    InvalidLocation,
    InvalidGeometry,
    IncoherentEpoch,
    Orientation(OrientationError),
}

impl fmt::Display for HouseAnglesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocation => write!(
                f,
                "finite latitude inside (-90, 90) and east longitude in [-180, 180] are required"
            ),
            Self::InvalidGeometry => write!(f, "undefined horizon or meridian intersection"),
            Self::IncoherentEpoch => write!(f, "UT1 and TT epochs must describe the same instant"),
            Self::Orientation(e) => write!(f, "orientation: {e}"),
        }
    }
}
impl Error for HouseAnglesError {}

/// Ecliptic intersections from local apparent sidereal angle ARMC, latitude,
/// and true obliquity. All arguments and results are degrees. Singular
/// intersections fail explicitly; no polar house fallback is chosen.
pub fn from_armc(
    armc_deg: f64,
    latitude_deg: f64,
    true_obliquity_deg: f64,
) -> Result<HouseAngles, HouseAnglesError> {
    if !latitude_deg.is_finite() || latitude_deg.abs() >= 90.0 {
        return Err(HouseAnglesError::InvalidLocation);
    }
    if !armc_deg.is_finite()
        || !true_obliquity_deg.is_finite()
        || !(0.0..90.0).contains(&true_obliquity_deg)
    {
        return Err(HouseAnglesError::InvalidGeometry);
    }
    let armc = armc_deg.rem_euclid(360.0);
    let (sin_theta, cos_theta) = armc.to_radians().sin_cos();
    let (sin_eps, cos_eps) = true_obliquity_deg.to_radians().sin_cos();
    let asc_x = sin_eps * latitude_deg.to_radians().tan() + cos_eps * sin_theta;
    let asc_y = -cos_theta;
    if asc_x.hypot(asc_y) < 1e-14 {
        return Err(HouseAnglesError::InvalidGeometry);
    }
    let ascendant_deg = (asc_y.atan2(asc_x).to_degrees() + 180.0).rem_euclid(360.0);
    let midheaven_deg = sin_theta
        .atan2(cos_theta * cos_eps)
        .to_degrees()
        .rem_euclid(360.0);
    Ok(HouseAngles {
        armc_deg: armc,
        ascendant_deg,
        midheaven_deg,
        true_obliquity_deg,
    })
}

/// Compute angles for explicit UT1 and TT Julian days and an east-positive
/// geodetic location. Sidereal time uses the leading equation-of-equinoxes
/// term, so it is approximate. The caller supplies actual UT1; this function
/// does not silently approximate UTC as UT1. A generous 0.1-day epoch
/// difference guard detects obvious mismatches; it does not supply Delta T.
pub fn from_epochs(
    ut1_jd: f64,
    tt_jd: f64,
    latitude_deg: f64,
    longitude_deg_east: f64,
) -> Result<HouseAngles, HouseAnglesError> {
    if !ut1_jd.is_finite() || !tt_jd.is_finite() || (ut1_jd - tt_jd).abs() > 0.1 {
        return Err(HouseAnglesError::IncoherentEpoch);
    }
    if !longitude_deg_east.is_finite() || longitude_deg_east.abs() > 180.0 {
        return Err(HouseAnglesError::InvalidLocation);
    }
    let gast = gast_iau2006(ut1_jd, tt_jd).map_err(HouseAnglesError::Orientation)?;
    let nut = ecliptic_nutation(tt_jd).map_err(HouseAnglesError::Orientation)?;
    from_armc(
        gast.to_degrees() + longitude_deg_east,
        latitude_deg,
        nut.true_obliquity_deg(),
    )
}
