// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification 2026-11-17: opt-in historical polynomial estimate.
// Modification 2026-11-18: add separately bounded BCE and uncertainty estimates.
//! Espenak–Meeus 2006 polynomial estimate, explicitly bounded to years 0–3000.
//! Source: <https://eclipse.gsfc.nasa.gov/LEcat5/deltatpoly.html>.
//! The input is a caller-supplied decimal year, with astronomical numbering.
//! The output is an uncorrected historical model estimate, not an observation.
//! Values after 2005 are the publication's old projections, not current EOP.
//! No accuracy or uncertainty estimate is provided by this routine; the caller
//! must supply an explicit uncertainty separately when using the UT1 API.

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

/// A caller explicitly selected a historical estimate outside its stated domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoricalModelError {
    NonFinite,
    OutOfRange,
}
impl fmt::Display for HistoricalModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::NonFinite => "decimal year must be finite",
                Self::OutOfRange =>
                    "decimal year is outside this historical model's stated interval",
            }
        )
    }
}
impl Error for HistoricalModelError {}

fn polynomial_minus_500_to_500(year: f64) -> f64 {
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
}

/// Uncorrected Espenak–Meeus estimate, explicitly covering astronomical years
/// −1999 through 3000 (`-1999.0 <= year < 3001.0`). This is a model, not an
/// observation or an uncertainty. Its pre-−500 branch is an extrapolation;
/// the eclipse-canon lunar secular-acceleration correction is not applied.
/// Existing `espenak_meeus_2006_long_range` retains its original range.
pub fn espenak_meeus_2006_extended(year: f64) -> Result<f64, HistoricalModelError> {
    if !year.is_finite() {
        return Err(HistoricalModelError::NonFinite);
    }
    if !(-1999.0..3001.0).contains(&year) {
        return Err(HistoricalModelError::OutOfRange);
    }
    if year >= 0.0 {
        return espenak_meeus_2006_long_range(year).map_err(|_| HistoricalModelError::OutOfRange);
    }
    if year >= -500.0 {
        return Ok(polynomial_minus_500_to_500(year));
    }
    let u = (year - 1820.0) / 100.0;
    Ok(-20.0 + 32.0 * u * u)
}

/// Morrison–Stephenson (2004) approximate standard-error model in seconds,
/// as described by NASA: <https://eclipse.gsfc.nasa.gov/SEhelp/uncertainty2004.html>.
/// Conservatively restricted to astronomical decimal years [−999, 1200],
/// corresponding to the cited 1000 BCE–1200 CE interval. This is an estimated
/// standard error, not a guaranteed bound, a modern EOP value or an angular error.
/// No assumption is made that it applies to another ΔT model or source.
pub fn morrison_stephenson_2004_standard_error(year: f64) -> Result<f64, HistoricalModelError> {
    if !year.is_finite() {
        return Err(HistoricalModelError::NonFinite);
    }
    if !(-999.0..=1200.0).contains(&year) {
        return Err(HistoricalModelError::OutOfRange);
    }
    let t = (year - 1820.0) / 100.0;
    Ok(0.8 * t * t)
}

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
        polynomial_minus_500_to_500(year)
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
    fn extended_estimate_matches_published_branches_and_preserves_previous_api() {
        // Consistency with the published polynomial, not an Earth-rotation measurement.
        for (year, expected) in [
            (-1999.0, 46651.2352),
            (-1000.0, 25427.68),
            (-500.0, 17203.7),
            (-400.0, 15530.0),
        ] {
            assert!((espenak_meeus_2006_extended(year).unwrap() - expected).abs() < 5.0);
        }
        for year in [0.0, 0.5, 499.9, 500.0, 1600.0, 2000.0, 3000.999] {
            assert_eq!(
                espenak_meeus_2006_extended(year).unwrap().to_bits(),
                espenak_meeus_2006_long_range(year).unwrap().to_bits()
            );
        }
        assert!(espenak_meeus_2006_extended(-1999.0).is_ok());
        assert!(espenak_meeus_2006_extended(3000.999).is_ok());
        for year in [-1999.00001, 3001.0] {
            assert_eq!(
                espenak_meeus_2006_extended(year),
                Err(HistoricalModelError::OutOfRange)
            );
        }
        for year in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                espenak_meeus_2006_extended(year),
                Err(HistoricalModelError::NonFinite)
            );
        }
        assert_eq!(
            espenak_meeus_2006_long_range(-1.0),
            Err(LongRangeDeltaTError::OutOfRange)
        );
    }

    #[test]
    fn published_standard_error_is_explicit_and_bounded() {
        for (year, expected) in [
            (-999.0, 635.74088),
            (0.0, 264.992),
            (500.0, 139.392),
            (1000.0, 53.792),
            (1200.0, 30.752),
        ] {
            assert!(
                (morrison_stephenson_2004_standard_error(year).unwrap() - expected).abs() < 1e-8
            );
        }
        for year in [-999.00001, 1200.00001] {
            assert_eq!(
                morrison_stephenson_2004_standard_error(year),
                Err(HistoricalModelError::OutOfRange)
            );
        }
        for year in [f64::NAN, f64::INFINITY] {
            assert_eq!(
                morrison_stephenson_2004_standard_error(year),
                Err(HistoricalModelError::NonFinite)
            );
        }
    }
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
