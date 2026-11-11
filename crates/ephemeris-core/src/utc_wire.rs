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

//! Versioned scalar UTC↔TAI conversion record for future thin bindings.
//!
//! This is a *Rust data contract*, not a JSON serializer, a PyO3 extension,
//! a wasm-bindgen module or a stable production ABI. It contains the original
//! UTC clock fields (including a possible leap second), a split TAI Julian
//! date, and an explicit IANA schedule version with content fingerprint.

use crate::calendar::{JulianDate, TimeScale};
use crate::utc_leaps::{UtcClockTime, UtcDay, UtcLeapCatalog, UtcLeapError, VersionedUtcResult};

/// Identifier of the experimental scalar-record schema.
///
/// Bump the schema version and add a new record type instead of silently
/// reinterpreting existing fields when future bindings are introduced.
pub const UTC_TAI_RECORD_V1_SCHEMA: &str = "virgostars.utc-tai.v1";

/// Identifier of the experimental stable error-code schema.
///
/// Unlike Rust's human-readable `Display` strings, these short codes
/// are designed for future thin bindings and non-localized machine use.
pub const UTC_TAI_ERROR_V1_SCHEMA: &str = "virgostars.utc-tai-error.v1";

/// Language-neutral machine-readable error for UTC/TAI conversion.
///
/// This does not carry personally identifiable data, raw source files,
/// or any implicit fallback. The `code` is a stable ASCII snake_case
/// identifier; human-readable messages remain the caller's responsibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UtcTaiErrorV1 {
    pub schema: &'static str,
    pub code: &'static str,
}

impl From<UtcLeapError> for UtcTaiErrorV1 {
    fn from(value: UtcLeapError) -> Self {
        // Exhaustive match: adding a new Rust error requires intentionally
        // deciding the corresponding machine-facing v1 code.
        let code = match value {
            UtcLeapError::InvalidGregorianDate => "invalid_gregorian_date",
            UtcLeapError::Before1972 => "before_1972",
            UtcLeapError::InvalidCoverage => "invalid_coverage",
            UtcLeapError::InvalidInitialOffset => "invalid_initial_offset",
            UtcLeapError::InvalidTransition => "invalid_transition",
            UtcLeapError::InvalidOffsetStep => "invalid_offset_step",
            UtcLeapError::OutsideCoverage => "outside_coverage",
            UtcLeapError::InvalidClockTime => "invalid_clock_time",
            UtcLeapError::ClockTimeOutsideDay => "clock_time_outside_day",
            UtcLeapError::UnrepresentableTai => "unrepresentable_tai",
            UtcLeapError::WrongInputScale => "wrong_input_scale",
            UtcLeapError::InvalidPinnedLeapData => "invalid_pinned_leap_data",
            UtcLeapError::InvalidReviewedSnapshot => "invalid_reviewed_snapshot",
            UtcLeapError::InvalidSnapshotProvenance => "invalid_snapshot_provenance",
            UtcLeapError::DuplicateVersion => "duplicate_version",
            UtcLeapError::UnknownVersion => "unknown_version",
            UtcLeapError::SourceDigestMismatch => "source_digest_mismatch",
            UtcLeapError::InvalidWireRecord => "invalid_wire_record",
        };
        Self {
            schema: UTC_TAI_ERROR_V1_SCHEMA,
            code,
        }
    }
}

/// Language-neutral Gregorian UTC input for thin Python/WASM bindings.
///
/// A UTC time is kept as civil components instead of a uniform Julian date,
/// so a real positive leap second (23:59:60.x) can be represented. This
/// struct contains untrusted fields: validation happens in the Rust core.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UtcCivilInputV1 {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: f64,
}

/// Language-neutral split-date input for a TAI-to-UTC conversion.
///
/// The scale string is explicit and must equal "TAI"; a binding must not
/// silently reinterpret a TT/TDB/UTC number as an atomic-time instant.
#[derive(Clone, Debug, PartialEq)]
pub struct TaiSplitInputV1 {
    pub day_number: i64,
    pub midnight_fraction: f64,
    pub time_scale: String,
}

/// Language-neutral scalar record for one UTC ↔ TAI instant.
///
/// UTC fields describe a named Gregorian civil day and clock, **not** a
/// uniform Julian date in UTC. TAI fields use the project's split JD
/// convention: `tai_day_number` is the noon-based integer day number,
/// `tai_midnight_fraction` is the fraction of an SI day since the
/// preceding midnight, and the time scale is explicitly `"TAI"`.
///
/// All fields are owned/scalar to ease future binding to Python integers
/// and TypeScript number/string fields without exposing Rust lifetimes.
/// An external source review is still required for trusted provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct UtcTaiRecordV1 {
    pub schema: &'static str,
    pub leap_table_version: String,
    pub leap_table_sha256: String,
    pub leap_table_provenance: String,
    /// First *excluded* Gregorian UTC day, stored as an integer day number.
    pub leap_expires_utc_day_number: i64,
    /// Gregorian UTC civil day as a noon-based integer Julian day number.
    pub utc_day_number: i64,
    pub utc_hour: u8,
    pub utc_minute: u8,
    /// UTC seconds may be in [60,61) only during a validated positive leap.
    pub utc_second: f64,
    /// Noon-based integer Julian day number of the normalized TAI instant.
    pub tai_day_number: i64,
    /// Fraction of a uniform 86,400 SI-second TAI day since midnight.
    pub tai_midnight_fraction: f64,
    pub tai_time_scale: &'static str,
}

/// Untrusted scalar record received from a future Python/WASM boundary.
///
/// Unlike `UtcTaiRecordV1`, both schema and time-scale fields are owned
/// strings: a receiver can pass arbitrary values without pretending they
/// came from Rust's static schema constants. Validation never trusts the
/// claimed UTC instant, source identity, expiry or TAI split fields.
#[derive(Clone, Debug, PartialEq)]
pub struct ReceivedUtcTaiRecordV1 {
    pub schema: String,
    pub leap_table_version: String,
    pub leap_table_sha256: String,
    pub leap_table_provenance: String,
    pub leap_expires_utc_day_number: i64,
    pub utc_day_number: i64,
    pub utc_hour: u8,
    pub utc_minute: u8,
    pub utc_second: f64,
    pub tai_day_number: i64,
    pub tai_midnight_fraction: f64,
    pub tai_time_scale: String,
}

impl From<&UtcTaiRecordV1> for ReceivedUtcTaiRecordV1 {
    fn from(record: &UtcTaiRecordV1) -> Self {
        Self {
            schema: record.schema.to_owned(),
            leap_table_version: record.leap_table_version.clone(),
            leap_table_sha256: record.leap_table_sha256.clone(),
            leap_table_provenance: record.leap_table_provenance.clone(),
            leap_expires_utc_day_number: record.leap_expires_utc_day_number,
            utc_day_number: record.utc_day_number,
            utc_hour: record.utc_hour,
            utc_minute: record.utc_minute,
            utc_second: record.utc_second,
            tai_day_number: record.tai_day_number,
            tai_midnight_fraction: record.tai_midnight_fraction,
            tai_time_scale: record.tai_time_scale.to_owned(),
        }
    }
}

impl UtcTaiRecordV1 {
    fn from_components(
        versioned: VersionedUtcResult<JulianDate>,
        day: UtcDay,
        clock: UtcClockTime,
    ) -> Self {
        Self {
            schema: UTC_TAI_RECORD_V1_SCHEMA,
            leap_table_version: versioned.version,
            leap_table_sha256: versioned.source_sha256,
            leap_table_provenance: versioned.provenance,
            leap_expires_utc_day_number: versioned.expires_utc_day.julian_day_number(),
            utc_day_number: day.julian_day_number(),
            utc_hour: clock.hour(),
            utc_minute: clock.minute(),
            utc_second: clock.second(),
            tai_day_number: versioned.value.day_number(),
            tai_midnight_fraction: versioned.value.midnight_fraction(),
            tai_time_scale: "TAI",
        }
    }
}

impl UtcLeapCatalog {
    /// Validate a received v1 scalar record against the explicitly registered
    /// leap table, its SHA-256 and the actual inverse TAI → UTC conversion.
    ///
    /// This is intended for future Python/TypeScript binding boundaries: it
    /// refuses a self-inconsistent or tampered record rather than trusting
    /// its public fields. All non-metadata numeric fields are independently
    /// reconstructed from the TAI input. Reconstructed fractional UTC
    /// seconds are compared with an absolute 1e-8 s tolerance because the
    /// inverse uses floating-point arithmetic.
    ///
    /// This does not authenticate the IANA source or guarantee its publisher;
    /// the expected table content must already be independently approved.
    pub fn validate_record_v1(&self, record: &UtcTaiRecordV1) -> Result<(), UtcLeapError> {
        if record.schema != UTC_TAI_RECORD_V1_SCHEMA || record.tai_time_scale != "TAI" {
            return Err(UtcLeapError::InvalidWireRecord);
        }
        let tai = JulianDate::new(
            record.tai_day_number,
            record.tai_midnight_fraction,
            crate::calendar::TimeScale::Tai,
        )
        .map_err(|_| UtcLeapError::InvalidWireRecord)?;
        let expected =
            self.tai_to_utc_record_v1(&record.leap_table_version, &record.leap_table_sha256, tai)?;
        if record.leap_table_provenance != expected.leap_table_provenance
            || record.leap_expires_utc_day_number != expected.leap_expires_utc_day_number
            || record.utc_day_number != expected.utc_day_number
            || record.utc_hour != expected.utc_hour
            || record.utc_minute != expected.utc_minute
            || !record.utc_second.is_finite()
            || (record.utc_second - expected.utc_second).abs() > 1e-8
        {
            return Err(UtcLeapError::InvalidWireRecord);
        }
        Ok(())
    }

    /// Validate an untrusted cross-language record and return the canonical
    /// representation recalculated by the Rust core.
    ///
    /// Caller strings cannot impersonate static schema/time-scale tags;
    /// exact version and source SHA-256 are verified before conversion.
    /// Every UTC component, declared table provenance, expiry and the
    /// original split TAI instant must agree with the canonical result.
    /// Fractional UTC seconds use the same 1e-8 s round-trip tolerance as
    /// `validate_record_v1`. This is not a serializer or IANA source audit.
    pub fn validate_received_record_v1(
        &self,
        received: &ReceivedUtcTaiRecordV1,
    ) -> Result<UtcTaiRecordV1, UtcLeapError> {
        if received.schema != UTC_TAI_RECORD_V1_SCHEMA || received.tai_time_scale != "TAI" {
            return Err(UtcLeapError::InvalidWireRecord);
        }
        let split = TaiSplitInputV1 {
            day_number: received.tai_day_number,
            midnight_fraction: received.tai_midnight_fraction,
            time_scale: received.tai_time_scale.clone(),
        };
        let expected = self.tai_split_to_utc_record_v1(
            &received.leap_table_version,
            &received.leap_table_sha256,
            &split,
        )?;
        if received.leap_table_provenance != expected.leap_table_provenance
            || received.leap_expires_utc_day_number != expected.leap_expires_utc_day_number
            || received.utc_day_number != expected.utc_day_number
            || received.utc_hour != expected.utc_hour
            || received.utc_minute != expected.utc_minute
            || !received.utc_second.is_finite()
            || (received.utc_second - expected.utc_second).abs() > 1e-8
        {
            return Err(UtcLeapError::InvalidWireRecord);
        }
        Ok(expected)
    }

    /// Convert untrusted UTC civil primitives through the authoritative
    /// Rust calendar and leap-second schedule; no binding-side arithmetic.
    ///
    /// Version and SHA-256 are checked *before* interpreting civil fields.
    /// The input may represent 23:59:60.x only on a genuinely covered
    /// positive-leap UTC date. The result is the ordinary v1 record.
    pub fn utc_civil_to_tai_record_v1(
        &self,
        version: &str,
        expected_sha256: &str,
        civil: UtcCivilInputV1,
    ) -> Result<UtcTaiRecordV1, UtcLeapError> {
        self.select_matching_sha256(version, expected_sha256)?;
        let day = UtcDay::from_gregorian(civil.year, civil.month, civil.day)?;
        let clock = UtcClockTime::new(civil.hour, civil.minute, civil.second)?;
        self.utc_to_tai_record_v1(version, expected_sha256, day, clock)
    }

    /// Convert untrusted split TAI primitives without treating a UTC/TT/TDB
    /// date as TAI, collapsing the split Julian date or using host clocks.
    ///
    /// The exact schedule version and expected source SHA-256 are checked
    /// first; a malformed fraction returns the existing invalid-wire-record
    /// code, and a non-TAI label returns wrong-input-scale.
    pub fn tai_split_to_utc_record_v1(
        &self,
        version: &str,
        expected_sha256: &str,
        split: &TaiSplitInputV1,
    ) -> Result<UtcTaiRecordV1, UtcLeapError> {
        self.select_matching_sha256(version, expected_sha256)?;
        if split.time_scale != "TAI" {
            return Err(UtcLeapError::WrongInputScale);
        }
        let tai = JulianDate::new(split.day_number, split.midnight_fraction, TimeScale::Tai)
            .map_err(|_| UtcLeapError::InvalidWireRecord)?;
        self.tai_to_utc_record_v1(version, expected_sha256, tai)
    }

    /// Build a v1 wire-ready scalar record from UTC after exact-version and
    /// expected-source-SHA-256 checks. No serialization or I/O is performed.
    pub fn utc_to_tai_record_v1(
        &self,
        version: &str,
        expected_sha256: &str,
        day: UtcDay,
        clock: UtcClockTime,
    ) -> Result<UtcTaiRecordV1, UtcLeapError> {
        let checked = self.utc_to_tai_checked(version, expected_sha256, day, clock)?;
        Ok(UtcTaiRecordV1::from_components(checked, day, clock))
    }

    /// Build the same v1 scalar record from TAI. It returns the *same*
    /// field meanings as `utc_to_tai_record_v1`, regardless of direction,
    /// including reconstructing 23:59:60.x where valid.
    pub fn tai_to_utc_record_v1(
        &self,
        version: &str,
        expected_sha256: &str,
        tai: JulianDate,
    ) -> Result<UtcTaiRecordV1, UtcLeapError> {
        let checked = self.tai_to_utc_checked(version, expected_sha256, tai)?;
        let (day, clock) = checked.value;
        // Recombine metadata with the original TAI input: reconstructing
        // TAI again from the UTC result would add another f64 round trip.
        let metadata = VersionedUtcResult {
            version: checked.version,
            provenance: checked.provenance,
            source_sha256: checked.source_sha256,
            expires_utc_day: checked.expires_utc_day,
            value: tai,
        };
        Ok(UtcTaiRecordV1::from_components(metadata, day, clock))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::{JulianDate, TimeScale};
    use crate::utc_leaps::VersionedUtcLeapSchedule;

    const PINNED_SHA: &str = "e6120e71c65ff1969a4f63c285d6b99df8b02d9bdb90331669384d63534c41dd";

    fn catalog() -> UtcLeapCatalog {
        let mut catalog = UtcLeapCatalog::new();
        catalog
            .register(VersionedUtcLeapSchedule::iana_2026b().unwrap())
            .unwrap();
        catalog
    }

    #[test]
    fn historical_positive_leap_has_same_scalar_record_in_both_directions() {
        let catalog = catalog();
        let day = UtcDay::from_gregorian(2016, 12, 31).unwrap();
        let clock = UtcClockTime::new(23, 59, 60.5).unwrap();
        let forward = catalog
            .utc_to_tai_record_v1("iana-tzdb-2026b", PINNED_SHA, day, clock)
            .unwrap();
        assert_eq!(forward.schema, UTC_TAI_RECORD_V1_SCHEMA);
        assert_eq!(forward.leap_table_version, "iana-tzdb-2026b");
        assert_eq!(forward.leap_table_sha256, PINNED_SHA);
        assert!(forward.leap_table_provenance.contains("tzdb-2026b"));
        assert_eq!(forward.leap_expires_utc_day_number, 2_461_403);
        assert_eq!(forward.utc_day_number, 2_457_754);
        assert_eq!(forward.utc_hour, 23);
        assert_eq!(forward.utc_minute, 59);
        assert_eq!(forward.utc_second, 60.5);
        assert_eq!(forward.tai_day_number, 2_457_755);
        assert_eq!(forward.tai_time_scale, "TAI");
        assert!((forward.tai_midnight_fraction * 86_400.0 - 36.5).abs() < 1e-8);

        let tai = catalog
            .utc_to_tai("iana-tzdb-2026b", day, clock)
            .unwrap()
            .value;
        let reverse = catalog
            .tai_to_utc_record_v1("iana-tzdb-2026b", PINNED_SHA, tai)
            .unwrap();
        // The inverse reconstructs UTC fractional seconds through
        // floating-point division; compare the numeric field with a
        // documented tolerance instead of requiring bitwise identity.
        assert_eq!(forward.schema, reverse.schema);
        assert_eq!(forward.leap_table_version, reverse.leap_table_version);
        assert_eq!(forward.leap_table_sha256, reverse.leap_table_sha256);
        assert_eq!(
            forward.leap_expires_utc_day_number,
            reverse.leap_expires_utc_day_number
        );
        assert_eq!(forward.utc_day_number, reverse.utc_day_number);
        assert_eq!(forward.utc_hour, reverse.utc_hour);
        assert_eq!(forward.utc_minute, reverse.utc_minute);
        assert!((forward.utc_second - reverse.utc_second).abs() < 1e-8);
        assert_eq!(forward.tai_day_number, reverse.tai_day_number);
        assert_eq!(forward.tai_midnight_fraction, reverse.tai_midnight_fraction);
    }

    #[test]
    fn metadata_and_numeric_fields_outlive_catalog_without_rust_type_handles() {
        let record = {
            let catalog = catalog();
            catalog
                .utc_to_tai_record_v1(
                    "iana-tzdb-2026b",
                    PINNED_SHA,
                    UtcDay::from_gregorian(2017, 1, 1).unwrap(),
                    UtcClockTime::new(0, 0, 0.0).unwrap(),
                )
                .unwrap()
        };
        assert_eq!(record.leap_table_sha256, PINNED_SHA);
        assert_eq!(record.tai_time_scale, "TAI");
        assert_eq!(record.tai_day_number, 2_457_755);
        assert!((record.tai_midnight_fraction * 86_400.0 - 37.0).abs() < 1e-8);
    }

    #[test]
    fn civil_and_split_scalar_inputs_roundtrip_without_binding_side_arithmetic() {
        let catalog = catalog();
        let civil = UtcCivilInputV1 {
            year: 2016,
            month: 12,
            day: 31,
            hour: 23,
            minute: 59,
            second: 60.5,
        };
        let forward = catalog
            .utc_civil_to_tai_record_v1("iana-tzdb-2026b", PINNED_SHA, civil)
            .unwrap();
        assert_eq!(forward.schema, UTC_TAI_RECORD_V1_SCHEMA);
        assert_eq!(forward.utc_day_number, 2_457_754);
        assert_eq!(forward.utc_second, 60.5);
        assert_eq!(forward.tai_day_number, 2_457_755);
        assert!((forward.tai_midnight_fraction * 86_400.0 - 36.5).abs() < 1e-8);
        assert_eq!(forward.leap_table_sha256, PINNED_SHA);

        let split = TaiSplitInputV1 {
            day_number: forward.tai_day_number,
            midnight_fraction: forward.tai_midnight_fraction,
            time_scale: "TAI".to_owned(),
        };
        let reverse = catalog
            .tai_split_to_utc_record_v1("iana-tzdb-2026b", PINNED_SHA, &split)
            .unwrap();
        assert_eq!(reverse.utc_day_number, forward.utc_day_number);
        assert_eq!(reverse.utc_hour, 23);
        assert_eq!(reverse.utc_minute, 59);
        assert!((reverse.utc_second - 60.5).abs() < 1e-8);
        assert_eq!(reverse.tai_midnight_fraction, forward.tai_midnight_fraction);
        assert_eq!(reverse.leap_table_sha256, forward.leap_table_sha256);
        assert_eq!(catalog.validate_record_v1(&reverse), Ok(()));
    }

    #[test]
    fn received_record_revalidates_all_fields_and_returns_rust_canonical_output() {
        let catalog = catalog();
        let original = catalog
            .utc_civil_to_tai_record_v1(
                "iana-tzdb-2026b",
                PINNED_SHA,
                UtcCivilInputV1 {
                    year: 2016,
                    month: 12,
                    day: 31,
                    hour: 23,
                    minute: 59,
                    second: 60.5,
                },
            )
            .unwrap();
        let external = ReceivedUtcTaiRecordV1::from(&original);
        let canonical = catalog.validate_received_record_v1(&external).unwrap();
        assert_eq!(canonical.leap_table_sha256, PINNED_SHA);
        assert_eq!(canonical.tai_day_number, original.tai_day_number);
        assert_eq!(canonical.tai_midnight_fraction, original.tai_midnight_fraction);
        assert!((canonical.utc_second - 60.5).abs() < 1e-8);
        assert_eq!(catalog.validate_record_v1(&canonical), Ok(()));

        let mut inconsistent = external.clone();
        inconsistent.utc_second += 0.1;
        assert!(matches!(
            catalog.validate_received_record_v1(&inconsistent),
            Err(UtcLeapError::InvalidWireRecord)
        ));
        inconsistent = external.clone();
        inconsistent.utc_hour = 22;
        assert!(matches!(
            catalog.validate_received_record_v1(&inconsistent),
            Err(UtcLeapError::InvalidWireRecord)
        ));
        inconsistent = external.clone();
        inconsistent.utc_day_number += 1;
        assert!(matches!(
            catalog.validate_received_record_v1(&inconsistent),
            Err(UtcLeapError::InvalidWireRecord)
        ));
        inconsistent = external.clone();
        inconsistent.leap_table_provenance.push_str(" altered");
        assert!(matches!(
            catalog.validate_received_record_v1(&inconsistent),
            Err(UtcLeapError::InvalidWireRecord)
        ));
        inconsistent = external.clone();
        inconsistent.leap_expires_utc_day_number -= 1;
        assert!(matches!(
            catalog.validate_received_record_v1(&inconsistent),
            Err(UtcLeapError::InvalidWireRecord)
        ));
    }

    #[test]
    fn received_record_rejects_wrong_schema_digest_scale_and_nonfinite_time() {
        let catalog = catalog();
        let original = catalog
            .utc_civil_to_tai_record_v1(
                "iana-tzdb-2026b",
                PINNED_SHA,
                UtcCivilInputV1 {
                    year: 2017,
                    month: 1,
                    day: 1,
                    hour: 0,
                    minute: 0,
                    second: 0.0,
                },
            )
            .unwrap();
        let external = ReceivedUtcTaiRecordV1::from(&original);
        let mut bad = external.clone();
        bad.schema = "virgostars.utc-tai.v2".to_owned();
        assert!(matches!(
            catalog.validate_received_record_v1(&bad),
            Err(UtcLeapError::InvalidWireRecord)
        ));
        bad = external.clone();
        bad.tai_time_scale = "TT".to_owned();
        assert!(matches!(
            catalog.validate_received_record_v1(&bad),
            Err(UtcLeapError::InvalidWireRecord)
        ));
        bad = external.clone();
        bad.leap_table_sha256 = "0".repeat(64);
        assert!(matches!(
            catalog.validate_received_record_v1(&bad),
            Err(UtcLeapError::SourceDigestMismatch)
        ));
        bad = external.clone();
        bad.leap_table_version = "unknown".to_owned();
        assert!(matches!(
            catalog.validate_received_record_v1(&bad),
            Err(UtcLeapError::UnknownVersion)
        ));
        bad = external.clone();
        bad.tai_midnight_fraction = f64::NAN;
        assert!(matches!(
            catalog.validate_received_record_v1(&bad),
            Err(UtcLeapError::InvalidWireRecord)
        ));
        bad = external.clone();
        bad.tai_midnight_fraction = 1.0;
        assert!(matches!(
            catalog.validate_received_record_v1(&bad),
            Err(UtcLeapError::InvalidWireRecord)
        ));
        bad = external.clone();
        bad.utc_second = f64::NAN;
        assert!(matches!(
            catalog.validate_received_record_v1(&bad),
            Err(UtcLeapError::InvalidWireRecord)
        ));
    }

    #[test]
    fn scalar_inputs_fail_closed_on_digest_calendar_clock_and_time_scale() {
        let catalog = catalog();
        let good = UtcCivilInputV1 {
            year: 2017,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0.0,
        };
        assert!(matches!(
            catalog.utc_civil_to_tai_record_v1("iana-tzdb-2026b", "wrong", good),
            Err(UtcLeapError::SourceDigestMismatch)
        ));
        assert!(matches!(
            catalog.utc_civil_to_tai_record_v1("unknown", PINNED_SHA, good),
            Err(UtcLeapError::UnknownVersion)
        ));
        let invalid_day = UtcCivilInputV1 {
            month: 2,
            day: 30,
            ..good
        };
        assert!(matches!(
            catalog.utc_civil_to_tai_record_v1("iana-tzdb-2026b", PINNED_SHA, invalid_day),
            Err(UtcLeapError::InvalidGregorianDate)
        ));
        let nonexistent_leap = UtcCivilInputV1 {
            hour: 23,
            minute: 59,
            second: 60.0,
            ..good
        };
        assert!(matches!(
            catalog.utc_civil_to_tai_record_v1("iana-tzdb-2026b", PINNED_SHA, nonexistent_leap),
            Err(UtcLeapError::ClockTimeOutsideDay)
        ));
        let nan_clock = UtcCivilInputV1 {
            second: f64::NAN,
            ..good
        };
        assert!(matches!(
            catalog.utc_civil_to_tai_record_v1("iana-tzdb-2026b", PINNED_SHA, nan_clock),
            Err(UtcLeapError::InvalidClockTime)
        ));
        let expired = UtcCivilInputV1 {
            year: 2026,
            month: 12,
            day: 28,
            ..good
        };
        assert!(matches!(
            catalog.utc_civil_to_tai_record_v1("iana-tzdb-2026b", PINNED_SHA, expired),
            Err(UtcLeapError::OutsideCoverage)
        ));

        let accepted = catalog
            .utc_civil_to_tai_record_v1("iana-tzdb-2026b", PINNED_SHA, good)
            .unwrap();
        let split = TaiSplitInputV1 {
            day_number: accepted.tai_day_number,
            midnight_fraction: accepted.tai_midnight_fraction,
            time_scale: "TT".to_owned(),
        };
        assert!(matches!(
            catalog.tai_split_to_utc_record_v1("iana-tzdb-2026b", PINNED_SHA, &split),
            Err(UtcLeapError::WrongInputScale)
        ));
        let invalid_fraction = TaiSplitInputV1 {
            midnight_fraction: f64::NAN,
            time_scale: "TAI".to_owned(),
            ..split
        };
        assert!(matches!(
            catalog.tai_split_to_utc_record_v1("iana-tzdb-2026b", PINNED_SHA, &invalid_fraction),
            Err(UtcLeapError::InvalidWireRecord)
        ));
        assert!(matches!(
            catalog.tai_split_to_utc_record_v1("iana-tzdb-2026b", "wrong", &invalid_fraction),
            Err(UtcLeapError::SourceDigestMismatch)
        ));
    }

    #[test]
    fn input_validation_and_digest_fail_closed_before_creating_records() {
        let catalog = catalog();
        let day = UtcDay::from_gregorian(2017, 1, 1).unwrap();
        let midnight = UtcClockTime::new(0, 0, 0.0).unwrap();
        assert!(matches!(
            catalog.utc_to_tai_record_v1("iana-tzdb-2026b", "0", day, midnight),
            Err(UtcLeapError::SourceDigestMismatch)
        ));
        assert!(matches!(
            catalog.utc_to_tai_record_v1("unknown", PINNED_SHA, day, midnight),
            Err(UtcLeapError::UnknownVersion)
        ));
        assert!(matches!(
            catalog.utc_to_tai_record_v1(
                "iana-tzdb-2026b",
                PINNED_SHA,
                day,
                UtcClockTime::new(23, 59, 60.0).unwrap(),
            ),
            Err(UtcLeapError::ClockTimeOutsideDay)
        ));
        let tt = JulianDate::new(2_457_755, 0.5, TimeScale::Tt).unwrap();
        assert!(matches!(
            catalog.tai_to_utc_record_v1("iana-tzdb-2026b", PINNED_SHA, tt),
            Err(UtcLeapError::WrongInputScale)
        ));
    }

    #[test]
    fn synthetic_negative_leap_record_preserves_a_missing_utc_second() {
        // This is a hypothetical scenario, NOT an IERS announcement.
        // A -1s step at 2027-01-01 makes 2026-12-31 only 86,399 s long.
        let original = VersionedUtcLeapSchedule::iana_2026b().unwrap();
        let source = include_str!("../data/iana-tzdb-2026b-leap-seconds.list")
            .replace("#@ 4007404800", "#@ 4023388800");
        let reviewed = format!("{source}4007750400 36\n");
        let future = VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
            "synthetic-negative-2027",
            "test only: hypothetical negative leap; not IANA or IERS",
            &reviewed,
        )
        .unwrap();
        let future_digest = future.source_sha256().to_owned();

        let mut catalog = UtcLeapCatalog::new();
        catalog.register(original).unwrap();
        catalog.register(future).unwrap();

        let last_day = UtcDay::from_gregorian(2026, 12, 31).unwrap();
        let next_day = UtcDay::from_gregorian(2027, 1, 1).unwrap();
        let before = catalog
            .utc_to_tai_record_v1(
                "synthetic-negative-2027",
                &future_digest,
                last_day,
                UtcClockTime::new(23, 59, 58.5).unwrap(),
            )
            .unwrap();
        let after = catalog
            .utc_to_tai_record_v1(
                "synthetic-negative-2027",
                &future_digest,
                next_day,
                UtcClockTime::new(0, 0, 0.0).unwrap(),
            )
            .unwrap();

        assert_eq!(before.schema, UTC_TAI_RECORD_V1_SCHEMA);
        assert_eq!(before.leap_table_version, "synthetic-negative-2027");
        assert_eq!(before.leap_table_sha256, future_digest);
        assert_eq!(before.utc_day_number, last_day.julian_day_number());
        assert_eq!(before.utc_second, 58.5);
        assert_eq!(before.tai_day_number, next_day.julian_day_number());
        assert_eq!(after.tai_day_number, next_day.julian_day_number());
        assert!((before.tai_midnight_fraction * 86_400.0 - 35.5).abs() < 1e-8);
        assert!((after.tai_midnight_fraction * 86_400.0 - 36.0).abs() < 1e-8);
        assert_eq!(after.utc_second, 0.0);
        assert_eq!(
            before.leap_expires_utc_day_number,
            after.leap_expires_utc_day_number
        );

        for (clock, tai_record) in [
            (UtcClockTime::new(23, 59, 58.5).unwrap(), &before),
            (UtcClockTime::new(0, 0, 0.0).unwrap(), &after),
        ] {
            let utc = catalog
                .tai_to_utc_record_v1(
                    "synthetic-negative-2027",
                    &future_digest,
                    JulianDate::new(
                        tai_record.tai_day_number,
                        tai_record.tai_midnight_fraction,
                        TimeScale::Tai,
                    )
                    .unwrap(),
                )
                .unwrap();
            assert_eq!(utc.leap_table_sha256, future_digest);
            assert_eq!(utc.tai_day_number, tai_record.tai_day_number);
            assert_eq!(utc.utc_hour, clock.hour());
            assert_eq!(utc.utc_minute, clock.minute());
            assert!((utc.utc_second - clock.second()).abs() < 1e-8);
            assert_eq!(utc.utc_day_number, tai_record.utc_day_number);
        }

        let missing = catalog
            .utc_to_tai_record_v1(
                "synthetic-negative-2027",
                &future_digest,
                last_day,
                UtcClockTime::new(23, 59, 59.0).unwrap(),
            )
            .unwrap_err();
        assert_eq!(missing, UtcLeapError::ClockTimeOutsideDay);
        assert_eq!(UtcTaiErrorV1::from(missing).code, "clock_time_outside_day");

        // Registering the synthetic 2027 table must not lift the original
        // historical 2026b expiry or silently select a newer version.
        assert!(matches!(
            catalog.utc_to_tai_record_v1(
                "iana-tzdb-2026b",
                "e6120e71c65ff1969a4f63c285d6b99df8b02d9bdb90331669384d63534c41dd",
                last_day,
                UtcClockTime::new(23, 59, 58.5).unwrap(),
            ),
            Err(UtcLeapError::OutsideCoverage)
        ));
    }

    #[test]
    fn negative_leap_synthetic_source_rejects_prior_pinned_sha256() {
        let source = include_str!("../data/iana-tzdb-2026b-leap-seconds.list")
            .replace("#@ 4007404800", "#@ 4023388800");
        let future_text = format!("{source}4007750400 36\n");
        let future = VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
            "synthetic-negative-2027",
            "test only",
            &future_text,
        )
        .unwrap();
        let mut catalog = UtcLeapCatalog::new();
        catalog.register(future).unwrap();
        let old_pinned_digest = "e6120e71c65ff1969a4f63c285d6b99df8b02d9bdb90331669384d63534c41dd";
        let day = UtcDay::from_gregorian(2026, 12, 31).unwrap();
        assert!(matches!(
            catalog.utc_to_tai_record_v1(
                "synthetic-negative-2027",
                old_pinned_digest,
                day,
                UtcClockTime::new(23, 59, 58.0).unwrap(),
            ),
            Err(UtcLeapError::SourceDigestMismatch)
        ));
    }

    #[test]
    fn record_validator_accepts_both_directions_on_real_positive_leap() {
        let catalog = catalog();
        let utc_day = UtcDay::from_gregorian(2016, 12, 31).unwrap();
        let clock = UtcClockTime::new(23, 59, 60.5).unwrap();
        let record = catalog
            .utc_to_tai_record_v1("iana-tzdb-2026b", PINNED_SHA, utc_day, clock)
            .unwrap();
        assert_eq!(catalog.validate_record_v1(&record), Ok(()));

        let tai = JulianDate::new(
            record.tai_day_number,
            record.tai_midnight_fraction,
            TimeScale::Tai,
        )
        .unwrap();
        let reverse = catalog
            .tai_to_utc_record_v1("iana-tzdb-2026b", PINNED_SHA, tai)
            .unwrap();
        assert_eq!(catalog.validate_record_v1(&reverse), Ok(()));
    }

    #[test]
    fn record_validator_rejects_inconsistent_schema_metadata_and_times() {
        let catalog = catalog();
        let day = UtcDay::from_gregorian(2017, 1, 1).unwrap();
        let original = catalog
            .utc_to_tai_record_v1(
                "iana-tzdb-2026b",
                PINNED_SHA,
                day,
                UtcClockTime::new(0, 0, 0.25).unwrap(),
            )
            .unwrap();
        assert_eq!(catalog.validate_record_v1(&original), Ok(()));

        let mut modified = original.clone();
        modified.schema = "virgostars.utc-tai.v2";
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::InvalidWireRecord)
        );

        modified = original.clone();
        modified.tai_time_scale = "TT";
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::InvalidWireRecord)
        );

        modified = original.clone();
        modified.leap_table_provenance.push_str("/unreviewed");
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::InvalidWireRecord)
        );

        modified = original.clone();
        modified.leap_expires_utc_day_number += 1;
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::InvalidWireRecord)
        );

        modified = original.clone();
        modified.utc_day_number += 1;
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::InvalidWireRecord)
        );

        modified = original.clone();
        modified.utc_hour = 1;
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::InvalidWireRecord)
        );

        modified = original.clone();
        modified.utc_minute = 1;
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::InvalidWireRecord)
        );

        modified = original.clone();
        modified.utc_second += 0.5;
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::InvalidWireRecord)
        );

        modified = original.clone();
        modified.utc_second = f64::NAN;
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::InvalidWireRecord)
        );

        modified = original.clone();
        modified.tai_midnight_fraction += 1.0 / 86_400.0;
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::InvalidWireRecord)
        );

        modified = original.clone();
        modified.tai_midnight_fraction = f64::INFINITY;
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::InvalidWireRecord)
        );

        modified = original.clone();
        modified.leap_table_version = "unknown".to_owned();
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::UnknownVersion)
        );

        modified = original;
        modified.leap_table_sha256 = "0".repeat(64);
        assert_eq!(
            catalog.validate_record_v1(&modified),
            Err(UtcLeapError::SourceDigestMismatch)
        );
        assert_eq!(
            UtcTaiErrorV1::from(UtcLeapError::InvalidWireRecord).code,
            "invalid_wire_record"
        );
    }

    #[test]
    fn all_leap_error_variants_have_unique_stable_machine_codes() {
        use std::collections::HashSet;

        let expected = [
            (UtcLeapError::InvalidGregorianDate, "invalid_gregorian_date"),
            (UtcLeapError::Before1972, "before_1972"),
            (UtcLeapError::InvalidCoverage, "invalid_coverage"),
            (UtcLeapError::InvalidInitialOffset, "invalid_initial_offset"),
            (UtcLeapError::InvalidTransition, "invalid_transition"),
            (UtcLeapError::InvalidOffsetStep, "invalid_offset_step"),
            (UtcLeapError::OutsideCoverage, "outside_coverage"),
            (UtcLeapError::InvalidClockTime, "invalid_clock_time"),
            (UtcLeapError::ClockTimeOutsideDay, "clock_time_outside_day"),
            (UtcLeapError::UnrepresentableTai, "unrepresentable_tai"),
            (UtcLeapError::WrongInputScale, "wrong_input_scale"),
            (
                UtcLeapError::InvalidPinnedLeapData,
                "invalid_pinned_leap_data",
            ),
            (
                UtcLeapError::InvalidReviewedSnapshot,
                "invalid_reviewed_snapshot",
            ),
            (
                UtcLeapError::InvalidSnapshotProvenance,
                "invalid_snapshot_provenance",
            ),
            (UtcLeapError::DuplicateVersion, "duplicate_version"),
            (UtcLeapError::UnknownVersion, "unknown_version"),
            (UtcLeapError::SourceDigestMismatch, "source_digest_mismatch"),
            (UtcLeapError::InvalidWireRecord, "invalid_wire_record"),
        ];
        let mut distinct = HashSet::new();
        for (variant, code) in expected {
            let record = UtcTaiErrorV1::from(variant);
            assert_eq!(record.schema, UTC_TAI_ERROR_V1_SCHEMA);
            assert_eq!(record.code, code);
            assert!(record
                .code
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit()));
            assert!(distinct.insert(record.code), "duplicate machine error code");
        }
        assert_eq!(distinct.len(), 18);
    }

    #[test]
    fn checked_conversion_failures_map_to_documented_machine_codes() {
        let catalog = catalog();
        let day = UtcDay::from_gregorian(2017, 1, 1).unwrap();
        let clock = UtcClockTime::new(0, 0, 0.0).unwrap();

        let digest_error = catalog
            .utc_to_tai_record_v1("iana-tzdb-2026b", "wrong", day, clock)
            .unwrap_err();
        assert_eq!(
            UtcTaiErrorV1::from(digest_error).code,
            "source_digest_mismatch"
        );
        let version_error = catalog
            .utc_to_tai_record_v1("unknown", PINNED_SHA, day, clock)
            .unwrap_err();
        assert_eq!(UtcTaiErrorV1::from(version_error).code, "unknown_version");
        let leap_error = catalog
            .utc_to_tai_record_v1(
                "iana-tzdb-2026b",
                PINNED_SHA,
                day,
                UtcClockTime::new(23, 59, 60.0).unwrap(),
            )
            .unwrap_err();
        assert_eq!(
            UtcTaiErrorV1::from(leap_error).code,
            "clock_time_outside_day"
        );
    }

    #[test]
    fn previously_expired_2026b_schedule_cannot_issue_new_record() {
        let catalog = catalog();
        assert!(matches!(
            catalog.utc_to_tai_record_v1(
                "iana-tzdb-2026b",
                PINNED_SHA,
                UtcDay::from_gregorian(2026, 12, 28).unwrap(),
                UtcClockTime::new(0, 0, 0.0).unwrap(),
            ),
            Err(UtcLeapError::OutsideCoverage)
        ));
    }
}
