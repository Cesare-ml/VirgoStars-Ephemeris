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

//! Equatorial hour-angle and declination to ideal local horizontal coordinates.
//!
//! Geometry only: no ephemeris loading, timing, frame conversion, parallax,
//! atmospheric refraction, terrain, polar motion or Earth-orientation model.
//! The caller must provide a self-consistent hour angle and declination in
//! the same celestial equator/equinox frame, already corrected for its
//! intended observer and physical model.
//!
//! Independent formula reference:
//! <https://aa.usno.navy.mil/faq/alt_az>
//! SOFA/ERFA hd2ae is used as a separately published numerical test reference.
//!
//! Created 2026-11-11. Original implementation, no external source copied.

use crate::earth_orientation::{gast_iau2006, OrientationError};
use std::error::Error;
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::fmt;

/// Ideal geometric horizontal direction in the model's local north/east/up
/// axes, with azimuth clockwise from north through east.
///
/// Azimuth is undefined at zenith/nadir and at the exact geographic poles.
/// `None` reports this instead of returning an arbitrary number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizontalDirection {
    /// Signed altitude above the mathematical horizon, in [-π/2, π/2].
    pub altitude_rad: f64,
    /// Azimuth in [0, 2π), or None at an undefined bearing.
    pub azimuth_rad: Option<f64>,
}

/// Geometric altitude and both explicit azimuth conventions in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizontalDegrees {
    pub true_altitude_deg: f64,
    /// 0 north, 90 east; None at an undefined bearing.
    pub azimuth_north_east_deg: Option<f64>,
    /// 0 south, 90 west, as in the Swiss azimuth convention.
    pub azimuth_south_west_deg: Option<f64>,
}

/// NOAA solar-position refraction estimate in degrees. This fixed-atmosphere
/// approximation does not accept observed temperature or pressure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StandardRefraction {
    pub true_altitude_deg: f64,
    pub correction_deg: f64,
    pub apparent_altitude_deg: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidRefractionAltitude;
impl fmt::Display for InvalidRefractionAltitude {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "geometric altitude must be finite and between -1 and 90 degrees"
        )
    }
}
impl Error for InvalidRefractionAltitude {}

/// NOAA's published piecewise solar-position refraction approximation, with
/// an explicitly restricted input range of -1° to 90°. Atmosphere is not
/// inferred from site elevation. The output is not a visibility prediction
/// or a replacement for pressure/temperature-aware Swiss refraction.
/// <https://gml.noaa.gov/grad/solcalc/calcdetails.html>
/// Added 2026-11-15.
pub fn noaa_standard_refraction(
    true_altitude_deg: f64,
) -> Result<StandardRefraction, InvalidRefractionAltitude> {
    if !true_altitude_deg.is_finite() || !(-1.0..=90.0).contains(&true_altitude_deg) {
        return Err(InvalidRefractionAltitude);
    }
    let h = true_altitude_deg;
    let correction_arcsec = if h >= 85.0 {
        0.0
    } else if h > 5.0 {
        let t = h.to_radians().tan();
        58.1 / t - 0.07 / t.powi(3) + 0.000086 / t.powi(5)
    } else if h >= -0.575 {
        1735.0 + h * (-518.2 + h * (103.4 + h * (-12.79 + h * 0.711)))
    } else {
        -20.774 / h.to_radians().tan()
    };
    let correction_deg = correction_arcsec / 3600.0;
    Ok(StandardRefraction {
        true_altitude_deg,
        correction_deg,
        apparent_altitude_deg: h + correction_deg,
    })
}

#[derive(Debug)]
pub enum HorizontalConversionError {
    InvalidInput,
    IncoherentEpoch,
    Orientation(OrientationError),
    Geometry(HorizontalError),
}
impl fmt::Display for HorizontalConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => write!(f, "invalid equatorial direction or observer location"),
            Self::IncoherentEpoch => write!(f, "UT1 and TT must describe the same instant"),
            Self::Orientation(e) => write!(f, "orientation: {e}"),
            Self::Geometry(e) => write!(f, "horizontal geometry: {e}"),
        }
    }
}
impl Error for HorizontalConversionError {}

/// Transform a supplied true-equator-of-date direction to the ideal horizon.
/// TT and UT1 must be coherent and explicit; the generous 0.1-day difference
/// guard is not a time conversion. Geographic longitude is east positive.
/// GAST uses the leading equation-of-equinoxes term only. The input must
/// already include any desired topocentric parallax; no refraction is applied.
pub fn true_equatorial_to_horizontal(
    ut1_jd: f64,
    tt_jd: f64,
    latitude_deg: f64,
    longitude_deg_east: f64,
    right_ascension_deg: f64,
    declination_deg: f64,
) -> Result<HorizontalDegrees, HorizontalConversionError> {
    if !ut1_jd.is_finite() || !tt_jd.is_finite() || (ut1_jd - tt_jd).abs() > 0.1 {
        return Err(HorizontalConversionError::IncoherentEpoch);
    }
    if !latitude_deg.is_finite()
        || latitude_deg.abs() > 90.0
        || !longitude_deg_east.is_finite()
        || longitude_deg_east.abs() > 180.0
        || !right_ascension_deg.is_finite()
        || !(0.0..360.0).contains(&right_ascension_deg)
        || !declination_deg.is_finite()
        || declination_deg.abs() > 90.0
    {
        return Err(HorizontalConversionError::InvalidInput);
    }
    let gast = gast_iau2006(ut1_jd, tt_jd).map_err(HorizontalConversionError::Orientation)?;
    let hour_deg = (gast.to_degrees() + longitude_deg_east - right_ascension_deg + 180.0)
        .rem_euclid(360.0)
        - 180.0;
    let direction = equatorial_to_horizontal(
        hour_deg.to_radians(),
        declination_deg.to_radians(),
        latitude_deg.to_radians(),
    )
    .map_err(HorizontalConversionError::Geometry)?;
    let azimuth_north_east_deg = direction
        .azimuth_rad
        .map(|value| value.to_degrees().rem_euclid(360.0));
    let azimuth_south_west_deg =
        azimuth_north_east_deg.map(|value| (value + 180.0).rem_euclid(360.0));
    Ok(HorizontalDegrees {
        true_altitude_deg: direction.altitude_rad.to_degrees(),
        azimuth_north_east_deg,
        azimuth_south_west_deg,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizontalError {
    InvalidHourAngle,
    InvalidDeclination,
    InvalidLatitude,
    NonFiniteDirection,
}

impl fmt::Display for HorizontalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHourAngle => write!(f, "hour angle must be in [-pi, pi) radians"),
            Self::InvalidDeclination => {
                write!(f, "declination must be in [-pi/2, pi/2] radians")
            }
            Self::InvalidLatitude => write!(f, "latitude must be in [-pi/2, pi/2] radians"),
            Self::NonFiniteDirection => write!(f, "non-finite horizontal direction"),
        }
    }
}
impl Error for HorizontalError {}

/// Convert a local equatorial direction into geometric altitude and azimuth.
///
/// - `hour_angle_rad`: local H in [-π, π), **positive west** of the
///   local meridian. See the standalone LMST primitive in draft PR #79.
/// - `declination_rad`: equatorial declination in [-π/2, π/2].
/// - `latitude_rad`: observer geographic latitude in [-π/2, π/2],
///   positive north; this is treated as the adopted astronomical latitude,
///   without polar-motion or local vertical/deflection corrections.
///
/// All angular inputs and outputs use radians. The USNO/SOFA convention
/// gives azimuth north = 0, east = π/2, south = π, west = 3π/2.
///
/// In an east/north/up orthonormal local frame, for latitude φ,
/// declination δ, and local hour angle H:
///
/// `east = -cos(δ) sin(H)`
/// `north = sin(δ) cos(φ) - cos(δ) sin(φ) cos(H)`
/// `up = sin(δ) sin(φ) + cos(δ) cos(φ) cos(H)`.
///
/// Altitude is `atan2(up, hypot(east, north))`; azimuth is
/// `atan2(east, north)` normalized to [0, 2π).
/// At exact poles the north reference is not geographically unique.
/// A small angular-distance threshold (32*f64::EPSILON) marks azimuth
/// undefined near zenith/nadir; this is a numerical guard, not an accuracy
/// claim or a substitute for full observer geometry.
///
/// No atmospheric, apparent, parallax, light-time or physical horizon
/// corrections are performed. This is not Swiss `swe_azalt` parity.
pub fn equatorial_to_horizontal(
    hour_angle_rad: f64,
    declination_rad: f64,
    latitude_rad: f64,
) -> Result<HorizontalDirection, HorizontalError> {
    if !hour_angle_rad.is_finite() || !(-PI..PI).contains(&hour_angle_rad) {
        return Err(HorizontalError::InvalidHourAngle);
    }
    if !declination_rad.is_finite() || !(-FRAC_PI_2..=FRAC_PI_2).contains(&declination_rad) {
        return Err(HorizontalError::InvalidDeclination);
    }
    if !latitude_rad.is_finite() || !(-FRAC_PI_2..=FRAC_PI_2).contains(&latitude_rad) {
        return Err(HorizontalError::InvalidLatitude);
    }

    let (sin_dec, cos_dec) = declination_rad.sin_cos();
    let (sin_lat, cos_lat) = latitude_rad.sin_cos();
    let (sin_h, cos_h) = hour_angle_rad.sin_cos();

    let east = -cos_dec * sin_h;
    let north = sin_dec * cos_lat - cos_dec * sin_lat * cos_h;
    let up = sin_dec * sin_lat + cos_dec * cos_lat * cos_h;
    let planar = east.hypot(north);
    let altitude_rad = up.atan2(planar);

    if [east, north, up, planar, altitude_rad]
        .iter()
        .any(|v| !v.is_finite())
    {
        return Err(HorizontalError::NonFiniteDirection);
    }

    let azimuth_rad = if latitude_rad.abs() == FRAC_PI_2 || planar <= 32.0 * f64::EPSILON {
        None
    } else {
        Some(east.atan2(north).rem_euclid(TAU))
    };
    Ok(HorizontalDirection {
        altitude_rad,
        azimuth_rad,
    })
}
