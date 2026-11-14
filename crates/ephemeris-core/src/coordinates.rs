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

//! Spherical projection of geometric ICRF state vectors into fixed axes.
//!
//! An explicitly supplied fixed rotation is not an ecliptic-of-date model:
//! no precession, nutation, frame bias or apparent corrections are applied.
//!
//! Modified 2026-11-14: guard the fixed-axis longitude-rate numerator against
//! avoidable floating-point overflow when the final derivative is finite.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;

use crate::de441::{Body, JulianDateTdb, StateVector};

/// Geometric coordinates and derivatives in the fixed axes described by `tilt_rad`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SphericalState {
    pub target: Body,
    pub center: Body,
    pub epoch: JulianDateTdb,
    /// Fixed passive rotation of the axes about +X, in radians.
    pub tilt_rad: f64,
    /// Longitude in [0, 2π), measured from the original +X axis.
    pub longitude_rad: f64,
    /// Latitude in [-π/2, π/2].
    pub latitude_rad: f64,
    pub distance_km: f64,
    /// Longitude derivative, not multiplied by cos(latitude).
    pub longitude_rate_rad_per_day: f64,
    pub latitude_rate_rad_per_day: f64,
    pub distance_rate_km_per_day: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    /// An input or output is non-finite or exceeds the floating-point range.
    NonFinite,
    /// Longitude and angular rates are undefined at the origin or a pole.
    UndefinedDirection,
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => write!(f, "non-finite spherical projection input or result"),
            Self::UndefinedDirection => write!(f, "longitude or angular rate is undefined"),
        }
    }
}
impl Error for ProjectionError {}

/// Project a geometric ICRF state into fixed axes rotated about +X.
///
/// Rotation is passive: `x'=x`, `y'=cos(tilt)*y+sin(tilt)*z`,
/// `z'=-sin(tilt)*y+cos(tilt)*z`. The tilt is held constant when calculating
/// derivatives, so this is not suitable for speeds in a rotating date frame.
/// Use zero tilt for the original equatorial ICRF axes. A caller can explicitly
/// supply an obliquity for an approximate fixed ecliptic projection.
pub fn project_fixed_axes(
    state: &StateVector,
    tilt_rad: f64,
) -> Result<SphericalState, ProjectionError> {
    if !tilt_rad.is_finite()
        || !state.epoch.0.is_finite()
        || state
            .position_km
            .iter()
            .chain(state.velocity_km_per_day.iter())
            .any(|v| !v.is_finite())
    {
        return Err(ProjectionError::NonFinite);
    }
    let (sin_tilt, cos_tilt) = tilt_rad.sin_cos();
    let rotate = |v: [f64; 3]| {
        [
            v[0],
            cos_tilt * v[1] + sin_tilt * v[2],
            -sin_tilt * v[1] + cos_tilt * v[2],
        ]
    };
    let [x, y, z] = rotate(state.position_km);
    let [vx, vy, vz] = rotate(state.velocity_km_per_day);
    if [x, y, z, vx, vy, vz].iter().any(|v| !v.is_finite()) {
        return Err(ProjectionError::NonFinite);
    }
    let planar = x.hypot(y);
    let distance = planar.hypot(z);
    if !distance.is_finite() {
        return Err(ProjectionError::NonFinite);
    }
    if planar == 0.0 {
        return Err(ProjectionError::UndefinedDirection);
    }
    let (ux, uy) = (x / planar, y / planar);
    let planar_rate = ux * vx + uy * vy;
    // Divide before taking the cross product for distances >= 1 km. With
    // finite but very large velocities, the unscaled numerator can overflow
    // even when the angular rate itself is representable. For sub-km planar
    // distances retain the numerator-first form to avoid premature overflow
    // from dividing otherwise finite velocity components by tiny distances.
    let longitude_rate = if planar >= 1.0 {
        ux * (vy / planar) - uy * (vx / planar)
    } else {
        (ux * vy - uy * vx) / planar
    };
    let latitude_rate =
        (planar / distance) * (vz / distance) - (z / distance) * (planar_rate / distance);
    let distance_rate = (planar / distance) * planar_rate + (z / distance) * vz;
    let longitude = y.atan2(x).rem_euclid(TAU);
    let longitude = if longitude == 0.0 || longitude >= TAU {
        0.0
    } else {
        longitude
    };
    let result = SphericalState {
        target: state.target,
        center: state.center,
        epoch: state.epoch,
        tilt_rad,
        longitude_rad: longitude,
        latitude_rad: z.atan2(planar),
        distance_km: distance,
        longitude_rate_rad_per_day: longitude_rate,
        latitude_rate_rad_per_day: latitude_rate,
        distance_rate_km_per_day: distance_rate,
    };
    if [
        result.longitude_rad,
        result.latitude_rad,
        result.distance_km,
        result.longitude_rate_rad_per_day,
        result.latitude_rate_rad_per_day,
        result.distance_rate_km_per_day,
    ]
    .iter()
    .any(|v| !v.is_finite())
    {
        return Err(ProjectionError::NonFinite);
    }
    Ok(result)
}
