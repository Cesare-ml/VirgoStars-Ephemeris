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

//! IERS celestial intermediate pole (CIP) and origin (CIO) orientation.
//!
//! Pure orthogonal geometry: given externally supplied CIP X,Y and CIO
//! locator s, form a GCRS -> CIRS matrix and its inverse. This does not
//! calculate the IAU precession-nutation series, X/Y/s at any TT epoch,
//! their derivatives, polar motion, UT1/ERA or a planetary ephemeris.
//!
//! Derived from the geometric requirements that the CIP (X,Y,Z) become
//! the intermediate +Z axis and that s locate the CIO on its equator.
//! A published independent ERFA c2ixys numerical fixture is used in tests,
//! without incorporating SOFA/ERFA implementation code.
//!
//! Standards: <https://iers-conventions.obspm.fr/conventions_material.php>.
//! Independent test values: <https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c>.
//!
//! Created 2026-11-12. Original CPAL-1.0 VirgoStars implementation.

use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

/// Matrix multiplying column positions in GCRS into CIRS, row-major.
///
/// This is the **celestial-to-intermediate** Q(t)^T orientation in the
/// IERS CIO convention, not the full celestial-to-terrestrial matrix.
pub type GcrsToCirsMatrix = [[f64; 3]; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CioOrientationError {
    /// CIP X,Y must be finite unit-pole components with X²+Y² < 1.
    InvalidCipCoordinates,
    /// CIO locator must be finite radians in [-π, +π].
    InvalidCioLocator,
    /// Input Cartesian coordinates must be finite in kilometers.
    NonFinitePosition,
    /// A computed matrix element or transformed coordinate is non-finite.
    NonFiniteResult,
}

impl fmt::Display for CioOrientationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCipCoordinates => {
                write!(f, "CIP X,Y must be finite with X^2 + Y^2 < 1")
            }
            Self::InvalidCioLocator => write!(f, "CIO locator s must be in [-pi, pi] radians"),
            Self::NonFinitePosition => write!(f, "GCRS/CIRS position must be finite in kilometers"),
            Self::NonFiniteResult => write!(f, "non-finite CIO orientation result"),
        }
    }
}

impl Error for CioOrientationError {}

fn finite_vector(vector: [f64; 3]) -> bool {
    vector.iter().all(|coordinate| coordinate.is_finite())
}

/// Build the orthogonal matrix mapping GCRS Cartesian vectors into CIRS.
///
/// `cip_x` and `cip_y` are the **dimensionless GCRS Cartesian**
/// components of the unit Celestial Intermediate Pole, not independent
/// spherical longitude/latitude angles. `Z = sqrt(1-X²-Y²)` is taken
/// **positive** for the IERS conventional north-pole solution; require
/// finite X²+Y² < 1. The `cio_s_rad` locator is a finite radian angle
/// in [-π,π], supplied from a consistent external TT-based CIO model.
///
/// Construct a minimal right-handed orthogonal alignment of CIP with
/// intermediate +Z; with `a = 1/(1+Z)`, its rows are:
///
/// `B0 = [1-aX², -aXY, -X]`
/// `B1 = [-aXY, 1-aY², -Y]`
/// `B2 = [X, Y, Z]`.
///
/// The CIO equator direction rotates these first two rows by `s`:
///
/// `C0 = cos(s) B0 - sin(s) B1`
/// `C1 = sin(s) B0 + cos(s) B1`
/// `C2 = B2`.
///
/// `C` is the GCRS→CIRS matrix in the CIO convention. The inverse
/// CIRS→GCRS matrix is `C^T`; no time-dependent velocity derivative
/// is computed or implied. The IERS transformation from CIRS to
/// terrestrial also needs ERA(UT1) and polar motion W(t).
///
/// This module does not derive or validate X/Y/s for a given epoch
/// and does not implement IAU 2000A/2006 series, EOP/ΔT, BCRS/GCRS
/// origins, CIP time derivatives or DE441-to-CIRS state rates.
pub fn gcrs_to_cirs_cio_matrix(
    cip_x: f64,
    cip_y: f64,
    cio_s_rad: f64,
) -> Result<GcrsToCirsMatrix, CioOrientationError> {
    let r2 = cip_x * cip_x + cip_y * cip_y;
    if !cip_x.is_finite() || !cip_y.is_finite() || !r2.is_finite() || r2 >= 1.0 {
        return Err(CioOrientationError::InvalidCipCoordinates);
    }
    if !cio_s_rad.is_finite() || !(-PI..=PI).contains(&cio_s_rad) {
        return Err(CioOrientationError::InvalidCioLocator);
    }

    let z = (1.0 - r2).sqrt();
    let a = 1.0 / (1.0 + z);
    let b0 = [1.0 - a * cip_x * cip_x, -a * cip_x * cip_y, -cip_x];
    let b1 = [-a * cip_x * cip_y, 1.0 - a * cip_y * cip_y, -cip_y];
    let b2 = [cip_x, cip_y, z];
    let (sin_s, cos_s) = cio_s_rad.sin_cos();
    let matrix = [
        std::array::from_fn(|i| cos_s * b0[i] - sin_s * b1[i]),
        std::array::from_fn(|i| sin_s * b0[i] + cos_s * b1[i]),
        b2,
    ];
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(CioOrientationError::NonFiniteResult);
    }
    Ok(matrix)
}

/// Rotate a GCRS position into the CIO-based CIRS intermediate frame.
///
/// `position_gcrs_km` is an Earth-centered GCRS position in kilometers,
/// **not** a raw barycentric DE441 state unless the requisite observer,
/// origin, time-scale and frame conventions have been separately handled.
pub fn gcrs_to_cirs_position(
    position_gcrs_km: [f64; 3],
    cip_x: f64,
    cip_y: f64,
    cio_s_rad: f64,
) -> Result<[f64; 3], CioOrientationError> {
    if !finite_vector(position_gcrs_km) {
        return Err(CioOrientationError::NonFinitePosition);
    }
    let matrix = gcrs_to_cirs_cio_matrix(cip_x, cip_y, cio_s_rad)?;
    let rotated = matrix.map(|row| {
        row.iter()
            .zip(position_gcrs_km)
            .map(|(component, coordinate)| component * coordinate)
            .sum()
    });
    if !finite_vector(rotated) {
        return Err(CioOrientationError::NonFiniteResult);
    }
    Ok(rotated)
}

/// Rotate a CIRS position into GCRS via the transpose of the CIO matrix.
///
/// This reverses **only** the celestial-intermediate positional rotation.
/// In particular it is not the derivative of the time-dependent matrix,
/// a conversion from UT1 to TT, or a full topocentric velocity transform.
pub fn cirs_to_gcrs_position(
    position_cirs_km: [f64; 3],
    cip_x: f64,
    cip_y: f64,
    cio_s_rad: f64,
) -> Result<[f64; 3], CioOrientationError> {
    if !finite_vector(position_cirs_km) {
        return Err(CioOrientationError::NonFinitePosition);
    }
    let matrix = gcrs_to_cirs_cio_matrix(cip_x, cip_y, cio_s_rad)?;
    let rotated = std::array::from_fn(|j| {
        (0..3)
            .map(|i| matrix[i][j] * position_cirs_km[i])
            .sum()
    });
    if !finite_vector(rotated) {
        return Err(CioOrientationError::NonFiniteResult);
    }
    Ok(rotated)
}
