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

//! Geometric observer-to-target sightline in a common Earth-centered GCRS.
//!
//! Both position vectors must already refer to the same coordinate epoch,
//! Earth-centered origin, GCRS axes, and kilometers. This module neither
//! queries DE441 nor converts barycentric ICRF/planetary states to GCRS.
//! It applies **no** propagation time, gravitational light deflection,
//! aberration, atmospheric refraction, limb, apparent, or EOP correction.
//!
//! Independent spherical-vector oracle: SOFA/ERFA test t_p2s.
//! <https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c>
//! Original VirgoStars numerical implementation, 2026-11-12.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;

use crate::observer_gcrs::{
    wgs84_observer_to_gcrs, ObserverGcrsError, ObserverGcrsPosition, ObserverOrientationInputs,
    Wgs84ObserverSite,
};

/// **Geometric** direction from a fixed-epoch GCRS observer toward a
/// fixed-epoch GCRS target. All Cartesian quantities are kilometers.
///
/// Right ascension is measured about the supplied GCRS +Z axis from
/// +X toward +Y in [0,2π) radians. It is `None` at either exact
/// celestial pole, where the equatorial azimuth is undefined.
/// Declination lies in [-π/2,π/2], positive north.
/// These axes are **GCRS**, not an equinox/equator-of-date chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometricTopocentricDirection {
    /// Target position minus observer position, in GCRS kilometers.
    pub displacement_gcrs_km: [f64; 3],
    pub distance_km: f64,
    pub unit_direction_gcrs: [f64; 3],
    pub right_ascension_rad: Option<f64>,
    pub declination_rad: f64,
}

/// A site solution with a separately supplied geocentric target vector.
///
/// `observer` is obtained via explicit Earth-orientation parameters,
/// not inferred from UTC or a default EOP table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wgs84GeometricTopocentricDirection {
    pub observer: ObserverGcrsPosition,
    pub direction: GeometricTopocentricDirection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopocentricGeometryError {
    NonFiniteTarget,
    NonFiniteObserver,
    NonFiniteDisplacement,
    CoincidentTargetAndObserver,
    InvalidDistance,
    Observer(ObserverGcrsError),
}

impl fmt::Display for TopocentricGeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteTarget => write!(f, "target GCRS coordinates must be finite kilometers"),
            Self::NonFiniteObserver => {
                write!(f, "observer GCRS coordinates must be finite kilometers")
            }
            Self::NonFiniteDisplacement => write!(f, "non-finite target-observer displacement"),
            Self::CoincidentTargetAndObserver => write!(f, "target and observer coincide"),
            Self::InvalidDistance => write!(f, "non-finite or unrepresentable sightline distance"),
            Self::Observer(error) => write!(f, "observer position: {error}"),
        }
    }
}

impl Error for TopocentricGeometryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Observer(error) => Some(error),
            _ => None,
        }
    }
}

/// Compute instantaneous **geometric** target-minus-observer direction.
///
/// Both inputs are Earth-centered GCRS positions in **kilometers**, for
/// the **same physical coordinate epoch**. Their frame, center, time
/// and physical target definitions must already agree. In particular,
/// do **not** pass an unconverted DE441 barycentric/ICRF state as
/// `geocentric_target_gcrs_km`; use the correctly transformed
/// Earth-centered target position from an independently validated
/// upstream ephemeris and frame model.
///
/// This function subtracts the two position vectors, forms their
/// Euclidean range with nested `hypot`, normalizes the displacement,
/// and projects the GCRS equatorial direction. At exact +Z/-Z the
/// right ascension is undefined and returned as `None`; the
/// declination and geometric range remain well-defined.
///
/// There is no moving-observer velocity or light-time solve. The
/// resulting ascension and declination are **not apparent** or
/// equinox-of-date sky coordinates. The angular separation from a
/// similarly computed geocentric direction is geometric parallax,
/// not a complete observed parallax or physical astronomy validation.
pub fn geometric_topocentric_direction_gcrs(
    geocentric_target_gcrs_km: [f64; 3],
    observer_gcrs_km: [f64; 3],
) -> Result<GeometricTopocentricDirection, TopocentricGeometryError> {
    if geocentric_target_gcrs_km.iter().any(|v| !v.is_finite()) {
        return Err(TopocentricGeometryError::NonFiniteTarget);
    }
    if observer_gcrs_km.iter().any(|v| !v.is_finite()) {
        return Err(TopocentricGeometryError::NonFiniteObserver);
    }
    let displacement: [f64; 3] =
        std::array::from_fn(|i| geocentric_target_gcrs_km[i] - observer_gcrs_km[i]);
    if displacement.iter().any(|v| !v.is_finite()) {
        return Err(TopocentricGeometryError::NonFiniteDisplacement);
    }

    let [x, y, z] = displacement;
    let equatorial_plane_km = x.hypot(y);
    let distance_km = equatorial_plane_km.hypot(z);
    if distance_km == 0.0 {
        return Err(TopocentricGeometryError::CoincidentTargetAndObserver);
    }
    if !distance_km.is_finite() {
        return Err(TopocentricGeometryError::InvalidDistance);
    }
    let unit_direction_gcrs = [x / distance_km, y / distance_km, z / distance_km];
    let declination_rad = z.atan2(equatorial_plane_km);
    let right_ascension_rad = if equatorial_plane_km == 0.0 {
        None
    } else {
        let ra = y.atan2(x).rem_euclid(TAU);
        Some(if ra == 0.0 || ra >= TAU { 0.0 } else { ra })
    };
    if unit_direction_gcrs.iter().any(|v| !v.is_finite()) || !declination_rad.is_finite() {
        return Err(TopocentricGeometryError::InvalidDistance);
    }
    Ok(GeometricTopocentricDirection {
        displacement_gcrs_km: displacement,
        distance_km,
        unit_direction_gcrs,
        right_ascension_rad,
        declination_rad,
    })
}

/// Compose the WGS84 site->GCRS observer position and its geometric sightline.
///
/// `orientation` requires a tagged UT1 epoch and externally supplied
/// same-epoch polar-motion/CIP/CIO parameters, as described by
/// `observer_gcrs::wgs84_observer_to_gcrs`. The target must already be
/// an **Earth-centered GCRS vector** with the same physical epoch.
///
/// **This is not a DE441 reader or apparent planetary pipeline.**
pub fn wgs84_topocentric_geometric_direction_gcrs(
    site: Wgs84ObserverSite,
    orientation: ObserverOrientationInputs,
    geocentric_target_gcrs_km: [f64; 3],
) -> Result<Wgs84GeometricTopocentricDirection, TopocentricGeometryError> {
    if geocentric_target_gcrs_km.iter().any(|v| !v.is_finite()) {
        return Err(TopocentricGeometryError::NonFiniteTarget);
    }
    let observer =
        wgs84_observer_to_gcrs(site, orientation).map_err(TopocentricGeometryError::Observer)?;
    let direction =
        geometric_topocentric_direction_gcrs(geocentric_target_gcrs_km, observer.position_gcrs_km)?;
    Ok(Wgs84GeometricTopocentricDirection {
        observer,
        direction,
    })
}
