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

//! Geometric angular separation of two directions expressed in one frame.
//!
//! This function performs vector arithmetic only: no time-scale conversion,
//! observer translation, light-time, aberration or frame transformation.

use std::error::Error;
use std::fmt;

/// Errors when measuring the angle between two geometric directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngularSeparationError {
    /// At least one vector component was NaN or infinite.
    NonFiniteInput,
    /// The zero vector has no defined direction.
    UndefinedDirection,
}

impl fmt::Display for AngularSeparationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "non-finite geometric direction component"),
            Self::UndefinedDirection => write!(f, "zero vector has no defined direction"),
        }
    }
}

impl Error for AngularSeparationError {}

fn scaled_direction(vector: [f64; 3]) -> Result<[f64; 3], AngularSeparationError> {
    if vector.iter().any(|component| !component.is_finite()) {
        return Err(AngularSeparationError::NonFiniteInput);
    }
    // Divide by the largest absolute component BEFORE computing products.
    // Unlike a naive Euclidean norm, this handles both huge and subnormal
    // finite distances without intermediate squared-value overflow/underflow.
    let largest = vector
        .iter()
        .fold(0.0_f64, |maximum, component| maximum.max(component.abs()));
    if largest == 0.0 {
        return Err(AngularSeparationError::UndefinedDirection);
    }
    Ok(vector.map(|component| component / largest))
}

/// Angle in radians between two nonzero geometric directions, in [0, PI].
///
/// Both input vectors must point **from the same observer/center**, at the
/// **same epoch**, in the **same Cartesian reference frame**. Their linear
/// units and positive scale factors do not affect the result. These
/// preconditions cannot be verified from bare arrays, so the caller must
/// enforce them. This is a geometric separation, not an apparent elongation,
/// lunar phase angle, or zodiacal aspect.
///
/// Uses atan2(|a × b|, a · b) rather than acos of a normalized dot product.
/// This retains sensitivity near both alignment and opposition and avoids
/// vector norm overflow for very large or tiny finite coordinates.
///
/// No physical correction or ephemeris lookup is performed.
pub fn geometric_angular_separation_rad(
    first: [f64; 3],
    second: [f64; 3],
) -> Result<f64, AngularSeparationError> {
    let a = scaled_direction(first)?;
    let b = scaled_direction(second)?;

    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let cross_magnitude = cross[0].hypot(cross[1]).hypot(cross[2]);
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

    Ok(cross_magnitude.atan2(dot))
}
