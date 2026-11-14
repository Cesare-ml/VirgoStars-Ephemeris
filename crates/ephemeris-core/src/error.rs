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

use core::fmt;

/// Errors returned by the numerical foundations. No silent NaN fallbacks.
#[derive(Clone, Debug, PartialEq)]
pub enum EphemerisError {
    NonFinite { field: &'static str },
    EmptyCoefficients,
    MismatchedCoefficientLengths,
    InvalidRadius,
    OutsideRecord { normalized_time: f64 },
    ArithmeticOverflow,
}

pub type Result<T> = core::result::Result<T, EphemerisError>;

impl fmt::Display for EphemerisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => write!(f, "non-finite numeric input: {field}"),
            Self::EmptyCoefficients => write!(f, "Chebyshev coefficients cannot be empty"),
            Self::MismatchedCoefficientLengths => {
                write!(
                    f,
                    "three coordinate axes must have the same polynomial degree"
                )
            }
            Self::InvalidRadius => {
                write!(f, "Chebyshev record radius must be finite, positive, and resolvable at the record midpoint")
            }
            Self::OutsideRecord { normalized_time } => write!(
                f,
                "time outside Chebyshev record domain [-1, 1]: {normalized_time}"
            ),
            Self::ArithmeticOverflow => write!(f, "numeric result is not finite"),
        }
    }
}

impl std::error::Error for EphemerisError {}
