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

//! Historical MHB2000 *planetary-series* lunar arguments (four, not five).
//!
//! The original IAU 2000A planetary nutation family uses simplified
//! mean lunar anomaly l, lunar latitude argument F, lunar elongation D,
//! and lunar node longitude Omega. Its planetary terms do NOT use l'
//! (mean solar anomaly). IERS Conventions (2010), section 5.7.3 describes
//! this historical difference from the full Delaunay polynomials.
//!
//! Original Rust evaluation of published scientific coefficients, not
//! copied or translated SOFA/ERFA/Swiss software. No periodic terms,
//! nutation amplitudes or complete IAU 2000A evaluation is implemented.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;

use crate::calendar::{JulianDate, TimeScale};

const J2000_TDB_JD: f64 = 2_451_545.0;
const DAYS_PER_JULIAN_CENTURY: f64 = 36_525.0;
const MAX_TESTED_CENTURIES: f64 = 2.0;

/// Strict ordering of the four historical planetary-series lunar arguments.
/// The solar anomaly l' has deliberately **no** index.
pub const MOON_MEAN_ANOMALY: usize = 0;
pub const MOON_ARGUMENT_OF_LATITUDE: usize = 1;
pub const MOON_SUN_ELONGATION: usize = 2;
pub const MOON_ASCENDING_NODE: usize = 3;

/// Constant and linear rate, in radians and radians per Julian century.
/// Documented original MHB2000 simplified planetary-nutation expressions;
/// these are distinct from the five full quartic Delaunay arguments.
///
/// Source convention: IERS Conventions (2010), section 5.7.3, and the
/// original MHB2000 planetary nutation model as documented by IAU SOFA.
/// Using the same scientific coefficients does not copy library source.
const SIMPLIFIED_LUNAR: [[f64; 2]; 4] = [
    [2.355_555_98, 8_328.691_426_955_4],
    [1.627_905_234, 8_433.466_158_131],
    [5.198_466_741, 7_771.377_146_812_1],
    [2.182_439_20, -33.757_045],
];

/// Angles in [0, 2π), and rates of **unwrapped** angles in rad/TDB day.
///
/// These four arguments are specific to the MHB2000 **planetary** term
/// convention. Do not reuse them for the 678 full lunar-solar terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MhbPlanetaryLunarArguments {
    pub angles_rad: [f64; 4],
    pub rates_rad_per_tdb_day: [f64; 4],
}

/// Mathematical errors and explicit convention failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MhbPlanetaryLunarError {
    WrongTimeScale,
    UnrepresentableEpoch,
    OutsideTestedInterval,
    /// The historical MHB2000 planetary table has no solar-anomaly term.
    UnsupportedSolarAnomalyMultiplier,
    NonFinitePlanetaryArgument,
    UnnormalizedPlanetaryArgument,
    ArithmeticOverflow,
}

impl fmt::Display for MhbPlanetaryLunarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongTimeScale => write!(f, "MHB planetary lunar arguments require TDB"),
            Self::UnrepresentableEpoch => write!(f, "unsafe split TDB Julian date"),
            Self::OutsideTestedInterval => {
                write!(f, "outside tested J2000 +/-2 TDB-century range")
            }
            Self::UnsupportedSolarAnomalyMultiplier => {
                write!(f, "MHB planetary term cannot multiply the solar anomaly l'")
            }
            Self::NonFinitePlanetaryArgument => write!(f, "nonfinite planetary phase or rate"),
            Self::UnnormalizedPlanetaryArgument => {
                write!(f, "planetary phases must be normalized to [0,2pi)")
            }
            Self::ArithmeticOverflow => write!(f, "nonfinite planetary harmonic phase or rate"),
        }
    }
}
impl Error for MhbPlanetaryLunarError {}

impl MhbPlanetaryLunarArguments {
    /// Compose a harmonic phase from four multipliers in `[l,F,D,Omega]` order.
    /// The output derivative is signed rad/TDB day and phase is [0,2π).
    pub fn phase(&self, multipliers: [i8; 4]) -> (f64, f64) {
        let mut phase = 0.0;
        let mut rate = 0.0;
        for (index, coefficient) in multipliers.iter().enumerate() {
            phase += f64::from(*coefficient) * self.angles_rad[index];
            rate += f64::from(*coefficient) * self.rates_rad_per_tdb_day[index];
        }
        (phase.rem_euclid(TAU), rate)
    }

    /// Compose one *planetary* term with the nine additional F6..F14 phases.
    ///
    /// `multipliers` are in the full 14-slot IERS order
    /// [l,l',F,D,Omega,Mercury,Venus,Earth,Mars,Jupiter,Saturn,Uranus,
    /// Neptune,general precession]. Slot 1 (l') **must be zero** for the
    /// historical MHB2000 planetary series. This method checks it rather
    /// than silently inserting a fabricated solar argument.
    ///
    /// Additional nine planetary phases/rates are caller-supplied, at the
    /// **same TDB epoch** and in the required F6..F14 order; this function
    /// does not calculate them or validate their matching timestamps.
    /// In the original MHB2000 series Neptune has a historical distinct
    /// expression (see the independent planetary-argument draft PR).
    pub fn phase_with_planets(
        &self,
        multipliers: [i8; 14],
        planetary_angles_rad: [f64; 9],
        planetary_rates_rad_per_tdb_day: [f64; 9],
    ) -> Result<(f64, f64), MhbPlanetaryLunarError> {
        if multipliers[1] != 0 {
            return Err(MhbPlanetaryLunarError::UnsupportedSolarAnomalyMultiplier);
        }
        for (angle, rate) in planetary_angles_rad
            .iter()
            .zip(planetary_rates_rad_per_tdb_day)
        {
            if !angle.is_finite() || !rate.is_finite() {
                return Err(MhbPlanetaryLunarError::NonFinitePlanetaryArgument);
            }
            if !(0.0..TAU).contains(angle) {
                return Err(MhbPlanetaryLunarError::UnnormalizedPlanetaryArgument);
            }
        }
        let (mut phase, mut rate) = self.phase([
            multipliers[0],
            multipliers[2],
            multipliers[3],
            multipliers[4],
        ]);
        for i in 0..9 {
            phase += f64::from(multipliers[5 + i]) * planetary_angles_rad[i];
            rate += f64::from(multipliers[5 + i]) * planetary_rates_rad_per_tdb_day[i];
        }
        if !phase.is_finite() || !rate.is_finite() {
            return Err(MhbPlanetaryLunarError::ArithmeticOverflow);
        }
        Ok((phase.rem_euclid(TAU), rate))
    }
}

/// Evaluate the simplified **MHB2000 planetary-series** lunar arguments.
///
/// `T=(JD_TDB-2451545.0)/36525` is evaluated with the split Julian date.
/// There is **no** TT↔TDB conversion: TDB is the explicit numeric time
/// parameter used consistently with the still-draft companion argument
/// modules. The conventional full `Nut00a` reference uses **TT** for its
/// input date; scientific time-rate reconciliation remains outstanding.
///
/// J2000 +/-2 TDB centuries is only a local engineering guard, **not**
/// a model validity bound. This function is not a nutation model itself.
pub fn mhb2000_planetary_lunar_arguments(
    tdb_epoch: JulianDate,
) -> Result<MhbPlanetaryLunarArguments, MhbPlanetaryLunarError> {
    if tdb_epoch.scale() != TimeScale::Tdb {
        return Err(MhbPlanetaryLunarError::WrongTimeScale);
    }
    let (whole, fraction) = tdb_epoch
        .parts()
        .map_err(|_| MhbPlanetaryLunarError::UnrepresentableEpoch)?;
    let centuries = ((whole - J2000_TDB_JD) + fraction) / DAYS_PER_JULIAN_CENTURY;
    if !centuries.is_finite() {
        return Err(MhbPlanetaryLunarError::UnrepresentableEpoch);
    }
    if centuries.abs() > MAX_TESTED_CENTURIES {
        return Err(MhbPlanetaryLunarError::OutsideTestedInterval);
    }
    let mut angles_rad = [0.0_f64; 4];
    let mut rates_rad_per_tdb_day = [0.0_f64; 4];
    for (i, [initial, slope]) in SIMPLIFIED_LUNAR.into_iter().enumerate() {
        angles_rad[i] = (initial + slope * centuries).rem_euclid(TAU);
        rates_rad_per_tdb_day[i] = slope / DAYS_PER_JULIAN_CENTURY;
    }
    Ok(MhbPlanetaryLunarArguments {
        angles_rad,
        rates_rad_per_tdb_day,
    })
}
