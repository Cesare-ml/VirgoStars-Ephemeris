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

//! Passive rotation of Cartesian states into caller-supplied moving axes.
//!
//! This is a mathematical state-vector stage, not an IAU precession/nutation
//! model and not a Swiss-compatible apparent-of-date coordinate pipeline.

use std::error::Error;
use std::fmt;

use crate::de441::StateVector;

/// Rotation matrix and its derivative **with respect to observation TDB days**.
///
/// For a passive 3x3 rotation R(t) acting on an ICRF-aligned Cartesian state,
/// the rotated position is R*r and the velocity is R*v + dR/dt*r.
/// The derivative term is required for longitude/latitude speeds in
/// noninertial axes. The caller must supply both matrices for the same epoch,
/// reference-frame convention and observer. No time-scale or IAU model is
/// silently inferred; R and dR/dt are not supplied by this crate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotatingFrame {
    rotation: [[f64; 3]; 3],
    rotation_rate_per_tdb_day: [[f64; 3]; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotatingFrameError {
    NonFinite,
    /// A matrix is not an orthonormal right-handed rotation.
    InvalidRotation,
    /// dR/dt is inconsistent with the orthonormal constraint R*R^T=I.
    InconsistentDerivative,
    /// No accurate caller-supplied TT days per TDB day conversion.
    InvalidTimeScaleRate,
    /// Matrix multiplication produces a nonrepresentable state.
    ArithmeticOverflow,
}

impl fmt::Display for RotatingFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => write!(f, "non-finite moving-frame input"),
            Self::InvalidRotation => write!(f, "moving-frame matrix is not a proper rotation"),
            Self::InconsistentDerivative => {
                write!(f, "moving-frame derivative violates orthogonality")
            }
            Self::InvalidTimeScaleRate => {
                write!(f, "invalid caller-supplied d(TT days)/d(TDB days)")
            }
            Self::ArithmeticOverflow => write!(f, "non-finite transformed Cartesian state"),
        }
    }
}
impl Error for RotatingFrameError {}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

fn mat_vec(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|row| dot(matrix[row], vector))
}

fn mat_mul(left: [[f64; 3]; 3], right: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            left[i][0] * right[0][j] + left[i][1] * right[1][j] + left[i][2] * right[2][j]
        })
    })
}

/// Ratio of **TT days elapsed per TDB day elapsed at one epoch**.
///
/// Used only to convert a published matrix derivative per **TT day**
/// to the reception-time derivative per **TDB day**. It is **not**
/// the TT–TDB *epoch offset* or a ΔT (UT1–TT) value.
///
/// The ratio MUST be supplied by a separate, independently validated
/// TT↔TDB rate model at the **same physical instant** as both matrices.
/// No implicit unity assumption or unverified low-accuracy model is used.
///
/// The 0.999..1.001 bound is a wide engineering sanity guard against
/// common unit confusions (e.g. seconds per day or an absolute epoch),
/// **not** a precision certification or an error tolerance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TtPerTdbDay(f64);

impl TtPerTdbDay {
    pub fn new(rate: f64) -> Result<Self, RotatingFrameError> {
        if !rate.is_finite() || !(0.999..=1.001).contains(&rate) {
            return Err(RotatingFrameError::InvalidTimeScaleRate);
        }
        Ok(Self(rate))
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

impl RotatingFrame {
    /// Construct a moving frame from a caller-supplied **TT-day**
    /// matrix derivative, converting its rate to observation TDB days.
    ///
    /// The rotation and TT derivative must be evaluated for the **same**
    /// physical reception instant. The caller must provide the correct,
    /// independently determined instantaneous `dTT/dTDB` factor and
    /// must not confuse it with `TT-TDB`, `ΔT` or `dTDB/dTT`.
    ///
    /// The matrix may represent any independently computed passive
    /// orientation: no precession/nutation model or time conversion is
    /// inferred. All `RotatingFrame::new` orthogonality, determinant,
    /// tangent derivative and nonfinite checks remain in force.
    pub fn from_tt_day_rate(
        rotation: [[f64; 3]; 3],
        rotation_rate_per_tt_day: [[f64; 3]; 3],
        tt_days_per_tdb_day: TtPerTdbDay,
    ) -> Result<Self, RotatingFrameError> {
        let factor = tt_days_per_tdb_day.value();
        let rate_tdb = rotation_rate_per_tt_day.map(|row| row.map(|v| v * factor));
        Self::new(rotation, rate_tdb)
    }

    /// Compose externally supplied **IAU-style** precession and nutation
    /// matrices, including the full TT-day **product-rule derivative**.
    ///
    /// `precession` maps fixed GCRS/ICRF-aligned axes to **mean
    /// equator/equinox of date**. `nutation` maps mean to **true
    /// equator/equinox of date**. Thus the passive result is **N*P**:
    ///
    /// `R = N*P`
    /// `dR/dTT = (dN/dTT)*P + N*(dP/dTT)`
    /// `dR/dTDB = (dR/dTT) * (dTT/dTDB)`
    ///
    /// Each caller-supplied matrix and TT derivative is checked as a
    /// valid proper rotation/tangent pair before composition. The
    /// result is **true equator/equinox** only if the supplied inputs
    /// are genuine, correctly dated IAU matrices: this API does not
    /// compute or authenticate them. **True ecliptic of date** requires
    /// another independently validated rotation and its derivative.
    ///
    /// The physical reception instant and `TtPerTdbDay` must match
    /// the source vector's observation TDB instant; no epoch matching
    /// can be verified from these raw matrices.
    pub fn from_tt_precession_nutation(
        precession: [[f64; 3]; 3],
        precession_rate_per_tt_day: [[f64; 3]; 3],
        nutation: [[f64; 3]; 3],
        nutation_rate_per_tt_day: [[f64; 3]; 3],
        tt_days_per_tdb_day: TtPerTdbDay,
    ) -> Result<Self, RotatingFrameError> {
        let p =
            Self::from_tt_day_rate(precession, precession_rate_per_tt_day, tt_days_per_tdb_day)?;
        let n = Self::from_tt_day_rate(nutation, nutation_rate_per_tt_day, tt_days_per_tdb_day)?;
        let rotation = mat_mul(n.rotation, p.rotation);
        let n_dot_p = mat_mul(n.rotation_rate_per_tdb_day, p.rotation);
        let n_p_dot = mat_mul(n.rotation, p.rotation_rate_per_tdb_day);
        let rate_tdb =
            std::array::from_fn(|i| std::array::from_fn(|j| n_dot_p[i][j] + n_p_dot[i][j]));
        Self::new(rotation, rate_tdb)
    }

    /// Validate a proper passive rotation and a compatible time derivative.
    ///
    /// The absolute orthonormal/determinant acceptance threshold is 1e-10.
    /// The tangent constraint is validated to 1e-10 times max(1, largest
    /// derivative component). These are mathematical safeguards, **not**
    /// limits on precession/nutation accuracy or real ephemeris precision.
    pub fn new(
        rotation: [[f64; 3]; 3],
        rotation_rate_per_tdb_day: [[f64; 3]; 3],
    ) -> Result<Self, RotatingFrameError> {
        if rotation
            .iter()
            .flatten()
            .chain(rotation_rate_per_tdb_day.iter().flatten())
            .any(|value| !value.is_finite())
        {
            return Err(RotatingFrameError::NonFinite);
        }

        const TOLERANCE: f64 = 1e-10;
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                if (dot(rotation[i], rotation[j]) - expected).abs() > TOLERANCE {
                    return Err(RotatingFrameError::InvalidRotation);
                }
            }
        }
        // A reflection has orthonormal rows too, but is not a rotation.
        let cross = [
            rotation[1][1] * rotation[2][2] - rotation[1][2] * rotation[2][1],
            rotation[1][2] * rotation[2][0] - rotation[1][0] * rotation[2][2],
            rotation[1][0] * rotation[2][1] - rotation[1][1] * rotation[2][0],
        ];
        if (dot(rotation[0], cross) - 1.0).abs() > TOLERANCE {
            return Err(RotatingFrameError::InvalidRotation);
        }

        // Differentiate row orthogonality:
        // (dR/dt)*R^T + R*(dR/dt)^T = 0.
        let scale = rotation_rate_per_tdb_day
            .iter()
            .flatten()
            .fold(1.0_f64, |maximum, value| maximum.max(value.abs()));
        for i in 0..3 {
            for j in 0..3 {
                let derivative = dot(rotation_rate_per_tdb_day[i], rotation[j])
                    + dot(rotation[i], rotation_rate_per_tdb_day[j]);
                if !derivative.is_finite() || derivative.abs() > TOLERANCE * scale {
                    return Err(RotatingFrameError::InconsistentDerivative);
                }
            }
        }
        Ok(Self {
            rotation,
            rotation_rate_per_tdb_day,
        })
    }

    /// Rotate a Cartesian state while preserving its target, center and TDB epoch.
    ///
    /// Output has the same km and km/TDB-day units; its axes now match the
    /// provided R(t) instead of the input ICRF axes. The `StateVector` type
    /// itself has no frame tag: callers MUST track the frame externally and
    /// MUST NOT feed the result to routines requiring fixed ICRF input.
    pub fn rotate_state(&self, state: &StateVector) -> Result<StateVector, RotatingFrameError> {
        if !state.epoch.0.is_finite()
            || state
                .position_km
                .iter()
                .chain(state.velocity_km_per_day.iter())
                .any(|value| !value.is_finite())
        {
            return Err(RotatingFrameError::NonFinite);
        }
        let position_km = mat_vec(self.rotation, state.position_km);
        let velocity_rotated = mat_vec(self.rotation, state.velocity_km_per_day);
        let velocity_due_to_axes = mat_vec(self.rotation_rate_per_tdb_day, state.position_km);
        let velocity_km_per_day =
            std::array::from_fn(|i| velocity_rotated[i] + velocity_due_to_axes[i]);
        if position_km
            .iter()
            .chain(velocity_km_per_day.iter())
            .any(|value| !value.is_finite())
        {
            return Err(RotatingFrameError::ArithmeticOverflow);
        }
        Ok(StateVector {
            target: state.target,
            center: state.center,
            epoch: state.epoch,
            position_km,
            velocity_km_per_day,
        })
    }
}
