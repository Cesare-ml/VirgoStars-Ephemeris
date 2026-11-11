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

//! Fixed-declination geometric altitude crossings in local hour angle.
//!
//! This is a spherical-geometry primitive, **not** a rise/set event search.
//! No clock, UT1/TT conversion, ephemeris interpolation, atmospheric
//! refraction, limb radius, parallax, horizon profile or altitude-of-eye is
//! supplied here. The caller must use latitude, declination and hour angle
//! expressed in a consistent celestial/equatorial frame.
//!
//! Geometric identity (angles in radians):
//! sin(alt) = sin(lat) sin(dec) + cos(lat) cos(dec) cos(H).
//! This is independently described by the IMCCE rise/set guide:
//! https://promenade.imcce.fr/en/pages3/367.html
//!
//! Created 2026-11-11. Independent original implementation.

use std::error::Error;
use std::f64::consts::{FRAC_PI_2, PI};
use std::fmt;

/// Outcomes for a fixed-declination target and a constant altitude threshold.
///
/// A genuine pair of crossings has a rising angle in (-π, 0) and a
/// setting angle in (0, π). Both values are local hour angles, *not*
/// timestamps or lengths of civil or UT1 days.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AltitudeCrossings {
    /// Two strict crossings, assuming declination remains fixed as H varies.
    Two {
        rising_hour_angle_rad: f64,
        setting_hour_angle_rad: f64,
    },
    /// Altitude is strictly above the threshold for all hour angles.
    AlwaysAbove,
    /// Altitude is strictly below the threshold for all hour angles.
    AlwaysBelow,
    /// Touches the threshold only at upper meridian passage H=0.
    TangentAtUpperMeridian,
    /// Touches the threshold only at lower meridian passage H=±π.
    TangentAtLowerMeridian,
    /// Altitude does not change with hour angle and equals the threshold
    /// within the explicit near-pole floating-point degeneracy bound.
    ConstantAtThreshold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AltitudeCrossingError {
    InvalidLatitude,
    InvalidDeclination,
    InvalidThresholdAltitude,
    NonFiniteGeometry,
}

impl fmt::Display for AltitudeCrossingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLatitude => write!(f, "latitude must be in [-pi/2, pi/2] radians"),
            Self::InvalidDeclination => {
                write!(f, "declination must be in [-pi/2, pi/2] radians")
            }
            Self::InvalidThresholdAltitude => {
                write!(f, "altitude threshold must be in [-pi/2, pi/2] radians")
            }
            Self::NonFiniteGeometry => write!(f, "non-finite geometric altitude calculation"),
        }
    }
}
impl Error for AltitudeCrossingError {}

/// Solve the spherical altitude equation for the two local hour angles
/// at which a fixed-declination target reaches a geometric altitude.
///
/// `latitude_rad` is observer geodetic latitude (north positive);
/// `declination_rad` is equatorial declination in a consistent celestial
/// frame; `threshold_altitude_rad` is geometric center altitude, above
/// the local mathematical horizon (positive upward). All are radians in
/// [-π/2, π/2]. Input angle, frame and observer-model consistency is the
/// caller's responsibility.
///
/// The equation is
/// `cos(H) = (sin(h0) - sin(lat) sin(dec)) / (cos(lat) cos(dec))`.
/// Rise has negative H (east of meridian); set has positive H (west).
/// Polar and celestial-pole cases with vanishing diurnal amplitude
/// are explicitly classified rather than divided by a near-zero value.
/// The near-pole classification threshold is 32*f64::EPSILON and is a
/// numerical guard, **not** a physical event-time accuracy guarantee.
///
/// No apparent, refracted or topocentric rise/set times are returned;
/// fast-moving bodies such as the Moon require repeated ephemeris states
/// and an independent event solver.
pub fn geometric_altitude_hour_angles(
    latitude_rad: f64,
    declination_rad: f64,
    threshold_altitude_rad: f64,
) -> Result<AltitudeCrossings, AltitudeCrossingError> {
    if !latitude_rad.is_finite() || !(-FRAC_PI_2..=FRAC_PI_2).contains(&latitude_rad) {
        return Err(AltitudeCrossingError::InvalidLatitude);
    }
    if !declination_rad.is_finite() || !(-FRAC_PI_2..=FRAC_PI_2).contains(&declination_rad) {
        return Err(AltitudeCrossingError::InvalidDeclination);
    }
    if !threshold_altitude_rad.is_finite()
        || !(-FRAC_PI_2..=FRAC_PI_2).contains(&threshold_altitude_rad)
    {
        return Err(AltitudeCrossingError::InvalidThresholdAltitude);
    }

    let base = latitude_rad.sin() * declination_rad.sin();
    let amplitude = latitude_rad.cos() * declination_rad.cos();
    let threshold = threshold_altitude_rad.sin();
    if [base, amplitude, threshold].iter().any(|x| !x.is_finite()) {
        return Err(AltitudeCrossingError::NonFiniteGeometry);
    }

    const POLE_TOL: f64 = 32.0 * f64::EPSILON;
    if amplitude.abs() <= POLE_TOL {
        return Ok(if (threshold - base).abs() <= POLE_TOL {
            AltitudeCrossings::ConstantAtThreshold
        } else if base > threshold {
            AltitudeCrossings::AlwaysAbove
        } else {
            AltitudeCrossings::AlwaysBelow
        });
    }

    let minimum = base - amplitude;
    let maximum = base + amplitude;
    if threshold < minimum {
        return Ok(AltitudeCrossings::AlwaysAbove);
    }
    if threshold > maximum {
        return Ok(AltitudeCrossings::AlwaysBelow);
    }
    if threshold == maximum {
        return Ok(AltitudeCrossings::TangentAtUpperMeridian);
    }
    if threshold == minimum {
        return Ok(AltitudeCrossings::TangentAtLowerMeridian);
    }

    // The strict range test above establishes -1 < cos(H) < 1 in exact
    // arithmetic; clamp only against representational rounding.
    let cosine_hour_angle = ((threshold - base) / amplitude).clamp(-1.0, 1.0);
    let h = cosine_hour_angle.acos();
    if !h.is_finite() {
        return Err(AltitudeCrossingError::NonFiniteGeometry);
    }
    // ±π and zero are tangencies, not two distinguishable crossings.
    if h == 0.0 {
        return Ok(AltitudeCrossings::TangentAtUpperMeridian);
    }
    if h == PI {
        return Ok(AltitudeCrossings::TangentAtLowerMeridian);
    }
    Ok(AltitudeCrossings::Two {
        rising_hour_angle_rad: -h,
        setting_hour_angle_rad: h,
    })
}
