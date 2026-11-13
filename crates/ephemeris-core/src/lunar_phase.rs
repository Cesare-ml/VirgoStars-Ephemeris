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

//! Geometric lunar illumination from matched Earth-centered DE441 states.
//!
//! This is an uncorrected spherical illumination model, not an apparent
//! Moon phase ephemeris or an implementation of calendar phase events.

use std::error::Error;
use std::f64::consts::{FRAC_PI_2, PI};
use std::fmt;

use crate::de441::{Body, JulianDateTdb, StateVector};
use crate::geometry::{geometric_angular_separation_rad, AngularSeparationError};

/// A geometric, geocentric, ideal-sphere Moon illumination estimate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometricLunarIllumination {
    /// The already-TDB Julian date shared by both input states.
    pub epoch: JulianDateTdb,
    /// Sun–Moon–Earth angle at the Moon, radians: 0 = full; PI = new.
    pub phase_angle_rad: f64,
    /// Illuminated fraction of the ideal lunar disc, in [0, 1].
    pub illuminated_fraction: f64,
}

/// Fail-closed errors for matched geometric Moon/Sun input states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LunarIlluminationError {
    /// Input state pairs must be Sun/Earth and Moon/Earth.
    WrongTargetOrCenter,
    /// Both states must refer to exactly the same finite TDB epoch.
    MismatchedEpoch,
    /// The supplied epoch or a used position component is not finite.
    NonFiniteInput,
    /// Subtracting finite positions overflowed floating-point range.
    UnrepresentableDifference,
    /// The Moon's observer/Sun directions are undefined.
    Geometry(AngularSeparationError),
}

impl fmt::Display for LunarIlluminationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongTargetOrCenter => {
                write!(f, "expected Sun/Earth and Moon/Earth geometric states")
            }
            Self::MismatchedEpoch => write!(f, "Sun and Moon TDB epochs do not match"),
            Self::NonFiniteInput => write!(f, "non-finite TDB epoch or state position"),
            Self::UnrepresentableDifference => {
                write!(f, "Moon-to-Sun difference exceeds floating-point range")
            }
            Self::Geometry(error) => write!(f, "undefined lunar illumination geometry: {error}"),
        }
    }
}

impl Error for LunarIlluminationError {}

impl From<AngularSeparationError> for LunarIlluminationError {
    fn from(error: AngularSeparationError) -> Self {
        Self::Geometry(error)
    }
}

/// Ideal-sphere geometric Moon illuminated fraction from Earth-centered states.
///
/// `sun` must describe `Body::Sun` relative to `Body::Earth`, and `moon`
/// must describe `Body::Moon` relative to `Body::Earth`, both at the exact
/// same already-TDB Julian date. All vectors are geometric, in ICRF axes and
/// kilometers, as defined by the classic DE441 `StateVector` API.
///
/// Forms Moon-to-Sun = Earth-to-Sun minus Earth-to-Moon and Moon-to-Earth =
/// minus Earth-to-Moon, then measures their unsigned angle `alpha`. The
/// fraction is `cos(alpha / 2)^2` (computed using a complementary half-angle
/// near PI for an exact zero at a perfectly dark phase). No lookup, correction,
/// light-time, shadowing, lunar libration or atmospheric effects are applied.
///
/// This returns the **fraction illuminated**, not the lit area visible to a
/// topocentric observer after parallax, not waxing/waning, and not an event
/// time for new/full Moon. Velocity components are not used.
pub fn geometric_lunar_illumination(
    sun: &StateVector,
    moon: &StateVector,
) -> Result<GeometricLunarIllumination, LunarIlluminationError> {
    if sun.target != Body::Sun
        || sun.center != Body::Earth
        || moon.target != Body::Moon
        || moon.center != Body::Earth
    {
        return Err(LunarIlluminationError::WrongTargetOrCenter);
    }
    if !sun.epoch.0.is_finite() || !moon.epoch.0.is_finite() {
        return Err(LunarIlluminationError::NonFiniteInput);
    }
    if sun.epoch != moon.epoch {
        return Err(LunarIlluminationError::MismatchedEpoch);
    }
    if sun
        .position_km
        .iter()
        .chain(moon.position_km.iter())
        .any(|value| !value.is_finite())
    {
        return Err(LunarIlluminationError::NonFiniteInput);
    }

    let moon_to_sun: [f64; 3] =
        std::array::from_fn(|axis| sun.position_km[axis] - moon.position_km[axis]);
    if moon_to_sun.iter().any(|value| !value.is_finite()) {
        return Err(LunarIlluminationError::UnrepresentableDifference);
    }
    let moon_to_earth = moon.position_km.map(|component| -component);
    let alpha = geometric_angular_separation_rad(moon_to_sun, moon_to_earth)?;
    let half = 0.5 * alpha;
    let illuminated_fraction = if alpha <= FRAC_PI_2 {
        half.cos().powi(2)
    } else {
        ((PI - alpha) * 0.5).sin().powi(2)
    };

    Ok(GeometricLunarIllumination {
        epoch: moon.epoch,
        phase_angle_rad: alpha,
        illuminated_fraction,
    })
}
