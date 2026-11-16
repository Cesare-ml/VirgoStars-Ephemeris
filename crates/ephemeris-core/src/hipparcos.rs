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

//! Reader for the Hipparcos new reduction (van Leeuwen 2007, CDS I/311/hip2).
//!
//! **No catalog data is bundled.** The caller supplies the file; this module
//! only parses the documented fixed-width records and converts one record to a
//! [`FixedStarCatalogEntry`]. Redistribution terms of the catalog are separate
//! from this repository's CPAL-1.0 source license and must be reviewed by
//! whoever distributes derived data.
//!
//! # Format (CDS I/311 ReadMe, 1-based inclusive columns, one 276-byte record)
//!
//! | Columns | Field | Unit |
//! |---|---|---|
//! | 1–6 | `HIP` | |
//! | 8–10 | `Sn` solution type of the new reduction, `10*d+s` | |
//! | 14 | `Nc` number of components | |
//! | 16–28 | `RArad` | radian, ICRS, epoch J1991.25 |
//! | 30–42 | `DErad` | radian |
//! | 44–50 | `Plx` | mas |
//! | 52–59 | `pmRA` = μα·cos δ | mas/yr |
//! | 61–68 | `pmDE` | mas/yr |
//! | 70–75, 77–82, 84–89, 91–96, 98–103 | formal errors of RA, Dec, Plx, pmRA, pmDE | mas, mas, mas, mas/yr, mas/yr |
//!
//! The catalog holds **no radial velocity and no V magnitude**. A radial
//! velocity must be supplied by the caller through [`RadialVelocity`]; the
//! chosen source is kept in the result so an assumption is never invisible.
//!
//! The reference epoch J1991.25 is `JD 2448349.0625` by the Julian-epoch
//! definition `JD = 2451545.0 + (epoch − 2000.0)·365.25`.

use std::error::Error;
use std::fmt;
use std::io::BufRead;

use crate::de441::JulianDateTdb;
use crate::fixed_star::FixedStarCatalogEntry;

/// Length of one record without its line terminator.
pub const HIP2_RECORD_LENGTH: usize = 276;
/// Number of records in the published `hip2.dat`.
pub const HIP2_RECORD_COUNT: usize = 117_955;
/// Julian date of epoch J1991.25 (the catalog's reference epoch).
pub const HIP2_EPOCH_JD: f64 = 2_448_349.062_5;

/// How the barycentric radial velocity was obtained.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RadialVelocity {
    /// A cited barycentric measurement, km/s, positive when receding.
    Measured(f64),
    /// Explicit decision to use 0 km/s because no measurement is available.
    /// Its only effect here is the small perspective change of proper motion
    /// over long intervals; callers should disclose it.
    AssumedZero,
}

/// One parsed record, in the catalog's own units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hip2Record {
    pub hip: u32,
    pub solution_type: u16,
    pub components: u8,
    pub ra_rad: f64,
    pub dec_rad: f64,
    pub parallax_mas: f64,
    pub pm_ra_cos_dec_mas_per_year: f64,
    pub pm_dec_mas_per_year: f64,
    pub e_ra_mas: f64,
    pub e_dec_mas: f64,
    pub e_parallax_mas: f64,
    pub e_pm_ra_mas_per_year: f64,
    pub e_pm_dec_mas_per_year: f64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Hip2Error {
    /// The line is not exactly 276 ASCII bytes (before the terminator).
    BadRecordLength(usize),
    NonAscii,
    /// A field is blank, unparsable or non-finite.
    BadField(&'static str),
    DuplicateHip(u32),
    Io(std::io::ErrorKind),
    /// Fewer or more records than the published file.
    WrongRecordCount(usize),
    /// A value is outside what the catalog can legitimately hold.
    OutOfRange(&'static str),
}

impl fmt::Display for Hip2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRecordLength(n) => write!(f, "hip2 record has {n} bytes, expected 276"),
            Self::NonAscii => write!(f, "hip2 record contains non-ASCII bytes"),
            Self::BadField(name) => write!(f, "hip2 field {name} is blank or malformed"),
            Self::DuplicateHip(hip) => write!(f, "duplicate HIP number {hip}"),
            Self::Io(kind) => write!(f, "hip2 read error: {kind:?}"),
            Self::WrongRecordCount(n) => write!(f, "hip2 has {n} records, expected 117955"),
            Self::OutOfRange(name) => write!(f, "hip2 field {name} is out of range"),
        }
    }
}
impl Error for Hip2Error {}

/// Quality policy applied before a star is used. It is a **selection policy
/// of this project**, not a rule of the catalog.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hip2QualityPolicy {
    /// Required `Sn` values (e.g. 5 for the standard five-parameter solution).
    pub allowed_solution_types: &'static [u16],
    pub required_components: u8,
    pub min_parallax_snr: f64,
}

impl Hip2QualityPolicy {
    /// Standard single-star, five-parameter solution with parallax S/N ≥ 5.
    pub const STRICT: Self = Self {
        allowed_solution_types: &[5],
        required_components: 1,
        min_parallax_snr: 5.0,
    };

    pub fn accepts(&self, record: &Hip2Record) -> bool {
        self.allowed_solution_types.contains(&record.solution_type)
            && record.components == self.required_components
            && record.e_parallax_mas > 0.0
            && record.parallax_mas / record.e_parallax_mas >= self.min_parallax_snr
    }
}

fn field(line: &str, from: usize, to: usize) -> &str {
    // 1-based inclusive columns; the caller guarantees ASCII and length.
    &line[from - 1..to]
}

fn number(line: &str, from: usize, to: usize, name: &'static str) -> Result<f64, Hip2Error> {
    let text = field(line, from, to).trim();
    if text.is_empty() {
        return Err(Hip2Error::BadField(name));
    }
    let value: f64 = text.parse().map_err(|_| Hip2Error::BadField(name))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Hip2Error::BadField(name))
    }
}

fn integer(line: &str, from: usize, to: usize, name: &'static str) -> Result<u32, Hip2Error> {
    let text = field(line, from, to).trim();
    text.parse().map_err(|_| Hip2Error::BadField(name))
}

/// Parse one record (without its line terminator).
pub fn parse_hip2_record(line: &str) -> Result<Hip2Record, Hip2Error> {
    if !line.is_ascii() {
        return Err(Hip2Error::NonAscii);
    }
    if line.len() != HIP2_RECORD_LENGTH {
        return Err(Hip2Error::BadRecordLength(line.len()));
    }
    let record = Hip2Record {
        hip: integer(line, 1, 6, "HIP")?,
        solution_type: integer(line, 8, 10, "Sn")? as u16,
        components: integer(line, 14, 14, "Nc")? as u8,
        ra_rad: number(line, 16, 28, "RArad")?,
        dec_rad: number(line, 30, 42, "DErad")?,
        parallax_mas: number(line, 44, 50, "Plx")?,
        pm_ra_cos_dec_mas_per_year: number(line, 52, 59, "pmRA")?,
        pm_dec_mas_per_year: number(line, 61, 68, "pmDE")?,
        e_ra_mas: number(line, 70, 75, "e_RArad")?,
        e_dec_mas: number(line, 77, 82, "e_DErad")?,
        e_parallax_mas: number(line, 84, 89, "e_Plx")?,
        e_pm_ra_mas_per_year: number(line, 91, 96, "e_pmRA")?,
        e_pm_dec_mas_per_year: number(line, 98, 103, "e_pmDE")?,
    };
    if record.hip == 0 {
        return Err(Hip2Error::OutOfRange("HIP"));
    }
    if !(0.0..std::f64::consts::TAU).contains(&record.ra_rad) {
        return Err(Hip2Error::OutOfRange("RArad"));
    }
    if record.dec_rad.abs() > std::f64::consts::FRAC_PI_2 {
        return Err(Hip2Error::OutOfRange("DErad"));
    }
    Ok(record)
}

impl Hip2Record {
    /// Convert to the catalog entry used by the fixed-star engine.
    ///
    /// The parallax must be positive and finite; no distance is invented for a
    /// zero or negative parallax (the engine rejects it as well).
    pub fn to_catalog_entry(
        &self,
        radial_velocity: RadialVelocity,
    ) -> Result<FixedStarCatalogEntry, Hip2Error> {
        if self.parallax_mas <= 0.0 {
            return Err(Hip2Error::OutOfRange("Plx must be positive"));
        }
        let rv = match radial_velocity {
            RadialVelocity::Measured(v) if v.is_finite() && v.abs() < 299_792.458 => v,
            RadialVelocity::Measured(_) => {
                return Err(Hip2Error::OutOfRange("radial velocity"));
            }
            RadialVelocity::AssumedZero => 0.0,
        };
        Ok(FixedStarCatalogEntry {
            epoch_tdb: JulianDateTdb(HIP2_EPOCH_JD),
            right_ascension_deg: self.ra_rad.to_degrees(),
            declination_deg: self.dec_rad.to_degrees(),
            proper_motion_ra_cos_dec_mas_per_year: self.pm_ra_cos_dec_mas_per_year,
            proper_motion_dec_mas_per_year: self.pm_dec_mas_per_year,
            parallax_mas: self.parallax_mas,
            radial_velocity_km_per_second: rv,
        })
    }
}

/// Read a complete `hip2.dat`, validating every record, HIP uniqueness and the
/// published record count. Returns the records for the requested HIP numbers,
/// in the order requested; an absent number is an error, never skipped.
pub fn read_hip2_selected<R: BufRead>(
    reader: R,
    wanted: &[u32],
) -> Result<Vec<Hip2Record>, Hip2Error> {
    use std::collections::{HashMap, HashSet};
    let wanted_set: HashSet<u32> = wanted.iter().copied().collect();
    let mut seen = HashSet::new();
    let mut found: HashMap<u32, Hip2Record> = HashMap::new();
    let mut count = 0usize;
    for line in reader.lines() {
        let line = line.map_err(|e| Hip2Error::Io(e.kind()))?;
        let line = line.strip_suffix('\r').unwrap_or(&line);
        let record = parse_hip2_record(line)?;
        if !seen.insert(record.hip) {
            return Err(Hip2Error::DuplicateHip(record.hip));
        }
        if wanted_set.contains(&record.hip) {
            found.insert(record.hip, record);
        }
        count += 1;
    }
    if count != HIP2_RECORD_COUNT {
        return Err(Hip2Error::WrongRecordCount(count));
    }
    wanted
        .iter()
        .map(|hip| {
            found
                .get(hip)
                .copied()
                .ok_or(Hip2Error::OutOfRange("HIP not found"))
        })
        .collect()
}
