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

//! Independent mean-of-date to true-of-date nutation rotation and TT rate.
//!
//! The caller supplies mean obliquity, nutation angles, and their time
//! derivatives. This module does **not** evaluate the IAU 2000A 1365-term
//! series, IAU 2000B, Earth orientation, or complete apparent coordinates.
//! It only applies the conventional equinox-based three-rotation geometry.
//! See IERS Conventions (2010), Chapter 5, and the ERFA Numat fixture.

use std::error::Error;
use std::fmt;

type Matrix = [[f64; 3]; 3];

/// Mean obliquity and nutation inputs, all angles in radians.
///
/// All derivatives are radians per **TT day**, at the **same TT epoch**.
/// Inputs are not obtained from an ephemeris and no time conversion occurs.
/// Delta-psi is nutation in longitude; delta-epsilon is nutation in obliquity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NutationAngles {
    pub mean_obliquity_rad: f64,
    pub mean_obliquity_rate_rad_per_tt_day: f64,
    pub delta_psi_rad: f64,
    pub delta_psi_rate_rad_per_tt_day: f64,
    pub delta_epsilon_rad: f64,
    pub delta_epsilon_rate_rad_per_tt_day: f64,
}

/// Mean equator/equinox of date to true equator/equinox of date.
///
/// `rotation` is passive. `rate_per_tt_day` is its analytical derivative
/// per **TT** day, not per observation **TDB** day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NutationMatrix {
    pub rotation: Matrix,
    pub rate_per_tt_day: Matrix,
}

/// Nutation and its **matching true obliquity** derived from the same
/// caller-supplied, single-epoch equinox-based angles and TT-day rates.
///
/// Pass `nutation.rotation` and `nutation.rate_per_tt_day`, together
/// with `true_obliquity_rad` and
/// `true_obliquity_rate_rad_per_tt_day`, to a separately validated
/// true-ecliptic-of-date frame builder (draft PR #47) **only after**
/// normal branch integration and a separately validated dTT/dTDB
/// chain rule. This type does not claim that the source angles came
/// from the full IAU 2000A series or that the result is apparent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NutationWithTrueObliquity {
    pub nutation: NutationMatrix,
    pub true_obliquity_rad: f64,
    pub true_obliquity_rate_rad_per_tt_day: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NutationMatrixError {
    NonFiniteInput,
    /// Prevent values outside a physical obliquity interval or implausibly
    /// large nutation corrections from being labeled as astronomical.
    AngleOutsideGuardedRange,
    /// Mean obliquity plus delta-epsilon is not a physical positive tilt.
    InvalidTrueObliquity,
    /// Finite inputs produced a nonrepresentable matrix or derivative.
    ArithmeticOverflow,
}

impl fmt::Display for NutationMatrixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "non-finite nutation angle or TT derivative"),
            Self::AngleOutsideGuardedRange => {
                write!(f, "nutation angles outside conservative astronomical guard")
            }
            Self::InvalidTrueObliquity => {
                write!(f, "derived true obliquity outside (0, pi/2)")
            }
            Self::ArithmeticOverflow => {
                write!(
                    f,
                    "non-finite nutation orientation or TT derivative from finite inputs"
                )
            }
        }
    }
}
impl Error for NutationMatrixError {}

fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

fn add(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| a[i][j] + b[i][j]))
}

fn passive_x(angle: f64, rate: f64) -> (Matrix, Matrix) {
    let (sin, cos) = angle.sin_cos();
    (
        [[1.0, 0.0, 0.0], [0.0, cos, sin], [0.0, -sin, cos]],
        [
            [0.0, 0.0, 0.0],
            [0.0, -rate * sin, rate * cos],
            [0.0, -rate * cos, -rate * sin],
        ],
    )
}

fn passive_z(angle: f64, rate: f64) -> (Matrix, Matrix) {
    let (sin, cos) = angle.sin_cos();
    (
        [[cos, sin, 0.0], [-sin, cos, 0.0], [0.0, 0.0, 1.0]],
        [
            [-rate * sin, rate * cos, 0.0],
            [-rate * cos, -rate * sin, 0.0],
            [0.0, 0.0, 0.0],
        ],
    )
}

/// Build the classical nutation rotation from externally supplied angles.
///
/// Passive convention:
/// `R = Rx(-(epsilon_A + delta_epsilon)) * Rz(-delta_psi) * Rx(epsilon_A)`.
/// Its derivative includes mean-obliquity drift and both nutation angle rates
/// using the full matrix product rule. Thus it can later compose with a
/// separately validated GCRS-to-mean-of-date precession matrix.
///
/// Guardrails check 0 < epsilon_A < pi/2 and both nutation angles < 0.1 rad;
/// these are **engineering** bounds, not official IAU validity limits.
/// The function does NOT calculate epsilon_A, delta-psi, or delta-epsilon.
pub fn nutation_matrix_from_angles(
    angles: NutationAngles,
) -> Result<NutationMatrix, NutationMatrixError> {
    let values = [
        angles.mean_obliquity_rad,
        angles.mean_obliquity_rate_rad_per_tt_day,
        angles.delta_psi_rad,
        angles.delta_psi_rate_rad_per_tt_day,
        angles.delta_epsilon_rad,
        angles.delta_epsilon_rate_rad_per_tt_day,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(NutationMatrixError::NonFiniteInput);
    }
    if angles.mean_obliquity_rad <= 0.0
        || angles.mean_obliquity_rad >= std::f64::consts::FRAC_PI_2
        || angles.delta_psi_rad.abs() >= 0.1
        || angles.delta_epsilon_rad.abs() >= 0.1
    {
        return Err(NutationMatrixError::AngleOutsideGuardedRange);
    }

    // Apply inner-to-outer factors in the same passive rotation convention
    // as the independent IAU 2006 precession work. Matrix entries are
    // dimensionless; the time derivative is per TT day throughout.
    let factors = [
        passive_x(
            angles.mean_obliquity_rad,
            angles.mean_obliquity_rate_rad_per_tt_day,
        ),
        passive_z(-angles.delta_psi_rad, -angles.delta_psi_rate_rad_per_tt_day),
        passive_x(
            -(angles.mean_obliquity_rad + angles.delta_epsilon_rad),
            -(angles.mean_obliquity_rate_rad_per_tt_day + angles.delta_epsilon_rate_rad_per_tt_day),
        ),
    ];
    let mut rotation: Matrix = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut derivative: Matrix = [[0.0; 3]; 3];
    for (factor, factor_rate) in factors {
        derivative = add(
            multiply(factor_rate, rotation),
            multiply(factor, derivative),
        );
        rotation = multiply(factor, rotation);
    }
    if rotation
        .iter()
        .flatten()
        .chain(derivative.iter().flatten())
        .any(|v| !v.is_finite())
    {
        return Err(NutationMatrixError::ArithmeticOverflow);
    }
    Ok(NutationMatrix {
        rotation,
        rate_per_tt_day: derivative,
    })
}

/// Return both the nutation matrix and the **consistent true obliquity**
/// with its TT derivative from one validated set of supplied angles.
///
/// The scalar pair is derived **once**:
/// `epsilon_true = epsilon_mean + delta_epsilon` and
/// `d(epsilon_true)/dTT = d(epsilon_mean)/dTT + d(delta_epsilon)/dTT`.
/// These are the exact inverse-X-tilt parameters expected downstream
/// after the classical `Rx(-epsilon_true)` factor of the nutation
/// rotation. It is unsafe to substitute unrelated user inputs here.
///
/// No nutation series, precession matrix, TT/TDB rate, actual epoch
/// conversion or apparent-source correction is calculated.
/// Nonfinite and out-of-range results are rejected; this does
/// not prove that caller-supplied angles were astronomically correct.
pub fn nutation_with_true_obliquity(
    angles: NutationAngles,
) -> Result<NutationWithTrueObliquity, NutationMatrixError> {
    // Reuse the exact single source of truth for all input guards and
    // the analytically differentiated mean-to-true equatorial frame.
    let nutation = nutation_matrix_from_angles(angles)?;
    let true_obliquity_rad = angles.mean_obliquity_rad + angles.delta_epsilon_rad;
    let true_obliquity_rate_rad_per_tt_day =
        angles.mean_obliquity_rate_rad_per_tt_day + angles.delta_epsilon_rate_rad_per_tt_day;

    if !true_obliquity_rad.is_finite() || !true_obliquity_rate_rad_per_tt_day.is_finite() {
        return Err(NutationMatrixError::ArithmeticOverflow);
    }
    if true_obliquity_rad <= 0.0 || true_obliquity_rad >= std::f64::consts::FRAC_PI_2 {
        return Err(NutationMatrixError::InvalidTrueObliquity);
    }
    Ok(NutationWithTrueObliquity {
        nutation,
        true_obliquity_rad,
        true_obliquity_rate_rad_per_tt_day,
    })
}
