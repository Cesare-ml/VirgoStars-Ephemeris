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

//! Compose already-supplied IERS intermediate-frame orientation matrices.
//!
//! This module assembles the ITRS <-> GCRS *position-only* rotation
//! from a supplied ITRS->TIRS W matrix, a UT1 Earth Rotation Angle,
//! and a supplied GCRS->CIRS celestial-intermediate C matrix.
//!
//! IERS / SOFA column-vector convention:
//!     p_ITRS = W^T * Rz(-ERA) * C * p_GCRS,
//! where Rz(-ERA) is the ordinary active column-vector rotation.
//! This is the same convention as celestial-to-terrestrial RC2T in
//! the independent ERFA c2tcio test fixture.
//!
//! W, ERA and C MUST have been produced for the same physical epoch
//! using consistent ITRS, TIRS, CIRS, and GCRS realizations.
//! This code does NOT calculate EOP, CIP, CIO locator, time scales,
//! Earth rotation, observer velocity, or a complete DE441 pipeline.
//!
//! Created 2026-11-12. Original code; third-party numerical reference
//! values are used in independent tests, not third-party source code.
//! Reference: <https://iers-conventions.obspm.fr/conventions_material.php>.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;

/// Row-major proper rotation mapping column vectors in GCRS to ITRS.
pub type GcrsToItrsMatrix = [[f64; 3]; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrientationChainError {
    InvalidPolarMotionMatrix,
    InvalidCioOrientationMatrix,
    InvalidEarthRotationAngle,
    InvalidPosition,
    NonFiniteResult,
}

impl fmt::Display for OrientationChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolarMotionMatrix => {
                write!(
                    f,
                    "ITRS-to-TIRS W must be a finite proper orthogonal 3x3 matrix"
                )
            }
            Self::InvalidCioOrientationMatrix => {
                write!(
                    f,
                    "GCRS-to-CIRS C must be a finite proper orthogonal 3x3 matrix"
                )
            }
            Self::InvalidEarthRotationAngle => {
                write!(f, "Earth Rotation Angle must be finite in [0, 2pi) radians")
            }
            Self::InvalidPosition => write!(f, "GCRS/ITRS Cartesian position must be finite"),
            Self::NonFiniteResult => write!(f, "non-finite orientation-chain output"),
        }
    }
}

impl Error for OrientationChainError {}

const ORTHOGONALITY_TOLERANCE: f64 = 1.0e-10;

fn proper_rotation(matrix: &[[f64; 3]; 3]) -> bool {
    if matrix.iter().flatten().any(|x| !x.is_finite()) {
        return false;
    }
    for (i, row) in matrix.iter().enumerate() {
        for (j, other) in matrix.iter().enumerate() {
            let dot = row.iter().zip(other).map(|(a, b)| a * b).sum::<f64>();
            let expected = if i == j { 1.0 } else { 0.0 };
            if (dot - expected).abs() > ORTHOGONALITY_TOLERANCE {
                return false;
            }
        }
    }
    let m = matrix;
    let determinant = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    (determinant - 1.0).abs() <= ORTHOGONALITY_TOLERANCE
}

fn multiply(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

fn finite_vector(vector: &[f64; 3]) -> bool {
    vector.iter().all(|value| value.is_finite())
}

/// Fixed-epoch IERS rotation matrix and its position-only inverse.
///
/// Units of any transformed input position are preserved: both public
/// vector methods specify kilometers. No rotational or translational
/// velocity is provided. A full moving-frame state requires dW/dt,
/// dC/dt, dERA/dt and a declared time scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComposedOrientation {
    /// GCRS -> ITRS rotation, to be applied to column position vectors.
    pub gcrs_to_itrs: GcrsToItrsMatrix,
}

impl ComposedOrientation {
    /// Rotate an Earth-centered GCRS position (kilometers) into ITRS.
    pub fn gcrs_to_itrs_position(
        &self,
        position_gcrs_km: [f64; 3],
    ) -> Result<[f64; 3], OrientationChainError> {
        if !finite_vector(&position_gcrs_km) {
            return Err(OrientationChainError::InvalidPosition);
        }
        let out: [f64; 3] = self
            .gcrs_to_itrs
            .map(|row| row.iter().zip(position_gcrs_km).map(|(a, b)| a * b).sum());
        if !finite_vector(&out) {
            return Err(OrientationChainError::NonFiniteResult);
        }
        Ok(out)
    }

    /// Rotate an ITRS position (kilometers) into GCRS by matrix transpose.
    pub fn itrs_to_gcrs_position(
        &self,
        position_itrs_km: [f64; 3],
    ) -> Result<[f64; 3], OrientationChainError> {
        if !finite_vector(&position_itrs_km) {
            return Err(OrientationChainError::InvalidPosition);
        }
        let out: [f64; 3] = std::array::from_fn(|j| {
            (0..3)
                .map(|i| self.gcrs_to_itrs[i][j] * position_itrs_km[i])
                .sum()
        });
        if !finite_vector(&out) {
            return Err(OrientationChainError::NonFiniteResult);
        }
        Ok(out)
    }
}

/// Compose the complete **position-only** intermediate-frame rotation.
///
/// `itrs_to_tirs_w` is W(t), ITRS -> TIRS (NOT SOFA `rpom`,
/// which is its transpose). See the independent polar-motion PR #84.
/// `era_rad` is IAU 2000 Earth Rotation Angle in [0, 2π), **UT1**.
/// `gcrs_to_cirs_c` is the C2I orientation matrix, GCRS -> CIRS,
/// supplied by a matching **TT-based** CIP/CIO X,Y,s model (PR #86).
///
/// All matrices are row-major, multiplying Cartesian column vectors.
/// Both input matrices must be finite, proper, right-handed, and
/// orthogonal within a strict 1e-10 unitless guard. This validation
/// cannot establish their physical frame or epoch provenance.
///
/// No assumption of zero polar motion, zero CIO locator or
/// UT1=UTC is made. The function uses only the *supplied* values;
/// it never silently falls back to an approximate orientation.
pub fn compose_gcrs_to_itrs(
    itrs_to_tirs_w: [[f64; 3]; 3],
    era_rad: f64,
    gcrs_to_cirs_c: [[f64; 3]; 3],
) -> Result<ComposedOrientation, OrientationChainError> {
    if !proper_rotation(&itrs_to_tirs_w) {
        return Err(OrientationChainError::InvalidPolarMotionMatrix);
    }
    if !proper_rotation(&gcrs_to_cirs_c) {
        return Err(OrientationChainError::InvalidCioOrientationMatrix);
    }
    if !era_rad.is_finite() || !(0.0..TAU).contains(&era_rad) {
        return Err(OrientationChainError::InvalidEarthRotationAngle);
    }
    let (sin, cos) = era_rad.sin_cos();
    // CIRS -> TIRS, ordinary active column-vector Rz(-ERA).
    let cirs_to_tirs = [[cos, sin, 0.0], [-sin, cos, 0.0], [0.0, 0.0, 1.0]];
    let tirs_to_itrs: [[f64; 3]; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| itrs_to_tirs_w[j][i]));
    let rotated_celestial = multiply(&cirs_to_tirs, &gcrs_to_cirs_c);
    let gcrs_to_itrs = multiply(&tirs_to_itrs, &rotated_celestial);
    if gcrs_to_itrs
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(OrientationChainError::NonFiniteResult);
    }
    Ok(ComposedOrientation { gcrs_to_itrs })
}
