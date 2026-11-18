/*
EXHIBIT A. Common Public Attribution License Version 1.0.

The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

//! Deliberately narrow **geometric** geocentric ICRF -> true ecliptic of date.
//!
//! This reuses the existing IAU 2006 bias/precession and IAU 2000B nutation
//! orientation matrix. It does *not* compute light-time, annual aberration,
//! gravitational deflection, topocentric parallax or apparent longitude.
//! No longitude/latitude speeds are returned: moving-frame derivatives also
//! require `dR/dt * position`, which is not implemented by this adapter.
//!
//! The caller supplies TWO independently qualified epochs for the same
//! physical instant: DE441 Julian Date **TDB** and orientation Julian Date
//! **TT**. No UTC, TT <-> TDB or historical Delta-T conversion occurs here.

use std::{error::Error, f64::consts::TAU, fmt};

use crate::de441::{Body, JulianDateTdb, StateVector};
use crate::de441_pack_manager::{De441PackError, De441PackManager};
use crate::earth_orientation::{
    true_ecliptic_of_date_matrix, OrientationError,
};
use crate::six_field_units::IAU_AU_KM;

pub const ORIENTATION_MODEL: &str = "IAU_2006_BIAS_PRECESSION_IAU_2000B_NUTATION";

/// A geometric direction rotated into true ecliptic/equinox axes of date.
///
/// The frame is true ecliptic of **TT** date, position geometry is at **TDB**;
/// the target is relative to Earth's center, not a surface observer.
/// `target` may identify a planetary-system barycenter, not the planet center.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometricTrueEclipticOfDate {
    pub target: Body,
    pub center: Body,
    pub epoch_tdb: JulianDateTdb,
    pub orientation_epoch_tt_jd: f64,
    pub orientation_model: &'static str,
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub distance_km: f64,
    pub distance_au: f64,
}

#[derive(Debug)]
pub enum GeometricDateError {
    /// This adapter only accepts target-vs-Earth states with a direction.
    InvalidGeocentricState,
    /// A non-finite component, date, transformed coordinate or output.
    NonFinite,
    /// At the origin or a longitude pole, longitude is undefined.
    UndefinedLongitude,
    Orientation(OrientationError),
    Ephemeris(De441PackError),
}

impl fmt::Display for GeometricDateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGeocentricState => write!(f, "expected a target relative to Earth's center"),
            Self::NonFinite => write!(f, "nonfinite geometric ecliptic-of-date input or output"),
            Self::UndefinedLongitude => write!(f, "geocentric direction or longitude undefined"),
            Self::Orientation(error) => write!(f, "date-frame orientation: {error}"),
            Self::Ephemeris(error) => write!(f, "DE441 pack state: {error}"),
        }
    }
}

impl Error for GeometricDateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Orientation(error) => Some(error),
            Self::Ephemeris(error) => Some(error),
            _ => None,
        }
    }
}

/// Project a supplied **geometric ICRF** DE441 state into true ecliptic of date.
///
/// No ephemeris file I/O occurs. In particular, rotating a geometric vector
/// does NOT turn it into an astrometric or apparent one. Orientation is bounded
/// by the existing Rust IAU2000B model's 1900–2100 TT implementation range.
pub fn project_geocentric_geometric_state(
    state: &StateVector,
    orientation_epoch_tt_jd: f64,
) -> Result<GeometricTrueEclipticOfDate, GeometricDateError> {
    if state.center != Body::Earth || state.target == Body::Earth {
        return Err(GeometricDateError::InvalidGeocentricState);
    }
    if !state.epoch.0.is_finite() ||
        state.position_km.iter().chain(state.velocity_km_per_day.iter()).any(|x| !x.is_finite())
    {
        return Err(GeometricDateError::NonFinite);
    }
    let matrix = true_ecliptic_of_date_matrix(orientation_epoch_tt_jd)
        .map_err(GeometricDateError::Orientation)?;
    let position: [f64; 3] = std::array::from_fn(|row| {
        matrix[row].iter().zip(state.position_km).map(|(m, x)| (*m) * x).sum::<f64>()
    });
    if position.iter().any(|x| !x.is_finite()) {
        return Err(GeometricDateError::NonFinite);
    }
    let [x, y, z] = position;
    let planar = x.hypot(y);
    let distance_km = planar.hypot(z);
    if !planar.is_finite() || !distance_km.is_finite() {
        return Err(GeometricDateError::NonFinite);
    }
    if planar == 0.0 || distance_km == 0.0 {
        return Err(GeometricDateError::UndefinedLongitude);
    }

    let longitude_deg = y.atan2(x).rem_euclid(TAU).to_degrees();
    let latitude_deg = z.atan2(planar).to_degrees();
    let distance_au = distance_km / IAU_AU_KM;
    if [longitude_deg, latitude_deg, distance_au].iter().any(|x| !x.is_finite()) ||
        distance_au <= 0.0
    {
        return Err(GeometricDateError::NonFinite);
    }

    Ok(GeometricTrueEclipticOfDate {
        target: state.target,
        center: state.center,
        epoch_tdb: state.epoch,
        orientation_epoch_tt_jd,
        orientation_model: ORIENTATION_MODEL,
        longitude_deg,
        latitude_deg,
        distance_km,
        distance_au,
    })
}

/// Read the pack selected by the *TDB* epoch, then project into a separately
/// supplied *TT* date frame. Missing/corrupt packs never cause a fallback.
pub fn geocentric_geometric_true_ecliptic_of_date(
    manager: &mut De441PackManager,
    target: Body,
    epoch_tdb: JulianDateTdb,
    orientation_epoch_tt_jd: f64,
) -> Result<GeometricTrueEclipticOfDate, GeometricDateError> {
    // Validate the deterministic orientation/target contract before file I/O.
    if target == Body::Earth {
        return Err(GeometricDateError::InvalidGeocentricState);
    }
    true_ecliptic_of_date_matrix(orientation_epoch_tt_jd)
        .map_err(GeometricDateError::Orientation)?;
    let state = manager.state(target, Body::Earth, epoch_tdb)
        .map_err(GeometricDateError::Ephemeris)?;
    project_geocentric_geometric_state(&state, orientation_epoch_tt_jd)
}
