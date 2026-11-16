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

//! Gravitational deflection of light by the Sun, applied to a geocentric
//! light-time state **before** annual aberration.
//!
//! # Model
//!
//! Single-body weak-field deflection (Explanatory Supplement to the
//! Astronomical Almanac, 3rd ed., section 7.2.4; Klioner 2003, AJ 125, 1580),
//! with `p` the unit observer→source direction, `q` the unit Sun→source
//! direction at light emission, `e` the unit Sun→observer direction at
//! reception and `E` the Sun–observer distance in AU:
//!
//! ```text
//! p' = p + w * ( p × (e × q) ),   w = (2GM/c²)/E / max(q·(q+e), 1e-6)
//! ```
//!
//! `2GM/c²` for the Sun is [`SUN_SCHWARZSCHILD_RADIUS_AU`]. Only the Sun is
//! modeled; the planets' contribution (< 0.0002″ for Jupiter at the limb) is
//! omitted and the target `Body::Sun` is never deflected by itself.
//!
//! # Rates
//!
//! The velocity of the deflected state is `V + (V·p) δ + R dδ/dt` with
//! `δ = p' − p` and `dδ/dt` a central finite difference of `δ` computed from
//! the full light-time solution at `t ± 1/256` TDB day. `δ` is computed
//! directly (not as a difference of two nearly equal unit vectors), so the
//! difference carries no cancellation noise. The step is a power of two, so
//! `t ± h` is exact in binary floating point.
//!
//! # Limits
//!
//! The formula is first order in `GM/(c²b)`; it is not meant for rays that
//! pass within a few solar radii of the Sun. The `1e-6` floor on `q·(q+e)`
//! (the standard SOFA/IAU guard) bounds the effect for a source almost
//! exactly behind the Sun, where the actual ray would be occulted.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::de441::{Body, De441, De441Error, JulianDateTdb};
use crate::light_time::{geocentric_light_time, GeocentricLightTimeState, LightTimeError};

/// `2GM_sun/c²` in astronomical units (IAU 2015 nominal solar `GM`).
pub const SUN_SCHWARZSCHILD_RADIUS_AU: f64 = 1.974_125_743_36e-8;
const AU_KM: f64 = 149_597_870.7;
const DOT_FLOOR: f64 = 1.0e-6;
/// Half-step of the finite difference for `dδ/dt`, in TDB days (2⁻⁸).
pub const DEFLECTION_RATE_HALF_STEP_DAYS: f64 = 1.0 / 256.0;

#[derive(Debug)]
pub enum DeflectionError {
    InvalidGeometry,
    Ephemeris(De441Error),
    LightTime(LightTimeError),
}

impl fmt::Display for DeflectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGeometry => write!(f, "invalid solar deflection geometry"),
            Self::Ephemeris(e) => write!(f, "deflection ephemeris: {e}"),
            Self::LightTime(e) => write!(f, "deflection light-time: {e}"),
        }
    }
}

impl Error for DeflectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Ephemeris(e) => Some(e),
            Self::LightTime(e) => Some(e),
            Self::InvalidGeometry => None,
        }
    }
}

impl From<De441Error> for DeflectionError {
    fn from(e: De441Error) -> Self {
        Self::Ephemeris(e)
    }
}
impl From<LightTimeError> for DeflectionError {
    fn from(e: LightTimeError) -> Self {
        Self::LightTime(e)
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn unit(v: [f64; 3]) -> Result<[f64; 3], DeflectionError> {
    let n = norm(v);
    if !n.is_finite() || n == 0.0 {
        return Err(DeflectionError::InvalidGeometry);
    }
    Ok([v[0] / n, v[1] / n, v[2] / n])
}

/// Direction change `δ = p' − p` for unit vectors `p`, `q`, `e` and the
/// Sun–observer distance `sun_observer_au` (> 0).
pub fn solar_deflection_delta(
    p: [f64; 3],
    q: [f64; 3],
    e: [f64; 3],
    sun_observer_au: f64,
) -> Result<[f64; 3], DeflectionError> {
    if !(sun_observer_au.is_finite() && sun_observer_au > 0.0) {
        return Err(DeflectionError::InvalidGeometry);
    }
    let qpe = [q[0] + e[0], q[1] + e[1], q[2] + e[2]];
    let w = SUN_SCHWARZSCHILD_RADIUS_AU / sun_observer_au / dot(q, qpe).max(DOT_FLOOR);
    let peq = cross(p, cross(e, q));
    let delta = [w * peq[0], w * peq[1], w * peq[2]];
    if delta.iter().any(|x| !x.is_finite()) {
        return Err(DeflectionError::InvalidGeometry);
    }
    Ok(delta)
}

/// `δ` for `target` seen at `observation_epoch`, from a full light-time solution.
fn delta_at<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    observation_epoch: JulianDateTdb,
    state: &GeocentricLightTimeState,
) -> Result<[f64; 3], DeflectionError> {
    let p = unit(state.position_km)?;
    let earth = ephemeris.state(Body::Earth, Body::SolarSystemBarycenter, observation_epoch)?;
    let sun_obs = ephemeris.state(Body::Sun, Body::SolarSystemBarycenter, observation_epoch)?;
    let body_emit = ephemeris.state(target, Body::SolarSystemBarycenter, state.emission_epoch)?;
    let sun_emit = ephemeris.state(Body::Sun, Body::SolarSystemBarycenter, state.emission_epoch)?;
    let sun_to_observer: [f64; 3] =
        std::array::from_fn(|i| earth.position_km[i] - sun_obs.position_km[i]);
    let sun_to_source: [f64; 3] =
        std::array::from_fn(|i| body_emit.position_km[i] - sun_emit.position_km[i]);
    solar_deflection_delta(
        p,
        unit(sun_to_source)?,
        unit(sun_to_observer)?,
        norm(sun_to_observer) / AU_KM,
    )
}

/// Apply solar light deflection to a geocentric light-time state.
///
/// The returned state keeps the epochs, range and light-time metadata; only
/// the direction of the position and the velocity change. `Body::Sun` is
/// returned unchanged.
pub fn deflect_light_time_state_by_sun<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    state: &GeocentricLightTimeState,
) -> Result<GeocentricLightTimeState, DeflectionError> {
    if state.target == Body::Sun {
        return Ok(*state);
    }
    let t = state.observation_epoch;
    let h = DEFLECTION_RATE_HALF_STEP_DAYS;
    let delta = delta_at(ephemeris, state.target, t, state)?;
    let mut rates = [[0.0; 3]; 2];
    for (slot, step) in rates.iter_mut().zip([-h, h]) {
        let epoch = JulianDateTdb(t.0 + step);
        let lt = geocentric_light_time(ephemeris, state.target, epoch)?;
        *slot = delta_at(ephemeris, state.target, epoch, &lt)?;
    }
    let delta_rate: [f64; 3] = std::array::from_fn(|i| (rates[1][i] - rates[0][i]) / (2.0 * h));

    let range = norm(state.position_km);
    let p = unit(state.position_km)?;
    let deflected = unit([p[0] + delta[0], p[1] + delta[1], p[2] + delta[2]])?;
    let radial_speed = dot(state.velocity_km_per_day, p);
    let position_km = [
        range * deflected[0],
        range * deflected[1],
        range * deflected[2],
    ];
    let velocity_km_per_day: [f64; 3] = std::array::from_fn(|i| {
        state.velocity_km_per_day[i] + radial_speed * delta[i] + range * delta_rate[i]
    });
    if position_km
        .iter()
        .chain(velocity_km_per_day.iter())
        .any(|x| !x.is_finite())
    {
        return Err(DeflectionError::InvalidGeometry);
    }
    Ok(GeocentricLightTimeState {
        position_km,
        velocity_km_per_day,
        ..*state
    })
}
