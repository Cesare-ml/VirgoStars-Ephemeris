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

//! Standalone special-relativistic annual aberration of a source direction.
//!
//! ICRF (barycentric inertial) axes; geocentric Earth observer velocity at the
//! reception TDB epoch. This is ONE correction stage, not a complete apparent
//! geocentric/ecliptic-of-date position or a Swiss Ephemeris replacement.
//!
//! Special-relativistic photon-direction velocity addition is implemented
//! independently from the Lorentz boost, without external astronomy code.

use std::error::Error;
use std::fmt;

/// Exact SI speed of light, in km per TDB day, using 86,400 SI seconds/day.
pub const SPEED_OF_LIGHT_KM_PER_DAY: f64 = 299_792.458 * 86_400.0;

/// Errors applying the observer's Lorentz boost to a light-source direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AberrationError {
    /// At least one input component is NaN or infinite.
    NonFiniteInput,
    /// An all-zero source vector has no astronomical direction.
    UndefinedDirection,
    /// The observer's speed is at least the speed of light.
    SuperluminalObserver,
    /// A finite input cannot be safely evaluated in f64.
    ArithmeticOverflow,
}

impl fmt::Display for AberrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "non-finite aberration input"),
            Self::UndefinedDirection => write!(f, "undefined source direction"),
            Self::SuperluminalObserver => {
                write!(f, "observer barycentric speed must be less than c")
            }
            Self::ArithmeticOverflow => write!(f, "aberration arithmetic cannot be represented"),
        }
    }
}

impl Error for AberrationError {}

/// Apply **only** special-relativistic annual aberration to an ICRF source direction.
///
/// * `source_position_km`: nonzero vector **from Earth's center towards the
///   observed source**, in ICRF axes; its length is discarded. For a planet,
///   supply a separately computed reception light-time-retarded line of sight,
///   not a raw barycentric position.
/// * `earth_barycentric_velocity_km_per_day`: Earth's **barycentric velocity
///   at reception**, same ICRF axes, in km per TDB day. Do not pass a target
///   velocity, geocentric relative speed, or an Earth-of-date velocity.
/// * Return: unit vector toward the aberrated source in the **same fixed ICRF
///   axes**. It is neither an apparent ecliptic-of-date longitude nor a
///   position/velocity state; no angular speed or aberration rate is returned.
///
/// The Lorentz boost for source-pointing direction n and beta=v/c is:
/// n' = [n sqrt(1-beta²) + beta(1+(n·beta)/(1+sqrt(1-beta²)))]/(1+n·beta).
/// This uses the observer's velocity sign, so Earth motion towards +X
/// shifts a perpendicular source's apparent direction towards +X.
///
/// Missing: gravitational bending and potential, Shapiro delay,
/// precession/nutation, polar motion, diurnal aberration, observer parallax,
/// atmospheric refraction and rotating-frame derivative terms.
/// No conversion among UTC, TT or TDB is performed.
pub fn annual_aberration_icrf(
    source_position_km: [f64; 3],
    earth_barycentric_velocity_km_per_day: [f64; 3],
) -> Result<[f64; 3], AberrationError> {
    if source_position_km
        .iter()
        .chain(earth_barycentric_velocity_km_per_day.iter())
        .any(|component| !component.is_finite())
    {
        return Err(AberrationError::NonFiniteInput);
    }

    let range = source_position_km[0]
        .hypot(source_position_km[1])
        .hypot(source_position_km[2]);
    if range == 0.0 {
        return Err(AberrationError::UndefinedDirection);
    }
    if !range.is_finite() {
        return Err(AberrationError::ArithmeticOverflow);
    }
    let direction = source_position_km.map(|component| component / range);
    let beta = earth_barycentric_velocity_km_per_day
        .map(|velocity| velocity / SPEED_OF_LIGHT_KM_PER_DAY);
    let speed = beta[0].hypot(beta[1]).hypot(beta[2]);
    if speed >= 1.0 {
        return Err(AberrationError::SuperluminalObserver);
    }
    if !speed.is_finite() {
        return Err(AberrationError::ArithmeticOverflow);
    }

    // 1/gamma avoids infinities at high but subluminal speeds. 1+dot
    // is strictly positive since |beta| < 1, even for anti-parallel rays.
    let inverse_gamma = (1.0 - speed * speed).sqrt();
    let dot = direction
        .iter()
        .zip(beta)
        .map(|(n, b)| n * b)
        .sum::<f64>();
    let denominator = 1.0 + dot;
    let boost = 1.0 + dot / (1.0 + inverse_gamma);
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(AberrationError::ArithmeticOverflow);
    }
    let transformed = std::array::from_fn::<_, 3, _>(|i| {
        (inverse_gamma * direction[i] + boost * beta[i]) / denominator
    });
    let norm = transformed[0]
        .hypot(transformed[1])
        .hypot(transformed[2]);
    if !norm.is_finite() || norm == 0.0 {
        return Err(AberrationError::ArithmeticOverflow);
    }
    let result = transformed.map(|component| component / norm);
    if result.iter().any(|component| !component.is_finite()) {
        return Err(AberrationError::ArithmeticOverflow);
    }
    Ok(result)
}
