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

//! ΔT = TT − UT1 for 1900–1972, from the Espenak & Meeus polynomial fits.
//!
//! Source: F. Espenak and J. Meeus, *Five Millennium Canon of Solar Eclipses:
//! -1999 to +3000*, NASA/TP-2006-214141, "Polynomial Expressions for Delta T"
//! (the polynomials are published mathematical fits to historical
//! observations; this file contains no table copied from software).
//!
//! # Scope and uncertainty
//!
//! Only 1900.0 ≤ year < 1972.0 is provided. From 1972 the clock is UTC and
//! the IERS leap-second table gives TT − UTC exactly. The polynomial joins
//! that table at 1972.0 to about 0.07 s (checked in the tests against
//! TT − UTC = 42.184 s). The fits describe a smoothed history; real ΔT
//! deviates from them by a few tenths of a second before about 1955. Nothing
//! here predicts ΔT: dates outside the range return an error.

use std::error::Error;
use std::fmt;

pub const DELTA_T_1900_1972_MODEL_ID: &str = "Espenak-Meeus-2006-polynomials-1900-1972";
const MIN_YEAR: f64 = 1900.0;
const MAX_YEAR: f64 = 1972.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeltaTError {
    NonFinite,
    /// The decimal year is outside 1900.0 ≤ y < 1972.0.
    OutOfRange,
}

impl fmt::Display for DeltaTError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => write!(f, "non-finite decimal year"),
            Self::OutOfRange => write!(f, "decimal year outside 1900.0 <= y < 1972.0"),
        }
    }
}
impl Error for DeltaTError {}

/// Decimal year of a UT Julian date (Gregorian mean year of 365.2425 days).
pub fn decimal_year_from_jd(jd_ut: f64) -> f64 {
    2000.0 + (jd_ut - 2_451_544.5) / 365.2425
}

/// ΔT in seconds for a decimal year in `[1900, 1972)`.
pub fn delta_t_seconds_1900_1972(year: f64) -> Result<f64, DeltaTError> {
    if !year.is_finite() {
        return Err(DeltaTError::NonFinite);
    }
    if !(MIN_YEAR..MAX_YEAR).contains(&year) {
        return Err(DeltaTError::OutOfRange);
    }
    let value = if year < 1920.0 {
        let t = year - 1900.0;
        -2.79 + 1.494119 * t - 0.0598939 * t * t + 0.0061966 * t.powi(3) - 0.000197 * t.powi(4)
    } else if year < 1941.0 {
        let t = year - 1920.0;
        21.20 + 0.84493 * t - 0.076100 * t * t + 0.0020936 * t.powi(3)
    } else if year < 1961.0 {
        let t = year - 1950.0;
        29.07 + 0.407 * t - t * t / 233.0 + t.powi(3) / 2547.0
    } else {
        let t = year - 1975.0;
        45.45 + 1.067 * t - t * t / 260.0 - t.powi(3) / 718.0
    };
    Ok(value)
}
