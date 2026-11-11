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

//! Convert WGS84 geodetic observer coordinates into Earth-fixed XYZ.
//!
//! This is ellipsoid geometry **only**. The output uses Earth-fixed WGS84
//! Cartesian axes, not ICRF/J2000, GCRS, apparent sky axes, or a planetary
//! ephemeris center. It must not be subtracted directly from DE441 ICRF
//! states without an independently validated Earth-orientation transform.
//!
//! Parameters and geometry: WGS84 semi-major axis 6378137 m and inverse
//! flattening 298.257223563. Reference independent of this implementation:
//! <https://earth-info.nga.mil/index.php?dir=wgs84&action=wgs84>.
//! Independently published test oracle:
//! <https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c> (t_gd2gc).
//!
//! Created 2026-11-11. Original implementation; no external code copied.

use std::error::Error;
use std::f64::consts::{FRAC_PI_2, PI};
use std::fmt;

/// Equatorial radius of the WGS84 reference ellipsoid (meters).
pub const WGS84_EQUATORIAL_RADIUS_M: f64 = 6_378_137.0;
/// Inverse flattening of the WGS84 reference ellipsoid.
pub const WGS84_INVERSE_FLATTENING: f64 = 298.257_223_563;

/// Earth-fixed position in geocentric Cartesian WGS84 axes.
///
/// X points toward the longitude 0° equatorial meridian, Y toward
/// 90° east at the equator, Z toward the north geodetic pole.
/// Units are meters. This is *not* an inertial position or velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wgs84EcefPosition {
    pub x_m: f64,
    pub y_m: f64,
    pub z_m: f64,
}

impl Wgs84EcefPosition {
    /// Same Earth-fixed axes in kilometers, useful for future transforms
    /// that must still explicitly rotate the observer into an inertial frame.
    pub fn kilometers(self) -> [f64; 3] {
        [self.x_m / 1_000.0, self.y_m / 1_000.0, self.z_m / 1_000.0]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wgs84Error {
    InvalidLongitude,
    InvalidLatitude,
    InvalidHeight,
    NonFinitePosition,
}

impl fmt::Display for Wgs84Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLongitude => write!(f, "WGS84 longitude must be in [-pi, pi] radians"),
            Self::InvalidLatitude => {
                write!(
                    f,
                    "WGS84 geodetic latitude must be in [-pi/2, pi/2] radians"
                )
            }
            Self::InvalidHeight => write!(
                f,
                "WGS84 ellipsoid height must be finite and in [-12000, 100000] meters"
            ),
            Self::NonFinitePosition => write!(f, "WGS84 ECEF position is non-finite"),
        }
    }
}
impl Error for Wgs84Error {}

/// Compute Earth-fixed WGS84 Cartesian position from geodetic coordinates.
///
/// - `longitude_rad_east`: finite east-positive longitude in **[-π, π]**.
/// - `latitude_rad_north`: finite **geodetic** latitude in
///   **[-π/2, π/2]** (north positive). Geodetic and geocentric
///   latitudes differ away from the equator and poles.
/// - `height_m`: finite ellipsoidal height in **[-12,000, 100,000] m**.
///   This explicitly bounded domain matches the current experimental
///   `first_sky::Observer` contract; it is not orthometric/sea-level
///   height and no geoid model is applied.
///
/// Let `f = 1 / inverse_flattening`, `e2 = f(2-f)`, and
/// `N = a / sqrt(1 - e2 sin²(latitude))`. The Cartesian coordinates are
/// `X=(N+h)cos(latitude)cos(longitude)`,
/// `Y=(N+h)cos(latitude)sin(longitude)`, and
/// `Z=(N(1-e2)+h)sin(latitude)`.
///
/// At the poles longitude is geometrically irrelevant; tiny residual
/// floating-point X/Y components may remain. No coordinate epochs,
/// Earth rotation, UT1, precession, nutation, polar motion, geoid,
/// plate tectonics, refraction or topocentric celestial corrections
/// are applied; callers must provide their own frame/time pipeline.
pub fn wgs84_geodetic_to_ecef(
    longitude_rad_east: f64,
    latitude_rad_north: f64,
    height_m: f64,
) -> Result<Wgs84EcefPosition, Wgs84Error> {
    if !longitude_rad_east.is_finite() || !(-PI..=PI).contains(&longitude_rad_east) {
        return Err(Wgs84Error::InvalidLongitude);
    }
    if !latitude_rad_north.is_finite() || !(-FRAC_PI_2..=FRAC_PI_2).contains(&latitude_rad_north) {
        return Err(Wgs84Error::InvalidLatitude);
    }
    if !height_m.is_finite() || !(-12_000.0..=100_000.0).contains(&height_m) {
        return Err(Wgs84Error::InvalidHeight);
    }

    let flattening = 1.0 / WGS84_INVERSE_FLATTENING;
    let eccentricity_squared = flattening * (2.0 - flattening);
    let (sin_lat, cos_lat) = latitude_rad_north.sin_cos();
    let (sin_lon, cos_lon) = longitude_rad_east.sin_cos();
    let radius_of_curvature =
        WGS84_EQUATORIAL_RADIUS_M / (1.0 - eccentricity_squared * sin_lat * sin_lat).sqrt();
    let distance_from_spin_axis = (radius_of_curvature + height_m) * cos_lat;
    let xyz = Wgs84EcefPosition {
        x_m: distance_from_spin_axis * cos_lon,
        y_m: distance_from_spin_axis * sin_lon,
        z_m: (radius_of_curvature * (1.0 - eccentricity_squared) + height_m) * sin_lat,
    };
    if [xyz.x_m, xyz.y_m, xyz.z_m]
        .iter()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err(Wgs84Error::NonFinitePosition);
    }
    Ok(xyz)
}
