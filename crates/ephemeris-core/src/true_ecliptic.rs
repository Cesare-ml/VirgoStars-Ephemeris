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

//! Experimental light-time + aberration in the moving true ecliptic of date.
//!
//! IAU 2006/2000B frame rates use a centered numerical matrix derivative.
//! Solar deflection, exact TDB/TT rate conversion, complete nutation and
//! planet-center corrections are not included. This is not Swiss parity.
//! Added 2026-11-14 by composing existing core stages.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::aberration_state::{
    geocentric_light_time_annual_aberration_analytic, AberratedGeocentricState, AberratedStateError,
};
use crate::coordinates::{project_fixed_axes, ProjectionError};
use crate::de441::{Body, De441, JulianDateTdb, StateVector};
use crate::earth_orientation::{
    matrix_vector_multiply, true_ecliptic_of_date_matrix, true_equator_of_date_matrix, Matrix3x3,
    OrientationError,
};
use crate::first_sky::{CivilTimestamp, SkyError};
use crate::light_time::{heliocentric_light_time, LightTimeError};
use crate::six_field_units::IAU_AU_KM;

const MATRIX_STEP_DAYS: f64 = 0.01;

/// Geocentric true-ecliptic-of-date quantities, in degrees, AU and TDB days.
/// Outer-planet `Body` variants may denote system barycenters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrueEclipticState {
    pub target: Body,
    pub reception_tdb: JulianDateTdb,
    pub reception_tt_jd: f64,
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub distance_au: f64,
    pub longitude_speed_deg_per_tdb_day: f64,
    pub latitude_speed_deg_per_tdb_day: f64,
    pub distance_speed_au_per_tdb_day: f64,
}

/// Geocentric true-equator-of-date right ascension/declination in degrees,
/// distance in AU and derivatives per reception TDB day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrueEquatorialState {
    pub target: Body,
    pub reception_tdb: JulianDateTdb,
    pub reception_tt_jd: f64,
    pub right_ascension_deg: f64,
    pub declination_deg: f64,
    pub distance_au: f64,
    pub right_ascension_speed_deg_per_tdb_day: f64,
    pub declination_speed_deg_per_tdb_day: f64,
    pub distance_speed_au_per_tdb_day: f64,
}

/// Heliocentric true-ecliptic-of-date coordinates and reception-TDB-day rates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeliocentricEclipticState {
    pub target: Body,
    pub reception_tdb: JulianDateTdb,
    pub reception_tt_jd: f64,
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub distance_au: f64,
    pub longitude_speed_deg_per_tdb_day: f64,
    pub latitude_speed_deg_per_tdb_day: f64,
    pub distance_speed_au_per_tdb_day: f64,
}

#[derive(Debug)]
pub enum TrueEclipticError {
    IncoherentEpoch,
    InvalidState,
    Orientation(OrientationError),
    Projection(ProjectionError),
    Aberration(AberratedStateError),
    CivilTime(SkyError),
    LightTime(LightTimeError),
}

impl fmt::Display for TrueEclipticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncoherentEpoch => {
                write!(f, "TT and TDB must refer to the same reception instant")
            }
            Self::InvalidState => write!(f, "invalid position or velocity state"),
            Self::Orientation(e) => write!(f, "orientation: {e}"),
            Self::Projection(e) => write!(f, "projection: {e}"),
            Self::Aberration(e) => write!(f, "aberration: {e}"),
            Self::CivilTime(e) => write!(f, "civil time: {e}"),
            Self::LightTime(e) => write!(f, "light time: {e}"),
        }
    }
}
impl Error for TrueEclipticError {}

/// Rotate an aberrated ICRF position and reception-time velocity into the
/// moving true ecliptic. TT and TDB must represent the same instant; the
/// one-second check is only a guard, not a time-scale conversion. Matrix
/// derivatives assume dTT/dTDB = 1 and use a symmetric 0.01-day step.
pub fn project_true_ecliptic(
    state: &AberratedGeocentricState,
    tt_jd: f64,
) -> Result<TrueEclipticState, TrueEclipticError> {
    project_date_frame(state, tt_jd, true_ecliptic_of_date_matrix)
}

fn project_date_frame(
    state: &AberratedGeocentricState,
    tt_jd: f64,
    matrix_at: fn(f64) -> Result<Matrix3x3, OrientationError>,
) -> Result<TrueEclipticState, TrueEclipticError> {
    if !tt_jd.is_finite()
        || !state.observation_epoch.0.is_finite()
        || (tt_jd - state.observation_epoch.0).abs() > 1.0 / 86_400.0
    {
        return Err(TrueEclipticError::IncoherentEpoch);
    }
    if state.target == Body::Earth
        || !state.emission_epoch.0.is_finite()
        || state.emission_epoch.0 > state.observation_epoch.0
        || !state.light_time_seconds.is_finite()
        || state.light_time_seconds <= 0.0
        || !state.light_time_rate.is_finite()
    {
        return Err(TrueEclipticError::InvalidState);
    }
    let vector = StateVector {
        target: state.target,
        center: Body::Earth,
        epoch: state.observation_epoch,
        position_km: state.position_km,
        velocity_km_per_day: state.velocity_km_per_tdb_day,
    };
    project_vector_date_frame(&vector, tt_jd, matrix_at)
}

fn project_vector_date_frame(
    state: &StateVector,
    tt_jd: f64,
    matrix_at: fn(f64) -> Result<Matrix3x3, OrientationError>,
) -> Result<TrueEclipticState, TrueEclipticError> {
    if !tt_jd.is_finite()
        || !state.epoch.0.is_finite()
        || (tt_jd - state.epoch.0).abs() > 1.0 / 86_400.0
    {
        return Err(TrueEclipticError::IncoherentEpoch);
    }
    if state.target == state.center {
        return Err(TrueEclipticError::InvalidState);
    }
    let matrix = matrix_at(tt_jd).map_err(TrueEclipticError::Orientation)?;
    let before_jd = tt_jd - MATRIX_STEP_DAYS;
    let after_jd = tt_jd + MATRIX_STEP_DAYS;
    let before = matrix_at(before_jd).map_err(TrueEclipticError::Orientation)?;
    let after = matrix_at(after_jd).map_err(TrueEclipticError::Orientation)?;
    let position = matrix_vector_multiply(matrix, state.position_km);
    let velocity_rotated = matrix_vector_multiply(matrix, state.velocity_km_per_day);
    let before_position = matrix_vector_multiply(before, state.position_km);
    let after_position = matrix_vector_multiply(after, state.position_km);
    let velocity = std::array::from_fn(|i| {
        velocity_rotated[i] + (after_position[i] - before_position[i]) / (after_jd - before_jd)
    });
    let rotated = StateVector {
        target: state.target,
        center: state.center,
        epoch: state.epoch,
        position_km: position,
        velocity_km_per_day: velocity,
    };
    let spherical = project_fixed_axes(&rotated, 0.0).map_err(TrueEclipticError::Projection)?;
    let longitude_deg = spherical.longitude_rad.to_degrees();
    let longitude_deg = if longitude_deg >= 360.0 {
        f64::from_bits(360.0_f64.to_bits() - 1)
    } else {
        longitude_deg
    };
    let result = TrueEclipticState {
        target: state.target,
        reception_tdb: state.epoch,
        reception_tt_jd: tt_jd,
        longitude_deg,
        latitude_deg: spherical.latitude_rad.to_degrees(),
        distance_au: spherical.distance_km / IAU_AU_KM,
        longitude_speed_deg_per_tdb_day: spherical.longitude_rate_rad_per_day.to_degrees(),
        latitude_speed_deg_per_tdb_day: spherical.latitude_rate_rad_per_day.to_degrees(),
        distance_speed_au_per_tdb_day: spherical.distance_rate_km_per_day / IAU_AU_KM,
    };
    if !result.distance_au.is_finite()
        || result.distance_au <= 0.0
        || !result.longitude_speed_deg_per_tdb_day.is_finite()
        || !result.latitude_speed_deg_per_tdb_day.is_finite()
        || !result.distance_speed_au_per_tdb_day.is_finite()
    {
        return Err(TrueEclipticError::InvalidState);
    }
    Ok(result)
}

/// Project the same aberrated ICRF state to the moving true equator. The
/// shared centered matrix derivative includes the date-frame motion.
pub fn project_true_equatorial(
    state: &AberratedGeocentricState,
    tt_jd: f64,
) -> Result<TrueEquatorialState, TrueEclipticError> {
    let r = project_date_frame(state, tt_jd, true_equator_of_date_matrix)?;
    Ok(TrueEquatorialState {
        target: r.target,
        reception_tdb: r.reception_tdb,
        reception_tt_jd: r.reception_tt_jd,
        right_ascension_deg: r.longitude_deg,
        declination_deg: r.latitude_deg,
        distance_au: r.distance_au,
        right_ascension_speed_deg_per_tdb_day: r.longitude_speed_deg_per_tdb_day,
        declination_speed_deg_per_tdb_day: r.latitude_speed_deg_per_tdb_day,
        distance_speed_au_per_tdb_day: r.distance_speed_au_per_tdb_day,
    })
}

/// Calculate geocentric true-equator-of-date coordinates from DE441 and
/// explicit coherent TDB and TT reception epochs. Same correction and model
/// limitations apply as for [`geocentric_true_ecliptic`].
pub fn geocentric_true_equatorial<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    tdb: JulianDateTdb,
    tt_jd: f64,
) -> Result<TrueEquatorialState, TrueEclipticError> {
    let state = geocentric_light_time_annual_aberration_analytic(ephemeris, target, tdb)
        .map_err(TrueEclipticError::Aberration)?;
    project_true_equatorial(&state, tt_jd)
}

/// Civil-time entry point with the same bounded, approximate time model as
/// [`geocentric_true_ecliptic_from_civil`].
pub fn geocentric_true_equatorial_from_civil<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    civil: CivilTimestamp,
) -> Result<TrueEquatorialState, TrueEclipticError> {
    let (tt, tdb) = civil
        .to_tt_and_approximate_tdb()
        .map_err(TrueEclipticError::CivilTime)?;
    geocentric_true_equatorial(ephemeris, target, tdb, tt)
}

/// Compute a heliocentric, light-time-corrected ecliptic-of-date state.
/// No annual aberration or solar gravitational deflection is applied.
/// The Sun itself has no direction and returns an explicit error. Epochs
/// must be coherent TT and TDB. This is not established Swiss parity.
pub fn heliocentric_true_ecliptic<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    tdb: JulianDateTdb,
    tt_jd: f64,
) -> Result<HeliocentricEclipticState, TrueEclipticError> {
    let raw =
        heliocentric_light_time(ephemeris, target, tdb).map_err(TrueEclipticError::LightTime)?;
    let vector = StateVector {
        target,
        center: Body::Sun,
        epoch: tdb,
        position_km: raw.position_km,
        velocity_km_per_day: raw.velocity_km_per_day,
    };
    let r = project_vector_date_frame(&vector, tt_jd, true_ecliptic_of_date_matrix)?;
    Ok(HeliocentricEclipticState {
        target: r.target,
        reception_tdb: r.reception_tdb,
        reception_tt_jd: r.reception_tt_jd,
        longitude_deg: r.longitude_deg,
        latitude_deg: r.latitude_deg,
        distance_au: r.distance_au,
        longitude_speed_deg_per_tdb_day: r.longitude_speed_deg_per_tdb_day,
        latitude_speed_deg_per_tdb_day: r.latitude_speed_deg_per_tdb_day,
        distance_speed_au_per_tdb_day: r.distance_speed_au_per_tdb_day,
    })
}

/// Civil-time entry point, with the same bounded, approximate TT/TDB model
/// as the geocentric APIs.
pub fn heliocentric_true_ecliptic_from_civil<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    civil: CivilTimestamp,
) -> Result<HeliocentricEclipticState, TrueEclipticError> {
    let (tt, tdb) = civil
        .to_tt_and_approximate_tdb()
        .map_err(TrueEclipticError::CivilTime)?;
    heliocentric_true_ecliptic(ephemeris, target, tdb, tt)
}

/// Compute an experimental true-ecliptic state from a caller-provided DE441
/// file and explicit, coherent TDB and TT reception epochs.
pub fn geocentric_true_ecliptic<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    tdb: JulianDateTdb,
    tt_jd: f64,
) -> Result<TrueEclipticState, TrueEclipticError> {
    let state = geocentric_light_time_annual_aberration_analytic(ephemeris, target, tdb)
        .map_err(TrueEclipticError::Aberration)?;
    project_true_ecliptic(&state, tt_jd)
}

/// Civil-time entry point with explicit UTC offset and the bounded,
/// approximate time model in [`CivilTimestamp::to_tt_and_approximate_tdb`].
pub fn geocentric_true_ecliptic_from_civil<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    civil: CivilTimestamp,
) -> Result<TrueEclipticState, TrueEclipticError> {
    let (tt, tdb) = civil
        .to_tt_and_approximate_tdb()
        .map_err(TrueEclipticError::CivilTime)?;
    geocentric_true_ecliptic(ephemeris, target, tdb, tt)
}
