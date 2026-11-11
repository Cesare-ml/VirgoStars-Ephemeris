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

//! One-call geocentric six-field projection with light-time corrections.
//!
//! This deliberately DOES NOT implement Swiss `swe_calc_ut`:
//! output axes are **fixed**, not true ecliptic of date; stellar aberration,
//! gravitational deflection, frame bias/precession/nutation, UT/TT conversion
//! and outer-planet center corrections have NOT been performed.
//! The model is an integration step from an external classic DE441 reader
//! to the six scalar fields and speeds needed for an MVP-1 positions pipeline.

use std::error::Error;
use std::f64::consts::{PI, TAU};
use std::fmt;
use std::io::{Read, Seek};

use crate::coordinates::{project_fixed_axes, ProjectionError};
use crate::de441::{Body, De441, JulianDateTdb, StateVector};
use crate::light_time::{geocentric_light_time, GeocentricLightTimeState, LightTimeError};

/// IAU 2012 Resolution B2: the astronomical unit is exactly
/// 149_597_870_700 metres. A distance in kilometres is divided by this.
pub const ASTRONOMICAL_UNIT_KM: f64 = 149_597_870.7;

const RADIANS_TO_DEGREES: f64 = 180.0 / PI;

/// Exactly what reference axes are used, without implying a date frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeocentricFixedAxes {
    /// Equatorial ICRF aligned axes, with zero applied rotation.
    IcrfEquatorial,
    /// An explicitly chosen *constant* passive X-axis rotation, in radians.
    /// This does NOT establish a canonical IAU J2000 or ecliptic-of-date frame.
    TiltedIcrf { tilt_rad: f64 },
}

impl GeocentricFixedAxes {
    fn tilt(self) -> f64 {
        match self {
            Self::IcrfEquatorial => 0.0,
            Self::TiltedIcrf { tilt_rad } => tilt_rad,
        }
    }
}

/// Six geocentric reception-time fields, with physical and time provenance.
///
/// All angles/speeds refer to the selected fixed axes and are **not**
/// Swiss apparent/ecliptic-of-date outputs. Ranges: longitude [0,360)
/// degrees, latitude [-90,90] degrees, distance positive astronomical
/// units (exact IAU AU definition), angular rates degrees per TDB day,
/// distance rate AU per TDB day. No atmospheric or Earth-topocentric effect.
///
/// The target name is preserved verbatim. In classic DE441, `MarsBarycenter`
/// and `JupiterBarycenter` etc. are SYSTEM BARYCENTERS, not planet centers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeocentricFixedSixFields {
    pub target: Body,
    pub center: Body,
    pub axes: GeocentricFixedAxes,
    pub observation_epoch: JulianDateTdb,
    pub emission_epoch: JulianDateTdb,
    pub light_time_seconds: f64,
    pub light_time_rate: f64,
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub distance_au: f64,
    pub longitude_speed_deg_per_tdb_day: f64,
    pub latitude_speed_deg_per_tdb_day: f64,
    pub distance_speed_au_per_tdb_day: f64,
}

#[derive(Debug)]
pub enum GeocentricFieldsError {
    LightTime(LightTimeError),
    Projection(ProjectionError),
    /// A caller-supplied intermediate incorrectly labels Earth as target,
    /// or its light-time metadata is nonfinite/inconsistent.
    InvalidIntermediate,
}

impl fmt::Display for GeocentricFieldsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LightTime(error) => write!(f, "geocentric light-time: {error}"),
            Self::Projection(error) => write!(f, "geocentric fixed-axis projection: {error}"),
            Self::InvalidIntermediate => write!(f, "invalid geocentric light-time state"),
        }
    }
}

impl Error for GeocentricFieldsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LightTime(error) => Some(error),
            Self::Projection(error) => Some(error),
            Self::InvalidIntermediate => None,
        }
    }
}

impl From<LightTimeError> for GeocentricFieldsError {
    fn from(error: LightTimeError) -> Self {
        Self::LightTime(error)
    }
}

impl From<ProjectionError> for GeocentricFieldsError {
    fn from(error: ProjectionError) -> Self {
        Self::Projection(error)
    }
}

/// One-call bridge from a caller-supplied classic JPL DE441 reader to
/// six fixed-axis, light-time-corrected geocentric fields and speeds.
///
/// Earth is evaluated at observation TDB; the target is evaluated at
/// iterated emission TDB. This calls the existing validated *components*
/// (light-time correction and fixed-axis spherical projection) rather than
/// independently duplicating either astronomy algorithm.
///
/// Input time: a numeric `JulianDateTdb`, already converted to **TDB**.
/// This does not accept a UTC calendar value or a generic `swe_calc_ut` JD.
/// The DE441 reader requires a separately supplied classic binary file,
/// not an SPK `.bsp` or Swiss `.se1` collection.
///
/// Result is an experimental physical intermediate, **not** apparent
/// coordinates or source-parity with Swiss Ephemeris.
pub fn geocentric_fixed_six_fields<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    observation_epoch: JulianDateTdb,
    axes: GeocentricFixedAxes,
) -> Result<GeocentricFixedSixFields, GeocentricFieldsError> {
    let state = geocentric_light_time(ephemeris, target, observation_epoch)?;
    fields_from_light_time(&state, axes)
}

/// Reusable pure conversion for an existing geocentric light-time state.
///
/// The resulting six rates are derivatives with respect to observation
/// **TDB day**. These are the analytic light-time-corrected velocities,
/// not an uncorrected `target_velocity - earth_velocity` approximation.
///
/// `GeocentricLightTimeState` is an explicitly labeled intermediate;
/// no astronomical reference data are supplied by this function.
pub fn fields_from_light_time(
    state: &GeocentricLightTimeState,
    axes: GeocentricFixedAxes,
) -> Result<GeocentricFixedSixFields, GeocentricFieldsError> {
    if state.target == Body::Earth
        || !state.observation_epoch.0.is_finite()
        || !state.emission_epoch.0.is_finite()
        || state.emission_epoch.0 > state.observation_epoch.0
        || !state.light_time_seconds.is_finite()
        || state.light_time_seconds <= 0.0
        || !state.light_time_rate.is_finite()
    {
        return Err(GeocentricFieldsError::InvalidIntermediate);
    }
    let relative = StateVector {
        target: state.target,
        center: Body::Earth,
        // The vector is defined at reception time, even though its
        // target endpoint is the retarded emission-time position.
        epoch: state.observation_epoch,
        position_km: state.position_km,
        velocity_km_per_day: state.velocity_km_per_day,
    };
    let projected = project_fixed_axes(&relative, axes.tilt())?;
    let longitude_deg = (projected.longitude_rad * RADIANS_TO_DEGREES).rem_euclid(360.0);
    let latitude_deg = projected.latitude_rad * RADIANS_TO_DEGREES;
    let distance_au = projected.distance_km / ASTRONOMICAL_UNIT_KM;
    let longitude_speed_deg_per_tdb_day = projected.longitude_rate_rad_per_day * RADIANS_TO_DEGREES;
    let latitude_speed_deg_per_tdb_day = projected.latitude_rate_rad_per_day * RADIANS_TO_DEGREES;
    let distance_speed_au_per_tdb_day = projected.distance_rate_km_per_day / ASTRONOMICAL_UNIT_KM;
    if !longitude_deg.is_finite()
        || !latitude_deg.is_finite()
        || !distance_au.is_finite()
        || distance_au <= 0.0
        || !longitude_speed_deg_per_tdb_day.is_finite()
        || !latitude_speed_deg_per_tdb_day.is_finite()
        || !distance_speed_au_per_tdb_day.is_finite()
        || longitude_deg >= 360.0
        || projected.longitude_rad >= TAU
    {
        return Err(GeocentricFieldsError::InvalidIntermediate);
    }
    Ok(GeocentricFixedSixFields {
        target: state.target,
        center: Body::Earth,
        axes,
        observation_epoch: state.observation_epoch,
        emission_epoch: state.emission_epoch,
        light_time_seconds: state.light_time_seconds,
        light_time_rate: state.light_time_rate,
        longitude_deg,
        latitude_deg,
        distance_au,
        longitude_speed_deg_per_tdb_day,
        latitude_speed_deg_per_tdb_day,
        distance_speed_au_per_tdb_day,
    })
}
