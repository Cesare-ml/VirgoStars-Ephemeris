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

//! IERS 2010 polar-motion position transform, ITRS to TIRS and back.
//!
//! IERS TN36 chapter 5, Eq. (5.3) defines the *passive* W(t) matrix:
//! W = R3(-s') R2(xp) R1(yp) mapping ITRS -> TIRS.
//! With the IERS rotation convention, R1(a) transforms
//! (y,z) -> (cos(a)*y+sin(a)*z, -sin(a)*y+cos(a)*z);
//! R2(a) transforms (x,z) -> (cos(a)*x-sin(a)*z, sin(a)*x+cos(a)*z);
//! R3(a) transforms (x,y) -> (cos(a)*x+sin(a)*y, -sin(a)*x+cos(a)*y).
//!
//! SOFA/ERFA eraPom00 provides the transpose: TIRS -> ITRS. The
//! independently published t_pom00 reference matrix is checked by tests.
//!
//! Source specifications, not copied implementations:
//! <https://iers-conventions.obspm.fr/conventions_material.php>
//! <https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c>
//!
//! Created 2026-11-12. Original VirgoStars Rust implementation.

use std::error::Error;
use std::fmt;

/// Safety domain for the supplied EOP polar coordinates and TIO locator.
///
/// Each input is in *radians*, bounded to ±0.01 rad (~0.57 degrees) to
/// guard against accidental arcsecond/degree input. This is an explicit
/// numerical API policy, NOT an IERS physical bound or EOP provider model.
pub const MAX_POLAR_MOTION_ANGLE_RAD: f64 = 0.01;

/// Passive column-vector ITRS -> TIRS orthogonal rotation.
pub type PolarMotionMatrix = [[f64; 3]; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolarMotionError {
    InvalidXp,
    InvalidYp,
    InvalidTioLocator,
    NonFinitePosition,
    NonFiniteResult,
}

impl fmt::Display for PolarMotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidXp => write!(f, "xp must be finite radians within ±0.01"),
            Self::InvalidYp => write!(f, "yp must be finite radians within ±0.01"),
            Self::InvalidTioLocator => {
                write!(f, "TIO locator s' must be finite radians within ±0.01")
            }
            Self::NonFinitePosition => write!(f, "ITRS/TIRS Cartesian input must be finite"),
            Self::NonFiniteResult => write!(f, "non-finite polar-motion transformation output"),
        }
    }
}

impl Error for PolarMotionError {}

fn angles_valid(xp_rad: f64, yp_rad: f64, sp_rad: f64) -> Result<(), PolarMotionError> {
    if !xp_rad.is_finite() || xp_rad.abs() > MAX_POLAR_MOTION_ANGLE_RAD {
        return Err(PolarMotionError::InvalidXp);
    }
    if !yp_rad.is_finite() || yp_rad.abs() > MAX_POLAR_MOTION_ANGLE_RAD {
        return Err(PolarMotionError::InvalidYp);
    }
    if !sp_rad.is_finite() || sp_rad.abs() > MAX_POLAR_MOTION_ANGLE_RAD {
        return Err(PolarMotionError::InvalidTioLocator);
    }
    Ok(())
}

/// IERS Eq. (5.3) orthogonal polar-motion matrix, mapping ITRS -> TIRS.
///
/// Inputs are finite xp, yp, s' in *radians*, within the documented
/// numerical guard. xp and yp must come from an explicit Earth-orientation
/// data source, and s' is the supplied TIO locator at the same epoch.
/// No EOP file is opened, no s'(TT) approximation is assumed, and no
/// time-scale conversion or interpolation is performed here.
///
/// Returned matrix multiplies column vectors in Earth-fixed kilometers
/// (or in any other consistent length unit). SOFA/ERFA `eraPom00`
/// returns the *inverse* TIRS -> ITRS orientation, so its published
/// t_pom00 fixture must be transposed when comparing with this matrix.
///
/// This step does not include ERA (TIRS -> CIRS), Q(t) (CIRS -> GCRS),
/// precession, nutation, spin rates, plate velocity, or DE441 geometry.
pub fn itrs_to_tirs_polar_motion_matrix(
    xp_rad: f64,
    yp_rad: f64,
    sp_rad: f64,
) -> Result<PolarMotionMatrix, PolarMotionError> {
    angles_valid(xp_rad, yp_rad, sp_rad)?;

    // Apply the rightmost passive R1(yp), then R2(xp), then R3(-sp).
    // The resulting W is the transpose of SOFA's eraPom00 RPOM.
    let (sy, cy) = yp_rad.sin_cos();
    let (sx, cx) = xp_rad.sin_cos();
    let (ss, cs) = sp_rad.sin_cos();

    let matrix = [
        [cs * cx, cs * sx * sy - ss * cy, -cs * sx * cy - ss * sy],
        [ss * cx, ss * sx * sy + cs * cy, -ss * sx * cy + cs * sy],
        [sx, -cx * sy, cx * cy],
    ];
    if matrix.iter().flatten().any(|v| !v.is_finite()) {
        return Err(PolarMotionError::NonFiniteResult);
    }
    Ok(matrix)
}

/// Position-only ITRS -> TIRS polar motion for an ITRS position in km.
///
/// Do not feed the result directly into ICRF or DE441 without the
/// separately modeled ERA and celestial-pole orientation. Velocity
/// requires differentiated EOP parameters and is **not** computed.
pub fn itrs_to_tirs_position(
    position_itrs_km: [f64; 3],
    xp_rad: f64,
    yp_rad: f64,
    sp_rad: f64,
) -> Result<[f64; 3], PolarMotionError> {
    if position_itrs_km.iter().any(|v| !v.is_finite()) {
        return Err(PolarMotionError::NonFinitePosition);
    }
    let w = itrs_to_tirs_polar_motion_matrix(xp_rad, yp_rad, sp_rad)?;
    let transformed = w.map(|row| row.iter().zip(position_itrs_km).map(|(m, v)| m * v).sum());
    if transformed.iter().any(|v| !v.is_finite()) {
        return Err(PolarMotionError::NonFiniteResult);
    }
    Ok(transformed)
}

/// Inverse *position-only* TIRS -> ITRS transform using W(t) transpose.
///
/// The same xp, yp and s' must be supplied as for the forward transform.
pub fn tirs_to_itrs_position(
    position_tirs_km: [f64; 3],
    xp_rad: f64,
    yp_rad: f64,
    sp_rad: f64,
) -> Result<[f64; 3], PolarMotionError> {
    if position_tirs_km.iter().any(|v| !v.is_finite()) {
        return Err(PolarMotionError::NonFinitePosition);
    }
    let w = itrs_to_tirs_polar_motion_matrix(xp_rad, yp_rad, sp_rad)?;
    let transformed = std::array::from_fn(|j| (0..3).map(|i| w[i][j] * position_tirs_km[i]).sum());
    if transformed.iter().any(|v| !v.is_finite()) {
        return Err(PolarMotionError::NonFiniteResult);
    }
    Ok(transformed)
}
