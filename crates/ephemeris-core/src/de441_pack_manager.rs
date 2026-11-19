/*
EXHIBIT A. Common Public Attribution License Version 1.0.

The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

//! Offline selection of classic-DE441 self-contained packs.
//!
//! The manager does not download, delete, authenticate or redistribute data.
//! It routes caller-provided TDB Julian dates to explicitly registered
//! half-open package intervals, not the wider record-aligned binary coverage.
//! The selected file is read by the existing [`De441`] evaluator unchanged.
// Modification 2026-11-18: implement the opt-in long-term state provider.

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use crate::apparent_ecliptic_of_date::ExperimentalLongTermStateProvider;
use crate::calendar::{Calendar, CalendarDateTime, TimeScale};
use crate::de441::{
    Body, De441, De441Error, JulianDateTdb, StateVector, StateVectorWithAcceleration,
};

/// An explicitly registered interval in JD TDB. Normally the upper bound is
/// exclusive even though the underlying classic binary header includes it.
#[derive(Clone, Debug, PartialEq)]
pub struct De441PackSpec {
    pub id: String,
    pub file_path: PathBuf,
    pub start_tdb: JulianDateTdb,
    pub end_tdb: JulianDateTdb,
    pub end_inclusive: bool,
    pub required: bool,
}

impl De441PackSpec {
    /// Register an application-owned file with a half-open coverage interval.
    pub fn new(
        id: impl Into<String>,
        file_path: impl Into<PathBuf>,
        start_tdb: JulianDateTdb,
        end_exclusive_tdb: JulianDateTdb,
        required: bool,
    ) -> Self {
        Self {
            id: id.into(),
            file_path: file_path.into(),
            start_tdb,
            end_tdb: end_exclusive_tdb,
            end_inclusive: false,
            required,
        }
    }
}

/// Read-only package inventory for an application settings panel. `installed`
/// only checks that the path currently names a file, not its integrity.
#[derive(Clone, Debug, PartialEq)]
pub struct De441PackInfo {
    pub spec: De441PackSpec,
    pub installed: bool,
}

#[derive(Debug)]
pub enum De441PackError {
    InvalidCatalog(&'static str),
    InvalidEpoch(JulianDateTdb),
    NoPackForEpoch(JulianDateTdb),
    MissingPack {
        id: String,
        path: PathBuf,
    },
    InvalidPackCoverage {
        id: String,
        expected_start: JulianDateTdb,
        expected_end: JulianDateTdb,
        actual_start: JulianDateTdb,
        actual_end: JulianDateTdb,
    },
    Reader {
        id: String,
        source: De441Error,
    },
}

impl fmt::Display for De441PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCatalog(reason) => write!(f, "invalid DE441 pack catalog: {reason}"),
            Self::InvalidEpoch(epoch) => write!(f, "invalid JD TDB epoch: {}", epoch.0),
            Self::NoPackForEpoch(epoch) => {
                write!(f, "no DE441 pack registered for JD TDB {}", epoch.0)
            }
            Self::MissingPack { id, path } => {
                write!(f, "DE441 pack '{id}' is not installed: {}", path.display())
            }
            Self::InvalidPackCoverage {
                id,
                expected_start,
                expected_end,
                actual_start,
                actual_end,
            } => write!(
                f,
                "DE441 pack '{id}' does not cover requested JD TDB [{}, {}]: file reports [{}, {}]",
                expected_start.0, expected_end.0, actual_start.0, actual_end.0
            ),
            Self::Reader { id, source } => write!(f, "DE441 pack '{id}': {source}"),
        }
    }
}

impl Error for De441PackError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Reader { source, .. } => Some(source),
            _ => None,
        }
    }
}

struct LoadedPack {
    index: usize,
    reader: De441<File>,
}

impl ExperimentalLongTermStateProvider for De441PackManager {
    type Error = De441PackError;

    fn state(&mut self, body: Body, epoch: JulianDateTdb) -> Result<StateVector, Self::Error> {
        De441PackManager::state(self, body, Body::SolarSystemBarycenter, epoch)
    }

    fn state_with_acceleration(
        &mut self,
        body: Body,
        epoch: JulianDateTdb,
    ) -> Result<StateVectorWithAcceleration, Self::Error> {
        De441PackManager::state_with_acceleration(self, body, Body::SolarSystemBarycenter, epoch)
    }

    fn logical_coverage(
        &mut self,
        epoch: JulianDateTdb,
    ) -> Result<(JulianDateTdb, JulianDateTdb), Self::Error> {
        let selected = self.select_pack(epoch)?;
        // Even full-file inclusive coverage is conservatively treated as half-open.
        Ok((selected.start_tdb, selected.end_tdb))
    }
}

/// Selects DE441 packs in Rust while leaving file lifecycle and networking
/// to the caller. At most one file handle and DE441 coefficient cache is held.
pub struct De441PackManager {
    specs: Vec<De441PackSpec>,
    loaded: Option<LoadedPack>,
}

// IDs and file names are the same as the independently generated v1 manifest.
// Years are astronomical/proleptic Gregorian, with their Jan 1 interpreted
// directly as TDB, precisely as in the pack exporter.
const MOBILE_PACKS: [(&str, &str, i32, i32, bool); 10] = [
    ("core", "de441-core.eph", 1600, 2101, true),
    (
        "history-0000-0199",
        "de441-history-0000-0199.eph",
        0,
        200,
        false,
    ),
    (
        "history-0200-0399",
        "de441-history-0200-0399.eph",
        200,
        400,
        false,
    ),
    (
        "history-0400-0599",
        "de441-history-0400-0599.eph",
        400,
        600,
        false,
    ),
    (
        "history-0600-0799",
        "de441-history-0600-0799.eph",
        600,
        800,
        false,
    ),
    (
        "history-0800-0999",
        "de441-history-0800-0999.eph",
        800,
        1000,
        false,
    ),
    (
        "history-1000-1199",
        "de441-history-1000-1199.eph",
        1000,
        1200,
        false,
    ),
    (
        "history-1200-1399",
        "de441-history-1200-1399.eph",
        1200,
        1400,
        false,
    ),
    (
        "history-1400-1599",
        "de441-history-1400-1599.eph",
        1400,
        1600,
        false,
    ),
    (
        "future-2101-2300",
        "de441-future-2101-2300.eph",
        2101,
        2301,
        false,
    ),
];

fn january_first_tdb(year: i32) -> Result<JulianDateTdb, De441PackError> {
    CalendarDateTime {
        year,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
        scale: TimeScale::Tdb,
    }
    .to_julian_date(Calendar::ProlepticGregorian)
    .and_then(|date| date.to_de441_epoch())
    .map_err(|_| De441PackError::InvalidCatalog("invalid mobile calendar boundary"))
}

impl De441PackManager {
    /// Register caller-defined intervals. Input order is unimportant; catalogs
    /// with invalid/overlapping ranges or duplicate IDs are rejected.
    pub fn from_specs(mut specs: Vec<De441PackSpec>) -> Result<Self, De441PackError> {
        if specs.is_empty() || specs.len() > 256 {
            return Err(De441PackError::InvalidCatalog(
                "expected between 1 and 256 packages",
            ));
        }
        for spec in &specs {
            if spec.id.is_empty()
                || !spec.start_tdb.0.is_finite()
                || !spec.end_tdb.0.is_finite()
                || spec.start_tdb.0 >= spec.end_tdb.0
                || spec.file_path.as_os_str().is_empty()
            {
                return Err(De441PackError::InvalidCatalog(
                    "invalid ID, file path or JD TDB interval",
                ));
            }
        }
        specs.sort_by(|a, b| a.start_tdb.0.total_cmp(&b.start_tdb.0));
        for (index, spec) in specs.iter().enumerate() {
            if specs[..index].iter().any(|previous| previous.id == spec.id) {
                return Err(De441PackError::InvalidCatalog("duplicate pack ID"));
            }
            if index > 0 {
                let previous = &specs[index - 1];
                if previous.end_tdb.0 > spec.start_tdb.0
                    || (previous.end_inclusive && previous.end_tdb.0 == spec.start_tdb.0)
                {
                    return Err(De441PackError::InvalidCatalog(
                        "overlapping logical JD TDB intervals",
                    ));
                }
            }
        }
        Ok(Self {
            specs,
            loaded: None,
        })
    }

    /// Describe the ten standard mobile packages without opening or loading
    /// any of them. `directory` is expected to contain the generated .eph files.
    pub fn from_mobile_directory(directory: impl AsRef<Path>) -> Result<Self, De441PackError> {
        let mut specs = Vec::with_capacity(MOBILE_PACKS.len());
        for &(id, filename, first_year, next_year, required) in &MOBILE_PACKS {
            specs.push(De441PackSpec::new(
                id,
                directory.as_ref().join(filename),
                january_first_tdb(first_year)?,
                january_first_tdb(next_year)?,
                required,
            ));
        }
        Self::from_specs(specs)
    }

    /// Adapt the original full classic-DE441 file to the same state API.
    /// The original header's final endpoint is inclusive in this special case.
    pub fn from_full_file(file_path: impl Into<PathBuf>) -> Result<Self, De441PackError> {
        let file_path = file_path.into();
        let reader = De441::open(&file_path).map_err(|source| De441PackError::Reader {
            id: "full-de441".into(),
            source,
        })?;
        let (start, end) = reader.coverage();
        let mut spec = De441PackSpec::new("full-de441", file_path, start, end, true);
        spec.end_inclusive = true;
        let mut manager = Self::from_specs(vec![spec])?;
        manager.loaded = Some(LoadedPack { index: 0, reader });
        Ok(manager)
    }

    /// Returns a location and ID without attempting I/O, to allow an app to
    /// offer a download when the selected file is not installed.
    pub fn select_pack(&self, epoch: JulianDateTdb) -> Result<&De441PackSpec, De441PackError> {
        Ok(&self.specs[self.select_index(epoch)?])
    }

    fn select_index(&self, epoch: JulianDateTdb) -> Result<usize, De441PackError> {
        if !epoch.0.is_finite() {
            return Err(De441PackError::InvalidEpoch(epoch));
        }
        self.specs
            .iter()
            .position(|spec| {
                epoch.0 >= spec.start_tdb.0
                    && (epoch.0 < spec.end_tdb.0
                        || (spec.end_inclusive && epoch.0 == spec.end_tdb.0))
            })
            .ok_or(De441PackError::NoPackForEpoch(epoch))
    }

    /// Inventory reflects current file existence only, not checksum validity.
    pub fn inventory(&self) -> Vec<De441PackInfo> {
        self.specs
            .iter()
            .map(|spec| De441PackInfo {
                installed: spec.file_path.is_file(),
                spec: spec.clone(),
            })
            .collect()
    }

    /// Plan the packages needed to expand a verified, gap-free installation.
    ///
    /// `installed_ids` must contain only host-verified, fully activated files:
    /// path existence alone is insufficient. Required packages form the
    /// non-removable base, and downloaded optional packages must stay adjacent
    /// to it. The returned specs are ordered from the existing boundaries
    /// outward, so activating each completed optional download preserves a
    /// contiguous interval. A fresh installation receives its base first.
    ///
    /// The requested interval is half-open in JD TDB; in particular, an end
    /// exactly at a package boundary does not require the following package.
    /// This method does not read files, verify signatures, download or install.
    pub fn plan_contiguous_extension(
        &self,
        installed_ids: &[&str],
        desired_start: JulianDateTdb,
        desired_end_exclusive: JulianDateTdb,
    ) -> Result<Vec<De441PackSpec>, De441PackError> {
        let (base_first, base_last) = self.contiguous_base()?;
        let existing = self.verified_installation(installed_ids, base_first, base_last)?;
        if !desired_start.0.is_finite() {
            return Err(De441PackError::InvalidEpoch(desired_start));
        }
        if !desired_end_exclusive.0.is_finite() {
            return Err(De441PackError::InvalidEpoch(desired_end_exclusive));
        }
        if desired_start.0 >= desired_end_exclusive.0 {
            return Err(De441PackError::InvalidCatalog(
                "requested extension has an empty or reversed interval",
            ));
        }
        let requested_first = self.select_index(desired_start)?;
        let requested_last = self
            .specs
            .iter()
            .rposition(|spec| {
                spec.start_tdb.0 < desired_end_exclusive.0
                    && desired_end_exclusive.0 <= spec.end_tdb.0
            })
            .ok_or(De441PackError::NoPackForEpoch(desired_end_exclusive))?;
        let first = requested_first.min(base_first);
        let last = requested_last.max(base_last);
        let (active_first, active_last) = existing.unwrap_or((base_first, base_last));
        let first = first.min(active_first);
        let last = last.max(active_last);

        let mut pending = Vec::new();
        if existing.is_none() {
            pending.extend(self.specs[base_first..=base_last].iter().cloned());
        }
        pending.extend(self.specs[first..active_first].iter().rev().cloned());
        pending.extend(self.specs[(active_last + 1)..(last + 1)].iter().cloned());
        Ok(pending)
    }

    /// Optional packs may only be removed at the outer edge of a verified
    /// installation. Never remove a required pack or create a gap.
    pub fn can_remove_contiguous_pack(
        &self,
        installed_ids: &[&str],
        pack_id: &str,
    ) -> Result<bool, De441PackError> {
        let (base_first, base_last) = self.contiguous_base()?;
        let Some((first, last)) =
            self.verified_installation(installed_ids, base_first, base_last)?
        else {
            return Ok(false);
        };
        let Some(index) = self.specs.iter().position(|spec| spec.id == pack_id) else {
            return Err(De441PackError::InvalidCatalog("unknown package ID"));
        };
        Ok(!self.specs[index].required && (index == first || index == last))
    }

    fn contiguous_base(&self) -> Result<(usize, usize), De441PackError> {
        if self.specs.iter().any(|spec| spec.end_inclusive)
            || self
                .specs
                .windows(2)
                .any(|pair| pair[0].end_tdb.0 != pair[1].start_tdb.0)
        {
            return Err(De441PackError::InvalidCatalog(
                "contiguous planning requires adjacent half-open packages",
            ));
        }
        let Some(first) = self.specs.iter().position(|spec| spec.required) else {
            return Err(De441PackError::InvalidCatalog(
                "contiguous planning requires a non-removable base",
            ));
        };
        let last = self.specs.iter().rposition(|spec| spec.required).unwrap();
        if self.specs[first..=last].iter().any(|spec| !spec.required) {
            return Err(De441PackError::InvalidCatalog(
                "required base packages are not adjacent",
            ));
        }
        Ok((first, last))
    }

    fn verified_installation(
        &self,
        installed_ids: &[&str],
        base_first: usize,
        base_last: usize,
    ) -> Result<Option<(usize, usize)>, De441PackError> {
        if installed_ids.is_empty() {
            return Ok(None);
        }
        let mut present = vec![false; self.specs.len()];
        for id in installed_ids {
            let Some(index) = self.specs.iter().position(|spec| spec.id == *id) else {
                return Err(De441PackError::InvalidCatalog("unknown installed package"));
            };
            if present[index] {
                return Err(De441PackError::InvalidCatalog(
                    "duplicate installed package",
                ));
            }
            present[index] = true;
        }
        let first = present.iter().position(|installed| *installed).unwrap();
        let last = present.iter().rposition(|installed| *installed).unwrap();
        if first > base_first
            || last < base_last
            || present[first..=last].iter().any(|installed| !installed)
        {
            return Err(De441PackError::InvalidCatalog(
                "installed packages must include the complete base without gaps",
            ));
        }
        Ok(Some((first, last)))
    }

    /// Release the active file handle before an app attempts to uninstall or
    /// atomically replace a pack (particularly important on Windows).
    pub fn unload(&mut self) {
        self.loaded = None;
    }

    fn for_epoch<T>(
        &mut self,
        epoch: JulianDateTdb,
        evaluate: impl FnOnce(&mut De441<File>) -> Result<T, De441Error>,
    ) -> Result<T, De441PackError> {
        let index = self.select_index(epoch)?;
        if self.loaded.as_ref().map(|loaded| loaded.index) != Some(index) {
            // Drop any previous handle before switching. Missing or damaged
            // package files cannot silently fall back to an adjacent pack.
            self.unload();
            let spec = &self.specs[index];
            let reader = De441::open(&spec.file_path).map_err(|source| {
                if matches!(&source, De441Error::Io(e) if e.kind() == io::ErrorKind::NotFound) {
                    De441PackError::MissingPack {
                        id: spec.id.clone(),
                        path: spec.file_path.clone(),
                    }
                } else {
                    De441PackError::Reader {
                        id: spec.id.clone(),
                        source,
                    }
                }
            })?;
            let (actual_start, actual_end) = reader.coverage();
            if actual_start.0 > spec.start_tdb.0 || actual_end.0 < spec.end_tdb.0 {
                return Err(De441PackError::InvalidPackCoverage {
                    id: spec.id.clone(),
                    expected_start: spec.start_tdb,
                    expected_end: spec.end_tdb,
                    actual_start,
                    actual_end,
                });
            }
            self.loaded = Some(LoadedPack { index, reader });
        }
        let loaded = self.loaded.as_mut().expect("selected reader loaded");
        evaluate(&mut loaded.reader).map_err(|source| De441PackError::Reader {
            id: self.specs[index].id.clone(),
            source,
        })
    }

    /// Geometric position (km) and velocity (km/TDB day), target relative to
    /// center, ICRF equatorial axes. Epoch must already be JD TDB.
    pub fn state(
        &mut self,
        target: Body,
        center: Body,
        epoch: JulianDateTdb,
    ) -> Result<StateVector, De441PackError> {
        self.for_epoch(epoch, |reader| reader.state(target, center, epoch))
    }

    /// Batch interface; delegates to the existing 1-MiB-bounded Rust reader.
    pub fn states_at(
        &mut self,
        targets: &[Body],
        center: Body,
        epoch: JulianDateTdb,
    ) -> Result<Vec<StateVector>, De441PackError> {
        self.for_epoch(epoch, |reader| reader.states_at(targets, center, epoch))
    }

    /// Uses the same selected polynomial for analytic acceleration (km/day²).
    pub fn state_with_acceleration(
        &mut self,
        target: Body,
        center: Body,
        epoch: JulianDateTdb,
    ) -> Result<StateVectorWithAcceleration, De441PackError> {
        self.for_epoch(epoch, |reader| {
            reader.state_with_acceleration(target, center, epoch)
        })
    }
}
