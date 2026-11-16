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

//! Geocentric **apparent** ecliptic-of-date positions and rates (MVP 1, row 1).
//!
//! Pipeline, every stage reused from an existing independent module:
//!
//! ```text
//! classic DE441 (TDB, ICRF, km, km/day)
//!   → geocentric one-way light-time          (light_time)
//!   → special-relativistic annual aberration (aberration, aberration_state,
//!                                             analytic Earth acceleration)
//!   → IAU 2006 bias-precession + IAU 2000B nutation + true obliquity
//!                                            (earth_orientation)
//!   → spherical longitude/latitude/distance and the three rates
//!                                            (coordinates, six_field_units)
//! ```
//!
//! # Rates in a rotating frame
//!
//! For `r' = M(t) r` the velocity is `v' = M v + dM/dt r`. `dM/dt` is the
//! **central finite difference of the model matrix** with a half-step of
//! [`MATRIX_RATE_HALF_STEP_DAYS`] TT days. This is a numerical derivative of
//! the declared IAU 2006/2000B matrix, not an analytic precession/nutation
//! rate; its truncation error is far below the model's own accuracy for
//! the shortest 5.6-day nutation term used by IAU 2000B.
//!
//! # Time scales
//!
//! The caller supplies TDB. TT for the orientation matrix is obtained with
//! the first Fairhead & Bretagnon periodic term (1.657 ms amplitude); the
//! omitted terms are below 30 microseconds, which moves the true-of-date
//! frame by far less than 1e-9 arcsecond. Output rates are per TDB day.
//!
//! # Not included (stated, never silently approximated)
//!
//! - gravitational light deflection by the Sun and planets (up to roughly
//!   0.004 arcsecond near the Sun; Swiss applies it by default),
//! - topocentric parallax/diurnal aberration (this is **geocentric**),
//! - planet-center correction for system barycenters: a barycenter target
//!   stays a barycenter in the output `target` label,
//! - epochs outside the IAU 2000B validated range 1900–2100 TT, which fail
//!   closed with [`ApparentError::Orientation`],
//! - any claim of numerical parity with another software package: that
//!   requires the independent comparison described in the MVP 1 gates.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::aberration_state::{
    geocentric_light_time_annual_aberration_analytic,
    geocentric_light_time_annual_aberration_with_earth, AberratedGeocentricState,
    AberratedStateError, ObserverAccelerationSource,
};
use crate::coordinates::{project_fixed_axes, ProjectionError, SphericalState};
use crate::de441::{Body, De441, JulianDateTdb, StateVector};
use crate::earth_orientation::{
    matrix_vector_multiply, true_ecliptic_of_date_matrix, Matrix3x3, OrientationError,
    DAYS_PER_JULIAN_CENTURY, J2000_TT_JD,
};
use crate::six_field_units::{
    fixed_spherical_to_degrees_au, FixedSixFieldsDegreesAu, UnitConversionError,
};

/// Half-step, in TT days, of the central difference used for `dM/dt`.
pub const MATRIX_RATE_HALF_STEP_DAYS: f64 = 0.01;

/// Identifiers of the physical corrections that were applied, in order.
pub const APPLIED_CORRECTIONS: [&str; 4] = [
    "one-way geocentric light-time",
    "annual aberration (special-relativistic)",
    "IAU 2006 frame bias and precession",
    "IAU 2000B nutation and true obliquity",
];

/// Identifiers of corrections that are **not** applied.
pub const NOT_APPLIED_CORRECTIONS: [&str; 3] = [
    "gravitational light deflection",
    "topocentric parallax and diurnal aberration",
    "planetary-system barycenter to body-center offset",
];

/// Geocentric apparent spherical state in the true ecliptic and equinox of
/// date. `spherical.tilt_rad` is always zero (the rotation is already
/// included in the Cartesian vector); longitude is in `[0, 2π)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApparentEclipticOfDate {
    pub spherical: SphericalState,
    pub tt_jd: f64,
    pub emission_epoch: JulianDateTdb,
    pub light_time_seconds: f64,
    pub light_time_rate: f64,
    pub observer_acceleration_source: ObserverAccelerationSource,
}

/// The same six fields in degrees, AU and per **TDB** day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApparentSixFieldsDegreesAu {
    pub fields: FixedSixFieldsDegreesAu,
    pub tt_jd: f64,
    pub emission_epoch: JulianDateTdb,
    pub light_time_seconds: f64,
    pub light_time_rate: f64,
    pub observer_acceleration_source: ObserverAccelerationSource,
}

#[derive(Debug)]
pub enum ApparentError {
    InvalidIntermediate,
    Aberration(AberratedStateError),
    Orientation(OrientationError),
    Projection(ProjectionError),
    Units(UnitConversionError),
}

impl fmt::Display for ApparentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIntermediate => write!(f, "invalid aberrated intermediate state"),
            Self::Aberration(e) => write!(f, "light-time/aberration: {e}"),
            Self::Orientation(e) => write!(f, "earth orientation: {e}"),
            Self::Projection(e) => write!(f, "spherical projection: {e}"),
            Self::Units(e) => write!(f, "six-field units: {e}"),
        }
    }
}

impl Error for ApparentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Aberration(e) => Some(e),
            Self::Orientation(e) => Some(e),
            Self::Projection(e) => Some(e),
            Self::Units(e) => Some(e),
            Self::InvalidIntermediate => None,
        }
    }
}

impl From<AberratedStateError> for ApparentError {
    fn from(e: AberratedStateError) -> Self {
        Self::Aberration(e)
    }
}
impl From<OrientationError> for ApparentError {
    fn from(e: OrientationError) -> Self {
        Self::Orientation(e)
    }
}
impl From<ProjectionError> for ApparentError {
    fn from(e: ProjectionError) -> Self {
        Self::Projection(e)
    }
}
impl From<UnitConversionError> for ApparentError {
    fn from(e: UnitConversionError) -> Self {
        Self::Units(e)
    }
}

/// TT Julian date for a TDB Julian date (leading periodic term only).
///
/// `TT = TDB − 0.001657 s · sin(g)`, with `g` the Earth mean anomaly.
pub fn tdb_to_tt_jd(tdb_jd: f64) -> Result<f64, ApparentError> {
    if !tdb_jd.is_finite() {
        return Err(ApparentError::InvalidIntermediate);
    }
    let t = (tdb_jd - J2000_TT_JD) / DAYS_PER_JULIAN_CENTURY;
    let g = (357.5291092 + 35999.0502909 * t).to_radians();
    Ok(tdb_jd - 0.001657 * g.sin() / 86_400.0)
}

/// Rotation ICRF → true ecliptic of date and its derivative per TT day.
pub fn ecliptic_of_date_rotation_with_rate(
    tt_jd: f64,
) -> Result<(Matrix3x3, Matrix3x3), OrientationError> {
    let h = MATRIX_RATE_HALF_STEP_DAYS;
    let m = true_ecliptic_of_date_matrix(tt_jd)?;
    let plus = true_ecliptic_of_date_matrix(tt_jd + h)?;
    let minus = true_ecliptic_of_date_matrix(tt_jd - h)?;
    let mut dm = [[0.0; 3]; 3];
    for (i, row) in dm.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (plus[i][j] - minus[i][j]) / (2.0 * h);
        }
    }
    Ok((m, dm))
}

/// Rotate an already aberrated ICRF state into the true ecliptic of date.
pub fn apparent_ecliptic_of_date_from_aberrated_state(
    state: &AberratedGeocentricState,
) -> Result<ApparentEclipticOfDate, ApparentError> {
    if state.target == Body::Earth
        || !state.observation_epoch.0.is_finite()
        || !state.emission_epoch.0.is_finite()
        || state.emission_epoch.0 > state.observation_epoch.0
        || !state.light_time_seconds.is_finite()
        || state.light_time_seconds <= 0.0
        || !state.light_time_rate.is_finite()
    {
        return Err(ApparentError::InvalidIntermediate);
    }
    let tt_jd = tdb_to_tt_jd(state.observation_epoch.0)?;
    let (m, dm) = ecliptic_of_date_rotation_with_rate(tt_jd)?;
    let position = matrix_vector_multiply(m, state.position_km);
    let rotated_velocity = matrix_vector_multiply(m, state.velocity_km_per_tdb_day);
    let frame_velocity = matrix_vector_multiply(dm, state.position_km);
    // dM/dt is per TT day; TT and TDB days differ by < 1e-9 (relative).
    let velocity = [
        rotated_velocity[0] + frame_velocity[0],
        rotated_velocity[1] + frame_velocity[1],
        rotated_velocity[2] + frame_velocity[2],
    ];
    let cartesian = StateVector {
        target: state.target,
        center: Body::Earth,
        epoch: state.observation_epoch,
        position_km: position,
        velocity_km_per_day: velocity,
    };
    let spherical = project_fixed_axes(&cartesian, 0.0)?;
    Ok(ApparentEclipticOfDate {
        spherical,
        tt_jd,
        emission_epoch: state.emission_epoch,
        light_time_seconds: state.light_time_seconds,
        light_time_rate: state.light_time_rate,
        observer_acceleration_source: state.observer_acceleration_source,
    })
}

/// Convert to degrees / AU / per-TDB-day units; no further astronomy.
pub fn apparent_to_degrees_au(
    result: &ApparentEclipticOfDate,
) -> Result<ApparentSixFieldsDegreesAu, ApparentError> {
    Ok(ApparentSixFieldsDegreesAu {
        fields: fixed_spherical_to_degrees_au(&result.spherical)?,
        tt_jd: result.tt_jd,
        emission_epoch: result.emission_epoch,
        light_time_seconds: result.light_time_seconds,
        light_time_rate: result.light_time_rate,
        observer_acceleration_source: result.observer_acceleration_source,
    })
}

/// DE441 → apparent geocentric ecliptic of date, one body, TDB epoch.
pub fn geocentric_apparent_ecliptic_of_date<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
) -> Result<ApparentEclipticOfDate, ApparentError> {
    let state =
        geocentric_light_time_annual_aberration_analytic(ephemeris, target, reception_epoch)?;
    apparent_ecliptic_of_date_from_aberrated_state(&state)
}

/// DE441 → apparent geocentric six fields (degrees, AU, per TDB day).
pub fn geocentric_apparent_six_fields<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
) -> Result<ApparentSixFieldsDegreesAu, ApparentError> {
    let result = geocentric_apparent_ecliptic_of_date(ephemeris, target, reception_epoch)?;
    apparent_to_degrees_au(&result)
}

/// Several bodies at one TDB epoch; Earth is read once. All-or-error.
pub fn geocentric_apparent_six_fields_many<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    targets: &[Body],
    reception_epoch: JulianDateTdb,
) -> Result<Vec<ApparentSixFieldsDegreesAu>, ApparentError> {
    if !reception_epoch.0.is_finite() {
        return Err(ApparentError::InvalidIntermediate);
    }
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    let earth = ephemeris
        .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, reception_epoch)
        .map_err(AberratedStateError::from)?;
    targets
        .iter()
        .copied()
        .map(|target| {
            let state = geocentric_light_time_annual_aberration_with_earth(
                ephemeris,
                target,
                reception_epoch,
                &earth,
            )?;
            let apparent = apparent_ecliptic_of_date_from_aberrated_state(&state)?;
            apparent_to_degrees_au(&apparent)
        })
        .collect()
}
