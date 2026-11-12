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

//! Compose independently developed observer/site/rotation primitives.
//!
//! Experimental fixed-epoch position-only WGS84->ITRS->GCRS mapping.
//! Input Earth orientation parameters and CIO/CIP components must be
//! externally sourced and physically coherent at the specified epoch.
//! There are NO implicit EOP downloads, no TT/UT1/UTC conversions, no
//! fallback EOP values, and no DE441/ICRF or apparent-sky correction.
//!
//! Source geometry: IERS Conventions (2010) chapter 5.
//! <https://iers-conventions.obspm.fr/conventions_material.php>
//! This is an original VirgoStars integration, not third-party source code.

use std::error::Error;
use std::fmt;

use crate::calendar::JulianDate;
use crate::cio_orientation::{gcrs_to_cirs_cio_matrix, CioOrientationError};
use crate::earth_rotation::{earth_rotation_angle, EarthRotationError};
use crate::orientation_chain::{compose_gcrs_to_itrs, OrientationChainError};
use crate::polar_motion::{itrs_to_tirs_polar_motion_matrix, PolarMotionError};
use crate::wgs84_ecef::{wgs84_geodetic_to_ecef, Wgs84Error};

/// Observer geodetic site on the WGS84 ellipsoid.
///
/// Longitude and latitude in radians (east/north positive), height
/// above the WGS84 ellipsoid in meters. Orthometric height is NOT valid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wgs84ObserverSite {
    pub longitude_rad_east: f64,
    pub latitude_rad_north: f64,
    pub ellipsoidal_height_m: f64,
}

/// Externally obtained orientation parameters for ONE physical epoch.
///
/// All angular fields are in radians, except `cip_x` and `cip_y`
/// which are dimensionless GCRS CIP unit-vector components.
/// `ut1_epoch` must be explicitly tagged UT1; the CIO locator
/// and pole components must come from the matching TT epoch/model.
/// xp/yp/s' must come from a compatible EOP realization and epoch.
/// This API cannot verify the provenance or time equivalence of
/// externally supplied TT-based and UT1-based inputs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObserverOrientationInputs {
    pub ut1_epoch: JulianDate,
    pub polar_xp_rad: f64,
    pub polar_yp_rad: f64,
    pub tio_locator_sp_rad: f64,
    pub cip_x: f64,
    pub cip_y: f64,
    pub cio_locator_s_rad: f64,
}

/// The position of one geodetic site at the stated, caller-prepared
/// orientation epoch in the listed, Earth-centered reference frames.
///
/// `position_itrs_km` and `position_gcrs_km` are **positions only**.
/// No observer inertial velocity, frame-rate derivative, polar motion
/// rate, station motion, geocenter ephemeris, or apparent correction
/// is provided. The GCRS output must not be silently treated as a
/// DE441 barycentric/ICRF vector without further model reconciliation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObserverGcrsPosition {
    pub position_itrs_km: [f64; 3],
    pub position_gcrs_km: [f64; 3],
    pub era_rad: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObserverGcrsError {
    Geodesy(Wgs84Error),
    EarthRotation(EarthRotationError),
    PolarMotion(PolarMotionError),
    CioOrientation(CioOrientationError),
    Composition(OrientationChainError),
}

impl fmt::Display for ObserverGcrsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geodesy(error) => write!(f, "observer WGS84 geometry: {error}"),
            Self::EarthRotation(error) => write!(f, "observer UT1 Earth rotation: {error}"),
            Self::PolarMotion(error) => write!(f, "observer polar motion: {error}"),
            Self::CioOrientation(error) => write!(f, "observer CIP/CIO: {error}"),
            Self::Composition(error) => write!(f, "observer frame composition: {error}"),
        }
    }
}

impl Error for ObserverGcrsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Geodesy(error) => Some(error),
            Self::EarthRotation(error) => Some(error),
            Self::PolarMotion(error) => Some(error),
            Self::CioOrientation(error) => Some(error),
            Self::Composition(error) => Some(error),
        }
    }
}

/// Convert a WGS84 geodetic site to an Earth-centered GCRS position.
///
/// Steps, all at one caller-chosen physical epoch:
/// 1. Convert WGS84 longitude, geodetic latitude and ellipsoidal
///    height into ECEF Cartesian kilometers, assuming that the
///    coordinates identify the adopted ITRS realization.
/// 2. Evaluate IAU 2000 ERA from the tagged split-JD UT1 epoch.
/// 3. Form W(t) ITRS->TIRS using externally supplied xp/yp/s'.
/// 4. Form C(t) GCRS->CIRS using externally supplied CIP X/Y/CIO s.
/// 5. Compose R = W(t)^T * Rz(-ERA) * C(t); apply R^T to the ITRS
///    observer position, giving its GCRS Cartesian kilometers.
///
/// Input consistency, geodetic realization/epoch, TT-based X/Y/s
/// series, EOP acquisition/interpolation and geophysical effects are
/// the caller's responsibility. **This function does not compute**
/// physically validated EOP/CIP parameters or inertial velocities.
/// It is not yet a validated topocentric DE441 observation.
pub fn wgs84_observer_to_gcrs(
    site: Wgs84ObserverSite,
    orientation: ObserverOrientationInputs,
) -> Result<ObserverGcrsPosition, ObserverGcrsError> {
    let position_itrs_km = wgs84_geodetic_to_ecef(
        site.longitude_rad_east,
        site.latitude_rad_north,
        site.ellipsoidal_height_m,
    )
    .map_err(ObserverGcrsError::Geodesy)?
    .kilometers();

    let era = earth_rotation_angle(orientation.ut1_epoch)
        .map_err(ObserverGcrsError::EarthRotation)?;
    let w = itrs_to_tirs_polar_motion_matrix(
        orientation.polar_xp_rad,
        orientation.polar_yp_rad,
        orientation.tio_locator_sp_rad,
    )
    .map_err(ObserverGcrsError::PolarMotion)?;
    let c = gcrs_to_cirs_cio_matrix(
        orientation.cip_x,
        orientation.cip_y,
        orientation.cio_locator_s_rad,
    )
    .map_err(ObserverGcrsError::CioOrientation)?;
    let composed = compose_gcrs_to_itrs(w, era.angle_rad, c)
        .map_err(ObserverGcrsError::Composition)?;
    let position_gcrs_km = composed
        .itrs_to_gcrs_position(position_itrs_km)
        .map_err(ObserverGcrsError::Composition)?;
    Ok(ObserverGcrsPosition {
        position_itrs_km,
        position_gcrs_km,
        era_rad: era.angle_rad,
    })
}
