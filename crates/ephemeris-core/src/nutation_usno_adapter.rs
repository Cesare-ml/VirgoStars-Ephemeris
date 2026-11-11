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

//! Explicit USNO Circular 179 **printed-row** to VirgoStars term conversion.
//!
//! This module does not ship the 1,365 official coefficient rows, read a
//! file, assert that table reuse is authorized, or compute full nutation.
//! It maps caller-supplied numerical rows and arcsecond units into the
//! canonical, radians-based arithmetic types of the harmonic kernel.
//!
//! Source: G. H. Kaplan, USNO Circular 179 (2005), Appendix E, pp. 88ff.
//! <https://aa.usno.navy.mil/publications/Circular_179>
//! No SOFA/ERFA/Swiss/NAIF implementation code is imported or adapted.

use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

use crate::nutation_harmonics::{LuniSolarHarmonicTerm, PlanetaryHarmonicTerm};

const ARCSECOND_TO_RADIAN: f64 = PI / (180.0 * 3_600.0);

/// One **caller-supplied** numerical row in Circular 179 Appendix E order.
///
/// `printed_term_number` uses the *publication's* global 1..=1365
/// numbering: lunisolar = 1..=678, planetary = 679..=1365.
/// `multipliers` correspond to Circular 179 columns j=1..14:
/// `[Mercury,Venus,Earth,Mars,Jupiter,Saturn,Uranus,Neptune,pA,
///    l,lprime,F,D,Omega]`.
///
/// `coefficients_arcseconds` are **six arcsecond amplitudes**, in order
/// `[S,Sdot,Cprime,C,Cdot,Sprime]`. The two dot fields are
/// arcseconds **per Julian century**. No radians or microarcsecond input
/// is accepted by this explicitly arcsecond-denominated structure.
///
/// This structure does not record a verified source digest or license;
/// values must be independently checked against the identified source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UsnoPrintedHarmonicRow {
    pub printed_term_number: usize,
    pub multipliers: [i8; 14],
    pub coefficients_arcseconds: [f64; 6],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsnoConversionError {
    WrongPrintedTermRange,
    LunisolarHasPlanetaryMultiplier,
    PlanetaryHasSolarAnomalyMultiplier,
    PlanetaryHasAmplitudeSlope,
    NonFiniteCoefficient,
    ImplausibleCoefficient,
}

impl fmt::Display for UsnoConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongPrintedTermRange => {
                write!(f, "term index does not belong to the requested USNO family")
            }
            Self::LunisolarHasPlanetaryMultiplier => {
                write!(f, "lunisolar USNO term unexpectedly multiplies a planetary argument")
            }
            Self::PlanetaryHasSolarAnomalyMultiplier => {
                write!(f, "MHB2000 planetary term cannot multiply lunar argument lprime")
            }
            Self::PlanetaryHasAmplitudeSlope => {
                write!(f, "MHB2000 planetary term must have zero secular amplitude slopes")
            }
            Self::NonFiniteCoefficient => write!(f, "nonfinite USNO angular coefficient"),
            Self::ImplausibleCoefficient => {
                write!(f, "USNO coefficient exceeds the canonical one-radian safety guard")
            }
        }
    }
}

impl Error for UsnoConversionError {}

fn radians(row: &UsnoPrintedHarmonicRow) -> Result<[f64; 6], UsnoConversionError> {
    let mut result = [0.0; 6];
    for (index, arcseconds) in row.coefficients_arcseconds.iter().enumerate() {
        if !arcseconds.is_finite() {
            return Err(UsnoConversionError::NonFiniteCoefficient);
        }
        let rad = *arcseconds * ARCSECOND_TO_RADIAN;
        // This is the same generous bound used by the canonical v1 parser;
        // it catches unconverted data and is not an accuracy certificate.
        if !rad.is_finite() || rad.abs() > 1.0 {
            return Err(UsnoConversionError::ImplausibleCoefficient);
        }
        result[index] = rad;
    }
    Ok(result)
}

/// Convert a caller-supplied **lunisolar** USNO printed row.
///
/// Requires publication index 1..=678 and all nine printed planetary
/// multipliers zero. The **last five** printed multipliers are copied
/// into internal order `[l,lprime,F,D,Omega]`.
pub fn convert_usno_lunisolar_row(
    row: UsnoPrintedHarmonicRow,
) -> Result<LuniSolarHarmonicTerm, UsnoConversionError> {
    if !(1..=678).contains(&row.printed_term_number) {
        return Err(UsnoConversionError::WrongPrintedTermRange);
    }
    if row.multipliers[..9].iter().any(|value| *value != 0) {
        return Err(UsnoConversionError::LunisolarHasPlanetaryMultiplier);
    }
    let coefficients = radians(&row)?;
    let mut multipliers = [0_i8; 5];
    multipliers.copy_from_slice(&row.multipliers[9..]);
    Ok(LuniSolarHarmonicTerm {
        multipliers,
        longitude_sin_rad: coefficients[0],
        longitude_sin_slope_rad_per_century: coefficients[1],
        longitude_cos_rad: coefficients[2],
        obliquity_cos_rad: coefficients[3],
        obliquity_cos_slope_rad_per_century: coefficients[4],
        obliquity_sin_rad: coefficients[5],
    })
}

/// Convert a caller-supplied **planetary** USNO printed row.
///
/// Requires publication index 679..=1365, zero `lprime` multiplier
/// (printed column j=11) and zero amplitude slopes. The permutation is
/// explicitly `output[0..5]=printed[9..14]` and
/// `output[5..14]=printed[0..9]`.
///
/// It is still the caller's responsibility to use historical MHB2000
/// lunar argument and Neptune conventions, and a consistent epoch.
pub fn convert_usno_planetary_row(
    row: UsnoPrintedHarmonicRow,
) -> Result<PlanetaryHarmonicTerm, UsnoConversionError> {
    if !(679..=1365).contains(&row.printed_term_number) {
        return Err(UsnoConversionError::WrongPrintedTermRange);
    }
    if row.multipliers[10] != 0 {
        return Err(UsnoConversionError::PlanetaryHasSolarAnomalyMultiplier);
    }
    if row.coefficients_arcseconds[1] != 0.0 || row.coefficients_arcseconds[4] != 0.0 {
        return Err(UsnoConversionError::PlanetaryHasAmplitudeSlope);
    }
    let coefficients = radians(&row)?;
    let mut multipliers = [0_i8; 14];
    multipliers[..5].copy_from_slice(&row.multipliers[9..]);
    multipliers[5..].copy_from_slice(&row.multipliers[..9]);
    Ok(PlanetaryHarmonicTerm {
        multipliers,
        longitude_sin_rad: coefficients[0],
        longitude_sin_slope_rad_per_century: coefficients[1],
        longitude_cos_rad: coefficients[2],
        obliquity_cos_rad: coefficients[3],
        obliquity_cos_slope_rad_per_century: coefficients[4],
        obliquity_sin_rad: coefficients[5],
    })
}
