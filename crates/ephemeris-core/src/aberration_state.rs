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

//! Time derivatives of the **existing** special-relativistic aberration
//! direction, preserving one-way light-time reception velocities.
//!
//! This is an intermediate Cartesian **fixed ICRF** geocentric state, NOT
//! a complete true-ecliptic-of-date `swe_calc_ut` replacement. The source
//! aberration law is reused from `crate::aberration`, not reimplemented.
//! Derivatives below are obtained by the analytic chain rule applied to
//! its normalized special-relativistic transformation.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::aberration::{
    annual_aberration_icrf, AberrationError, SPEED_OF_LIGHT_KM_PER_DAY,
};
use crate::de441::{Body, De441, De441Error, JulianDateTdb};
use crate::light_time::{geocentric_light_time, GeocentricLightTimeState, LightTimeError};

/// Where the Earth's barycentric *acceleration* came from.
/// `FiniteDifference` denotes a numerical approximation, not an
/// analytic JPL second derivative; its step is a TDB-day offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObserverAccelerationSource {
    CallerProvided,
    CenteredEphemerisVelocity { step_tdb_days: f64 },
}

/// Light-time + special-relativistic annual-aberration vector and rate.
///
/// The length of the original light-time vector is retained; aberration
/// changes only the source direction, not Earth-to-target distance.
/// `velocity_km_per_tdb_day` is the time derivative of this **aberrated**
/// vector with respect to observation TDB, including the source's
/// light-time derivative and Earth's barycentric **acceleration**.
/// The center is Earth; the frame is fixed ICRF, not of date.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AberratedGeocentricState {
    pub target: Body,
    pub observation_epoch: JulianDateTdb,
    pub emission_epoch: JulianDateTdb,
    pub light_time_seconds: f64,
    pub light_time_rate: f64,
    pub observer_acceleration_source: ObserverAccelerationSource,
    pub position_km: [f64; 3],
    pub velocity_km_per_tdb_day: [f64; 3],
}

#[derive(Debug)]
pub enum AberratedStateError {
    InvalidIntermediate,
    InvalidAccelerationStep,
    NonFiniteAcceleration,
    ArithmeticOverflow,
    Ephemeris(De441Error),
    LightTime(LightTimeError),
    Aberration(AberrationError),
}

impl fmt::Display for AberratedStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIntermediate => write!(f, "invalid reception light-time state"),
            Self::InvalidAccelerationStep => {
                write!(f, "acceleration step must be representable and 0 < h <= 0.5 TDB day")
            }
            Self::NonFiniteAcceleration => {
                write!(f, "nonfinite barycentric Earth acceleration")
            }
            Self::ArithmeticOverflow => {
                write!(f, "aberration-state derivative is not representable")
            }
            Self::Ephemeris(error) => write!(f, "DE441: {error}"),
            Self::LightTime(error) => write!(f, "light-time: {error}"),
            Self::Aberration(error) => write!(f, "annual aberration: {error}"),
        }
    }
}

impl Error for AberratedStateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Ephemeris(error) => Some(error),
            Self::LightTime(error) => Some(error),
            Self::Aberration(error) => Some(error),
            _ => None,
        }
    }
}

impl From<De441Error> for AberratedStateError {
    fn from(error: De441Error) -> Self {
        Self::Ephemeris(error)
    }
}
impl From<LightTimeError> for AberratedStateError {
    fn from(error: LightTimeError) -> Self {
        Self::LightTime(error)
    }
}
impl From<AberrationError> for AberratedStateError {
    fn from(error: AberrationError) -> Self {
        Self::Aberration(error)
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

/// Apply annual aberration **with analytical TDB-day rates** to a
/// previously computed geocentric reception light-time state.
///
/// The three supplied observer velocity components MUST describe Earth's
/// barycentric velocity at the **same reception TDB epoch** as the state,
/// in km/TDB day and the same fixed ICRF frame. The three supplied
/// acceleration components are the derivative of that velocity at the
/// same epoch in km/TDB day². No epoch or physical reference frame can be
/// verified from raw arrays; the caller remains responsible.
///
/// This invokes `annual_aberration_icrf` to obtain the *existing*
/// independently tested source direction. Its time derivative is
/// calculated by differentiating the same Lorentz boost. Let
/// `n=r/R`, `ndot=(r_dot-n*(n·r_dot))/R`, `beta=vEarth/c`,
/// `w=sqrt(1-|beta|²)`, `k=n·beta`, `q=1+k/(1+w)`.
/// The boost points in the direction of `y=w*n+q*beta` (its positive
/// denominator cancels during normalization). The analytic
/// `ydot=w_dot*n+w*ndot+q_dot*beta+q*beta_dot`; its unit-direction
/// derivative is `(ydot - n_ab*(n_ab·ydot))/|y|`.
/// Finally `r_ab=R*n_ab`, `rdot_ab=Rdot*n_ab+R*ndot_ab`.
///
/// No second light-time correction or total velocity re-subtraction is
/// performed. The **radial rate is preserved** because the Lorentz boost
/// affects direction only. The result remains missing gravitational
/// deflection, precession/nutation, true ecliptic and TT/TDB conversion.
pub fn aberrate_light_time_state(
    state: &GeocentricLightTimeState,
    earth_barycentric_velocity_km_per_tdb_day: [f64; 3],
    earth_barycentric_acceleration_km_per_tdb_day2: [f64; 3],
) -> Result<AberratedGeocentricState, AberratedStateError> {
    if state.target == Body::Earth
        || !state.observation_epoch.0.is_finite()
        || !state.emission_epoch.0.is_finite()
        || state.emission_epoch.0 > state.observation_epoch.0
        || !state.light_time_seconds.is_finite()
        || state.light_time_seconds <= 0.0
        || !state.light_time_rate.is_finite()
        || state
            .position_km
            .iter()
            .chain(state.velocity_km_per_day.iter())
            .any(|v| !v.is_finite())
    {
        return Err(AberratedStateError::InvalidIntermediate);
    }
    if earth_barycentric_acceleration_km_per_tdb_day2
        .iter()
        .any(|a| !a.is_finite())
    {
        return Err(AberratedStateError::NonFiniteAcceleration);
    }

    // Reuse the existing independently tested Lorentz boost for the
    // actual direction and preserve the one-way reception radius.
    let observed_direction = annual_aberration_icrf(
        state.position_km,
        earth_barycentric_velocity_km_per_tdb_day,
    )?;
    let radius = norm(state.position_km);
    if !radius.is_finite() || radius == 0.0 {
        return Err(AberratedStateError::ArithmeticOverflow);
    }
    let n = state.position_km.map(|x| x / radius);
    let range_rate = dot(n, state.velocity_km_per_day);
    let n_rate = std::array::from_fn::<_, 3, _>(|i| {
        (state.velocity_km_per_day[i] - n[i] * range_rate) / radius
    });

    let beta = earth_barycentric_velocity_km_per_tdb_day
        .map(|v| v / SPEED_OF_LIGHT_KM_PER_DAY);
    let beta_rate = earth_barycentric_acceleration_km_per_tdb_day2
        .map(|a| a / SPEED_OF_LIGHT_KM_PER_DAY);
    let bnorm = norm(beta);
    let inverse_gamma = (1.0 - bnorm * bnorm).sqrt();
    if !inverse_gamma.is_finite() || inverse_gamma <= 0.0 {
        return Err(AberratedStateError::ArithmeticOverflow);
    }
    let inverse_gamma_rate = -dot(beta, beta_rate) / inverse_gamma;
    let k = dot(n, beta);
    let k_rate = dot(n_rate, beta) + dot(n, beta_rate);
    let q = 1.0 + k / (1.0 + inverse_gamma);
    let q_rate = k_rate / (1.0 + inverse_gamma)
        - k * inverse_gamma_rate / (1.0 + inverse_gamma).powi(2);

    let y = std::array::from_fn::<_, 3, _>(|i| inverse_gamma * n[i] + q * beta[i]);
    let y_rate = std::array::from_fn::<_, 3, _>(|i| {
        inverse_gamma_rate * n[i]
            + inverse_gamma * n_rate[i]
            + q_rate * beta[i]
            + q * beta_rate[i]
    });
    let y_norm = norm(y);
    if !y_norm.is_finite() || y_norm == 0.0 {
        return Err(AberratedStateError::ArithmeticOverflow);
    }
    let tangent_rate = dot(observed_direction, y_rate);
    let direction_rate = std::array::from_fn::<_, 3, _>(|i| {
        (y_rate[i] - observed_direction[i] * tangent_rate) / y_norm
    });
    let position_km = observed_direction.map(|direction| radius * direction);
    let velocity_km_per_tdb_day = std::array::from_fn::<_, 3, _>(|i| {
        range_rate * observed_direction[i] + radius * direction_rate[i]
    });
    if position_km
        .iter()
        .chain(velocity_km_per_tdb_day.iter())
        .chain(direction_rate.iter())
        .any(|v| !v.is_finite())
        || !range_rate.is_finite()
        || !q_rate.is_finite()
    {
        return Err(AberratedStateError::ArithmeticOverflow);
    }
    Ok(AberratedGeocentricState {
        target: state.target,
        observation_epoch: state.observation_epoch,
        emission_epoch: state.emission_epoch,
        light_time_seconds: state.light_time_seconds,
        light_time_rate: state.light_time_rate,
        observer_acceleration_source: ObserverAccelerationSource::CallerProvided,
        position_km,
        velocity_km_per_tdb_day,
    })
}

/// Compose classic DE441 barycentric Earth samples, existing light-time
/// correction and the existing aberration law into a **single operation**.
///
/// Earth acceleration is estimated by the **explicit** centered velocity
/// difference `(v(t+h)-v(t-h))/(2h)` in km/TDB day². The caller chooses
/// `h`; this method does NOT claim an analytic JPL acceleration or
/// a validated acceleration error bound. Samples near file coverage edges
/// fail instead of switching to an unstated one-sided difference.
///
/// The output is a **light-time + annual-aberration** Cartesian ICRF
/// intermediate with velocities, not a full Swiss apparent-of-date state.
pub fn geocentric_light_time_annual_aberration<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    observation_epoch: JulianDateTdb,
    acceleration_step_tdb_days: f64,
) -> Result<AberratedGeocentricState, AberratedStateError> {
    let t = observation_epoch.0;
    let h = acceleration_step_tdb_days;
    if !t.is_finite()
        || !h.is_finite()
        || h <= 0.0
        || h > 0.5
        || t - h == t
        || t + h == t
        || !(t - h).is_finite()
        || !(t + h).is_finite()
    {
        return Err(AberratedStateError::InvalidAccelerationStep);
    }

    let earth_velocity = ephemeris
        .state(
            Body::Earth,
            Body::SolarSystemBarycenter,
            observation_epoch,
        )?
        .velocity_km_per_day;
    let earlier = ephemeris
        .state(
            Body::Earth,
            Body::SolarSystemBarycenter,
            JulianDateTdb(t - h),
        )?
        .velocity_km_per_day;
    let later = ephemeris
        .state(
            Body::Earth,
            Body::SolarSystemBarycenter,
            JulianDateTdb(t + h),
        )?
        .velocity_km_per_day;
    let acceleration = std::array::from_fn::<_, 3, _>(|i| (later[i] - earlier[i]) / (2.0 * h));
    if acceleration.iter().any(|value| !value.is_finite()) {
        return Err(AberratedStateError::NonFiniteAcceleration);
    }

    let light_time = geocentric_light_time(ephemeris, target, observation_epoch)?;
    let mut result = aberrate_light_time_state(&light_time, earth_velocity, acceleration)?;
    result.observer_acceleration_source =
        ObserverAccelerationSource::CenteredEphemerisVelocity { step_tdb_days: h };
    Ok(result)
}
