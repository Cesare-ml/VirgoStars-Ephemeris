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

//! Pure unit conversion of a **fixed-axis** spherical state and its rates.
//!
//! This adapter performs NO astronomical correction, reference-frame
//! rotation, interpolation or time-scale conversion. The six fields use
//! the same scalar units as common ephemeris consumers, but are NOT
//! `swe_calc_ut` apparent true-ecliptic-of-date results. An eventual
//! independent PR #70 integration should reuse this conversion instead
//! of maintaining a second inline rad/km formatter.

use std::error::Error;
use std::f64::consts::{PI, TAU};
use std::fmt;

use crate::coordinates::SphericalState;
use crate::de441::{Body, JulianDateTdb};

/// Exact IAU 2012 B2 astronomical unit, defined as 149,597,870,700 m.
pub const IAU_AU_KM: f64 = 149_597_870.7;
const DEG_PER_RAD: f64 = 180.0 / PI;

/// Fixed-frame six-field result in degrees/AU and TDB-day derivatives.
///
/// **NOT Swiss-compatible apparent coordinates**: the input's frame,
/// center, epoch and applied physical correction set are unchanged.
/// `fixed_tilt_rad` is a *constant* passive X tilt, not an official
/// mean/true ecliptic of date. `target` may be a SYSTEM BARYCENTER.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedSixFieldsDegreesAu {
    pub target: Body,
    pub center: Body,
    pub epoch: JulianDateTdb,
    pub fixed_tilt_rad: f64,
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub distance_au: f64,
    pub longitude_speed_deg_per_tdb_day: f64,
    pub latitude_speed_deg_per_tdb_day: f64,
    pub distance_speed_au_per_tdb_day: f64,
}

/// Malformed or unrepresentable **source** six-field values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitConversionError {
    NonFinite,
    InvalidLongitude,
    InvalidLatitude,
    NonPositiveDistance,
    Overflow,
}

impl fmt::Display for UnitConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => write!(f, "nonfinite fixed spherical input"),
            Self::InvalidLongitude => write!(f, "longitude outside [0,2pi)"),
            Self::InvalidLatitude => write!(f, "latitude outside [-pi/2,pi/2]"),
            Self::NonPositiveDistance => write!(f, "geocentric distance must be positive"),
            Self::Overflow => write!(f, "fixed spherical unit conversion overflow"),
        }
    }
}
impl Error for UnitConversionError {}

/// Convert an already computed `SphericalState` to the explicit
/// degrees/AU consumer units, without reapplying astronomy.
///
/// This accepts the mathematical fixed-frame `SphericalState` of
/// `coordinates::project_fixed_axes` and rejects NaNs, out-of-range
/// angles and undefined/non-positive distances. The three derivative
/// components are converted *linearly* from rad/km **per TDB day** to
/// deg/AU per TDB day; they retain their signed retrograde/radial sense.
/// No implicit TT/UTC conversion or body-ID substitution occurs.
///
/// The existing projector normalizes longitude into [0,2pi).
/// This function deliberately does not silently wrap arbitrary caller
/// angles: doing so could hide malformed external input.
pub fn fixed_spherical_to_degrees_au(
    spherical: &SphericalState,
) -> Result<FixedSixFieldsDegreesAu, UnitConversionError> {
    let v = [
        spherical.epoch.0,
        spherical.tilt_rad,
        spherical.longitude_rad,
        spherical.latitude_rad,
        spherical.distance_km,
        spherical.longitude_rate_rad_per_day,
        spherical.latitude_rate_rad_per_day,
        spherical.distance_rate_km_per_day,
    ];
    if v.iter().any(|x| !x.is_finite()) {
        return Err(UnitConversionError::NonFinite);
    }
    if !(0.0..TAU).contains(&spherical.longitude_rad) {
        return Err(UnitConversionError::InvalidLongitude);
    }
    if spherical.latitude_rad.abs() > PI / 2.0 {
        return Err(UnitConversionError::InvalidLatitude);
    }
    if spherical.distance_km <= 0.0 {
        return Err(UnitConversionError::NonPositiveDistance);
    }

    let longitude_deg = spherical.longitude_rad * DEG_PER_RAD;
    let latitude_deg = spherical.latitude_rad * DEG_PER_RAD;
    let distance_au = spherical.distance_km / IAU_AU_KM;
    let longitude_speed_deg_per_tdb_day = spherical.longitude_rate_rad_per_day * DEG_PER_RAD;
    let latitude_speed_deg_per_tdb_day = spherical.latitude_rate_rad_per_day * DEG_PER_RAD;
    let distance_speed_au_per_tdb_day = spherical.distance_rate_km_per_day / IAU_AU_KM;
    let output = [
        longitude_deg,
        latitude_deg,
        distance_au,
        longitude_speed_deg_per_tdb_day,
        latitude_speed_deg_per_tdb_day,
        distance_speed_au_per_tdb_day,
    ];
    if output.iter().any(|x| !x.is_finite()) || distance_au <= 0.0 {
        return Err(UnitConversionError::Overflow);
    }
    Ok(FixedSixFieldsDegreesAu {
        target: spherical.target,
        center: spherical.center,
        epoch: spherical.epoch,
        fixed_tilt_rad: spherical.tilt_rad,
        longitude_deg,
        latitude_deg,
        distance_au,
        longitude_speed_deg_per_tdb_day,
        latitude_speed_deg_per_tdb_day,
        distance_speed_au_per_tdb_day,
    })
}
