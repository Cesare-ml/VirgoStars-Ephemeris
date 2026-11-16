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

//! Spherical projection of ICRF state vectors into fixed axes.
//!
//! An explicitly supplied fixed rotation is not an ecliptic-of-date model:
//! no precession, nutation, frame bias or apparent corrections are applied.
//!
//! Modified 2026-11-14: guard the fixed-axis longitude-rate numerator against
//! avoidable floating-point overflow when the final derivative is finite.
//!
//! Modified 2026-11-14: share a single fixed-axis projection implementation
//! for classic DE441 (km/day) and SPK (km/second) geometric state adapters.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;

use crate::de441::{Body, JulianDateTdb, StateVector};
use crate::spk_relative_state::SpkRelativeState;

/// Coordinates and derivatives in the fixed axes described by `tilt_rad`.
/// Applied physical corrections are determined by the input, not this type.
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

/// Geometric SPK relative state projected into explicitly fixed axes.
///
/// Input ET is TDB seconds since J2000. Angular rates are rad/TDB-second
/// and radial speed is km/TDB-second, not the DE441 adapter's per-day units.
/// Only fixed J2000/ICRF frame-1 states are supported; this does not perform
/// precession, nutation, time conversion or apparent corrections.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpkSphericalState {
    pub target_id: i32,
    pub center_id: i32,
    pub common_ancestor_id: i32,
    pub tdb_seconds_past_j2000: f64,
    /// Constant passive rotation around +X in radians.
    pub tilt_rad: f64,
    /// Longitude in [0, 2π).
    pub longitude_rad: f64,
    /// Latitude in [-π/2, π/2].
    pub latitude_rad: f64,
    pub distance_km: f64,
    /// Longitude derivative, not multiplied by cos(latitude).
    pub longitude_rate_rad_per_second: f64,
    pub latitude_rate_rad_per_second: f64,
    pub distance_rate_km_per_second: f64,
}

struct FixedSphericalComponents {
    longitude_rad: f64,
    latitude_rad: f64,
    distance_km: f64,
    longitude_rate: f64,
    latitude_rate: f64,
    distance_rate: f64,
}

/// One numerical implementation for input velocities expressed per day or
/// per TDB second: all derivatives remain in the caller's velocity time unit.
fn fixed_spherical_components(
    position_km: [f64; 3],
    velocity_km_per_time_unit: [f64; 3],
    tilt_rad: f64,
) -> Result<FixedSphericalComponents, ProjectionError> {
    if !tilt_rad.is_finite()
        || position_km
            .iter()
            .chain(velocity_km_per_time_unit.iter())
            .any(|value| !value.is_finite())
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
    let [x, y, z] = rotate(position_km);
    let [vx, vy, vz] = rotate(velocity_km_per_time_unit);
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
    // For >=1 km, divide before the cross product to avoid overflow in an
    // otherwise finite longitude rate; keep numerator-first below 1 km.
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
    let result = FixedSphericalComponents {
        longitude_rad: longitude,
        latitude_rad: z.atan2(planar),
        distance_km: distance,
        longitude_rate,
        latitude_rate,
        distance_rate,
    };
    if [
        result.longitude_rad,
        result.latitude_rad,
        result.distance_km,
        result.longitude_rate,
        result.latitude_rate,
        result.distance_rate,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(ProjectionError::NonFinite);
    }
    Ok(result)
}

/// Project a DE441-shaped ICRF state into fixed axes rotated about +X.
///
/// The passive rotation about +X and its angular rates are evaluated with
/// a constant tilt; this is not a date-frame model. Zero tilt selects ICRF.
/// All velocity and angular-rate units here remain per TDB day. This
/// mathematical projection neither adds nor removes physical corrections.
pub fn project_fixed_axes(
    state: &StateVector,
    tilt_rad: f64,
) -> Result<SphericalState, ProjectionError> {
    if !state.epoch.0.is_finite() {
        return Err(ProjectionError::NonFinite);
    }
    let components =
        fixed_spherical_components(state.position_km, state.velocity_km_per_day, tilt_rad)?;
    Ok(SphericalState {
        target: state.target,
        center: state.center,
        epoch: state.epoch,
        tilt_rad,
        longitude_rad: components.longitude_rad,
        latitude_rad: components.latitude_rad,
        distance_km: components.distance_km,
        longitude_rate_rad_per_day: components.longitude_rate,
        latitude_rate_rad_per_day: components.latitude_rate,
        distance_rate_km_per_day: components.distance_rate,
    })
}

/// Project a geometric frame-1 J2000/ICRF SPK target-minus-center state.
///
/// ET is in TDB seconds from J2000; all output rates are per TDB second.
/// A fixed passive X-axis tilt can be applied, but no rotating frame,
/// precession/nutation, light-time, aberration or observer correction occurs.
/// Rejects nonfinite epochs, axes/rates and undefined longitude at poles.
pub fn project_spk_fixed_axes(
    state: &SpkRelativeState,
    tilt_rad: f64,
) -> Result<SpkSphericalState, ProjectionError> {
    if !state.tdb_seconds_past_j2000.is_finite() {
        return Err(ProjectionError::NonFinite);
    }
    let components =
        fixed_spherical_components(state.position_km, state.velocity_km_per_second, tilt_rad)?;
    Ok(SpkSphericalState {
        target_id: state.target_id,
        center_id: state.center_id,
        common_ancestor_id: state.common_ancestor_id,
        tdb_seconds_past_j2000: state.tdb_seconds_past_j2000,
        tilt_rad,
        longitude_rad: components.longitude_rad,
        latitude_rad: components.latitude_rad,
        distance_km: components.distance_km,
        longitude_rate_rad_per_second: components.longitude_rate,
        latitude_rate_rad_per_second: components.latitude_rate,
        distance_rate_km_per_second: components.distance_rate,
    })
}
