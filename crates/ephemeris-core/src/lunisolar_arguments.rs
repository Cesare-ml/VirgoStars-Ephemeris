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

//! Five lunar-solar Delaunay fundamental arguments for IAU nutation series.
//!
//! Independently evaluates scientific polynomials from the IERS Conventions
//! (2010), Chapter 5, Eq. 5.43. The l' and D constant terms can also use the
//! original MHB2000 decimal rounding documented in IERS section 5.7.2,
//! selected explicitly to reproduce the IAU 2000A nutation-series convention.
//! This module has NO IAU 2000A periodic coefficient tables or actual
//! delta-psi/delta-epsilon evaluation. No third-party program code reused.

use std::error::Error;
use std::f64::consts::{PI, TAU};
use std::fmt;

use crate::calendar::{JulianDate, TimeScale};

const J2000_JD_TDB: f64 = 2_451_545.0;
const JULIAN_CENTURY_DAYS: f64 = 36_525.0;
const ARCSECOND_TO_RADIAN: f64 = PI / (180.0 * 3_600.0);
const ARCSECONDS_PER_TURN: f64 = 1_296_000.0;

/// Restricted engineering test interval, not an official IAU validity bound.
const MAX_ABS_CENTURIES: f64 = 2.0;

/// Indices in Delaunay order `[l, l_prime, F, D, Omega]`.
pub const MOON_MEAN_ANOMALY: usize = 0;
pub const SUN_MEAN_ANOMALY: usize = 1;
pub const MOON_ARGUMENT_OF_LATITUDE: usize = 2;
pub const MOON_SUN_ELONGATION: usize = 3;
pub const MOON_ASCENDING_NODE: usize = 4;

/// Public scientific coefficients in **arcseconds** and powers of Julian
/// centuries from J2000; each row is c0..c4 in ascending power order.
///
/// Source: IERS Conventions 2010, Eq. 5.43 (unrounded constants).
/// MHB2000 differs by +2 microarcsec for l' and -2 microarcsec for D.
/// The explicit convention below selects these historical variants.
const LUNISOLAR_POLYNOMIALS_ARCSEC: [[f64; 5]; 5] = [
    [
        485_868.249_036,
        1_717_915_923.217_8,
        31.879_2,
        0.051_635,
        -0.000_244_70,
    ],
    [
        1_287_104.793_048,
        129_596_581.048_1,
        -0.553_2,
        0.000_136,
        -0.000_011_49,
    ],
    [
        335_779.526_232,
        1_739_527_262.847_8,
        -12.751_2,
        -0.001_037,
        0.000_004_17,
    ],
    [
        1_072_260.703_692,
        1_602_961_601.209_0,
        -6.370_6,
        0.006_593,
        -0.000_031_69,
    ],
    [
        450_160.398_036,
        -6_962_890.543_1,
        7.472_2,
        0.007_702,
        -0.000_059_39,
    ],
];

/// Choice of the two historically distinct constant terms in the fundamental
/// argument polynomials. Always choose explicitly; neither variant silently
/// substitutes for the other in a high-precision IAU 2000A comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LuniSolarConvention {
    /// IERS Conventions 2010, Eq. 5.43 (Sun and D unrounded constants).
    /// Matches the ERFA standalone Falp03/Fad03 regression test inputs.
    Iers2010,
    /// Original MHB2000/SOFA Nut00a historical rounding for Sun and D.
    /// Each differs from IERS Eq. 5.43 by exactly 2 microarcseconds.
    Mhb2000,
}

/// Five fundamental arguments and their rates, in Delaunay order.
///
/// `angles_rad` are reduced to **[0, 2π)**. `rates_rad_per_tdb_day`
/// are the signed derivatives of the *unwrapped* polynomials, so wrapping
/// cannot create a spurious rate discontinuity.
///
/// All values refer to the same TDB epoch. This is not the actual nutation
/// in longitude/obliquity and is not a complete IAU 2000A evaluator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LuniSolarArguments {
    pub angles_rad: [f64; 5],
    pub rates_rad_per_tdb_day: [f64; 5],
}

impl LuniSolarArguments {
    /// Produce the phase and instantaneous phase rate of a lunar-solar term.
    ///
    /// `multipliers` is the five integer coefficients, ordered as
    /// `[l, l_prime, F, D, Omega]`. Returned phase is in [0,2π);
    /// its signed derivative is rad/TDB day. This does **not** apply a
    /// periodic-series amplitude or compute nutation angles.
    pub fn phase(&self, multipliers: [i8; 5]) -> (f64, f64) {
        let mut angle = 0.0;
        let mut rate = 0.0;
        for (i, multiplier) in multipliers.iter().enumerate() {
            angle += f64::from(*multiplier) * self.angles_rad[i];
            rate += f64::from(*multiplier) * self.rates_rad_per_tdb_day[i];
        }
        (angle.rem_euclid(TAU), rate)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LuniSolarError {
    WrongTimeScale,
    UnrepresentableEpoch,
    OutsideTestedInterval,
}

impl fmt::Display for LuniSolarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongTimeScale => write!(f, "lunisolar argument polynomials require TDB epoch"),
            Self::UnrepresentableEpoch => {
                write!(f, "lunisolar argument epoch cannot be represented safely")
            }
            Self::OutsideTestedInterval => {
                write!(f, "epoch outside J2000 ±2 TDB-Julian-century test interval")
            }
        }
    }
}

impl Error for LuniSolarError {}

/// Evaluate the five **lunar-solar** Delaunay arguments and analytic rates.
///
/// Input: already-TDB split Julian date, J2000 = JD 2451545.0 **TDB**.
/// TDB centuries `T = (JD_TDB - 2451545.0) / 36525`.
/// There is no conversion from UTC/TT/TAI/UT1, and planetary fundamental
/// arguments F6..F14 are NOT included.
///
/// The polynomial coefficients are the published physical model rather than
/// implementation code from SOFA/ERFA/Swiss. The conservative ±2-century
/// boundary is a project test policy, NOT an IAU model validity statement.
pub fn lunisolar_fundamental_arguments(
    tdb_epoch: JulianDate,
    convention: LuniSolarConvention,
) -> Result<LuniSolarArguments, LuniSolarError> {
    if tdb_epoch.scale() != TimeScale::Tdb {
        return Err(LuniSolarError::WrongTimeScale);
    }
    let (whole, fraction) = tdb_epoch
        .parts()
        .map_err(|_| LuniSolarError::UnrepresentableEpoch)?;
    let centuries = ((whole - J2000_JD_TDB) + fraction) / JULIAN_CENTURY_DAYS;
    if !centuries.is_finite() {
        return Err(LuniSolarError::UnrepresentableEpoch);
    }
    if centuries.abs() > MAX_ABS_CENTURIES {
        return Err(LuniSolarError::OutsideTestedInterval);
    }

    let mut angles = [0.0; 5];
    let mut rates = [0.0; 5];
    for (index, coefficients) in LUNISOLAR_POLYNOMIALS_ARCSEC.iter().enumerate() {
        let mut value_arcsec = coefficients[4];
        let mut rate_arcsec_per_century = 0.0;
        for coefficient in coefficients[..4].iter().rev() {
            rate_arcsec_per_century = rate_arcsec_per_century * centuries + value_arcsec;
            value_arcsec = value_arcsec * centuries + *coefficient;
        }
        // Reduce the potentially billion-arcsecond unwrapped polynomial
        // BEFORE adding the historical 2-microarcsecond constant offsets.
        // Adding those tiny offsets to the huge unwrapped result would lose
        // meaningful digits in f64 and damage explicit MHB/IERS parity.
        let mut reduced_arcsec = value_arcsec.rem_euclid(ARCSECONDS_PER_TURN);
        if convention == LuniSolarConvention::Mhb2000 {
            if index == SUN_MEAN_ANOMALY {
                reduced_arcsec += 0.000_002;
            } else if index == MOON_SUN_ELONGATION {
                reduced_arcsec -= 0.000_002;
            }
        }
        angles[index] = reduced_arcsec.rem_euclid(ARCSECONDS_PER_TURN) * ARCSECOND_TO_RADIAN;
        rates[index] = rate_arcsec_per_century * ARCSECOND_TO_RADIAN / JULIAN_CENTURY_DAYS;
    }
    Ok(LuniSolarArguments {
        angles_rad: angles,
        rates_rad_per_tdb_day: rates,
    })
}
