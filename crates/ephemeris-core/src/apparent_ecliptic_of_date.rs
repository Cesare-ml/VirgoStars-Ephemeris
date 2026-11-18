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
// Modification (2026-11-17): add an opt-in direct-obliquity experimental frame.
// Modification (2026-11-18): add a separately labeled radial-rate experiment.

//! Geocentric **apparent** ecliptic-of-date positions and rates (MVP 1, row 1).
//!
//! Pipeline, every stage reused from an existing independent module:
//!
//! ```text
//! classic DE441 (TDB, ICRF, km, km/day)
//!   → geocentric one-way light-time          (light_time)
//!   → solar gravitational light deflection  (light_deflection, optional)
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
//! - gravitational light deflection by the planets (Jupiter < 0.0002 arcsecond),
//!   and the Sun's when `ApparentOptions::solar_light_deflection` is false,
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
    aberrate_light_time_state, AberratedGeocentricState, AberratedStateError,
    ObserverAccelerationSource,
};
use crate::coordinates::{project_fixed_axes, ProjectionError, SphericalState};
use crate::de441::{
    Body, De441, De441Error, JulianDateTdb, StateVector, StateVectorWithAcceleration,
};
use crate::earth_orientation::{
    experimental_extrapolated_nutation, iau2006_bias_precession_matrix, matrix_multiply,
    matrix_vector_multiply, true_ecliptic_of_date_matrix, true_equator_of_date_matrix, x_rotation,
    z_rotation, Matrix3x3, OrientationError, DAYS_PER_JULIAN_CENTURY, J2000_TT_JD,
};
use crate::light_deflection::{
    deflect_light_time_state_by_sun, deflect_light_time_state_by_sun_with, DeflectionError,
};
use crate::light_time::{
    geocentric_light_time, heliocentric_light_time, light_time_with_states,
    refine_emission_rounding,
};
use crate::long_term_precession::{experimental_direct_obliquity, mean_date_matrices};
use crate::six_field_units::{
    fixed_spherical_to_degrees_au, FixedSixFieldsDegreesAu, UnitConversionError, IAU_AU_KM,
};
use crate::spk_type21::SmallBodySpk;

/// Half-step, in TT days, of the central difference used for `dM/dt`.
pub const MATRIX_RATE_HALF_STEP_DAYS: f64 = 0.01;

/// Reference frame of the output coordinates (both are of date, true).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApparentFrame {
    /// True ecliptic and equinox of date: longitude, latitude.
    TrueEclipticOfDate,
    /// True equator and equinox of date: right ascension, declination.
    TrueEquatorOfDate,
}

/// Where the observer is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApparentCenter {
    /// Earth's center: light-time, aberration, optional solar deflection.
    Geocentric,
    /// The Sun's center: light-time only (a photon arriving at the Sun has no
    /// annual aberration and is not deflected by the Sun it is falling into).
    Heliocentric,
}

/// Which optional corrections to apply and which frame/center to use. The default
/// is the physically complete geocentric ecliptic set (solar deflection on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApparentOptions {
    /// Apply first-order solar light deflection before aberration (geocentric only).
    pub solar_light_deflection: bool,
    pub frame: ApparentFrame,
    pub center: ApparentCenter,
}

impl Default for ApparentOptions {
    fn default() -> Self {
        Self {
            solar_light_deflection: true,
            frame: ApparentFrame::TrueEclipticOfDate,
            center: ApparentCenter::Geocentric,
        }
    }
}

/// Corrections applied for `options`, in order.
pub fn applied_corrections(options: ApparentOptions) -> Vec<&'static str> {
    let mut list = vec![match options.center {
        ApparentCenter::Geocentric => "one-way geocentric light-time",
        ApparentCenter::Heliocentric => "one-way heliocentric light-time",
    }];
    if options.center == ApparentCenter::Geocentric {
        if options.solar_light_deflection {
            list.push("solar gravitational light deflection (first order, Sun only)");
        }
        list.push("annual aberration (special-relativistic)");
    }
    list.push("IAU 2006 frame bias and precession");
    list.push("IAU 2000B nutation");
    list.push(match options.frame {
        ApparentFrame::TrueEclipticOfDate => "true obliquity (rotation to the ecliptic of date)",
        ApparentFrame::TrueEquatorOfDate => "true equator of date (no ecliptic rotation)",
    });
    list
}

/// Corrections that are **not** applied for `options`.
pub fn not_applied_corrections(options: ApparentOptions) -> Vec<&'static str> {
    let mut list = Vec::new();
    if options.center == ApparentCenter::Geocentric {
        if !options.solar_light_deflection {
            list.push("solar gravitational light deflection");
        }
        list.push("planetary gravitational light deflection");
        list.push("topocentric parallax and diurnal aberration");
    } else {
        list.push(
            "annual aberration and solar deflection (not defined for a Sun-centered observer)",
        );
    }
    list.push("planetary-system barycenter to body-center offset");
    list
}

/// Geocentric apparent spherical state in the true ecliptic and equinox of
/// date. `spherical.tilt_rad` is always zero (the rotation is already
/// included in the Cartesian vector); longitude is in `[0, 2π)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApparentEclipticOfDate {
    pub spherical: SphericalState,
    pub frame: ApparentFrame,
    pub tt_jd: f64,
    pub solar_deflection_applied: bool,
    pub emission_epoch: JulianDateTdb,
    pub light_time_seconds: f64,
    pub light_time_rate: f64,
    pub observer_acceleration_source: ObserverAccelerationSource,
}

/// The same six fields in degrees, AU and per **TDB** day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApparentSixFieldsDegreesAu {
    pub fields: FixedSixFieldsDegreesAu,
    pub frame: ApparentFrame,
    pub tt_jd: f64,
    pub solar_deflection_applied: bool,
    pub emission_epoch: JulianDateTdb,
    pub light_time_seconds: f64,
    pub light_time_rate: f64,
    pub observer_acceleration_source: ObserverAccelerationSource,
}

#[derive(Debug)]
pub enum ApparentError {
    InvalidIntermediate,
    Aberration(AberratedStateError),
    Deflection(DeflectionError),
    Orientation(OrientationError),
    Projection(ProjectionError),
    Units(UnitConversionError),
}

impl fmt::Display for ApparentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIntermediate => write!(f, "invalid aberrated intermediate state"),
            Self::Aberration(e) => write!(f, "light-time/aberration: {e}"),
            Self::Deflection(e) => write!(f, "solar deflection: {e}"),
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
            Self::Deflection(e) => Some(e),
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
impl From<DeflectionError> for ApparentError {
    fn from(e: DeflectionError) -> Self {
        Self::Deflection(e)
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

/// Rotation ICRF → true equator and equinox of date and its derivative per TT day.
pub fn equator_of_date_rotation_with_rate(
    tt_jd: f64,
) -> Result<(Matrix3x3, Matrix3x3), OrientationError> {
    let h = MATRIX_RATE_HALF_STEP_DAYS;
    let m = true_equator_of_date_matrix(tt_jd)?;
    let plus = true_equator_of_date_matrix(tt_jd + h)?;
    let minus = true_equator_of_date_matrix(tt_jd - h)?;
    let mut dm = [[0.0; 3]; 3];
    for (i, row) in dm.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (plus[i][j] - minus[i][j]) / (2.0 * h);
        }
    }
    Ok((m, dm))
}

/// Everything a finished ICRF state needs to become a spherical result.
struct IcrfState {
    target: Body,
    center: Body,
    observation_epoch: JulianDateTdb,
    emission_epoch: JulianDateTdb,
    light_time_seconds: f64,
    light_time_rate: f64,
    source: ObserverAccelerationSource,
    position_km: [f64; 3],
    velocity_km_per_tdb_day: [f64; 3],
}

fn finish(
    state: &IcrfState,
    frame: ApparentFrame,
    solar_deflection_applied: bool,
) -> Result<ApparentEclipticOfDate, ApparentError> {
    if state.target == state.center
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
    let (m, dm) = match frame {
        ApparentFrame::TrueEclipticOfDate => ecliptic_of_date_rotation_with_rate(tt_jd)?,
        ApparentFrame::TrueEquatorOfDate => equator_of_date_rotation_with_rate(tt_jd)?,
    };
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
        center: state.center,
        epoch: state.observation_epoch,
        position_km: position,
        velocity_km_per_day: velocity,
    };
    let spherical = project_fixed_axes(&cartesian, 0.0)?;
    Ok(ApparentEclipticOfDate {
        spherical,
        frame,
        tt_jd,
        solar_deflection_applied,
        emission_epoch: state.emission_epoch,
        light_time_seconds: state.light_time_seconds,
        light_time_rate: state.light_time_rate,
        observer_acceleration_source: state.source,
    })
}

/// Rotate an already aberrated ICRF state into the true ecliptic of date.
pub fn apparent_ecliptic_of_date_from_aberrated_state(
    state: &AberratedGeocentricState,
    solar_deflection_applied: bool,
) -> Result<ApparentEclipticOfDate, ApparentError> {
    apparent_from_aberrated_state_in_frame(
        state,
        ApparentFrame::TrueEclipticOfDate,
        solar_deflection_applied,
    )
}

/// As above, in either true frame of date.
pub fn apparent_from_aberrated_state_in_frame(
    state: &AberratedGeocentricState,
    frame: ApparentFrame,
    solar_deflection_applied: bool,
) -> Result<ApparentEclipticOfDate, ApparentError> {
    finish(
        &IcrfState {
            target: state.target,
            center: Body::Earth,
            observation_epoch: state.observation_epoch,
            emission_epoch: state.emission_epoch,
            light_time_seconds: state.light_time_seconds,
            light_time_rate: state.light_time_rate,
            source: state.observer_acceleration_source,
            position_km: state.position_km,
            velocity_km_per_tdb_day: state.velocity_km_per_tdb_day,
        },
        frame,
        solar_deflection_applied,
    )
}

/// Convert to degrees / AU / per-TDB-day units; no further astronomy.
pub fn apparent_to_degrees_au(
    result: &ApparentEclipticOfDate,
) -> Result<ApparentSixFieldsDegreesAu, ApparentError> {
    Ok(ApparentSixFieldsDegreesAu {
        fields: fixed_spherical_to_degrees_au(&result.spherical)?,
        frame: result.frame,
        tt_jd: result.tt_jd,
        solar_deflection_applied: result.solar_deflection_applied,
        emission_epoch: result.emission_epoch,
        light_time_seconds: result.light_time_seconds,
        light_time_rate: result.light_time_rate,
        observer_acceleration_source: result.observer_acceleration_source,
    })
}

/// Light-time → optional solar deflection → aberration, from Earth's state
/// at the reception epoch (`earth` must be Earth relative to the barycenter
/// at exactly `reception_epoch`).
fn aberrated_state<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
    earth: &StateVectorWithAcceleration,
    options: ApparentOptions,
) -> Result<AberratedGeocentricState, ApparentError> {
    if earth.target != Body::Earth
        || earth.center != Body::SolarSystemBarycenter
        || earth.epoch != reception_epoch
    {
        return Err(ApparentError::InvalidIntermediate);
    }
    let mut light_time = geocentric_light_time(ephemeris, target, reception_epoch)
        .map_err(AberratedStateError::from)?;
    if options.solar_light_deflection {
        light_time = deflect_light_time_state_by_sun(ephemeris, &light_time)?;
    }
    let mut state = aberrate_light_time_state(
        &light_time,
        earth.velocity_km_per_day,
        earth.acceleration_km_per_day2,
    )?;
    state.observer_acceleration_source = ObserverAccelerationSource::AnalyticDe441Chebyshev;
    Ok(state)
}

/// Isolated experiment: remove the leading light-time emission rounding error.
fn experimental_aberrated_state<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
    earth: &StateVectorWithAcceleration,
) -> Result<AberratedGeocentricState, ApparentError> {
    let mut lt = geocentric_light_time(ephemeris, target, reception_epoch)
        .map_err(AberratedStateError::from)?;
    let emission = ephemeris
        .state_with_acceleration(target, Body::SolarSystemBarycenter, lt.emission_epoch)
        .map_err(AberratedStateError::from)?;
    lt = refine_emission_rounding(lt, emission, *earth, ephemeris.coverage())
        .map_err(AberratedStateError::from)?;
    lt = deflect_light_time_state_by_sun(ephemeris, &lt)?;
    let mut state = aberrate_light_time_state(
        &lt,
        earth.velocity_km_per_day,
        earth.acceleration_km_per_day2,
    )?;
    state.observer_acceleration_source = ObserverAccelerationSource::AnalyticDe441Chebyshev;
    Ok(state)
}

/// Diagnostic geocentric apparent vector in fixed ICRF axes, at TDB reception.
/// Includes light-time, solar deflection and annual aberration, but no Earth
/// orientation or atmospheric refraction. Distance is the light-time range.
pub fn experimental_apparent_icrf<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
) -> Result<AberratedGeocentricState, ApparentError> {
    let earth = ephemeris
        .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, reception_epoch)
        .map_err(AberratedStateError::from)?;
    experimental_aberrated_state(ephemeris, target, reception_epoch, &earth)
}

/// Experimental geocentric direction with light-time, solar deflection and
/// annual aberration, in the **mean** long-term ecliptic of date. Nutation is
/// deliberately absent. Input is TDB, not UT. This is not a true apparent
/// ecliptic longitude and is not used by the normal apparent pipeline.
pub fn experimental_long_term_mean_ecliptic<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
) -> Result<FixedSixFieldsDegreesAu, ApparentError> {
    experimental_long_term_ecliptic(ephemeris, target, reception_epoch, false, false)
}

/// Experimental long-range geocentric apparent ecliptic direction. Uses
/// extrapolated 77-term nutation with full IERS fundamental arguments;
/// accuracy outside the modern range is not established. TT/TDB input only,
/// no UT/Delta-T conversion, and never selected by the standard API.
pub fn experimental_long_term_apparent_ecliptic<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
) -> Result<FixedSixFieldsDegreesAu, ApparentError> {
    experimental_long_term_ecliptic(ephemeris, target, reception_epoch, true, false)
}

/// Explicit alternative with the separately fitted Vondrák ε_A series. The
/// equator/equinox still come from the pole fit, so this is a mixed convention,
/// not the pole-derived ecliptic and not a certified accuracy improvement.
pub fn experimental_long_term_apparent_direct_obliquity<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
) -> Result<FixedSixFieldsDegreesAu, ApparentError> {
    experimental_long_term_ecliptic(ephemeris, target, reception_epoch, true, true)
}

/// A diagnostic candidate, separate from the derivative in `base`.
/// The candidate is not asserted to be a position derivative or Swiss parity.
#[derive(Clone, Copy, Debug)]
pub struct ExperimentalRadialRateCandidate {
    pub base: FixedSixFieldsDegreesAu,
    pub correction_au_per_tdb_day: f64,
    pub candidate_au_per_tdb_day: f64,
    pub correction_applied: bool,
}

/// Independently formulated first-order diagnostic: (r · a_Earth) / (c AU),
/// with the apparent ICRF position in km and barycentric Earth acceleration in
/// km/TDB day². It modifies no normal calculation and makes no claim about the
/// implementation of the comparison software. Angular rates are unchanged.
/// The Sun is explicitly excluded: sampled comparisons worsened systematically.
pub fn experimental_radial_rate_candidate<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
) -> Result<ExperimentalRadialRateCandidate, ApparentError> {
    let base =
        experimental_long_term_apparent_direct_obliquity(ephemeris, target, reception_epoch)?;
    if target == Body::Sun {
        return Ok(ExperimentalRadialRateCandidate {
            base,
            correction_au_per_tdb_day: 0.0,
            candidate_au_per_tdb_day: base.distance_speed_au_per_tdb_day,
            correction_applied: false,
        });
    }
    let position = experimental_apparent_icrf(ephemeris, target, reception_epoch)?.position_km;
    let acceleration = ephemeris
        .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, reception_epoch)
        .map_err(AberratedStateError::from)?
        .acceleration_km_per_day2;
    let correction = radial_rate_candidate_correction(position, acceleration)?;
    let candidate = base.distance_speed_au_per_tdb_day + correction;
    if !candidate.is_finite() {
        return Err(ApparentError::InvalidIntermediate);
    }
    Ok(ExperimentalRadialRateCandidate {
        base,
        correction_au_per_tdb_day: correction,
        candidate_au_per_tdb_day: candidate,
        correction_applied: true,
    })
}

fn radial_rate_candidate_correction(
    position: [f64; 3],
    acceleration: [f64; 3],
) -> Result<f64, ApparentError> {
    if position
        .iter()
        .chain(acceleration.iter())
        .any(|v| !v.is_finite())
        || position.iter().all(|v| *v == 0.0)
    {
        return Err(ApparentError::InvalidIntermediate);
    }
    let correction = position
        .iter()
        .zip(acceleration)
        .map(|(r, a)| r * a)
        .sum::<f64>()
        / (crate::aberration::SPEED_OF_LIGHT_KM_PER_DAY * IAU_AU_KM);
    if !correction.is_finite() {
        return Err(ApparentError::InvalidIntermediate);
    }
    Ok(correction)
}

fn experimental_long_term_ecliptic<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
    nutation: bool,
    direct_obliquity: bool,
) -> Result<FixedSixFieldsDegreesAu, ApparentError> {
    let earth = ephemeris
        .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, reception_epoch)
        .map_err(AberratedStateError::from)?;
    let state = experimental_aberrated_state(ephemeris, target, reception_epoch, &earth)?;
    let tt = tdb_to_tt_jd(reception_epoch.0)?;
    // ICRF to J2000 mean axes via the existing IAU-2006 frame bias.
    let bias = iau2006_bias_precession_matrix(J2000_TT_JD)?;
    let matrix = |date| -> Result<Matrix3x3, OrientationError> {
        let (equator, ecliptic) = mean_date_matrices(date)?;
        let axes = if direct_obliquity {
            matrix_multiply(x_rotation(experimental_direct_obliquity(date)?), equator)
        } else {
            ecliptic
        };
        let mean = matrix_multiply(axes, bias);
        if nutation {
            let dpsi = experimental_extrapolated_nutation(date)?.d_psi_rad;
            Ok(matrix_multiply(z_rotation(-dpsi), mean))
        } else {
            Ok(mean)
        }
    };
    let h = MATRIX_RATE_HALF_STEP_DAYS;
    let m = matrix(tt)?;
    let plus = matrix(tt + h)?;
    let minus = matrix(tt - h)?;
    let dm: Matrix3x3 =
        std::array::from_fn(|i| std::array::from_fn(|j| (plus[i][j] - minus[i][j]) / (2.0 * h)));
    let p = matrix_vector_multiply(m, state.position_km);
    let v = matrix_vector_multiply(m, state.velocity_km_per_tdb_day);
    let drift = matrix_vector_multiply(dm, state.position_km);
    let cartesian = StateVector {
        target,
        center: Body::Earth,
        epoch: reception_epoch,
        position_km: p,
        velocity_km_per_day: std::array::from_fn(|i| v[i] + drift[i]),
    };
    Ok(fixed_spherical_to_degrees_au(&project_fixed_axes(
        &cartesian, 0.0,
    )?)?)
}

/// DE441 → apparent geocentric ecliptic of date, one body, TDB epoch,
/// default options (solar deflection on).
pub fn geocentric_apparent_ecliptic_of_date<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
) -> Result<ApparentEclipticOfDate, ApparentError> {
    geocentric_apparent_ecliptic_of_date_with_options(
        ephemeris,
        target,
        reception_epoch,
        ApparentOptions::default(),
    )
}

/// As [`geocentric_apparent_ecliptic_of_date`] with explicit options.
pub fn geocentric_apparent_ecliptic_of_date_with_options<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
    options: ApparentOptions,
) -> Result<ApparentEclipticOfDate, ApparentError> {
    if options.center == ApparentCenter::Heliocentric {
        return heliocentric_apparent(ephemeris, target, reception_epoch, options);
    }
    let earth = ephemeris
        .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, reception_epoch)
        .map_err(AberratedStateError::from)?;
    let state = aberrated_state(ephemeris, target, reception_epoch, &earth, options)?;
    apparent_from_aberrated_state_in_frame(&state, options.frame, options.solar_light_deflection)
}

/// Sun-centered position: light-time only, then the same frame rotation and rates.
fn heliocentric_apparent<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
    options: ApparentOptions,
) -> Result<ApparentEclipticOfDate, ApparentError> {
    let light_time = heliocentric_light_time(ephemeris, target, reception_epoch)
        .map_err(AberratedStateError::from)?;
    finish(
        &IcrfState {
            target,
            center: Body::Sun,
            observation_epoch: light_time.observation_epoch,
            emission_epoch: light_time.emission_epoch,
            light_time_seconds: light_time.light_time_seconds,
            light_time_rate: light_time.light_time_rate,
            source: ObserverAccelerationSource::NotApplicable,
            position_km: light_time.position_km,
            velocity_km_per_tdb_day: light_time.velocity_km_per_day,
        },
        options.frame,
        false,
    )
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

/// Several bodies at one TDB epoch with default options; Earth is read once.
/// All-or-error.
pub fn geocentric_apparent_six_fields_many<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    targets: &[Body],
    reception_epoch: JulianDateTdb,
) -> Result<Vec<ApparentSixFieldsDegreesAu>, ApparentError> {
    geocentric_apparent_six_fields_many_with_options(
        ephemeris,
        targets,
        reception_epoch,
        ApparentOptions::default(),
    )
}

/// As [`geocentric_apparent_six_fields_many`] with explicit options.
pub fn geocentric_apparent_six_fields_many_with_options<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    targets: &[Body],
    reception_epoch: JulianDateTdb,
    options: ApparentOptions,
) -> Result<Vec<ApparentSixFieldsDegreesAu>, ApparentError> {
    if !reception_epoch.0.is_finite() {
        return Err(ApparentError::InvalidIntermediate);
    }
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    if options.center == ApparentCenter::Heliocentric {
        return targets
            .iter()
            .copied()
            .map(|target| {
                let apparent = heliocentric_apparent(ephemeris, target, reception_epoch, options)?;
                apparent_to_degrees_au(&apparent)
            })
            .collect();
    }
    let earth = ephemeris
        .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, reception_epoch)
        .map_err(AberratedStateError::from)?;
    targets
        .iter()
        .copied()
        .map(|target| {
            let state = aberrated_state(ephemeris, target, reception_epoch, &earth, options)?;
            let apparent = apparent_from_aberrated_state_in_frame(
                &state,
                options.frame,
                options.solar_light_deflection,
            )?;
            apparent_to_degrees_au(&apparent)
        })
        .collect()
}

/// Apparent position of a **small body** (asteroid or comet) whose ephemeris is a
/// type 21 SPK relative to the Sun, observed from the Earth's or the Sun's center
/// exactly as the DE441 bodies are: the same light-time solver, solar deflection
/// (geocentric), annual aberration (geocentric), IAU 2006/2000B frame and rates.
///
/// The barycentric state of the body is the DE441 Sun plus the SPK state, so the
/// accuracy of the result is that of the supplied SPK plus DE441. Only SPKs centered
/// on the Sun (NAIF 10) are accepted; the epoch must lie inside the SPK coverage.
pub fn small_body_apparent_six_fields<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    small_body: &SmallBodySpk,
    reception_epoch: JulianDateTdb,
    options: ApparentOptions,
) -> Result<ApparentSixFieldsDegreesAu, ApparentError> {
    if !reception_epoch.0.is_finite() || small_body.center_id() != 10 {
        return Err(ApparentError::InvalidIntermediate);
    }
    let target = Body::SmallBody(small_body.target_id());
    let earth = if options.center == ApparentCenter::Geocentric {
        Some(
            ephemeris
                .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, reception_epoch)
                .map_err(AberratedStateError::from)?,
        )
    } else {
        None
    };
    let mut states = |body: Body, epoch: JulianDateTdb| -> Result<StateVector, De441Error> {
        match body {
            Body::SmallBody(id) if id == small_body.target_id() => {
                let (position, velocity) = small_body.state_at(epoch).map_err(|_| {
                    let (start, end) = small_body.coverage_jd_tdb();
                    De441Error::OutsideCoverage {
                        epoch,
                        start: JulianDateTdb(start),
                        end: JulianDateTdb(end),
                    }
                })?;
                let sun = ephemeris.state(Body::Sun, Body::SolarSystemBarycenter, epoch)?;
                Ok(StateVector {
                    target: body,
                    center: Body::SolarSystemBarycenter,
                    epoch,
                    position_km: std::array::from_fn(|i| sun.position_km[i] + position[i]),
                    velocity_km_per_day: std::array::from_fn(|i| {
                        sun.velocity_km_per_day[i] + velocity[i]
                    }),
                })
            }
            other => ephemeris.state(other, Body::SolarSystemBarycenter, epoch),
        }
    };
    let result = match earth {
        None => {
            let light_time =
                light_time_with_states(Body::Sun, target, reception_epoch, &mut states)
                    .map_err(AberratedStateError::from)?;
            finish(
                &IcrfState {
                    target,
                    center: Body::Sun,
                    observation_epoch: light_time.observation_epoch,
                    emission_epoch: light_time.emission_epoch,
                    light_time_seconds: light_time.light_time_seconds,
                    light_time_rate: light_time.light_time_rate,
                    source: ObserverAccelerationSource::NotApplicable,
                    position_km: light_time.position_km,
                    velocity_km_per_tdb_day: light_time.velocity_km_per_day,
                },
                options.frame,
                false,
            )?
        }
        Some(earth) => {
            let mut light_time =
                light_time_with_states(Body::Earth, target, reception_epoch, &mut states)
                    .map_err(AberratedStateError::from)?;
            if options.solar_light_deflection {
                light_time = deflect_light_time_state_by_sun_with(&mut states, &light_time)?;
            }
            let mut state = aberrate_light_time_state(
                &light_time,
                earth.velocity_km_per_day,
                earth.acceleration_km_per_day2,
            )?;
            state.observer_acceleration_source = ObserverAccelerationSource::AnalyticDe441Chebyshev;
            apparent_from_aberrated_state_in_frame(
                &state,
                options.frame,
                options.solar_light_deflection,
            )?
        }
    };
    apparent_to_degrees_au(&result)
}

#[cfg(test)]
mod radial_candidate_tests {
    use super::*;

    #[test]
    fn projection_respects_sign_and_rejects_invalid_geometry() {
        let c = crate::aberration::SPEED_OF_LIGHT_KM_PER_DAY;
        let value = radial_rate_candidate_correction([IAU_AU_KM, 0.0, 0.0], [c, 5.0, 0.0]).unwrap();
        assert!((value - 1.0).abs() < 1e-15);
        let opposite =
            radial_rate_candidate_correction([-IAU_AU_KM, 0.0, 0.0], [c, 5.0, 0.0]).unwrap();
        assert!((opposite + 1.0).abs() < 1e-15);
        assert!(radial_rate_candidate_correction([0.0; 3], [0.0; 3]).is_err());
        assert!(radial_rate_candidate_correction([f64::NAN, 0.0, 0.0], [0.0; 3]).is_err());
        assert!(
            radial_rate_candidate_correction([f64::MAX, 0.0, 0.0], [f64::MAX, 0.0, 0.0]).is_err()
        );
    }
}
