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

//! Nine planetary-nutation fundamental arguments (IAU 2000A preparation).
//!
//! Original Rust evaluation of the publicly specified IERS Conventions (2010)
//! Chapter 5, section 5.7.3, Eq. (5.44): eight planetary mean longitudes
//! and general precession in longitude.  A separate explicit MHB2000
//! convention handles the historically different Neptune polynomial.
//!
//! This is NOT a periodic nutation series or an astronomical ephemeris:
//! it does not evaluate delta-psi/delta-epsilon, and does not reproduce the
//! simplified lunar arguments used by the original MHB2000 planetary terms.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;

use crate::calendar::{JulianDate, TimeScale};

const J2000_JD_TDB: f64 = 2_451_545.0;
const JULIAN_CENTURY_TDB_DAYS: f64 = 36_525.0;
const MAX_ABS_CENTURIES: f64 = 2.0;

/// Ordered `[Mercury, Venus, Earth, Mars, Jupiter, Saturn, Uranus,
/// Neptune, general precession in longitude]`, corresponding to F6..F14
/// of IERS Conventions 2010, Eq. 5.44.
pub const MERCURY_MEAN_LONGITUDE: usize = 0;
pub const VENUS_MEAN_LONGITUDE: usize = 1;
pub const EARTH_MEAN_LONGITUDE: usize = 2;
pub const MARS_MEAN_LONGITUDE: usize = 3;
pub const JUPITER_MEAN_LONGITUDE: usize = 4;
pub const SATURN_MEAN_LONGITUDE: usize = 5;
pub const URANUS_MEAN_LONGITUDE: usize = 6;
pub const NEPTUNE_MEAN_LONGITUDE: usize = 7;
pub const GENERAL_PRECESSION_IN_LONGITUDE: usize = 8;

/// Constant and linear rate in radians and radians per TDB Julian century.
/// These are fundamental nutation arguments, NOT actual planetary ecliptic
/// longitudes from DE441 or observed positions.
const MEAN_LONGITUDES: [[f64; 2]; 8] = [
    [4.402_608_842, 2_608.790_314_157_4],
    [3.176_146_697, 1_021.328_554_621_1],
    [1.753_470_314, 628.307_584_999_1],
    [6.203_480_913, 334.061_242_670_0],
    [0.599_546_497, 52.969_096_264_1],
    [0.874_016_757, 21.329_910_496_0],
    [5.481_293_872, 7.478_159_856_7],
    [5.311_886_287, 3.813_303_563_8],
];

/// The original IAU 2000A MHB2000 planetary series used a distinct Neptune
/// mean-longitude expression, explicitly noted by IERS after Eq. (5.44).
const MHB2000_NEPTUNE: [f64; 2] = [5.321_159_000, 3.812_777_400];

/// Required convention for the Neptune polynomial. No implicit fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetaryArgumentConvention {
    /// IERS Conventions 2010, Eq. (5.44), and ERFA standalone FaNe03.
    Iers2010,
    /// Historical MHB2000/SOFA IAU 2000A planetary-series convention.
    Mhb2000,
}

/// Eight planetary mean-longitude arguments plus general precession.
/// All angles are normalized to [0, 2π), with signed derivatives of the
/// unwrapped polynomials in radians per **TDB day**.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetaryArguments {
    pub angles_rad: [f64; 9],
    pub rates_rad_per_tdb_day: [f64; 9],
}

impl PlanetaryArguments {
    /// Form a planetary-series phase and its derivative from nine integer
    /// coefficients in F6..F14 order. This is only a phase calculation.
    ///
    /// A full planetary IAU 2000A term additionally needs F1..F5 lunar
    /// arguments and the independently licensed harmonic coefficient tables.
    pub fn phase(&self, multipliers: [i8; 9]) -> (f64, f64) {
        let mut phase = 0.0;
        let mut rate = 0.0;
        for (i, coefficient) in multipliers.iter().enumerate() {
            phase += f64::from(*coefficient) * self.angles_rad[i];
            rate += f64::from(*coefficient) * self.rates_rad_per_tdb_day[i];
        }
        (phase.rem_euclid(TAU), rate)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetaryArgumentError {
    WrongTimeScale,
    UnrepresentableEpoch,
    OutsideTestedInterval,
}

impl fmt::Display for PlanetaryArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongTimeScale => write!(f, "planetary nutation arguments require TDB epoch"),
            Self::UnrepresentableEpoch => {
                write!(f, "planetary nutation epoch cannot be represented safely")
            }
            Self::OutsideTestedInterval => {
                write!(f, "epoch outside J2000 ±2 TDB-Julian-century test interval")
            }
        }
    }
}

impl Error for PlanetaryArgumentError {}

/// Evaluate the nine planetary nutation arguments and analytic rates.
///
/// A **split** JulianDate explicitly labeled TDB is mandatory: no UTC, TAI,
/// TT, UT1 or TT↔TDB conversion is performed. `T` is TDB Julian centuries
/// from J2000 noon. The guarded ±2-century interval is an engineering test
/// limit, not an official IAU model accuracy or validity guarantee.
///
/// Source of scientific formulae: IERS Conventions (2010), §5.7.3,
/// Eq. (5.44), and its note on the original MHB2000 Neptune expression.
/// No ERFA/SOFA source code or external ephemeris files are used.
pub fn planetary_fundamental_arguments(
    tdb_epoch: JulianDate,
    convention: PlanetaryArgumentConvention,
) -> Result<PlanetaryArguments, PlanetaryArgumentError> {
    if tdb_epoch.scale() != TimeScale::Tdb {
        return Err(PlanetaryArgumentError::WrongTimeScale);
    }
    let (whole, fraction) = tdb_epoch
        .parts()
        .map_err(|_| PlanetaryArgumentError::UnrepresentableEpoch)?;
    let centuries = ((whole - J2000_JD_TDB) + fraction) / JULIAN_CENTURY_TDB_DAYS;
    if !centuries.is_finite() {
        return Err(PlanetaryArgumentError::UnrepresentableEpoch);
    }
    if centuries.abs() > MAX_ABS_CENTURIES {
        return Err(PlanetaryArgumentError::OutsideTestedInterval);
    }

    let mut angles_rad = [0.0_f64; 9];
    let mut rates_rad_per_tdb_day = [0.0_f64; 9];
    for i in 0..8 {
        let [initial, slope] =
            if i == NEPTUNE_MEAN_LONGITUDE && convention == PlanetaryArgumentConvention::Mhb2000 {
                MHB2000_NEPTUNE
            } else {
                MEAN_LONGITUDES[i]
            };
        angles_rad[i] = (initial + slope * centuries).rem_euclid(TAU);
        rates_rad_per_tdb_day[i] = slope / JULIAN_CENTURY_TDB_DAYS;
    }

    // IERS 2010 Eq. (5.44): general precession in longitude (radians),
    // not an ecliptic-orientation matrix nor a true-of-date transform.
    const P_A_LINEAR: f64 = 0.024_381_75;
    const P_A_QUADRATIC: f64 = 0.000_005_386_91;
    let p_a = (P_A_LINEAR + P_A_QUADRATIC * centuries) * centuries;
    angles_rad[GENERAL_PRECESSION_IN_LONGITUDE] = p_a.rem_euclid(TAU);
    rates_rad_per_tdb_day[GENERAL_PRECESSION_IN_LONGITUDE] =
        (P_A_LINEAR + 2.0 * P_A_QUADRATIC * centuries) / JULIAN_CENTURY_TDB_DAYS;

    Ok(PlanetaryArguments {
        angles_rad,
        rates_rad_per_tdb_day,
    })
}
