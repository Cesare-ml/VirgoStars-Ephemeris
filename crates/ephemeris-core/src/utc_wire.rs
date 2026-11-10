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

use crate::calendar::JulianDate;
use crate::utc_leaps::{
    UtcClockTime, UtcDay, UtcLeapCatalog, UtcLeapError, VersionedUtcResult,
};

/// Identifier of the experimental scalar-record schema.
///
/// Bump the schema version and add a new record type instead of silently
/// reinterpreting existing fields when future bindings are introduced.
pub const UTC_TAI_RECORD_V1_SCHEMA: &str = "virgostars.utc-tai.v1";

/// Language-neutral scalar record for one UTC ↔ TAI instant.
///
/// UTC fields describe a named Gregorian civil day and clock, **not** a
/// uniform Julian date in UTC. TAI fields use the project's split JD
/// convention: \`tai_day_number\` is the noon-based integer day number,
/// \`tai_midnight_fraction\` is the fraction of an SI day since the
/// preceding midnight, and the time scale is explicitly \`"TAI"\`.
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
    /// field meanings as \`utc_to_tai_record_v1\`, regardless of direction,
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

    const PINNED_SHA: &str =
        "e6120e71c65ff1969a4f63c285d6b99df8b02d9bdb90331669384d63534c41dd";

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
        assert_eq!(forward, reverse);
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
