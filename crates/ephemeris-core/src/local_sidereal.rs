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

//! Local **mean** sidereal angle and hour-angle geometry (radians).
//!
//! This module composes an already calculated Greenwich *mean* sidereal time
//! with an east-positive terrestrial longitude. It does not compute ERA,
//! precession, nutation, GMST, UT1, TT or any time-scale conversion.
//!
//! The angles are mean-equator/equinox-of-date quantities; combining these
//! outputs with apparent right ascension or GAST without a matched model
//! would be physically inconsistent.
//!
//! Created 2026-11-11. Original elementary spherical-angle implementation.

use std::error::Error;
use std::f64::consts::{PI, TAU};
use std::fmt;

/// Input validation failures for normalized mean sidereal geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalSiderealError {
    InvalidGreenwichMeanSiderealAngle,
    InvalidObserverLongitude,
    InvalidLocalMeanSiderealAngle,
    InvalidMeanRightAscension,
}

impl fmt::Display for LocalSiderealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGreenwichMeanSiderealAngle => {
                write!(f, "Greenwich mean sidereal angle must be in [0, 2pi) radians")
            }
            Self::InvalidObserverLongitude => {
                write!(f, "observer longitude must be finite and in [-180, 180] degrees east")
            }
            Self::InvalidLocalMeanSiderealAngle => {
                write!(f, "local mean sidereal angle must be in [0, 2pi) radians")
            }
            Self::InvalidMeanRightAscension => {
                write!(f, "mean-of-date right ascension must be in [0, 2pi) radians")
            }
        }
    }
}
impl Error for LocalSiderealError {}

/// Rotate Greenwich *mean* sidereal time to an observer's local meridian.
///
/// Input `gmst_rad` must be an already-calculated, finite GMST angle in
/// [0, 2π), derived consistently from UT1 and TT (e.g. IAU 2006).
/// `longitude_deg_east` is geodetic longitude, east-positive, in the
/// closed interval [-180°, +180°]. North/south latitude and height are
/// irrelevant to this meridian-angle transformation.
///
/// Output is a local **mean** sidereal angle in [0, 2π) radians.
/// The east-positive convention is **addition**: LMST = GMST + longitude.
/// This is not local apparent sidereal time, ARMC, a house cusp or an
/// Earth orientation model. No UTC/UT1/TT conversion is performed.
pub fn local_mean_sidereal_time(
    gmst_rad: f64,
    longitude_deg_east: f64,
) -> Result<f64, LocalSiderealError> {
    if !gmst_rad.is_finite() || !(0.0..TAU).contains(&gmst_rad) {
        return Err(LocalSiderealError::InvalidGreenwichMeanSiderealAngle);
    }
    if !longitude_deg_east.is_finite()
        || !(-180.0..=180.0).contains(&longitude_deg_east)
    {
        return Err(LocalSiderealError::InvalidObserverLongitude);
    }
    Ok((gmst_rad + longitude_deg_east.to_radians()).rem_euclid(TAU))
}

/// Signed local mean hour angle for a mean-equator/equinox-of-date direction.
///
/// Both `local_mean_sidereal_rad` and `mean_right_ascension_rad` must
/// be finite normalized angles in [0, 2π), expressed in the **same mean
/// equinox-of-date reference system**. The signed result is in [-π, π)
/// radians and is **positive to the west**, corresponding to the
/// astronomical hour-angle convention H = LMST − RA.
///
/// A mean RA cannot be mixed silently with GAST or apparent RA; no such
/// transformation is performed. The exact 180° opposition maps to −π.
pub fn local_mean_hour_angle(
    local_mean_sidereal_rad: f64,
    mean_right_ascension_rad: f64,
) -> Result<f64, LocalSiderealError> {
    if !local_mean_sidereal_rad.is_finite()
        || !(0.0..TAU).contains(&local_mean_sidereal_rad)
    {
        return Err(LocalSiderealError::InvalidLocalMeanSiderealAngle);
    }
    if !mean_right_ascension_rad.is_finite()
        || !(0.0..TAU).contains(&mean_right_ascension_rad)
    {
        return Err(LocalSiderealError::InvalidMeanRightAscension);
    }

    let difference = (local_mean_sidereal_rad - mean_right_ascension_rad).rem_euclid(TAU);
    Ok(if difference >= PI {
        difference - TAU
    } else {
        difference
    })
}
