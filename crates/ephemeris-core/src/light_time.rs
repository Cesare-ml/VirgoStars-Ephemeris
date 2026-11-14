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

//! Geocentric reception light-time correction in fixed ICRF axes.
//!
//! This is one stage of an apparent-position pipeline, not an apparent
//! position, an Earth-of-date frame, or a Swiss-compatible output.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::de441::{Body, De441, De441Error, JulianDateTdb, StateVector};

/// Exact SI speed of light in kilometres per second.
pub const SPEED_OF_LIGHT_KM_PER_SECOND: f64 = 299_792.458;
const SECONDS_PER_DAY: f64 = 86_400.0;
const C_KM_PER_DAY: f64 = SPEED_OF_LIGHT_KM_PER_SECOND * SECONDS_PER_DAY;
const MAX_ITERATIONS: usize = 32;

/// Reception light-time corrected target relative to Earth's center.
///
/// The observation and emission epochs are TDB JD; axes are fixed ICRF.
/// The velocity is the derivative of the returned position with respect to
/// observation TDB, including the derivative of the one-way light-time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeocentricLightTimeState {
    pub target: Body,
    pub observation_epoch: JulianDateTdb,
    pub emission_epoch: JulianDateTdb,
    pub light_time_seconds: f64,
    /// Derivative of light time with respect to observation time (s/s).
    pub light_time_rate: f64,
    pub position_km: [f64; 3],
    pub velocity_km_per_day: [f64; 3],
}

#[derive(Debug)]
pub enum LightTimeError {
    Ephemeris(De441Error),
    /// No geocentric direction exists for Earth or a zero-range target.
    UndefinedDirection,
    /// Non-finite geometry or an invalid light-time derivative denominator.
    InvalidGeometry,
    /// The fixed-point solution did not settle on a representable TDB JD.
    DidNotConverge,
}

impl fmt::Display for LightTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ephemeris(error) => write!(f, "light-time ephemeris error: {error}"),
            Self::UndefinedDirection => write!(f, "light-time direction is undefined"),
            Self::InvalidGeometry => write!(f, "invalid light-time geometry or velocity"),
            Self::DidNotConverge => write!(f, "light-time iteration did not converge"),
        }
    }
}

impl Error for LightTimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Ephemeris(error) => Some(error),
            _ => None,
        }
    }
}

impl From<De441Error> for LightTimeError {
    fn from(error: De441Error) -> Self {
        Self::Ephemeris(error)
    }
}

/// One-way reception correction for a target observed from Earth's center.
///
/// Samples the Earth at observation TDB and the target at the iterated emission
/// TDB using the caller's classic DE441 reader. Only Newtonian light-time is
/// included: there is no stellar aberration, gravitational light deflection,
/// precession/nutation, terrestrial observer, refraction or time conversion.
/// The DE441 reader uses a single-f64 JD, limiting emission-time resolution.
/// Earth as target is rejected; out-of-coverage emission epochs are errors.
pub fn geocentric_light_time<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    observation_epoch: JulianDateTdb,
) -> Result<GeocentricLightTimeState, LightTimeError> {
    solve(target, observation_epoch, |body, epoch| {
        ephemeris.state(body, Body::SolarSystemBarycenter, epoch)
    })
}

fn solve<F>(
    target: Body,
    observation_epoch: JulianDateTdb,
    mut barycentric_state: F,
) -> Result<GeocentricLightTimeState, LightTimeError>
where
    F: FnMut(Body, JulianDateTdb) -> Result<StateVector, De441Error>,
{
    if target == Body::Earth {
        return Err(LightTimeError::UndefinedDirection);
    }
    if !observation_epoch.0.is_finite() {
        return Err(LightTimeError::InvalidGeometry);
    }
    let observer = barycentric_state(Body::Earth, observation_epoch)?;
    let mut emission = observation_epoch;
    for _ in 0..MAX_ITERATIONS {
        let body = barycentric_state(target, emission)?;
        let position =
            std::array::from_fn::<_, 3, _>(|i| body.position_km[i] - observer.position_km[i]);
        let range = position[0].hypot(position[1]).hypot(position[2]);
        if !range.is_finite() {
            return Err(LightTimeError::InvalidGeometry);
        }
        if range == 0.0 {
            return Err(LightTimeError::UndefinedDirection);
        }
        let delay_seconds = range / SPEED_OF_LIGHT_KM_PER_SECOND;
        let next = observation_epoch.0 - delay_seconds / SECONDS_PER_DAY;
        if !next.is_finite() {
            return Err(LightTimeError::InvalidGeometry);
        }
        if next == emission.0 {
            let direction = position.map(|x| x / range);
            let dot = |velocity: [f64; 3]| -> f64 {
                direction.iter().zip(velocity).map(|(a, b)| a * b).sum()
            };
            let target_radial = dot(body.velocity_km_per_day);
            let observer_radial = dot(observer.velocity_km_per_day);
            let denominator = C_KM_PER_DAY + target_radial;
            if !denominator.is_finite() || denominator <= 0.0 {
                return Err(LightTimeError::InvalidGeometry);
            }
            let delay_rate = (target_radial - observer_radial) / denominator;
            let velocity = std::array::from_fn::<_, 3, _>(|i| {
                body.velocity_km_per_day[i] * (1.0 - delay_rate) - observer.velocity_km_per_day[i]
            });
            if !delay_rate.is_finite() || velocity.iter().any(|x| !x.is_finite()) {
                return Err(LightTimeError::InvalidGeometry);
            }
            return Ok(GeocentricLightTimeState {
                target,
                observation_epoch,
                emission_epoch: emission,
                light_time_seconds: delay_seconds,
                light_time_rate: delay_rate,
                position_km: position,
                velocity_km_per_day: velocity,
            });
        }
        emission = JulianDateTdb(next);
    }
    Err(LightTimeError::DidNotConverge)
}

#[cfg(test)]
mod tests {
    use super::*;

    const JD: f64 = 2_451_545.0;

    fn linear(body: Body, epoch: JulianDateTdb) -> Result<StateVector, De441Error> {
        let (initial, rate) = match body {
            Body::Earth => ([0.0, 0.0, 0.0], [10.0 * SECONDS_PER_DAY, 0.0, 0.0]),
            Body::MarsBarycenter => (
                [150_000_000.0, 0.0, 0.0],
                [30.0 * SECONDS_PER_DAY, 0.0, 0.0],
            ),
            _ => return Err(De441Error::MissingSeries("test body")),
        };
        Ok(StateVector {
            target: body,
            center: Body::SolarSystemBarycenter,
            epoch,
            position_km: std::array::from_fn(|i| initial[i] + (epoch.0 - JD) * rate[i]),
            velocity_km_per_day: rate,
        })
    }

    #[test]
    fn moving_target_has_analytic_delay_and_derivative() {
        let state = solve(Body::MarsBarycenter, JulianDateTdb(JD), linear).unwrap();
        let expected_delay = 150_000_000.0 / (SPEED_OF_LIGHT_KM_PER_SECOND + 30.0);
        let expected_rate = (30.0 - 10.0) / (SPEED_OF_LIGHT_KM_PER_SECOND + 30.0);
        assert!((state.light_time_seconds - expected_delay).abs() < 0.000_001);
        assert!((state.light_time_rate - expected_rate).abs() < 1.0e-15);
        let expected_velocity = (30.0 * (1.0 - expected_rate) - 10.0) * SECONDS_PER_DAY;
        assert!((state.velocity_km_per_day[0] - expected_velocity).abs() < 1.0e-7);
        assert!(state.emission_epoch.0 < state.observation_epoch.0);
    }

    #[test]
    fn rejects_undefined_direction_and_invalid_epoch() {
        assert!(matches!(
            solve(Body::Earth, JulianDateTdb(JD), linear),
            Err(LightTimeError::UndefinedDirection)
        ));
        assert!(matches!(
            solve(Body::Sun, JulianDateTdb(f64::NAN), linear),
            Err(LightTimeError::InvalidGeometry)
        ));
    }

    #[test]
    fn propagates_ephemeris_errors() {
        assert!(matches!(
            solve(Body::Sun, JulianDateTdb(JD), linear),
            Err(LightTimeError::Ephemeris(De441Error::MissingSeries(_)))
        ));
    }

    #[test]
    fn rejects_nonfinite_velocity() {
        let result = solve(Body::MarsBarycenter, JulianDateTdb(JD), |body, epoch| {
            let mut state = linear(body, epoch)?;
            if body == Body::MarsBarycenter {
                state.velocity_km_per_day[0] = f64::NAN;
            }
            Ok(state)
        });
        assert!(matches!(result, Err(LightTimeError::InvalidGeometry)));
    }

    #[test]
    fn reports_nonconvergence_for_near_light_speed_synthetic_target() {
        let result = solve(Body::MarsBarycenter, JulianDateTdb(JD), |body, epoch| {
            let mut state = linear(body, epoch)?;
            if body == Body::Earth {
                state.position_km = [0.0; 3];
                state.velocity_km_per_day = [0.0; 3];
            } else {
                let speed = 0.99 * C_KM_PER_DAY;
                state.position_km = [1.0e9 + speed * (epoch.0 - JD), 0.0, 0.0];
                state.velocity_km_per_day = [speed, 0.0, 0.0];
            }
            Ok(state)
        });
        assert!(matches!(result, Err(LightTimeError::DidNotConverge)));
    }
}
