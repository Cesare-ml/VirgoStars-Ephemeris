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

//! Earth orientation: IAU 2000B nutation, true equator/ecliptic of date and ECL_NUT.
//!
//! Provides the 77-term IAU 2000B nutation series (McCarthy & Luzum 2003) with
//! ERFA scientific coefficients, combined with IAU 2006 bias-precession
//! (Fukushima–Williams 2006) and mean obliquity (Hilton et al. 2006).
//!
//! # Timescales and Conventions
//! - Nutation, precession, and obliquity strictly require Terrestrial Time (TT).
//! - Earth Rotation Angle (ERA) and Greenwich Mean Sidereal Time (GMST) strictly
//!   require Universal Time (UT1); TT is passed separately for the precession term.
//! - Validated epoch coverage for the 77-term IAU 2000B series is 1900–2100 TT
//!   (JD 2415020.0 .. 2488070.0).
//!
//! Third-party coefficient reference data: see `THIRD_PARTY_NOTICES.md` (ERFA BSD-3).

use core::f64::consts::{PI, TAU};
use std::error::Error;
use std::fmt;

pub const ARCSEC_TO_RAD: f64 = PI / (180.0 * 3600.0);
pub const UNIT_TO_RAD: f64 = ARCSEC_TO_RAD / 1e7;
pub const MILLIARCSEC_TO_RAD: f64 = ARCSEC_TO_RAD / 1000.0;
pub const TURN_ARCSEC: f64 = 1_296_000.0;

/// Validated epoch range for experimental IAU 2000B (1900-01-01 to 2100-01-01 TT).
pub const JD_TT_MIN: f64 = 2_415_020.0;
pub const JD_TT_MAX: f64 = 2_488_070.0;

pub const J2000_TT_JD: f64 = 2_451_545.0;
pub const DAYS_PER_JULIAN_CENTURY: f64 = 36_525.0;

pub const NUTATION_MODEL_ID: &str = "IAU_2000B_77_TERMS";

pub type Matrix3x3 = [[f64; 3]; 3];

/// Nutation components in longitude and obliquity at a given TT epoch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NutationAngles {
    pub tt_jd: f64,
    /// Nutation in longitude (dPsi) in radians.
    pub d_psi_rad: f64,
    /// Nutation in obliquity (dEpsilon) in radians.
    pub d_epsilon_rad: f64,
    /// Nutation in longitude (dPsi) in arcseconds.
    pub d_psi_arcsec: f64,
    /// Nutation in obliquity (dEpsilon) in arcseconds.
    pub d_epsilon_arcsec: f64,
    /// Number of series terms evaluated (77).
    pub terms: usize,
}

/// Swiss Ephemeris `SE_ECL_NUT` compatible ecliptic and nutation parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EclipticNutation {
    pub tt_jd: f64,
    /// True obliquity of date (eps_true = eps_mean + dEpsilon), radians.
    pub true_obliquity_rad: f64,
    /// IAU 2006 mean obliquity of date, radians.
    pub mean_obliquity_rad: f64,
    /// Nutation in longitude (dPsi), radians.
    pub nutation_longitude_rad: f64,
    /// Nutation in obliquity (dEpsilon), radians.
    pub nutation_obliquity_rad: f64,
    /// Equation of the equinoxes (dPsi * cos(eps_mean)), radians.
    pub equation_of_equinoxes_rad: f64,
}

impl EclipticNutation {
    #[inline]
    pub fn true_obliquity_deg(&self) -> f64 {
        self.true_obliquity_rad.to_degrees()
    }

    #[inline]
    pub fn mean_obliquity_deg(&self) -> f64 {
        self.mean_obliquity_rad.to_degrees()
    }

    #[inline]
    pub fn nutation_longitude_deg(&self) -> f64 {
        self.nutation_longitude_rad.to_degrees()
    }

    #[inline]
    pub fn nutation_obliquity_deg(&self) -> f64 {
        self.nutation_obliquity_rad.to_degrees()
    }

    #[inline]
    pub fn equation_of_equinoxes_deg(&self) -> f64 {
        self.equation_of_equinoxes_rad.to_degrees()
    }

    /// Formats as the 6-element array returned by Swiss Ephemeris `swe_calc(..., SE_ECL_NUT)`.
    ///
    /// Indices:
    /// - `[0]`: True obliquity of date (degrees).
    /// - `[1]`: Mean obliquity of date (degrees).
    /// - `[2]`: Nutation in longitude (degrees).
    /// - `[3]`: Nutation in obliquity (degrees).
    /// - `[4]`: Equation of the equinoxes (degrees).
    /// - `[5]`: Reserved / 0.0.
    pub fn to_swiss_ecl_nut(&self) -> [f64; 6] {
        [
            self.true_obliquity_deg(),
            self.mean_obliquity_deg(),
            self.nutation_longitude_deg(),
            self.nutation_obliquity_deg(),
            self.equation_of_equinoxes_deg(),
            0.0,
        ]
    }
}

/// Errors occurring during Earth orientation calculations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OrientationError {
    /// Non-finite or NaN numeric input.
    NonFinite { field: &'static str },
    /// Epoch outside validated range 1900–2100 TT.
    UnsupportedEpoch { epoch_tt_jd: f64 },
    /// Non-finite or out-of-range obliquity.
    InvalidObliquity,
    /// Singularity in spherical or vector normalization.
    SingularVector,
}

impl fmt::Display for OrientationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => write!(f, "non-finite orientation input: {field}"),
            Self::UnsupportedEpoch { epoch_tt_jd } => {
                write!(
                    f,
                    "epoch {epoch_tt_jd} TT outside validated IAU 2000B range (1900–2100 TT)"
                )
            }
            Self::InvalidObliquity => write!(f, "finite obliquity in radians required"),
            Self::SingularVector => write!(f, "zero or singular vector encountered"),
        }
    }
}

impl Error for OrientationError {}

/// IAU 2000B 77-term luni-solar nutation coefficient table.
///
/// Derived from the independently distributed BSD-3 licensed ERFA data table:
/// <https://github.com/liberfa/erfa/blob/master/src/nut00b.c>
/// Copyright (C) 2013-2023 NumFOCUS Foundation.
///
/// Columns per row:
/// 0..4: Delaunay argument multipliers `[l, l', F, D, Omega]`
/// 5..7: Longitude coefficients in 0.1 microarcseconds (constant, linear per century, complementary)
/// 8..10: Obliquity coefficients in 0.1 microarcseconds (constant, linear per century, complementary)
const IAU2000B_TERMS: [[i32; 11]; 77] = [
    [
        0, 0, 0, 0, 1, -172064161, -174666, 33386, 92052331, 9086, 15377,
    ],
    [
        0, 0, 2, -2, 2, -13170906, -1675, -13696, 5730336, -3015, -4587,
    ],
    [0, 0, 2, 0, 2, -2276413, -234, 2796, 978459, -485, 1374],
    [0, 0, 0, 0, 2, 2074554, 207, -698, -897492, 470, -291],
    [0, 1, 0, 0, 0, 1475877, -3633, 11817, 73871, -184, -1924],
    [0, 1, 2, -2, 2, -516821, 1226, -524, 224386, -677, -174],
    [1, 0, 0, 0, 0, 711159, 73, -872, -6750, 0, 358],
    [0, 0, 2, 0, 1, -387298, -367, 380, 200728, 18, 318],
    [1, 0, 2, 0, 2, -301461, -36, 816, 129025, -63, 367],
    [0, -1, 2, -2, 2, 215829, -494, 111, -95929, 299, 132],
    [0, 0, 2, -2, 1, 128227, 137, 181, -68982, -9, 39],
    [-1, 0, 2, 0, 2, 123457, 11, 19, -53311, 32, -4],
    [-1, 0, 0, 2, 0, 156994, 10, -168, -1235, 0, 82],
    [1, 0, 0, 0, 1, 63110, 63, 27, -33228, 0, -9],
    [-1, 0, 0, 0, 1, -57976, -63, -189, 31429, 0, -75],
    [-1, 0, 2, 2, 2, -59641, -11, 149, 25543, -11, 66],
    [1, 0, 2, 0, 1, -51613, -42, 129, 26366, 0, 78],
    [-2, 0, 2, 0, 1, 45893, 50, 31, -24236, -10, 20],
    [0, 0, 0, 2, 0, 63384, 11, -150, -1220, 0, 29],
    [0, 0, 2, 2, 2, -38571, -1, 158, 16452, -11, 68],
    [0, -2, 2, -2, 2, 32481, 0, 0, -13870, 0, 0],
    [-2, 0, 0, 2, 0, -47722, 0, -18, 477, 0, -25],
    [2, 0, 2, 0, 2, -31046, -1, 131, 13238, -11, 59],
    [1, 0, 2, -2, 2, 28593, 0, -1, -12338, 10, -3],
    [-1, 0, 2, 0, 1, 20441, 21, 10, -10758, 0, -3],
    [2, 0, 0, 0, 0, 29243, 0, -74, -609, 0, 13],
    [0, 0, 2, 0, 0, 25887, 0, -66, -550, 0, 11],
    [0, 1, 0, 0, 1, -14053, -25, 79, 8551, -2, -45],
    [-1, 0, 0, 2, 1, 15164, 10, 11, -8001, 0, -1],
    [0, 2, 2, -2, 2, -15794, 72, -16, 6850, -42, -5],
    [0, 0, -2, 2, 0, 21783, 0, 13, -167, 0, 13],
    [1, 0, 0, -2, 1, -12873, -10, -37, 6953, 0, -14],
    [0, -1, 0, 0, 1, -12654, 11, 63, 6415, 0, 26],
    [-1, 0, 2, 2, 1, -10204, 0, 25, 5222, 0, 15],
    [0, 2, 0, 0, 0, 16707, -85, -10, 168, -1, 10],
    [1, 0, 2, 2, 2, -7691, 0, 44, 3268, 0, 19],
    [-2, 0, 2, 0, 0, -11024, 0, -14, 104, 0, 2],
    [0, 1, 2, 0, 2, 7566, -21, -11, -3250, 0, -5],
    [0, 0, 2, 2, 1, -6637, -11, 25, 3353, 0, 14],
    [0, -1, 2, 0, 2, -7141, 21, 8, 3070, 0, 4],
    [0, 0, 0, 2, 1, -6302, -11, 2, 3272, 0, 4],
    [1, 0, 2, -2, 1, 5800, 10, 2, -3045, 0, -1],
    [2, 0, 2, -2, 2, 6443, 0, -7, -2768, 0, -4],
    [-2, 0, 0, 2, 1, -5774, -11, -15, 3041, 0, -5],
    [2, 0, 2, 0, 1, -5350, 0, 21, 2695, 0, 12],
    [0, -1, 2, -2, 1, -4752, -11, -3, 2719, 0, -3],
    [0, 0, 0, -2, 1, -4940, -11, -21, 2720, 0, -9],
    [-1, -1, 0, 2, 0, 7350, 0, -8, -51, 0, 4],
    [2, 0, 0, -2, 1, 4065, 0, 6, -2206, 0, 1],
    [1, 0, 0, 2, 0, 6579, 0, -24, -199, 0, 2],
    [0, 1, 2, -2, 1, 3579, 0, 5, -1900, 0, 1],
    [1, -1, 0, 0, 0, 4725, 0, -6, -41, 0, 3],
    [-2, 0, 2, 0, 2, -3075, 0, -2, 1313, 0, -1],
    [3, 0, 2, 0, 2, -2904, 0, 15, 1233, 0, 7],
    [0, -1, 0, 2, 0, 4348, 0, -10, -81, 0, 2],
    [1, -1, 2, 0, 2, -2878, 0, 8, 1232, 0, 4],
    [0, 0, 0, 1, 0, -4230, 0, 5, -20, 0, -2],
    [-1, -1, 2, 2, 2, -2819, 0, 7, 1207, 0, 3],
    [-1, 0, 2, 0, 0, -4056, 0, 5, 40, 0, -2],
    [0, -1, 2, 2, 2, -2647, 0, 11, 1129, 0, 5],
    [-2, 0, 0, 0, 1, -2294, 0, -10, 1266, 0, -4],
    [1, 1, 2, 0, 2, 2481, 0, -7, -1062, 0, -3],
    [2, 0, 0, 0, 1, 2179, 0, -2, -1129, 0, -2],
    [-1, 1, 0, 1, 0, 3276, 0, 1, -9, 0, 0],
    [1, 1, 0, 0, 0, -3389, 0, 5, 35, 0, -2],
    [1, 0, 2, 0, 0, 3339, 0, -13, -107, 0, 1],
    [-1, 0, 2, -2, 1, -1987, 0, -6, 1073, 0, -2],
    [1, 0, 0, 0, 2, -1981, 0, 0, 854, 0, 0],
    [-1, 0, 0, 1, 0, 4026, 0, -353, -553, 0, -139],
    [0, 0, 2, 1, 2, 1660, 0, -5, -710, 0, -2],
    [-1, 0, 2, 4, 2, -1521, 0, 9, 647, 0, 4],
    [-1, 1, 0, 1, 1, 1314, 0, 0, -700, 0, 0],
    [0, -2, 2, -2, 1, -1283, 0, 0, 672, 0, 0],
    [1, 0, 2, 2, 1, -1331, 0, 8, 663, 0, 4],
    [-2, 0, 2, 2, 2, 1383, 0, -2, -594, 0, -2],
    [-1, 0, 0, 0, 2, 1405, 0, 4, -610, 0, 2],
    [1, 1, 2, -2, 2, 1290, 0, 0, -556, 0, 0],
];

fn validate_tt_jd(tt_jd: f64) -> Result<f64, OrientationError> {
    if !tt_jd.is_finite() {
        return Err(OrientationError::NonFinite { field: "tt_jd" });
    }
    if !(JD_TT_MIN..JD_TT_MAX).contains(&tt_jd) {
        return Err(OrientationError::UnsupportedEpoch { epoch_tt_jd: tt_jd });
    }
    Ok((tt_jd - J2000_TT_JD) / DAYS_PER_JULIAN_CENTURY)
}

/// Linearized Simon et al. (1994) Delaunay fundamental arguments for IAU 2000B (radians).
fn delaunay_arguments(t: f64) -> [f64; 5] {
    const INITIAL: [f64; 5] = [
        485868.249036,
        1287104.79305,
        335779.526232,
        1072260.70369,
        450160.398036,
    ];
    const RATES: [f64; 5] = [
        1717915923.2178,
        129596581.0481,
        1739527262.8478,
        1602961601.2090,
        -6962890.5431,
    ];

    std::array::from_fn(|i| {
        let arcsec = (INITIAL[i] + RATES[i] * t).rem_euclid(TURN_ARCSEC);
        arcsec * ARCSEC_TO_RAD
    })
}

/// Evaluates the 77-term IAU 2000B luni-solar nutation series in longitude (dPsi)
/// and obliquity (dEpsilon) at epoch `tt_jd` (TT).
pub fn iau2000b_nutation(tt_jd: f64) -> Result<NutationAngles, OrientationError> {
    let t = validate_tt_jd(tt_jd)?;
    let a = delaunay_arguments(t);

    let mut longitude_sum = 0.0;
    let mut obliquity_sum = 0.0;

    for row in IAU2000B_TERMS.iter().rev() {
        let mut angle = 0.0;
        for k in 0..5 {
            angle += f64::from(row[k]) * a[k];
        }
        let angle = angle.rem_euclid(TAU);
        let (sn, cs) = angle.sin_cos();

        longitude_sum += (f64::from(row[5]) + f64::from(row[6]) * t) * sn + f64::from(row[7]) * cs;
        obliquity_sum += (f64::from(row[8]) + f64::from(row[9]) * t) * cs + f64::from(row[10]) * sn;
    }

    // Published IAU 2000B planetary bias offsets: -0.135 mas and +0.388 mas.
    let d_psi_rad = longitude_sum * UNIT_TO_RAD - 0.135 * MILLIARCSEC_TO_RAD;
    let d_epsilon_rad = obliquity_sum * UNIT_TO_RAD + 0.388 * MILLIARCSEC_TO_RAD;

    Ok(NutationAngles {
        tt_jd,
        d_psi_rad,
        d_epsilon_rad,
        d_psi_arcsec: d_psi_rad / ARCSEC_TO_RAD,
        d_epsilon_arcsec: d_epsilon_rad / ARCSEC_TO_RAD,
        terms: IAU2000B_TERMS.len(),
    })
}

fn poly(t: f64, coefficients: &[f64]) -> f64 {
    coefficients.iter().rev().fold(0.0, |sum, c| sum * t + c) * ARCSEC_TO_RAD
}

/// IAU 2006 mean obliquity of the ecliptic (Hilton et al. 2006) in radians.
pub fn iau2006_mean_obliquity(tt_jd: f64) -> Result<f64, OrientationError> {
    let t = validate_tt_jd(tt_jd)?;
    Ok(poly(
        t,
        &[
            84381.406,
            -46.836769,
            -0.0001831,
            0.00200340,
            -0.000000576,
            -0.0000000434,
        ],
    ))
}

/// Computes the true obliquity of the ecliptic (radians) given mean obliquity and nutation in obliquity.
#[inline]
pub fn true_obliquity(mean_obliquity_rad: f64, d_epsilon_rad: f64) -> f64 {
    mean_obliquity_rad + d_epsilon_rad
}

/// Passive rotation matrix about +X axis by `angle` radians.
pub fn x_rotation(angle: f64) -> Matrix3x3 {
    let (s, c) = angle.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]]
}

/// Passive rotation matrix about +Z axis by `angle` radians.
pub fn z_rotation(angle: f64) -> Matrix3x3 {
    let (s, c) = angle.sin_cos();
    [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]]
}

/// Multiplies two 3x3 matrices: `a * b`.
pub fn matrix_multiply(a: Matrix3x3, b: Matrix3x3) -> Matrix3x3 {
    let mut out = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    out
}

/// Transpose of a 3x3 matrix.
pub fn matrix_transpose(m: Matrix3x3) -> Matrix3x3 {
    [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ]
}

/// Multiplies a 3x3 matrix by a 3D vector: `m * v`.
pub fn matrix_vector_multiply(m: Matrix3x3, v: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| (0..3).map(|j| m[i][j] * v[j]).sum())
}

/// Rotation matrix from Mean Equator of Date to True Equator of Date (Nutation Matrix).
///
/// If `mean_obliquity_rad` is provided, it is used for reference comparison
/// (e.g., against ERFA `num00b`); otherwise IAU 2006 mean obliquity is computed.
pub fn iau2000b_nutation_matrix(
    tt_jd: f64,
    mean_obliquity_rad: Option<f64>,
) -> Result<Matrix3x3, OrientationError> {
    let nut = iau2000b_nutation(tt_jd)?;
    let eps = match mean_obliquity_rad {
        Some(e) => {
            if !e.is_finite() {
                return Err(OrientationError::InvalidObliquity);
            }
            e
        }
        None => iau2006_mean_obliquity(tt_jd)?,
    };

    // N = R1(-eps - deps) * R3(-dpsi) * R1(eps)
    let r1_eps = x_rotation(eps);
    let r3_dpsi = z_rotation(-nut.d_psi_rad);
    let r1_true = x_rotation(-eps - nut.d_epsilon_rad);

    Ok(matrix_multiply(matrix_multiply(r1_true, r3_dpsi), r1_eps))
}

/// IAU 2006 Fukushima–Williams 4-angle bias-precession matrix from ICRF to Mean Equator of Date.
pub fn iau2006_bias_precession_matrix(tt_jd: f64) -> Result<Matrix3x3, OrientationError> {
    let t = validate_tt_jd(tt_jd)?;
    let g = poly(
        t,
        &[
            -0.052928,
            10.556378,
            0.4932044,
            -0.00031238,
            -0.000002788,
            0.0000000260,
        ],
    );
    let p = poly(
        t,
        &[
            84381.412819,
            -46.811016,
            0.0511268,
            0.00053289,
            -0.000000440,
            -0.0000000176,
        ],
    );
    let q = poly(
        t,
        &[
            -0.041775,
            5038.481484,
            1.5584175,
            -0.00018522,
            -0.000026452,
            -0.0000000148,
        ],
    );
    let e = poly(
        t,
        &[
            84381.406,
            -46.836769,
            -0.0001831,
            0.00200340,
            -0.000000576,
            -0.0000000434,
        ],
    );

    // P_B = R1(-e) * R3(-q) * R1(p) * R3(g)
    let m = matrix_multiply(
        matrix_multiply(
            matrix_multiply(x_rotation(-e), z_rotation(-q)),
            x_rotation(p),
        ),
        z_rotation(g),
    );
    Ok(m)
}

/// Rotation matrix from ICRF to True Equator of Date (`M_eq = N * P_B`).
pub fn true_equator_of_date_matrix(tt_jd: f64) -> Result<Matrix3x3, OrientationError> {
    let nut_mat = iau2000b_nutation_matrix(tt_jd, None)?;
    let bp_mat = iau2006_bias_precession_matrix(tt_jd)?;
    Ok(matrix_multiply(nut_mat, bp_mat))
}

/// Rotation matrix from ICRF to True Ecliptic of Date (`M_ecl = R1(eps_true) * M_eq`).
pub fn true_ecliptic_of_date_matrix(tt_jd: f64) -> Result<Matrix3x3, OrientationError> {
    let nut = iau2000b_nutation(tt_jd)?;
    let eps_mean = iau2006_mean_obliquity(tt_jd)?;
    let eps_true = true_obliquity(eps_mean, nut.d_epsilon_rad);
    let eq_mat = true_equator_of_date_matrix(tt_jd)?;
    Ok(matrix_multiply(x_rotation(eps_true), eq_mat))
}

/// Earth Rotation Angle (ERA) IAU 2000 in radians, strictly taking `ut1_jd`.
#[allow(clippy::excessive_precision)]
pub fn era_iau2000(ut1_jd: f64) -> Result<f64, OrientationError> {
    if !ut1_jd.is_finite() {
        return Err(OrientationError::NonFinite { field: "ut1_jd" });
    }
    let d_u = ut1_jd - J2000_TT_JD;
    let theta = TAU * (0.7790572732640 + 1.00273781191135448 * d_u);
    Ok(theta.rem_euclid(TAU))
}

/// Greenwich Mean Sidereal Time (GMST) IAU 2006 in radians [0, 2π), distinguishing `ut1_jd` from `tt_jd`.
pub fn gmst_iau2006(ut1_jd: f64, tt_jd: f64) -> Result<f64, OrientationError> {
    let era = era_iau2000(ut1_jd)?;
    let t = validate_tt_jd(tt_jd)?;

    // Precession polynomial in right ascension along intermediate equator (arcseconds).
    let p_ra = 0.014506 + 4612.156534 * t + 1.3915817 * t.powi(2)
        - 0.00000044 * t.powi(3)
        - 0.000029956 * t.powi(4)
        - 0.0000000368 * t.powi(5);

    let gmst = era + p_ra * ARCSEC_TO_RAD;
    Ok(gmst.rem_euclid(TAU))
}

/// Equation of the equinoxes in radians: `EE = dPsi * cos(eps_mean)`.
#[inline]
pub fn equation_of_equinoxes(d_psi_rad: f64, mean_obliquity_rad: f64) -> f64 {
    d_psi_rad * mean_obliquity_rad.cos()
}

/// Greenwich Apparent Sidereal Time (GAST) IAU 2006/2000B in radians [0, 2π).
pub fn gast_iau2006(ut1_jd: f64, tt_jd: f64) -> Result<f64, OrientationError> {
    let gmst = gmst_iau2006(ut1_jd, tt_jd)?;
    let nut = iau2000b_nutation(tt_jd)?;
    let eps_mean = iau2006_mean_obliquity(tt_jd)?;
    let ee = equation_of_equinoxes(nut.d_psi_rad, eps_mean);
    Ok((gmst + ee).rem_euclid(TAU))
}

/// Computes the comprehensive `EclipticNutation` parameter set for `SE_ECL_NUT` compatibility.
pub fn ecliptic_nutation(tt_jd: f64) -> Result<EclipticNutation, OrientationError> {
    let nut = iau2000b_nutation(tt_jd)?;
    let eps_mean = iau2006_mean_obliquity(tt_jd)?;
    let eps_true = true_obliquity(eps_mean, nut.d_epsilon_rad);
    let ee = equation_of_equinoxes(nut.d_psi_rad, eps_mean);

    Ok(EclipticNutation {
        tt_jd,
        true_obliquity_rad: eps_true,
        mean_obliquity_rad: eps_mean,
        nutation_longitude_rad: nut.d_psi_rad,
        nutation_obliquity_rad: nut.d_epsilon_rad,
        equation_of_equinoxes_rad: ee,
    })
}

#[cfg(test)]
#[allow(clippy::excessive_precision, clippy::needless_range_loop)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "{a} != {b} (diff: {}, max tol: {tolerance})",
            (a - b).abs()
        );
    }

    #[test]
    fn exactly_77_full_series_nutation_terms() {
        assert_eq!(IAU2000B_TERMS.len(), 77);
    }

    #[test]
    fn matches_published_erfa_nut00b_reference() {
        // Reference epoch: 2006-01-15 00:00:00 TT (JD 2453736.5).
        let jd = 2_453_736.5;
        let nut = iau2000b_nutation(jd).expect("valid epoch");

        let expected_d_psi = -0.9632552291148362783e-5;
        let expected_d_eps = 0.4063197106621159367e-4;

        near(nut.d_psi_rad, expected_d_psi, 1e-13);
        near(nut.d_epsilon_rad, expected_d_eps, 1e-13);
        assert_eq!(nut.terms, 77);
    }

    #[test]
    fn matches_independent_erfa_num00b_3x3_matrix() {
        let jd = 2_453_736.5;
        let ref_mean_obliquity = 0.4090791789404229916; // ERFA pn00b reference obliquity
        let matrix = iau2000b_nutation_matrix(jd, Some(ref_mean_obliquity)).expect("valid matrix");

        let expected = [
            [
                0.9999999999536069682,
                0.8837746144871248011e-5,
                0.3831488838252202945e-5,
            ],
            [
                -0.8837590456632304720e-5,
                0.9999999991354692733,
                -0.4063198798559591654e-4,
            ],
            [
                -0.3831847930134941271e-5,
                0.4063195412258168380e-4,
                0.9999999991671806225,
            ],
        ];

        for i in 0..3 {
            for j in 0..3 {
                near(matrix[i][j], expected[i][j], 1e-13);
            }
        }
    }

    #[test]
    fn nutation_and_orientation_matrices_are_orthogonal() {
        let jd = 2_453_736.5;
        let matrices = [
            iau2000b_nutation_matrix(jd, None).unwrap(),
            iau2006_bias_precession_matrix(jd).unwrap(),
            true_equator_of_date_matrix(jd).unwrap(),
            true_ecliptic_of_date_matrix(jd).unwrap(),
        ];

        for m in matrices {
            let mt = matrix_transpose(m);
            let product = matrix_multiply(m, mt);
            for i in 0..3 {
                for j in 0..3 {
                    let target = if i == j { 1.0 } else { 0.0 };
                    near(product[i][j], target, 1e-12);
                }
            }
            // Length of unit test vector is preserved.
            let v: [f64; 3] = [1.0, 2.0, 3.0];
            let v_len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            let rotated = matrix_vector_multiply(m, v);
            let rot_len =
                (rotated[0] * rotated[0] + rotated[1] * rotated[1] + rotated[2] * rotated[2])
                    .sqrt();
            near(rot_len, v_len, 1e-12);
        }
    }

    #[test]
    fn swiss_ecl_nut_contract_has_consistent_obliquity_and_rates() {
        let jd = 2_453_736.5;
        let ecl = ecliptic_nutation(jd).expect("valid ecliptic nutation");
        near(
            ecl.true_obliquity_rad,
            ecl.mean_obliquity_rad + ecl.nutation_obliquity_rad,
            1e-15,
        );
        near(
            ecl.equation_of_equinoxes_rad,
            ecl.nutation_longitude_rad * ecl.mean_obliquity_rad.cos(),
            1e-15,
        );

        let swiss = ecl.to_swiss_ecl_nut();
        assert_eq!(swiss.len(), 6);
        near(swiss[0], ecl.true_obliquity_deg(), 1e-12);
        near(swiss[1], ecl.mean_obliquity_deg(), 1e-12);
        near(swiss[2], ecl.nutation_longitude_deg(), 1e-12);
        near(swiss[3], ecl.nutation_obliquity_deg(), 1e-12);
        near(swiss[4], ecl.equation_of_equinoxes_deg(), 1e-12);
        assert_eq!(swiss[5], 0.0);
    }

    #[test]
    fn rejects_unsupported_epochs_and_nonfinite_inputs() {
        assert!(matches!(
            iau2000b_nutation(f64::NAN),
            Err(OrientationError::NonFinite { field: "tt_jd" })
        ));
        assert!(matches!(
            iau2000b_nutation(2_400_000.0), // Before 1900
            Err(OrientationError::UnsupportedEpoch { .. })
        ));
        assert!(matches!(
            iau2000b_nutation(2_500_000.0), // After 2100
            Err(OrientationError::UnsupportedEpoch { .. })
        ));
        assert!(matches!(
            iau2000b_nutation_matrix(2_453_736.5, Some(f64::NAN)),
            Err(OrientationError::InvalidObliquity)
        ));
        assert!(matches!(
            era_iau2000(f64::INFINITY),
            Err(OrientationError::NonFinite { field: "ut1_jd" })
        ));
    }
}
