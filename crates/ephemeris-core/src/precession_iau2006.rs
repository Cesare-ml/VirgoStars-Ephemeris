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

//! IAU 2006 bias-precession orientation, with derivative per **TT** day.
//!
//! The four Fukushima-Williams angles come from the published IERS
//! Conventions (2010), Technical Note 36, Eq. (5.40). The rotation is the
//! GCRS-to-mean-equator-and-equinox-of-date **passive** orientation.
//! This is NOT true-of-date (nutation is absent), an apparent planetary state,
//! nor a TDB-day moving-frame transform.
//!
//! References (scientific formulae only, no third-party source code):
//! <https://iers-conventions.obspm.fr/content/tn36.pdf>
//! <https://www.iausofa.org/> (Fukushima-Williams convention).
//! Independent numeric cross-check: ERFA test `eraPmat06` at JD 2450124.4999.

use std::f64::consts::PI;

use crate::calendar::JulianDate;
use crate::mean_obliquity::{mean_obliquity_iau2006, tt_centuries_iau2006, MeanObliquityError};

const ARCSEC_TO_RADIANS: f64 = PI / (180.0 * 3_600.0);
const TT_DAYS_PER_CENTURY: f64 = 36_525.0;

/// Coefficients in increasing powers of TT Julian centuries since J2000,
/// in arcseconds. IERS Conventions (2010), equation (5.40).
const GAMMA_BAR: [f64; 6] = [
    -0.052_928,
    10.556_378,
    0.493_204_4,
    -0.000_312_38,
    -0.000_002_788,
    0.000_000_026,
];
const PHI_BAR: [f64; 6] = [
    84_381.412_819,
    -46.811_016,
    0.051_126_8,
    0.000_532_89,
    -0.000_000_440,
    -0.000_000_017_6,
];
const PSI_BAR: [f64; 6] = [
    -0.041_775,
    5_038.481_484,
    1.558_417_5,
    -0.000_185_22,
    -0.000_026_452,
    -0.000_000_014_8,
];

type Matrix = [[f64; 3]; 3];

/// Orientation of **mean** equatorial axes of date relative to the GCRS.
///
/// `rotation` is an orthonormal passive matrix mapping GCRS Cartesian
/// coordinates into the mean equator/equinox of date. `rate_per_tt_day` is
/// its exact analytical time derivative with respect to **TT days**.
///
/// No TDB or UTC conversion is performed. The derivative must not be used as
/// dR/d(TDB) without an explicit, validated TT↔TDB rate conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrecessionBiasIau2006 {
    pub rotation: Matrix,
    pub rate_per_tt_day: Matrix,
}

fn polynomial_and_tt_day_rate(coefficients: &[f64; 6], centuries: f64) -> (f64, f64) {
    let mut value = coefficients[5];
    let mut derivative_per_century = 0.0;
    for coefficient in coefficients[..5].iter().rev() {
        derivative_per_century = derivative_per_century * centuries + value;
        value = value * centuries + *coefficient;
    }
    (
        value * ARCSEC_TO_RADIANS,
        derivative_per_century * ARCSEC_TO_RADIANS / TT_DAYS_PER_CENTURY,
    )
}

fn multiply(left: Matrix, right: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| left[i][k] * right[k][j]).sum()))
}

fn add(left: Matrix, right: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| left[i][j] + right[i][j]))
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

/// Compute IAU 2006 frame bias and precession, **excluding** nutation.
///
/// Fukushima-Williams convention, ordered as matrix multiplication:
///
/// `R = Rx(-eps_a) * Rz(-psi_bar) * Rx(phi_bar) * Rz(gamma_bar)`
///
/// with **passive** X/Z rotations (the matrix maps GCRS components into
/// mean-of-date components). Its derivative is obtained by applying the
/// matrix product rule to every component rotation, including the changing
/// obliquity. Both `R` and `dR/dTT` use the same split TT Julian date.
///
/// Same conservative J2000 ±2 TT-Julian-century limit as the standalone
/// mean-obliquity API. Neither model is an IAU 2000A nutation evaluation.
pub fn precession_bias_iau2006(
    tt_epoch: JulianDate,
) -> Result<PrecessionBiasIau2006, MeanObliquityError> {
    let centuries = tt_centuries_iau2006(tt_epoch)?;
    let (gamma, gamma_rate) = polynomial_and_tt_day_rate(&GAMMA_BAR, centuries);
    let (phi, phi_rate) = polynomial_and_tt_day_rate(&PHI_BAR, centuries);
    let (psi, psi_rate) = polynomial_and_tt_day_rate(&PSI_BAR, centuries);
    let obliquity = mean_obliquity_iau2006(tt_epoch)?;

    let factors = [
        passive_z(gamma, gamma_rate),
        passive_x(phi, phi_rate),
        passive_z(-psi, -psi_rate),
        passive_x(-obliquity.radians, -obliquity.rate_radians_per_tt_day),
    ];
    let mut orientation: Matrix = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut derivative: Matrix = [[0.0; 3]; 3];

    for (rotation, rate) in factors {
        // Product rule for left composition: (A R)' = A' R + A R'.
        derivative = add(multiply(rate, orientation), multiply(rotation, derivative));
        orientation = multiply(rotation, orientation);
    }

    Ok(PrecessionBiasIau2006 {
        rotation: orientation,
        rate_per_tt_day: derivative,
    })
}
