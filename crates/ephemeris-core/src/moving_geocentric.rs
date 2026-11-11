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

//! Geocentric reception light-time vectors projected into **caller-supplied
//! moving axes**, with exact Cartesian frame-rate contribution.
//!
//! One original, compositional step toward apparent-of-date six-field
//! astronomy. No ephemeris coefficient, IAU frame model, aberration,
//! precession or nutation is invented here. This is NOT `swe_calc_ut`.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::coordinates::{project_fixed_axes, ProjectionError, SphericalState};
use crate::de441::{Body, De441, JulianDateTdb, StateVector};
use crate::light_time::{
    geocentric_light_time, GeocentricLightTimeState, LightTimeError,
};
use crate::rotating_frame::{RotatingFrame, RotatingFrameError};

/// The six spherical fields and three reception-time **TDB day** rates,
/// expressed in an explicitly supplied **moving** set of axes.
///
/// `spherical.tilt_rad` is **0** only because no *additional constant
/// X tilt* is applied AFTER the general moving-frame rotation. The
/// coordinates themselves are already in **moving axes**. This
/// wrapper must be retained downstream to avoid mislabeling them as
/// fixed ICRF or apparent true-ecliptic-of-date coordinates.
///
/// The `target` is kept exactly as specified by DE441; a planet system
/// barycenter is NOT automatically a physical planet center. The
/// observer center is `Body::Earth`. Epochs are Julian dates **TDB**.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovingGeocentricSpherical {
    pub spherical: SphericalState,
    pub emission_epoch: JulianDateTdb,
    pub light_time_seconds: f64,
    pub light_time_rate: f64,
}

/// Typed errors ensure no invalid state is presented as an output.
#[derive(Debug)]
pub enum MovingGeocentricError {
    InvalidIntermediate,
    LightTime(LightTimeError),
    Frame(RotatingFrameError),
    Projection(ProjectionError),
}

impl fmt::Display for MovingGeocentricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIntermediate => {
                write!(f, "invalid geocentric light-time intermediate")
            }
            Self::LightTime(error) => write!(f, "geocentric reception light time: {error}"),
            Self::Frame(error) => write!(f, "moving-frame rotation: {error}"),
            Self::Projection(error) => write!(f, "moving-frame spherical projection: {error}"),
        }
    }
}
impl Error for MovingGeocentricError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LightTime(error) => Some(error),
            Self::Frame(error) => Some(error),
            Self::Projection(error) => Some(error),
            Self::InvalidIntermediate => None,
        }
    }
}
impl From<LightTimeError> for MovingGeocentricError {
    fn from(error: LightTimeError) -> Self {
        Self::LightTime(error)
    }
}
impl From<RotatingFrameError> for MovingGeocentricError {
    fn from(error: RotatingFrameError) -> Self {
        Self::Frame(error)
    }
}
impl From<ProjectionError> for MovingGeocentricError {
    fn from(error: ProjectionError) -> Self {
        Self::Projection(error)
    }
}

/// A moving-axes spherical result whose origin and frame are supplied
/// explicitly by its caller, WITHOUT assuming any physical correction.
///
/// This wrapper is important because the reused `SphericalState` field
/// has `tilt_rad = 0` after a general moving-axis rotation. The zero
/// means zero **extra** constant X tilt, not fixed ICRF axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovingSphericalState {
    pub spherical: SphericalState,
}

/// Rotate an already-computed fixed-ICRF `StateVector` and its reception
/// velocity into caller-declared **moving axes**, including `Rdot*r`,
/// then reuse the existing spherical projector with zero *extra* tilt.
///
/// This is the future integration point for **already-aberrated**
/// geocentric Cartesian position and rate vectors (from independent
/// draft PR #77). It deliberately does not calculate or reapply
/// light-time, annual aberration or any astronomical model. The caller
/// must supply matching epoch/frame semantics for the input state and
/// the per-reception-TDB-day `RotatingFrame` derivative. The target
/// and center are copied without relabeling system barycenters.
pub fn moving_spherical_from_icrf_state(
    state: &StateVector,
    frame: &RotatingFrame,
) -> Result<MovingSphericalState, MovingGeocentricError> {
    let moved = frame.rotate_state(state)?;
    Ok(MovingSphericalState {
        spherical: project_fixed_axes(&moved, 0.0)?,
    })
}

/// Project an existing **ICRF** Earth-to-target light-time state through a
/// caller-supplied `RotatingFrame` at the **same reception TDB epoch**.
///
/// This reuses `RotatingFrame::rotate_state`, which includes the essential
/// `v_rot = R*v + Rdot*r` (not just R*v), and the existing
/// `coordinates::project_fixed_axes` spherical position/rate arithmetic.
/// The fixed-axes projector is invoked with **zero additional tilt**.
/// No additional light-time correction is applied and no IAU model,
/// TT-to-TDB rate ratio or time-origin conversion is assumed.
///
/// `RotatingFrame` does not carry an epoch or frame name; the caller MUST
/// associate its R and Rdot with this specific reception epoch and with
/// the input's original fixed ICRF-aligned axes. Feeding the output to
/// routines expecting ICRF would be a scientific error.
pub fn moving_spherical_from_light_time(
    source: &GeocentricLightTimeState,
    frame: &RotatingFrame,
) -> Result<MovingGeocentricSpherical, MovingGeocentricError> {
    if source.target == Body::Earth
        || !source.observation_epoch.0.is_finite()
        || !source.emission_epoch.0.is_finite()
        || source.emission_epoch.0 > source.observation_epoch.0
        || !source.light_time_seconds.is_finite()
        || source.light_time_seconds <= 0.0
        || !source.light_time_rate.is_finite()
        || source
            .position_km
            .iter()
            .chain(source.velocity_km_per_day.iter())
            .any(|value| !value.is_finite())
    {
        return Err(MovingGeocentricError::InvalidIntermediate);
    }

    let original = StateVector {
        target: source.target,
        center: Body::Earth,
        epoch: source.observation_epoch,
        position_km: source.position_km,
        velocity_km_per_day: source.velocity_km_per_day,
    };
    let spherical = moving_spherical_from_icrf_state(&original, frame)?.spherical;
    Ok(MovingGeocentricSpherical {
        spherical,
        emission_epoch: source.emission_epoch,
        light_time_seconds: source.light_time_seconds,
        light_time_rate: source.light_time_rate,
    })
}

/// One-call *geometric* classic DE441 → reception light-time → caller-supplied
/// moving frame → six spherical coordinates and three signed rates.
///
/// This is **not an apparent position**: stellar aberration, light
/// deflection, frame bias/precession/nutation and true-ecliptic-of-date
/// orientation have not been supplied or evaluated. The `RotatingFrame`
/// matrices must be separately validated, expressed per observation
/// **TDB day**, and consistent with this TDB reception epoch; published
/// IAU precession/nutation TT derivatives cannot be substituted without
/// an explicit correct TT/TDB derivative conversion.
///
/// A `.bsp` SPK or Swiss `.se1` is not a classic DE441 `.eph` file.
pub fn geocentric_light_time_moving_spherical<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    observation_epoch: JulianDateTdb,
    frame: &RotatingFrame,
) -> Result<MovingGeocentricSpherical, MovingGeocentricError> {
    let light_time = geocentric_light_time(ephemeris, target, observation_epoch)?;
    moving_spherical_from_light_time(&light_time, frame)
}
