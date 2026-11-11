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

//! Strict, versioned in-memory importer for **user-provided** harmonic data.
//!
//! This is NOT an IAU 2000A coefficient distribution or a license verifier.
//! Input must already use the canonical internal F1..F14 argument order,
//! radians and radians per TDB Julian century, not USNO/IERS raw tables.
//! See docs/NUTATION_TABLE_IMPORT.md for the complete v1 contract.

use std::error::Error;
use std::fmt;

use crate::nutation_harmonics::NutationHarmonicTerm;

const MAGIC: &str = "VIRGOSTARS_NUTATION_TERMS_V1";
const MAX_FILE_BYTES: usize = 256 * 1024;

/// This metadata is **self-declared by the input** and is not a rights grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableSourceKind {
    Synthetic,
    ExternalUnverified,
}

/// Validated shape, units, row count and index order for one coefficient
/// family; it does not imply independent astronomical or legal validation.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedNutationTable<const N: usize> {
    pub source_kind: TableSourceKind,
    pub source_reference: String,
    pub terms: Vec<NutationHarmonicTerm<N>>,
}

impl<const N: usize> ImportedNutationTable<N> {
    /// Whether the **row count** equals the historical family's length.
    /// This does not prove that the values are official or correct.
    pub fn has_full_family_length(&self) -> bool {
        match N {
            5 => self.terms.len() == 678,
            14 => self.terms.len() == 687,
            _ => false,
        }
    }
}

/// Typed reason for refusing a malformed or ambiguous canonical table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableParseErrorKind {
    FileTooLarge,
    MissingField,
    InvalidHeader,
    InvalidSourceReference,
    InvalidCount,
    InvalidRowWidth,
    InvalidRowSequence,
    InvalidMultiplier,
    InvalidCoefficient,
    NonFiniteCoefficient,
    ImplausibleCoefficient,
    SolarAnomalyNotUsed,
    PlanetarySecularSlopeNotUsed,
    CountMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableParseError {
    /// The 1-based offending line (or the next expected line at EOF).
    pub line: usize,
    pub kind: TableParseErrorKind,
}

impl fmt::Display for TableParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nutation table error at line {}: {:?}", self.line, self.kind)
    }
}
impl Error for TableParseError {}

fn error(line: usize, kind: TableParseErrorKind) -> TableParseError {
    TableParseError { line, kind }
}

fn read<'a>(lines: &[&'a str], index: usize) -> Result<&'a str, TableParseError> {
    lines
        .get(index)
        .map(|line| line.trim_end_matches('\r'))
        .ok_or_else(|| error(index + 1, TableParseErrorKind::MissingField))
}

fn header_equals(
    lines: &[&str],
    index: usize,
    expected: &str,
) -> Result<(), TableParseError> {
    if read(lines, index)? != expected {
        return Err(error(index + 1, TableParseErrorKind::InvalidHeader));
    }
    Ok(())
}

fn header_value<'a>(
    lines: &[&'a str],
    index: usize,
    prefix: &str,
) -> Result<&'a str, TableParseError> {
    read(lines, index)?
        .strip_prefix(prefix)
        .ok_or_else(|| error(index + 1, TableParseErrorKind::InvalidHeader))
}

fn parse_table<const N: usize>(
    text: &str,
    family: &str,
    argument_order: &str,
    maximum: usize,
) -> Result<ImportedNutationTable<N>, TableParseError> {
    if text.len() > MAX_FILE_BYTES {
        return Err(error(1, TableParseErrorKind::FileTooLarge));
    }
    let lines: Vec<&str> = text.lines().collect();
    header_equals(&lines, 0, MAGIC)?;
    header_equals(&lines, 1, family)?;
    header_equals(&lines, 2, argument_order)?;
    header_equals(&lines, 3, "amplitudes=radians")?;
    header_equals(&lines, 4, "slopes=radians_per_tdb_century")?;
    let source_kind = match header_value(&lines, 5, "source=")? {
        "synthetic" => TableSourceKind::Synthetic,
        "external_unverified" => TableSourceKind::ExternalUnverified,
        _ => return Err(error(6, TableParseErrorKind::InvalidHeader)),
    };
    let reference = header_value(&lines, 6, "reference=")?;
    if reference.len() > 300
        || reference.is_empty()
        || !reference.bytes().all(|c| c.is_ascii_graphic())
        || match source_kind {
            TableSourceKind::Synthetic => !reference.starts_with("synthetic:"),
            TableSourceKind::ExternalUnverified => !reference.starts_with("https://"),
        }
    {
        return Err(error(7, TableParseErrorKind::InvalidSourceReference));
    }

    let declared_str = header_value(&lines, 7, "count=")?;
    let declared: usize = declared_str
        .parse()
        .map_err(|_| error(8, TableParseErrorKind::InvalidCount))?;
    if declared == 0 || declared > maximum || declared.to_string() != declared_str {
        return Err(error(8, TableParseErrorKind::InvalidCount));
    }
    header_equals(&lines, 8, "DATA")?;

    // The caller-provided file is small and bounded. Parse completely
    // before returning any term so malformed files cannot be partially used.
    let mut terms = Vec::with_capacity(declared);
    for (index, row) in lines.iter().enumerate().skip(9) {
        let line_number = index + 1;
        if terms.len() >= declared {
            return Err(error(line_number, TableParseErrorKind::CountMismatch));
        }
        let columns: Vec<&str> = row.trim_end_matches('\r').split(',').collect();
        if columns.len() != N + 7 {
            return Err(error(line_number, TableParseErrorKind::InvalidRowWidth));
        }
        // One-based, consecutive canonical term indices. A silent skipped,
        // duplicated or reordered term would compromise later reference
        // comparisons and is therefore rejected.
        let expected = (terms.len() + 1).to_string();
        if columns[0] != expected {
            return Err(error(line_number, TableParseErrorKind::InvalidRowSequence));
        }
        let mut multipliers = [0_i8; N];
        for (index, destination) in multipliers.iter_mut().enumerate() {
            *destination = columns[1 + index]
                .parse::<i8>()
                .map_err(|_| error(line_number, TableParseErrorKind::InvalidMultiplier))?;
        }
        // The historical MHB2000 planetary terms have no l' multiplier.
        if N == 14 && multipliers[1] != 0 {
            return Err(error(
                line_number,
                TableParseErrorKind::SolarAnomalyNotUsed,
            ));
        }

        let mut coefficients = [0.0_f64; 6];
        for (i, destination) in coefficients.iter_mut().enumerate() {
            *destination = columns[N + 1 + i]
                .parse::<f64>()
                .map_err(|_| error(line_number, TableParseErrorKind::InvalidCoefficient))?;
            if !destination.is_finite() {
                return Err(error(line_number, TableParseErrorKind::NonFiniteCoefficient));
            }
            // A deliberately wide safety guard catches accidental imports
            // of unconverted arcsecond or integer table units. This bound
            // is NOT a model completeness/precision certification.
            if destination.abs() > 1.0 {
                return Err(error(line_number, TableParseErrorKind::ImplausibleCoefficient));
            }
        }
        if N == 14 && (coefficients[1] != 0.0 || coefficients[4] != 0.0) {
            return Err(error(
                line_number,
                TableParseErrorKind::PlanetarySecularSlopeNotUsed,
            ));
        }
        terms.push(NutationHarmonicTerm {
            multipliers,
            longitude_sin_rad: coefficients[0],
            longitude_sin_slope_rad_per_century: coefficients[1],
            longitude_cos_rad: coefficients[2],
            obliquity_cos_rad: coefficients[3],
            obliquity_cos_slope_rad_per_century: coefficients[4],
            obliquity_sin_rad: coefficients[5],
        });
    }
    if terms.len() != declared {
        return Err(error(lines.len() + 1, TableParseErrorKind::CountMismatch));
    }
    Ok(ImportedNutationTable {
        source_kind,
        source_reference: reference.to_owned(),
        terms,
    })
}

/// Parse a versioned, canonical **5-multiplier** lunar-solar table.
///
/// The caller must supply radians, not raw arcseconds. See the v1 schema
/// in docs/NUTATION_TABLE_IMPORT.md. The returned data is **unverified**
/// scientifically and legally, even for a full 678-row table.
pub fn parse_lunisolar_table(
    text: &str,
) -> Result<ImportedNutationTable<5>, TableParseError> {
    parse_table::<5>(
        text,
        "family=lunisolar",
        "arguments=l,lprime,F,D,Omega",
        678,
    )
}

/// Parse a versioned, canonical **14-multiplier** planetary table.
///
/// Requires historical planetary l' multiplier **zero** and secular
/// amplitude slopes **zero**, with explicit canonical argument ordering.
/// The returned data is **unverified**, not official IAU 2000A output.
pub fn parse_planetary_table(
    text: &str,
) -> Result<ImportedNutationTable<14>, TableParseError> {
    parse_table::<14>(
        text,
        "family=planetary",
        "arguments=l,lprime,F,D,Omega,Mercury,Venus,Earth,Mars,Jupiter,Saturn,Uranus,Neptune,pA",
        687,
    )
}
