// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification 2026-11-17: opt-in historical polynomial estimate.
//! Espenak–Meeus 2006 polynomial estimate, explicitly bounded to years 0–3000.
//! Source: <https://eclipse.gsfc.nasa.gov/LEcat5/deltatpoly.html>.
//! The input is a caller-supplied decimal year, with astronomical numbering.
//! The output is an uncorrected historical model estimate, not an observation.
//! Values after 2005 are the publication's old projections, not current EOP.
//! No accuracy or uncertainty estimate is provided by this routine; the caller
//! must supply an explicit uncertainty separately when using the UT1 API.
//! The 2026 reference audit reports material disagreement with modern USNO
//! observations and a published 1514 eclipse constraint; see
//! `docs/DE441_DELTAT_REFERENCE_VALIDATION_2026-11-18.md`. No model was
//! silently replaced and the audit does not establish historical truth.

use crate::delta_t::delta_t_seconds_1900_1972;
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LongRangeDeltaTError {
    NonFinite,
    OutOfRange,
}
impl fmt::Display for LongRangeDeltaTError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::NonFinite => "decimal year must be finite",
                Self::OutOfRange => "decimal year must be in [0, 3001)",
            }
        )
    }
}
impl Error for LongRangeDeltaTError {}

/// Estimate TT−UT1 in seconds for `0.0 <= decimal_year < 3001.0`.
/// The eclipse-canon lunar secular-acceleration correction is not applied.
pub fn espenak_meeus_2006_long_range(year: f64) -> Result<f64, LongRangeDeltaTError> {
    if !year.is_finite() {
        return Err(LongRangeDeltaTError::NonFinite);
    }
    if !(0.0..3001.0).contains(&year) {
        return Err(LongRangeDeltaTError::OutOfRange);
    }
    let value = if year < 500.0 {
        let u = year / 100.0;
        [
            10583.6,
            -1014.41,
            33.78311,
            -5.952053,
            -0.1798452,
            0.022174192,
            0.0090316521,
        ]
        .iter()
        .rev()
        .fold(0.0, |a, c| a * u + c)
    } else if year < 1600.0 {
        let u = (year - 1000.0) / 100.0;
        [
            1574.2,
            -556.01,
            71.23472,
            0.319781,
            -0.8503463,
            -0.005050998,
            0.0083572073,
        ]
        .iter()
        .rev()
        .fold(0.0, |a, c| a * u + c)
    } else if year < 1700.0 {
        let t = year - 1600.0;
        120.0 - 0.9808 * t - 0.01532 * t * t + t.powi(3) / 7129.0
    } else if year < 1800.0 {
        let t = year - 1700.0;
        8.83 + 0.1603 * t - 0.0059285 * t * t + 0.00013336 * t.powi(3) - t.powi(4) / 1_174_000.0
    } else if year < 1860.0 {
        let t = year - 1800.0;
        [
            13.72,
            -0.332447,
            0.0068612,
            0.0041116,
            -0.00037436,
            0.0000121272,
            -0.0000001699,
            0.000000000875,
        ]
        .iter()
        .rev()
        .fold(0.0, |a, c| a * t + c)
    } else if year < 1900.0 {
        let t = year - 1860.0;
        7.62 + 0.5737 * t - 0.251754 * t * t + 0.01680668 * t.powi(3) - 0.0004473624 * t.powi(4)
            + t.powi(5) / 233174.0
    } else if year < 1972.0 {
        return delta_t_seconds_1900_1972(year).map_err(|_| LongRangeDeltaTError::OutOfRange);
    } else if year < 1986.0 {
        let t = year - 1975.0;
        45.45 + 1.067 * t - t * t / 260.0 - t.powi(3) / 718.0
    } else if year < 2005.0 {
        let t = year - 2000.0;
        63.86 + 0.3345 * t - 0.060374 * t * t
            + 0.0017275 * t.powi(3)
            + 0.000651814 * t.powi(4)
            + 0.00002373599 * t.powi(5)
    } else if year < 2050.0 {
        let t = year - 2000.0;
        62.92 + 0.32217 * t + 0.005589 * t * t
    } else if year < 2150.0 {
        let u = (year - 1820.0) / 100.0;
        -20.0 + 32.0 * u * u - 0.5628 * (2150.0 - year)
    } else {
        let u = (year - 1820.0) / 100.0;
        -20.0 + 32.0 * u * u
    };
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_piecewise_anchor_values() {
        for (year, expected) in [
            (0.0, 10583.6),
            (1000.0, 1574.2),
            (1600.0, 120.0),
            (1700.0, 8.83),
            (1800.0, 13.72),
            (1860.0, 7.62),
            (1900.0, -2.79),
            (1920.0, 21.20),
            (1950.0, 29.07),
            (1975.0, 45.45),
            (2000.0, 63.86),
            (2050.0, 93.0),
            (2150.0, 328.48),
            (3000.0, 4435.68),
        ] {
            assert!((espenak_meeus_2006_long_range(year).unwrap() - expected).abs() < 1e-8);
        }
    }
    #[test]
    fn rejects_outside_scope_and_nonfinite() {
        assert_eq!(
            espenak_meeus_2006_long_range(-0.1),
            Err(LongRangeDeltaTError::OutOfRange)
        );
        assert_eq!(
            espenak_meeus_2006_long_range(3001.0),
            Err(LongRangeDeltaTError::OutOfRange)
        );
        assert_eq!(
            espenak_meeus_2006_long_range(f64::NAN),
            Err(LongRangeDeltaTError::NonFinite)
        );
        assert!(espenak_meeus_2006_long_range(3000.999).is_ok());
    }
    #[test]
    fn agrees_with_selected_nasa_published_historical_table_points() {
        // NASA Table 1 historical spline estimates and Table 2 observations;
        // these are source-model consistency checks, not independent truth.
        for (year, reference) in [
            (0.0, 10580.0),
            (500.0, 5710.0),
            (1000.0, 1570.0),
            (1500.0, 200.0),
            (1600.0, 120.0),
            (1700.0, 9.0),
            (1800.0, 14.0),
            (1900.0, -3.0),
            (1950.0, 29.0),
            (2000.0, 63.8),
        ] {
            assert!((espenak_meeus_2006_long_range(year).unwrap() - reference).abs() <= 5.0);
        }
    }
}
