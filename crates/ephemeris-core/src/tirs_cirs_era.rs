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

//! Earth-rotation-only mapping from TIRS coordinates to CIRS coordinates.
//!
//! According to IERS Conventions (2010) §5.4.2, the transformation from
//! Terrestrial Intermediate Reference System (TIRS) to Celestial Intermediate
//! Reference System (CIRS) is R3(-ERA) in the IERS/SOFA passive convention.
//! For column vectors with the usual counter-clockwise active rotation,
//! this is (x',y') = (cos(ERA)*x - sin(ERA)*y,
//!                  sin(ERA)*x + cos(ERA)*y).
//!
//! A WGS84 Earth-fixed position is **not automatically TIRS**: before using
//! this API with an ITRS/ECEF site, the caller must handle the polar-motion
//! transformation W(t), including the TIO locator s'. Similarly, CIRS is
//! not GCRS/ICRF or an equinox-of-date frame. The Q(t) celestial-pole
//! orientation and precession/nutation stages are NOT applied here.
//!
//! References:
//! <https://iers-conventions.obspm.fr/conventions_material.php>
//! <https://juliaastro.org/SOFA/stable/api/precession/>
//!
//! Created 2026-11-12. Independent original implementation, no IERS,
//! SOFA/ERFA or Swiss source code copied.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;

/// CIRS position and its **Earth-spin-only** derivative.
///
/// Position is in kilometers. The rate is kilometers per **UT1 day**.
/// It is the derivative for an ideal point held fixed in TIRS as ERA
/// changes: it does NOT include polar motion, non-rigid site motion,
/// plate drift, precession/nutation, or the time-dependent Q(t) matrix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CirsEarthSpinState {
    pub position_km: [f64; 3],
    pub spin_velocity_km_per_ut1_day: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TirsCirsError {
    NonFinitePosition,
    InvalidEarthRotationAngle,
    InvalidEarthRotationRate,
    NonFiniteResult,
}

impl fmt::Display for TirsCirsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePosition => write!(f, "TIRS/CIRS position must be finite in kilometers"),
            Self::InvalidEarthRotationAngle => {
                write!(f, "IAU Earth Rotation Angle must be in [0, 2pi) radians")
            }
            Self::InvalidEarthRotationRate => {
                write!(
                    f,
                    "Earth rotation rate must be positive and finite in rad/UT1 day"
                )
            }
            Self::NonFiniteResult => write!(f, "non-finite Earth rotation output"),
        }
    }
}
impl Error for TirsCirsError {}

fn valid_position(position: [f64; 3]) -> bool {
    position.iter().all(|value| value.is_finite())
}

fn valid_era(era_rad: f64) -> bool {
    era_rad.is_finite() && (0.0..TAU).contains(&era_rad)
}

/// Rotate the **TIRS** position of a point held fixed in TIRS into CIRS.
///
/// The ERA input must be the IAU 2000 Earth Rotation Angle at a specified
/// UT1 epoch (radians in [0, 2π)). The caller supplies its positive rate
/// in rad/UT1 day; the original IAU 2000 ERA rate can be obtained from
/// the independently reviewed ERA primitive (draft PR #75).
///
/// The output is *not* a full geocentric inertial position/velocity:
/// TIRS must already have been obtained from ITRS polar motion, and the
/// CIRS-to-GCRS Q(t) rotation remains outstanding. The returned derivative
/// assumes the TIRS point is frozen at the supplied epoch.
/// This function performs no clock/time-scale conversions.
pub fn tirs_to_cirs_earth_spin(
    position_tirs_km: [f64; 3],
    era_rad: f64,
    era_rate_rad_per_ut1_day: f64,
) -> Result<CirsEarthSpinState, TirsCirsError> {
    if !valid_position(position_tirs_km) {
        return Err(TirsCirsError::NonFinitePosition);
    }
    if !valid_era(era_rad) {
        return Err(TirsCirsError::InvalidEarthRotationAngle);
    }
    if !era_rate_rad_per_ut1_day.is_finite() || era_rate_rad_per_ut1_day <= 0.0 {
        return Err(TirsCirsError::InvalidEarthRotationRate);
    }

    let [x, y, z] = position_tirs_km;
    let (sine, cosine) = era_rad.sin_cos();
    let x_cirs = cosine * x - sine * y;
    let y_cirs = sine * x + cosine * y;
    let result = CirsEarthSpinState {
        position_km: [x_cirs, y_cirs, z],
        spin_velocity_km_per_ut1_day: [
            -era_rate_rad_per_ut1_day * y_cirs,
            era_rate_rad_per_ut1_day * x_cirs,
            0.0,
        ],
    };
    if !valid_position(result.position_km) || !valid_position(result.spin_velocity_km_per_ut1_day) {
        return Err(TirsCirsError::NonFiniteResult);
    }
    Ok(result)
}

/// Inverse *position-only* rotation, CIRS back to TIRS at the same ERA.
///
/// This reverses only the Earth's R3(-ERA) intermediate-frame step.
/// It neither reverses polar motion nor maps into the original WGS84
/// Earth-fixed axes without the separate ITRS↔TIRS transform.
/// Velocities cannot be inverted by position rotation alone.
pub fn cirs_to_tirs_position(
    position_cirs_km: [f64; 3],
    era_rad: f64,
) -> Result<[f64; 3], TirsCirsError> {
    if !valid_position(position_cirs_km) {
        return Err(TirsCirsError::NonFinitePosition);
    }
    if !valid_era(era_rad) {
        return Err(TirsCirsError::InvalidEarthRotationAngle);
    }

    let [x, y, z] = position_cirs_km;
    let (sine, cosine) = era_rad.sin_cos();
    let position_tirs = [cosine * x + sine * y, -sine * x + cosine * y, z];
    if !valid_position(position_tirs) {
        return Err(TirsCirsError::NonFiniteResult);
    }
    Ok(position_tirs)
}
