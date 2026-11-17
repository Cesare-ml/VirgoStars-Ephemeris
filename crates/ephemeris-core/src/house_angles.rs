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

//! Astronomical inputs for house systems: apparent sidereal time, true
//! obliquity, and the two angles that follow from them (Midheaven and
//! Ascendant). House *systems* (cusps, house positions) are astrological
//! methods and live in `astrological-functions`; this module only supplies the
//! astronomy they need, with explicit time scales.
//!
//! # Definitions
//!
//! - `armc_deg`: right ascension of the meridian = Greenwich apparent sidereal
//!   time (IAU 2006 / IAU 2000B, [`crate::earth_orientation::gast_iau2006`]) plus
//!   the east longitude, in `[0, 360)`.
//! - `true_obliquity_deg`: IAU 2006 mean obliquity plus the IAU 2000B nutation
//!   in obliquity, at the supplied TT.
//! - Midheaven: `atan2(sin ARMC, cos ARMC · cos ε)`.
//! - Ascendant: `atan2(cos ARMC, −(sin ARMC · cos ε + tan φ · sin ε))`, the
//!   ecliptic longitude of the east horizon point, valid for all latitudes
//!   with `|φ| < 90°`. Whether a *house system* is defined at that latitude is
//!   decided by the house method, not here.
//!
//! UT1 and TT are separate arguments; this module never converts between them.
//! Valid for the IAU 2000B range (1900–2100 TT). No geodetic-to-geocentric
//! latitude correction is applied: the caller supplies the latitude its house
//! method expects (the reference convention is geodetic latitude used as is).

use std::error::Error;
use std::fmt;

use crate::earth_orientation::{ecliptic_nutation, gast_iau2006, OrientationError};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HouseAngles {
    pub ut1_jd: f64,
    pub tt_jd: f64,
    pub latitude_deg: f64,
    pub longitude_deg_east: f64,
    pub armc_deg: f64,
    pub true_obliquity_deg: f64,
    pub ascendant_deg: f64,
    pub midheaven_deg: f64,
    /// Further angles of the reference's house calculation, see [`ExtraAngles`].
    pub extra: ExtraAngles,
}

/// The five points that accompany the Ascendant and Midheaven in the reference's
/// `swe_houses` result. All are ecliptic longitudes of the true ecliptic of date,
/// built from the same ascendant formula with another sidereal angle and latitude
/// (`asc(a, φ)` is the ecliptic point rising at ARMC `a` for latitude `φ`):
///
/// - `vertex`: `asc(ARMC + 180°, 90° − φ)`, the intersection of the ecliptic and the
///   prime vertical; where two exist (|φ| ≤ obliquity) the one west of the Midheaven;
/// - `equatorial_ascendant` (East Point): `asc(ARMC, 0)`;
/// - `koch_co_ascendant` (W. Koch): `asc(ARMC, −φ)`;
/// - `munkasey_co_ascendant` (M. Munkasey): `asc(ARMC, 90° − φ)`;
/// - `munkasey_polar_ascendant`: `asc(ARMC, −φ) + 180°`.
///
/// The constructions were identified by comparison with the reference over 2,500
/// sites and dates (no difference above 0.5 arcsecond); they are definitions of
/// points, not of house systems.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtraAngles {
    pub vertex_deg: f64,
    pub equatorial_ascendant_deg: f64,
    pub koch_co_ascendant_deg: f64,
    pub munkasey_co_ascendant_deg: f64,
    pub munkasey_polar_ascendant_deg: f64,
}

#[derive(Debug, PartialEq)]
pub enum HouseAnglesError {
    /// Latitude must satisfy `|φ| < 90°`; longitude must be finite.
    InvalidSite,
    Orientation(OrientationError),
}

impl fmt::Display for HouseAnglesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSite => write!(
                f,
                "latitude must be finite with |lat| < 90 and longitude finite"
            ),
            Self::Orientation(e) => write!(f, "earth orientation: {e}"),
        }
    }
}

impl Error for HouseAnglesError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Orientation(e) => Some(e),
            Self::InvalidSite => None,
        }
    }
}

impl From<OrientationError> for HouseAnglesError {
    fn from(e: OrientationError) -> Self {
        Self::Orientation(e)
    }
}

fn wrap_360(degrees: f64) -> f64 {
    let wrapped = degrees.rem_euclid(360.0);
    if wrapped >= 360.0 {
        0.0
    } else {
        wrapped
    }
}

/// Ascendant and Midheaven (in that order) for a known ARMC and true obliquity.
pub fn angles_from_armc(armc_deg: f64, true_obliquity_deg: f64, latitude_deg: f64) -> (f64, f64) {
    let (sin_a, cos_a) = armc_deg.to_radians().sin_cos();
    let (sin_e, cos_e) = true_obliquity_deg.to_radians().sin_cos();
    let tan_phi = latitude_deg.to_radians().tan();
    let midheaven = wrap_360(sin_a.atan2(cos_a * cos_e).to_degrees());
    let ascendant = wrap_360(cos_a.atan2(-(sin_a * cos_e + tan_phi * sin_e)).to_degrees());
    (ascendant, midheaven)
}

/// The [`ExtraAngles`] for a known ARMC, true obliquity, latitude and Midheaven.
pub fn extra_angles_from_armc(
    armc_deg: f64,
    true_obliquity_deg: f64,
    latitude_deg: f64,
    midheaven_deg: f64,
) -> ExtraAngles {
    let asc = |armc: f64, lat: f64| angles_from_armc(armc, true_obliquity_deg, lat).0;
    let mut vertex = asc(armc_deg + 180.0, 90.0 - latitude_deg);
    // Within the tropics the ecliptic crosses the prime vertical twice: the vertex is
    // the western crossing, i.e. the one that is not east of the Midheaven.
    if latitude_deg.abs() <= true_obliquity_deg
        && (vertex - midheaven_deg + 540.0).rem_euclid(360.0) - 180.0 > 0.0
    {
        vertex = wrap_360(vertex + 180.0);
    }
    let koch = asc(armc_deg, -latitude_deg);
    ExtraAngles {
        vertex_deg: vertex,
        equatorial_ascendant_deg: asc(armc_deg, 0.0),
        koch_co_ascendant_deg: koch,
        munkasey_co_ascendant_deg: asc(armc_deg, 90.0 - latitude_deg),
        munkasey_polar_ascendant_deg: wrap_360(koch + 180.0),
    }
}

/// House astronomy for a site at `(ut1_jd, tt_jd)`.
pub fn house_angles(
    ut1_jd: f64,
    tt_jd: f64,
    latitude_deg: f64,
    longitude_deg_east: f64,
) -> Result<HouseAngles, HouseAnglesError> {
    if !latitude_deg.is_finite()
        || latitude_deg.abs() >= 90.0
        || !longitude_deg_east.is_finite()
        || longitude_deg_east.abs() > 360.0
    {
        return Err(HouseAnglesError::InvalidSite);
    }
    let gast = gast_iau2006(ut1_jd, tt_jd)?.to_degrees();
    let armc = wrap_360(gast + longitude_deg_east);
    let obliquity = ecliptic_nutation(tt_jd)?.true_obliquity_deg();
    let (ascendant, midheaven) = angles_from_armc(armc, obliquity, latitude_deg);
    Ok(HouseAngles {
        ut1_jd,
        tt_jd,
        latitude_deg,
        longitude_deg_east,
        armc_deg: armc,
        true_obliquity_deg: obliquity,
        ascendant_deg: ascendant,
        midheaven_deg: midheaven,
        extra: extra_angles_from_armc(armc, obliquity, latitude_deg, midheaven),
    })
}
