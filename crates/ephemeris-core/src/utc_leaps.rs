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

//! Explicit post-1972 UTC leap-second day schedule.
//!
//! Models UTC day lengths, TAI-UTC offsets and bidirectional UTC/TAI
//! conversions using caller-provided transitions or a strictly bounded
//! public-domain IANA 2026b snapshot. It never implicitly converts to
//! TT/TDB, downloads updates or extrapolates beyond declared coverage.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::calendar::{Calendar, CalendarDateTime, JulianDate, TimeScale};

/// A Gregorian UTC calendar day (not a uniform-scale Julian date).
///
/// The stored integer is the Julian day number of the civil day at noon,
/// matching the integer day number used by the split \`calendar::JulianDate\`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtcDay {
    julian_day_number: i64,
}

/// A leap-second schedule error. None of these conditions can be repaired
/// by silently extrapolating or assuming UTC has 86,400 seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UtcLeapError {
    InvalidGregorianDate,
    /// The integer leap-second model is not valid for pre-1972 UTC.
    Before1972,
    InvalidCoverage,
    InvalidInitialOffset,
    /// Duplicated, unordered, out-of-coverage or non-month-boundary transition.
    InvalidTransition,
    /// A leap event must change TAI-UTC by exactly +1 or -1 second.
    InvalidOffsetStep,
    OutsideCoverage,
    /// Invalid hour, minute, fractional second, or leap-second placement.
    InvalidClockTime,
    /// This clock reading does not exist on the named UTC day.
    ClockTimeOutsideDay,
    /// Conversion to a normalized split TAI Julian date was not representable.
    UnrepresentableTai,
    /// Only a TAI-tagged JulianDate can be converted back to UTC.
    WrongInputScale,
    /// Bundled version-pinned IANA leap-second history is malformed.
    InvalidPinnedLeapData,
    /// Invalid or out-of-bounds reviewed numeric NTP leap snapshot.
    InvalidReviewedSnapshot,
    /// Version identifier or mandatory provenance field is missing/unsafe.
    InvalidSnapshotProvenance,
    /// An approved version ID was registered more than once in this catalog.
    DuplicateVersion,
    /// The requested version was not explicitly registered.
    UnknownVersion,
    /// The exact snapshot bytes disagree with the requested SHA-256.
    SourceDigestMismatch,
    /// An incoming v1 scalar record has inconsistent schema, fields or metadata.
    InvalidWireRecord,
}

impl fmt::Display for UtcLeapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGregorianDate => write!(f, "invalid proleptic Gregorian UTC date"),
            Self::Before1972 => write!(f, "integer UTC leap-second schedule begins in 1972"),
            Self::InvalidCoverage => write!(f, "invalid UTC day coverage interval"),
            Self::InvalidInitialOffset => write!(f, "invalid initial TAI minus UTC offset"),
            Self::InvalidTransition => write!(f, "invalid UTC leap-second transition date/order"),
            Self::InvalidOffsetStep => write!(f, "TAI minus UTC must change by exactly one second"),
            Self::OutsideCoverage => write!(f, "UTC day lies outside approved schedule coverage"),
            Self::InvalidClockTime => write!(f, "invalid UTC clock fields"),
            Self::ClockTimeOutsideDay => write!(f, "UTC clock reading does not exist on this day"),
            Self::UnrepresentableTai => {
                write!(f, "TAI Julian date cannot represent this UTC instant")
            }
            Self::WrongInputScale => write!(f, "TAI input time scale is required"),
            Self::InvalidPinnedLeapData => write!(f, "invalid pinned IANA leap-second snapshot"),
            Self::InvalidReviewedSnapshot => write!(f, "invalid reviewed leap-second snapshot"),
            Self::InvalidSnapshotProvenance => write!(f, "invalid leap-second snapshot provenance"),
            Self::DuplicateVersion => write!(f, "leap-second schedule version already registered"),
            Self::UnknownVersion => write!(f, "requested leap-second schedule version is unknown"),
            Self::SourceDigestMismatch => {
                write!(
                    f,
                    "leap-second source SHA-256 differs from the expected digest"
                )
            }
            Self::InvalidWireRecord => write!(f, "invalid or inconsistent UTC/TAI v1 record"),
        }
    }
}

impl Error for UtcLeapError {}

impl UtcDay {
    /// Validate a Gregorian UTC day in the post-1972 leap-second regime.
    ///
    /// This uses the existing *calendar-only* conversion to a Julian day
    /// number. The internal dummy TAI scale label is never exposed and never
    /// interpreted as converting between UTC and TAI.
    pub fn from_gregorian(year: i32, month: u8, day: u8) -> Result<Self, UtcLeapError> {
        if year < 1972 {
            return Err(UtcLeapError::Before1972);
        }
        let midnight = CalendarDateTime {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0.0,
            scale: TimeScale::Tai,
        }
        .to_julian_date(Calendar::ProlepticGregorian)
        .map_err(|_| UtcLeapError::InvalidGregorianDate)?;
        Ok(Self {
            julian_day_number: midnight.day_number(),
        })
    }

    /// Integer Julian day number corresponding to the named Gregorian day.
    pub fn julian_day_number(self) -> i64 {
        self.julian_day_number
    }

    fn starts_month(self) -> Result<bool, UtcLeapError> {
        let day = JulianDate::new(self.julian_day_number, 0.0, TimeScale::Tai)
            .map_err(|_| UtcLeapError::InvalidGregorianDate)?;
        let date = day
            .to_calendar(Calendar::ProlepticGregorian)
            .map_err(|_| UtcLeapError::InvalidGregorianDate)?;
        Ok(date.day == 1)
    }
}

/// A UTC civil-clock reading, independently of the named day.
///
/// Second 60 (including its fractional portion) is syntactically permitted
/// **only at 23:59**. It is valid for a particular date only if the explicitly
/// supplied leap-second schedule confirms a positive leap on that day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UtcClockTime {
    hour: u8,
    minute: u8,
    second: f64,
}

impl UtcClockTime {
    /// Construct a UTC clock reading without assuming a leap-second calendar.
    pub fn new(hour: u8, minute: u8, second: f64) -> Result<Self, UtcLeapError> {
        if hour > 23
            || minute > 59
            || !second.is_finite()
            || !(0.0..61.0).contains(&second)
            || (second >= 60.0 && (hour != 23 || minute != 59))
        {
            return Err(UtcLeapError::InvalidClockTime);
        }
        Ok(Self {
            hour,
            minute,
            second,
        })
    }

    pub fn hour(self) -> u8 {
        self.hour
    }

    pub fn minute(self) -> u8 {
        self.minute
    }

    pub fn second(self) -> f64 {
        self.second
    }

    fn seconds_since_midnight(self) -> f64 {
        f64::from(self.hour) * 3_600.0 + f64::from(self.minute) * 60.0 + self.second
    }
}

/// Transition effective at UTC 00:00 on a Gregorian day.
///
/// A positive leap second happens at 23:59:60 on the **preceding** day.
/// At the effective midnight, TAI-UTC increases by one second. This type
/// deliberately cannot be treated as a TDB/TT Julian date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UtcLeapTransition {
    pub effective_utc_day: UtcDay,
    pub tai_minus_utc_seconds: i32,
}

/// Explicit, non-extrapolating schedule of post-1972 leap-second UTC days.
///
/// Input transitions must be provided in ascending effective-date order.
/// The final covered day is inclusive; a transition on the following day
/// may describe the leap second at the final covered midnight boundary.
#[derive(Clone, Debug)]
pub struct UtcLeapSchedule {
    first_day: UtcDay,
    last_day: UtcDay,
    initial_tai_minus_utc_seconds: i32,
    transitions: Vec<UtcLeapTransition>,
}

/// Immutable, provenance-bearing wrapper around a validated UTC leap schedule.
///
/// A caller's `version` and `provenance` are declared metadata, **not**
/// independent proof of authenticity. Candidate files must first pass the
/// offline official-source SHA-1 and historical-drift audit, and be reviewed.
///
/// Each instance owns its schedule and does not mutate other loaded versions.
/// The historic `UtcLeapSchedule::from_iana_2026b()` remains unchanged.
#[derive(Clone, Debug)]
pub struct VersionedUtcLeapSchedule {
    version: String,
    provenance: String,
    source_sha256: String,
    expires_utc_day: UtcDay,
    schedule: UtcLeapSchedule,
}

impl VersionedUtcLeapSchedule {
    /// Construct a named, immutable view of the repository's pinned IANA
    /// tzdb-2026b series, without changing the legacy schedule API.
    pub fn iana_2026b() -> Result<Self, UtcLeapError> {
        Ok(Self {
            version: "iana-tzdb-2026b".to_owned(),
            provenance: "https://data.iana.org/time-zones/tzdb-2026b/leap-seconds.list".to_owned(),
            source_sha256: crate::fingerprint::sha256_hex(
                include_str!("../data/iana-tzdb-2026b-leap-seconds.list").as_bytes(),
            ),
            expires_utc_day: UtcDay::from_gregorian(2026, 12, 28)?,
            schedule: UtcLeapSchedule::from_iana_2026b()?,
        })
    }

    /// Load **reviewed, curated numeric** NTP leap data from memory.
    ///
    /// This accepts the format produced by the offline update proposal:
    /// comments, exactly one `#@ EXPIRY_NTP` marker, followed by ascending
    /// `NTP_MIDNIGHT TAI_MINUS_UTC_SECONDS` rows starting at 1972-01-01.
    /// NTP seconds are counted from 1900-01-01 UTC. Each leap transition
    /// must occur at a month boundary and change TAI-UTC by exactly +/-1.
    ///
    /// Unprocessed upstream IANA `#$` update and `#h` checksum markers
    /// are rejected rather than ignored: this importer checks numeric
    /// structure, **not** official checksum integrity or publisher identity.
    /// First audit the official file separately, then pass the reviewed
    /// numeric extract. The `version` and `provenance` are declarations;
    /// no filesystem, network or global "current version" is accessed.
    pub fn from_reviewed_numeric_snapshot(
        version: &str,
        provenance: &str,
        source: &str,
    ) -> Result<Self, UtcLeapError> {
        const FIRST_NTP: i64 = 2_272_060_800; // 1972-01-01 UTC
        const MAX_INPUT_BYTES: usize = 128 * 1024;
        const MAX_RECORDS: usize = 1024;
        let safe_version = !version.is_empty()
            && version.len() <= 80
            && version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'));
        if !safe_version || provenance.trim().is_empty() || provenance.len() > 1024 {
            return Err(UtcLeapError::InvalidSnapshotProvenance);
        }
        if source.len() > MAX_INPUT_BYTES {
            return Err(UtcLeapError::InvalidReviewedSnapshot);
        }

        let mut expiry: Option<i64> = None;
        let mut records: Vec<(i64, i32)> = Vec::new();
        for line in source.lines() {
            let text = line.trim();
            if let Some(value) = text.strip_prefix("#@") {
                if expiry.is_some() {
                    return Err(UtcLeapError::InvalidReviewedSnapshot);
                }
                let value = value
                    .trim()
                    .parse::<i64>()
                    .map_err(|_| UtcLeapError::InvalidReviewedSnapshot)?;
                expiry = Some(value);
            } else if matches!(text.split_whitespace().next(), Some("#$" | "#h")) {
                // The upstream IANA integrity marker cannot be silently
                // ignored by a parser that does not verify the #h digest.
                return Err(UtcLeapError::InvalidReviewedSnapshot);
            } else if text.is_empty() || text.starts_with('#') {
                continue;
            } else {
                let mut parts = text.split_whitespace();
                let ntp = parts
                    .next()
                    .ok_or(UtcLeapError::InvalidReviewedSnapshot)?
                    .parse::<i64>()
                    .map_err(|_| UtcLeapError::InvalidReviewedSnapshot)?;
                let offset = parts
                    .next()
                    .ok_or(UtcLeapError::InvalidReviewedSnapshot)?
                    .parse::<i32>()
                    .map_err(|_| UtcLeapError::InvalidReviewedSnapshot)?;
                if parts.next().is_some() || records.len() >= MAX_RECORDS {
                    return Err(UtcLeapError::InvalidReviewedSnapshot);
                }
                records.push((ntp, offset));
            }
        }

        let expires_ntp = expiry.ok_or(UtcLeapError::InvalidReviewedSnapshot)?;
        if records.first() != Some(&(FIRST_NTP, 10))
            || expires_ntp <= FIRST_NTP
            || expires_ntp % 86_400 != 0
            || records.last().is_none_or(|row| row.0 >= expires_ntp)
        {
            return Err(UtcLeapError::InvalidReviewedSnapshot);
        }
        let first = UtcDay::from_gregorian(1972, 1, 1)?;
        let ntp_day = |ntp: i64| -> Result<UtcDay, UtcLeapError> {
            let delta = ntp
                .checked_sub(FIRST_NTP)
                .ok_or(UtcLeapError::InvalidReviewedSnapshot)?;
            if delta < 0 || delta % 86_400 != 0 {
                return Err(UtcLeapError::InvalidReviewedSnapshot);
            }
            let day_number = first
                .julian_day_number()
                .checked_add(delta / 86_400)
                .ok_or(UtcLeapError::InvalidReviewedSnapshot)?;
            Ok(UtcDay {
                julian_day_number: day_number,
            })
        };
        let expires_utc_day = ntp_day(expires_ntp)?;
        // Confirm both boundaries lie in the supported Gregorian calendar,
        // not merely in the machine's i64 range.
        let expiry_date = JulianDate::new(expires_utc_day.julian_day_number, 0.0, TimeScale::Tai)
            .map_err(|_| UtcLeapError::InvalidReviewedSnapshot)?;
        expiry_date
            .to_calendar(Calendar::ProlepticGregorian)
            .map_err(|_| UtcLeapError::InvalidReviewedSnapshot)?;
        let last_day = UtcDay {
            julian_day_number: expires_utc_day
                .julian_day_number
                .checked_sub(1)
                .ok_or(UtcLeapError::InvalidReviewedSnapshot)?,
        };
        let mut transitions = Vec::with_capacity(records.len() - 1);
        let mut previous_ntp = FIRST_NTP;
        for (ntp, offset) in records.into_iter().skip(1) {
            if ntp <= previous_ntp {
                return Err(UtcLeapError::InvalidReviewedSnapshot);
            }
            previous_ntp = ntp;
            let day = ntp_day(ntp)?;
            if day > last_day {
                return Err(UtcLeapError::InvalidReviewedSnapshot);
            }
            transitions.push(UtcLeapTransition {
                effective_utc_day: day,
                tai_minus_utc_seconds: offset,
            });
        }
        let schedule = UtcLeapSchedule::new(first, last_day, 10, transitions)
            .map_err(|_| UtcLeapError::InvalidReviewedSnapshot)?;

        // The independently reviewed IANA 2026b history is immutable.
        // Requiring its transition prefix alone is not enough: an added,
        // syntactically valid leap could alter the already-approved
        // 2017-2026 interval. Future data may only extend coverage and
        // add transitions after the pinned historical coverage ends.
        let pinned = UtcLeapSchedule::from_iana_2026b()?;
        if expires_utc_day <= pinned.last_day
            || !schedule.transitions.starts_with(&pinned.transitions)
            || schedule
                .transitions
                .get(pinned.transitions.len())
                .is_some_and(|next| next.effective_utc_day <= pinned.last_day)
        {
            return Err(UtcLeapError::InvalidReviewedSnapshot);
        }
        Ok(Self {
            version: version.to_owned(),
            provenance: provenance.to_owned(),
            source_sha256: crate::fingerprint::sha256_hex(source.as_bytes()),
            expires_utc_day,
            schedule,
        })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn provenance(&self) -> &str {
        &self.provenance
    }

    /// SHA-256 of exact loaded curated source bytes, lowercase hex.
    ///
    /// This identifies bytes; it does not authenticate the source or
    /// replace verification of the upstream IANA #h integrity checksum.
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }

    /// UTC midnight at which coverage ends, **exclusive**.
    pub fn expires_utc_day(&self) -> UtcDay {
        self.expires_utc_day
    }

    /// Borrow the validated schedule without changing its provenance.
    pub fn schedule(&self) -> &UtcLeapSchedule {
        &self.schedule
    }
}

/// A calendar or atomic-time value annotated with the selected leap table.
///
/// Metadata is copied into each result, so records remain interpretable even
/// after the catalog is dropped. `version` and `provenance` are declared
/// identifiers, not cryptographic evidence of the source content. Persist
/// an independently reviewed digest in the provenance when auditing results.
#[derive(Clone, Debug, PartialEq)]
pub struct VersionedUtcResult<T> {
    /// Exact schedule version explicitly selected for this conversion.
    pub version: String,
    /// Declared source provenance (not authenticated by the Rust loader).
    pub provenance: String,
    /// Exact-byte SHA-256 fingerprint of the curated snapshot (not IANA #h).
    pub source_sha256: String,
    /// First UTC day excluded from this schedule's validity.
    pub expires_utc_day: UtcDay,
    /// Converted TAI Julian date, or UTC civil day plus clock, as requested.
    pub value: T,
}

/// Explicit catalog of independently owned leap-second schedule versions.
///
/// No default, best, or latest version exists. An unknown version and a
/// duplicate registration are errors. Inserting an instance does **not**
/// validate external source authenticity: callers must first complete the
/// independent official-source audit and human review.
#[derive(Debug, Default)]
pub struct UtcLeapCatalog {
    schedules: BTreeMap<String, VersionedUtcLeapSchedule>,
}

impl UtcLeapCatalog {
    /// Create an empty catalog, deliberately with no implicit default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an independently validated, version-named schedule.
    ///
    /// The same version string cannot be replaced, even with different
    /// contents: use a new ID for a new approved data snapshot.
    pub fn register(&mut self, schedule: VersionedUtcLeapSchedule) -> Result<(), UtcLeapError> {
        let version = schedule.version().to_owned();
        if self.schedules.contains_key(&version) {
            return Err(UtcLeapError::DuplicateVersion);
        }
        self.schedules.insert(version, schedule);
        Ok(())
    }

    /// List registered version IDs in stable lexical order.
    pub fn versions(&self) -> impl Iterator<Item = &str> {
        self.schedules.keys().map(String::as_str)
    }

    /// Select an exact previously registered version, without fallback.
    pub fn select(&self, version: &str) -> Result<&VersionedUtcLeapSchedule, UtcLeapError> {
        self.schedules
            .get(version)
            .ok_or(UtcLeapError::UnknownVersion)
    }

    /// Require an exact version and matching, independently expected SHA-256.
    ///
    /// A matching digest identifies bytes, not the identity of the publisher.
    pub fn select_matching_sha256(
        &self,
        version: &str,
        expected_sha256: &str,
    ) -> Result<&VersionedUtcLeapSchedule, UtcLeapError> {
        let selected = self.select(version)?;
        if selected.source_sha256() != expected_sha256 {
            return Err(UtcLeapError::SourceDigestMismatch);
        }
        Ok(selected)
    }

    /// Convert UTC to TAI only when the selected table matches a trusted
    /// expected SHA-256 of its exact curated snapshot bytes.
    ///
    /// Both version and digest are required on every call. This does not
    /// authenticate the publisher: obtain the expected digest from a
    /// separately reviewed manifest, not the same untrusted source.
    /// An unknown version, then a mismatching digest, fails before the
    /// clock/coverage conversion is attempted.
    pub fn utc_to_tai_checked(
        &self,
        version: &str,
        expected_sha256: &str,
        day: UtcDay,
        clock: UtcClockTime,
    ) -> Result<VersionedUtcResult<JulianDate>, UtcLeapError> {
        self.select_matching_sha256(version, expected_sha256)?;
        self.utc_to_tai(version, day, clock)
    }

    /// Convert TAI to UTC only when the chosen table has the externally
    /// expected SHA-256; this retains positive leap-second semantics and
    /// the same explicit coverage checks as `tai_to_utc`.
    pub fn tai_to_utc_checked(
        &self,
        version: &str,
        expected_sha256: &str,
        tai: JulianDate,
    ) -> Result<VersionedUtcResult<(UtcDay, UtcClockTime)>, UtcLeapError> {
        self.select_matching_sha256(version, expected_sha256)?;
        self.tai_to_utc(version, tai)
    }

    /// Convert UTC to TAI using precisely the requested version, preserving
    /// that version's provenance and exclusive expiry in the result.
    pub fn utc_to_tai(
        &self,
        version: &str,
        day: UtcDay,
        clock: UtcClockTime,
    ) -> Result<VersionedUtcResult<JulianDate>, UtcLeapError> {
        let selected = self.select(version)?;
        let value = selected.schedule().utc_to_tai(day, clock)?;
        Ok(VersionedUtcResult {
            version: selected.version().to_owned(),
            provenance: selected.provenance().to_owned(),
            source_sha256: selected.source_sha256().to_owned(),
            expires_utc_day: selected.expires_utc_day(),
            value,
        })
    }

    /// Convert TAI to UTC using precisely the requested version. The output
    /// identifies the schedule that determined whether a leap second exists.
    pub fn tai_to_utc(
        &self,
        version: &str,
        tai: JulianDate,
    ) -> Result<VersionedUtcResult<(UtcDay, UtcClockTime)>, UtcLeapError> {
        let selected = self.select(version)?;
        let value = selected.schedule().tai_to_utc(tai)?;
        Ok(VersionedUtcResult {
            version: selected.version().to_owned(),
            provenance: selected.provenance().to_owned(),
            source_sha256: selected.source_sha256().to_owned(),
            expires_utc_day: selected.expires_utc_day(),
            value,
        })
    }
}

impl UtcLeapSchedule {
    pub fn new(
        first_day: UtcDay,
        last_day: UtcDay,
        initial_tai_minus_utc_seconds: i32,
        transitions: Vec<UtcLeapTransition>,
    ) -> Result<Self, UtcLeapError> {
        if first_day > last_day {
            return Err(UtcLeapError::InvalidCoverage);
        }
        if initial_tai_minus_utc_seconds < 0 {
            return Err(UtcLeapError::InvalidInitialOffset);
        }
        let last_boundary = last_day
            .julian_day_number
            .checked_add(1)
            .ok_or(UtcLeapError::InvalidCoverage)?;
        let mut previous_day = first_day.julian_day_number;
        let mut previous_offset = initial_tai_minus_utc_seconds;
        for step in &transitions {
            let effective = step.effective_utc_day.julian_day_number;
            if effective <= previous_day || effective > last_boundary {
                return Err(UtcLeapError::InvalidTransition);
            }
            if !step.effective_utc_day.starts_month()? {
                return Err(UtcLeapError::InvalidTransition);
            }
            let change = i64::from(step.tai_minus_utc_seconds) - i64::from(previous_offset);
            if step.tai_minus_utc_seconds < 0 || change.abs() != 1 {
                return Err(UtcLeapError::InvalidOffsetStep);
            }
            previous_day = effective;
            previous_offset = step.tai_minus_utc_seconds;
        }
        Ok(Self {
            first_day,
            last_day,
            initial_tai_minus_utc_seconds,
            transitions,
        })
    }

    /// Load the complete post-1972 **IANA tzdb-2026b** leap-second history.
    ///
    /// Bundled data is a compact, curated public-domain snapshot of
    /// `leap-seconds.list` at:
    /// <https://data.iana.org/time-zones/tzdb-2026b/leap-seconds.list>
    ///
    /// The IANA source declares NTP expiration 4007404800, corresponding
    /// to 2026-12-28 at 00:00 UTC. This constructor conservatively covers
    /// January 1, 1972 through **December 27, 2026 inclusive**, and will
    /// never extrapolate beyond that date. Update the source, provenance,
    /// expected metadata and independent tests in a reviewed new version.
    /// No network access or automatic updates are performed.
    pub fn from_iana_2026b() -> Result<Self, UtcLeapError> {
        const FIRST_NTP: i64 = 2_272_060_800; // 1972-01-01 UTC
        const EXPIRY_NTP: i64 = 4_007_404_800; // 2026-12-28 UTC
        let mut expiry: Option<i64> = None;
        let mut records: Vec<(i64, i32)> = Vec::new();
        let source = include_str!("../data/iana-tzdb-2026b-leap-seconds.list");
        for line in source.lines() {
            let line = line.trim();
            if let Some(stamp) = line.strip_prefix("#@") {
                let stamp = stamp
                    .trim()
                    .parse::<i64>()
                    .map_err(|_| UtcLeapError::InvalidPinnedLeapData)?;
                if expiry.replace(stamp).is_some() {
                    return Err(UtcLeapError::InvalidPinnedLeapData);
                }
                continue;
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut columns = line.split('#').next().unwrap_or("").split_whitespace();
            let ntp = columns
                .next()
                .ok_or(UtcLeapError::InvalidPinnedLeapData)?
                .parse::<i64>()
                .map_err(|_| UtcLeapError::InvalidPinnedLeapData)?;
            let offset = columns
                .next()
                .ok_or(UtcLeapError::InvalidPinnedLeapData)?
                .parse::<i32>()
                .map_err(|_| UtcLeapError::InvalidPinnedLeapData)?;
            if columns.next().is_some() {
                return Err(UtcLeapError::InvalidPinnedLeapData);
            }
            records.push((ntp, offset));
        }
        if expiry != Some(EXPIRY_NTP)
            || records.len() != 28
            || records.first() != Some(&(FIRST_NTP, 10))
            || records.last() != Some(&(3_692_217_600, 37))
        {
            return Err(UtcLeapError::InvalidPinnedLeapData);
        }
        let first_day = UtcDay::from_gregorian(1972, 1, 1)?;
        let ntp_day = |stamp: i64| -> Result<UtcDay, UtcLeapError> {
            let elapsed = stamp
                .checked_sub(FIRST_NTP)
                .ok_or(UtcLeapError::InvalidPinnedLeapData)?;
            if elapsed < 0 || elapsed % 86_400 != 0 {
                return Err(UtcLeapError::InvalidPinnedLeapData);
            }
            let number = first_day
                .julian_day_number
                .checked_add(elapsed / 86_400)
                .ok_or(UtcLeapError::InvalidPinnedLeapData)?;
            Ok(UtcDay {
                julian_day_number: number,
            })
        };
        let last_day = UtcDay {
            julian_day_number: ntp_day(EXPIRY_NTP)?
                .julian_day_number
                .checked_sub(1)
                .ok_or(UtcLeapError::InvalidPinnedLeapData)?,
        };
        let mut transitions = Vec::with_capacity(records.len() - 1);
        for (stamp, offset) in records.into_iter().skip(1) {
            let effective_utc_day = ntp_day(stamp)?;
            if effective_utc_day > last_day {
                return Err(UtcLeapError::InvalidPinnedLeapData);
            }
            transitions.push(UtcLeapTransition {
                effective_utc_day,
                tai_minus_utc_seconds: offset,
            });
        }
        Self::new(first_day, last_day, 10, transitions)
            .map_err(|_| UtcLeapError::InvalidPinnedLeapData)
    }

    fn offset_at_boundary(&self, day_number: i64) -> i32 {
        let count = self
            .transitions
            .partition_point(|step| step.effective_utc_day.julian_day_number <= day_number);
        if count == 0 {
            self.initial_tai_minus_utc_seconds
        } else {
            self.transitions[count - 1].tai_minus_utc_seconds
        }
    }

    /// Return TAI-UTC in integer SI seconds **at 00:00 UTC of this day**.
    pub fn tai_minus_utc_at_day_start(&self, day: UtcDay) -> Result<i32, UtcLeapError> {
        if day < self.first_day || day > self.last_day {
            return Err(UtcLeapError::OutsideCoverage);
        }
        Ok(self.offset_at_boundary(day.julian_day_number))
    }

    /// Validate that a UTC clock reading exists on a covered civil day.
    ///
    /// The result is elapsed SI seconds into that **named UTC day**, not a
    /// Julian date or a TAI/TT/TDB instant. A positive leap permits
    /// 23:59:60.x; a hypothetical negative leap omits 23:59:59.x.
    /// At every day's end, the next valid reading is the next day's 00:00.
    pub fn seconds_into_utc_day(
        &self,
        day: UtcDay,
        clock: UtcClockTime,
    ) -> Result<f64, UtcLeapError> {
        let day_length = f64::from(self.utc_day_length_seconds(day)?);
        let seconds = clock.seconds_since_midnight();
        if seconds >= day_length {
            return Err(UtcLeapError::ClockTimeOutsideDay);
        }
        Ok(seconds)
    }

    /// Convert a validated UTC civil day and clock into a split **TAI** JD.
    ///
    /// The caller-provided leap schedule determines the integer TAI-UTC
    /// offset at the **start of the UTC day**. Add that offset to the elapsed
    /// SI seconds within the named day, then normalize across TAI midnight.
    /// This preserves distinct instants at 23:59:59, 23:59:60 (positive
    /// leap), and the next day's 00:00:00, without treating UTC as a uniform
    /// Julian-day time scale. A hypothetical negative leap is also supported.
    ///
    /// The output is explicitly tagged `TimeScale::Tai`. It is not TT/TDB;
    /// the schedule must have been independently sourced and approved.
    pub fn utc_to_tai(&self, day: UtcDay, clock: UtcClockTime) -> Result<JulianDate, UtcLeapError> {
        let utc_elapsed = self.seconds_into_utc_day(day, clock)?;
        let offset = self.tai_minus_utc_at_day_start(day)?;
        let tai_seconds = utc_elapsed + f64::from(offset);
        let tai_day_carry = (tai_seconds / 86_400.0).floor() as i64;
        let tai_day_number = day
            .julian_day_number
            .checked_add(tai_day_carry)
            .ok_or(UtcLeapError::UnrepresentableTai)?;
        let tai_fraction = (tai_seconds - (tai_day_carry as f64) * 86_400.0) / 86_400.0;
        let tai = JulianDate::new(tai_day_number, tai_fraction, TimeScale::Tai)
            .map_err(|_| UtcLeapError::UnrepresentableTai)?;
        tai.parts().map_err(|_| UtcLeapError::UnrepresentableTai)?;
        Ok(tai)
    }

    /// Compute the TAI instant of a UTC midnight, including the exclusive
    /// boundary just after the last covered UTC day. The offset is integral
    /// SI seconds; Euclidean division keeps the split Julian date normalized
    /// even if a caller's unusual schedule offset spans multiple TAI days.
    fn tai_at_utc_midnight(&self, utc_day_number: i64) -> Result<JulianDate, UtcLeapError> {
        let offset = i64::from(self.offset_at_boundary(utc_day_number));
        let tai_day = utc_day_number
            .checked_add(offset.div_euclid(86_400))
            .ok_or(UtcLeapError::UnrepresentableTai)?;
        let fraction = (offset.rem_euclid(86_400) as f64) / 86_400.0;
        let tai = JulianDate::new(tai_day, fraction, TimeScale::Tai)
            .map_err(|_| UtcLeapError::UnrepresentableTai)?;
        tai.parts().map_err(|_| UtcLeapError::UnrepresentableTai)?;
        Ok(tai)
    }

    /// Compare two normalized TAI Julian dates without collapsing them to a
    /// single large `f64` Julian day.
    fn compare_tai(a: JulianDate, b: JulianDate) -> Ordering {
        a.day_number()
            .cmp(&b.day_number())
            .then_with(|| a.midnight_fraction().total_cmp(&b.midnight_fraction()))
    }

    /// Convert a **TAI** split Julian date to a named UTC day and clock time.
    ///
    /// Finds the UTC day whose *TAI* midnight boundaries surround the input,
    /// then recovers the elapsed SI seconds into that actual UTC day. This
    /// preserves the positive 23:59:60.x clock reading and excludes the
    /// missing final second of a hypothetical negative-leap UTC day.
    ///
    /// The conversion requires the same independently sourced, explicitly
    /// bounded leap schedule as `utc_to_tai`; it refuses UTC instants before
    /// the first covered midnight or at/after the end of the final covered
    /// day. There is no guessing or extrapolation.
    pub fn tai_to_utc(&self, tai: JulianDate) -> Result<(UtcDay, UtcClockTime), UtcLeapError> {
        if tai.scale() != TimeScale::Tai {
            return Err(UtcLeapError::WrongInputScale);
        }
        tai.parts().map_err(|_| UtcLeapError::UnrepresentableTai)?;
        let end_exclusive = self
            .last_day
            .julian_day_number
            .checked_add(1)
            .ok_or(UtcLeapError::InvalidCoverage)?;
        let first_tai = self.tai_at_utc_midnight(self.first_day.julian_day_number)?;
        let last_tai = self.tai_at_utc_midnight(end_exclusive)?;
        if Self::compare_tai(tai, first_tai) == Ordering::Less
            || Self::compare_tai(tai, last_tai) != Ordering::Less
        {
            return Err(UtcLeapError::OutsideCoverage);
        }

        // UTC midnight boundaries are monotonic because every leap changes
        // the day duration by only one SI second. Binary search is O(log N)
        // in covered days, independent of the number of leap transitions.
        let mut first = self.first_day.julian_day_number;
        let mut end = end_exclusive;
        while end - first > 1 {
            let middle = first + (end - first) / 2;
            let middle_tai = self.tai_at_utc_midnight(middle)?;
            if Self::compare_tai(tai, middle_tai) == Ordering::Less {
                end = middle;
            } else {
                first = middle;
            }
        }

        let day = UtcDay {
            julian_day_number: first,
        };
        let start_tai = self.tai_at_utc_midnight(first)?;
        let day_delta = tai.day_number() - start_tai.day_number();
        let elapsed = (day_delta as f64) * 86_400.0
            + (tai.midnight_fraction() - start_tai.midnight_fraction()) * 86_400.0;
        let day_length = f64::from(self.utc_day_length_seconds(day)?);
        if !elapsed.is_finite() || elapsed < 0.0 || elapsed >= day_length {
            return Err(UtcLeapError::UnrepresentableTai);
        }
        let clock = if elapsed >= 86_400.0 {
            // Only a positive leap day can contain seconds beyond 86400.
            UtcClockTime::new(23, 59, elapsed - 86_340.0)
        } else {
            let hour = (elapsed / 3_600.0).floor() as u8;
            let remaining = elapsed - f64::from(hour) * 3_600.0;
            let minute = (remaining / 60.0).floor() as u8;
            let second = remaining - f64::from(minute) * 60.0;
            UtcClockTime::new(hour, minute, second)
        }
        .map_err(|_| UtcLeapError::UnrepresentableTai)?;
        Ok((day, clock))
    }

    /// Return 86,399, 86,400 or 86,401 SI seconds for a UTC civil day.
    ///
    /// This describes the actual named UTC civil day, including any positive
    /// leap second (23:59:60) or hypothetical negative leap second. It does
    /// not declare that a UTC Julian date is a uniform-duration time scale.
    pub fn utc_day_length_seconds(&self, day: UtcDay) -> Result<u32, UtcLeapError> {
        let before = self.tai_minus_utc_at_day_start(day)?;
        let after = self.offset_at_boundary(day.julian_day_number + 1);
        let length = 86_400 + after - before;
        Ok(length as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(year: i32, month: u8, day: u8) -> UtcDay {
        UtcDay::from_gregorian(year, month, day).unwrap()
    }

    #[test]
    fn positive_2016_leap_second_has_86401_seconds_only_on_december_31() {
        // Demonstrative transition from the official IERS history. The
        // schedule explicitly covers only the named test dates.
        let schedule = UtcLeapSchedule::new(
            d(2016, 12, 30),
            d(2017, 1, 2),
            36,
            vec![UtcLeapTransition {
                effective_utc_day: d(2017, 1, 1),
                tai_minus_utc_seconds: 37,
            }],
        )
        .unwrap();
        assert_eq!(schedule.utc_day_length_seconds(d(2016, 12, 30)), Ok(86_400));
        assert_eq!(schedule.utc_day_length_seconds(d(2016, 12, 31)), Ok(86_401));
        assert_eq!(schedule.utc_day_length_seconds(d(2017, 1, 1)), Ok(86_400));
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(2016, 12, 31)), Ok(36));
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(2017, 1, 1)), Ok(37));
        assert_eq!(
            schedule.tai_minus_utc_at_day_start(d(2017, 1, 3)),
            Err(UtcLeapError::OutsideCoverage)
        );
    }

    #[test]
    fn hypothetical_negative_leap_day_is_86399_seconds() {
        // No negative leap second had occurred as of the dated source.
        let schedule = UtcLeapSchedule::new(
            d(2020, 6, 29),
            d(2020, 7, 2),
            37,
            vec![UtcLeapTransition {
                effective_utc_day: d(2020, 7, 1),
                tai_minus_utc_seconds: 36,
            }],
        )
        .unwrap();
        assert_eq!(schedule.utc_day_length_seconds(d(2020, 6, 30)), Ok(86_399));
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(2020, 7, 1)), Ok(36));
    }

    #[test]
    fn validates_transition_order_step_month_boundary_and_coverage() {
        let first = d(2016, 12, 30);
        let last = d(2017, 1, 3);
        let positive = UtcLeapTransition {
            effective_utc_day: d(2017, 1, 1),
            tai_minus_utc_seconds: 37,
        };
        assert!(UtcLeapSchedule::new(last, first, 36, vec![]).is_err());
        assert!(UtcLeapSchedule::new(first, last, -1, vec![]).is_err());
        assert_eq!(
            UtcLeapSchedule::new(first, last, 36, vec![positive, positive]).unwrap_err(),
            UtcLeapError::InvalidTransition
        );
        let jump = UtcLeapTransition {
            tai_minus_utc_seconds: 38,
            ..positive
        };
        assert_eq!(
            UtcLeapSchedule::new(first, last, 36, vec![jump]).unwrap_err(),
            UtcLeapError::InvalidOffsetStep
        );
        let middle_of_month = UtcLeapTransition {
            effective_utc_day: d(2017, 1, 2),
            tai_minus_utc_seconds: 37,
        };
        assert_eq!(
            UtcLeapSchedule::new(first, last, 36, vec![middle_of_month]).unwrap_err(),
            UtcLeapError::InvalidTransition
        );
        let beyond = UtcLeapTransition {
            effective_utc_day: d(2017, 2, 1),
            tai_minus_utc_seconds: 37,
        };
        assert_eq!(
            UtcLeapSchedule::new(first, last, 36, vec![beyond]).unwrap_err(),
            UtcLeapError::InvalidTransition
        );
    }

    #[test]
    fn rejects_legacy_dates_and_invalid_gregorian_days() {
        assert_eq!(
            UtcDay::from_gregorian(1971, 12, 31),
            Err(UtcLeapError::Before1972)
        );
        assert_eq!(
            UtcDay::from_gregorian(2019, 2, 29),
            Err(UtcLeapError::InvalidGregorianDate)
        );
        assert!(UtcDay::from_gregorian(2020, 2, 29).is_ok());
    }

    #[test]
    fn utc_clock_accepts_positive_leap_second_and_fraction_only_on_that_day() {
        let schedule = UtcLeapSchedule::new(
            d(2016, 12, 31),
            d(2017, 1, 1),
            36,
            vec![UtcLeapTransition {
                effective_utc_day: d(2017, 1, 1),
                tai_minus_utc_seconds: 37,
            }],
        )
        .unwrap();
        let leap = UtcClockTime::new(23, 59, 60.5).unwrap();
        assert_eq!(leap.hour(), 23);
        assert_eq!(leap.minute(), 59);
        assert_eq!(leap.second(), 60.5);
        assert_eq!(
            schedule.seconds_into_utc_day(d(2016, 12, 31), leap),
            Ok(86_400.5)
        );
        assert_eq!(
            schedule.seconds_into_utc_day(d(2017, 1, 1), leap),
            Err(UtcLeapError::ClockTimeOutsideDay)
        );
        assert_eq!(
            schedule.seconds_into_utc_day(d(2017, 1, 1), UtcClockTime::new(0, 0, 0.0).unwrap()),
            Ok(0.0)
        );
    }

    #[test]
    fn negative_leap_day_omits_235959_but_keeps_235958() {
        // Synthetic negative leap; no historical negative leap is asserted.
        let schedule = UtcLeapSchedule::new(
            d(2020, 6, 30),
            d(2020, 7, 1),
            37,
            vec![UtcLeapTransition {
                effective_utc_day: d(2020, 7, 1),
                tai_minus_utc_seconds: 36,
            }],
        )
        .unwrap();
        assert_eq!(
            schedule.seconds_into_utc_day(d(2020, 6, 30), UtcClockTime::new(23, 59, 58.5).unwrap()),
            Ok(86_398.5)
        );
        assert_eq!(
            schedule.seconds_into_utc_day(d(2020, 6, 30), UtcClockTime::new(23, 59, 59.0).unwrap()),
            Err(UtcLeapError::ClockTimeOutsideDay)
        );
        assert_eq!(
            schedule.seconds_into_utc_day(d(2020, 7, 1), UtcClockTime::new(23, 59, 59.0).unwrap()),
            Ok(86_399.0)
        );
    }

    #[test]
    fn rejects_invalid_clock_fields_and_nonfinite_seconds() {
        for (hour, minute, second) in [
            (24, 0, 0.0),
            (0, 60, 0.0),
            (0, 0, -0.001),
            (0, 0, 60.0),
            (23, 58, 60.0),
            (23, 59, 61.0),
            (23, 59, f64::NAN),
            (23, 59, f64::INFINITY),
        ] {
            assert_eq!(
                UtcClockTime::new(hour, minute, second),
                Err(UtcLeapError::InvalidClockTime)
            );
        }
        assert!(UtcClockTime::new(23, 59, 60.999).is_ok());
        assert!(UtcClockTime::new(12, 30, 59.999).is_ok());
    }

    #[test]
    fn clock_validation_requires_explicit_coverage() {
        let schedule = UtcLeapSchedule::new(d(2020, 1, 1), d(2020, 1, 1), 37, vec![]).unwrap();
        let midnight = UtcClockTime::new(0, 0, 0.0).unwrap();
        assert_eq!(
            schedule.seconds_into_utc_day(d(2019, 12, 31), midnight),
            Err(UtcLeapError::OutsideCoverage)
        );
        assert_eq!(
            schedule.seconds_into_utc_day(d(2020, 1, 2), midnight),
            Err(UtcLeapError::OutsideCoverage)
        );
    }

    #[test]
    fn utc_to_tai_distinguishes_positive_leap_second_and_next_midnight() {
        let schedule = UtcLeapSchedule::new(
            d(2016, 12, 31),
            d(2017, 1, 1),
            36,
            vec![UtcLeapTransition {
                effective_utc_day: d(2017, 1, 1),
                tai_minus_utc_seconds: 37,
            }],
        )
        .unwrap();
        let last_day = d(2016, 12, 31);
        let next_day = d(2017, 1, 1);

        let before = schedule
            .utc_to_tai(last_day, UtcClockTime::new(23, 59, 59.0).unwrap())
            .unwrap();
        let leap = schedule
            .utc_to_tai(last_day, UtcClockTime::new(23, 59, 60.0).unwrap())
            .unwrap();
        let after = schedule
            .utc_to_tai(next_day, UtcClockTime::new(0, 0, 0.0).unwrap())
            .unwrap();
        assert_eq!(before.scale(), TimeScale::Tai);
        assert_eq!(leap.scale(), TimeScale::Tai);
        assert_eq!(after.scale(), TimeScale::Tai);
        assert_eq!(before.day_number(), next_day.julian_day_number());
        assert_eq!(leap.day_number(), next_day.julian_day_number());
        assert_eq!(after.day_number(), next_day.julian_day_number());
        for (epoch, expected_seconds) in [(before, 35.0), (leap, 36.0), (after, 37.0)] {
            assert!((epoch.midnight_fraction() * 86_400.0 - expected_seconds).abs() < 1e-8);
        }
        let fraction = schedule
            .utc_to_tai(last_day, UtcClockTime::new(23, 59, 60.5).unwrap())
            .unwrap();
        assert!((fraction.midnight_fraction() * 86_400.0 - 36.5).abs() < 1e-8);
    }

    #[test]
    fn utc_to_tai_negative_leap_has_no_missing_tai_second() {
        // Synthetic negative leap: valid 23:59:58 -> 00:00:00 UTC
        // corresponds to consecutive TAI clock seconds 00:00:35 and :36.
        let schedule = UtcLeapSchedule::new(
            d(2020, 6, 30),
            d(2020, 7, 1),
            37,
            vec![UtcLeapTransition {
                effective_utc_day: d(2020, 7, 1),
                tai_minus_utc_seconds: 36,
            }],
        )
        .unwrap();
        let before = schedule
            .utc_to_tai(d(2020, 6, 30), UtcClockTime::new(23, 59, 58.0).unwrap())
            .unwrap();
        let after = schedule
            .utc_to_tai(d(2020, 7, 1), UtcClockTime::new(0, 0, 0.0).unwrap())
            .unwrap();
        assert_eq!(before.day_number(), d(2020, 7, 1).julian_day_number());
        assert_eq!(after.day_number(), d(2020, 7, 1).julian_day_number());
        assert!((before.midnight_fraction() * 86_400.0 - 35.0).abs() < 1e-8);
        assert!((after.midnight_fraction() * 86_400.0 - 36.0).abs() < 1e-8);
        assert_eq!(
            schedule.utc_to_tai(d(2020, 6, 30), UtcClockTime::new(23, 59, 59.0).unwrap()),
            Err(UtcLeapError::ClockTimeOutsideDay)
        );
    }

    #[test]
    fn utc_to_tai_normal_midday_and_tai_to_tt_composition() {
        let day = d(2017, 1, 2);
        let schedule = UtcLeapSchedule::new(day, day, 37, vec![]).unwrap();
        let tai = schedule
            .utc_to_tai(day, UtcClockTime::new(12, 0, 0.25).unwrap())
            .unwrap();
        assert_eq!(tai.day_number(), day.julian_day_number());
        assert!((tai.midnight_fraction() * 86_400.0 - 43_237.25).abs() < 1e-8);
        let tt = crate::time::tai_to_tt(tai).unwrap();
        assert_eq!(tt.scale(), TimeScale::Tt);
        assert!((tt.midnight_fraction() * 86_400.0 - 43_269.434).abs() < 1e-8);
    }

    #[test]
    fn utc_to_tai_rejects_invalid_date_specific_clock_and_coverage() {
        let day = d(2017, 1, 2);
        let schedule = UtcLeapSchedule::new(day, day, 37, vec![]).unwrap();
        assert_eq!(
            schedule.utc_to_tai(day, UtcClockTime::new(23, 59, 60.0).unwrap()),
            Err(UtcLeapError::ClockTimeOutsideDay)
        );
        assert_eq!(
            schedule.utc_to_tai(d(2017, 1, 3), UtcClockTime::new(0, 0, 0.0).unwrap()),
            Err(UtcLeapError::OutsideCoverage)
        );
    }

    #[test]
    fn tai_to_utc_roundtrips_positive_leap_and_next_midnight() {
        let last = d(2016, 12, 31);
        let next = d(2017, 1, 1);
        let schedule = UtcLeapSchedule::new(
            last,
            next,
            36,
            vec![UtcLeapTransition {
                effective_utc_day: next,
                tai_minus_utc_seconds: 37,
            }],
        )
        .unwrap();
        for (day, clock) in [
            (last, UtcClockTime::new(23, 59, 59.0).unwrap()),
            (last, UtcClockTime::new(23, 59, 60.0).unwrap()),
            (last, UtcClockTime::new(23, 59, 60.5).unwrap()),
            (next, UtcClockTime::new(0, 0, 0.0).unwrap()),
            (next, UtcClockTime::new(12, 30, 0.25).unwrap()),
        ] {
            let tai = schedule.utc_to_tai(day, clock).unwrap();
            let (recovered_day, recovered_clock) = schedule.tai_to_utc(tai).unwrap();
            assert_eq!(recovered_day, day);
            assert_eq!(recovered_clock.hour(), clock.hour());
            assert_eq!(recovered_clock.minute(), clock.minute());
            assert!((recovered_clock.second() - clock.second()).abs() < 1e-8);
        }
    }

    #[test]
    fn tai_to_utc_roundtrips_hypothetical_negative_leap() {
        let last = d(2020, 6, 30);
        let next = d(2020, 7, 1);
        let schedule = UtcLeapSchedule::new(
            last,
            next,
            37,
            vec![UtcLeapTransition {
                effective_utc_day: next,
                tai_minus_utc_seconds: 36,
            }],
        )
        .unwrap();
        for (day, clock) in [
            (last, UtcClockTime::new(23, 59, 58.0).unwrap()),
            (last, UtcClockTime::new(23, 59, 58.5).unwrap()),
            (next, UtcClockTime::new(0, 0, 0.0).unwrap()),
        ] {
            let (actual_day, actual_clock) = schedule
                .tai_to_utc(schedule.utc_to_tai(day, clock).unwrap())
                .unwrap();
            assert_eq!(actual_day, day);
            assert_eq!(actual_clock.hour(), clock.hour());
            assert_eq!(actual_clock.minute(), clock.minute());
            assert!((actual_clock.second() - clock.second()).abs() < 1e-8);
        }
    }

    #[test]
    fn tai_to_utc_rejects_wrong_scale_and_coverage_boundaries() {
        let first = d(2017, 1, 2);
        let last = d(2017, 1, 3);
        let schedule = UtcLeapSchedule::new(first, last, 37, vec![]).unwrap();
        let first_tai = schedule
            .utc_to_tai(first, UtcClockTime::new(0, 0, 0.0).unwrap())
            .unwrap();
        let one_second = JulianDate::new(
            first_tai.day_number(),
            first_tai.midnight_fraction() - 1.0 / 86_400.0,
            TimeScale::Tai,
        )
        .unwrap();
        assert_eq!(
            schedule.tai_to_utc(one_second),
            Err(UtcLeapError::OutsideCoverage)
        );
        let end_boundary = JulianDate::new(
            last.julian_day_number() + 1,
            37.0 / 86_400.0,
            TimeScale::Tai,
        )
        .unwrap();
        assert_eq!(
            schedule.tai_to_utc(end_boundary),
            Err(UtcLeapError::OutsideCoverage)
        );
        let tt = crate::time::tai_to_tt(first_tai).unwrap();
        assert_eq!(schedule.tai_to_utc(tt), Err(UtcLeapError::WrongInputScale));
    }

    #[test]
    fn tai_to_utc_roundtrips_multi_day_atomic_offset() {
        // A synthetic schedule with an unusually large, but valid, initial
        // offset verifies that the inverse does not assume TAI and UTC day
        // numbers differ by at most one.
        let day = d(2020, 1, 1);
        let schedule = UtcLeapSchedule::new(day, day, 200_000, vec![]).unwrap();
        for clock in [
            UtcClockTime::new(0, 0, 0.0).unwrap(),
            UtcClockTime::new(12, 34, 56.0).unwrap(),
            UtcClockTime::new(23, 59, 59.0).unwrap(),
        ] {
            let tai = schedule.utc_to_tai(day, clock).unwrap();
            let (back_day, back_clock) = schedule.tai_to_utc(tai).unwrap();
            assert_eq!(back_day, day);
            assert_eq!(back_clock.hour(), clock.hour());
            assert_eq!(back_clock.minute(), clock.minute());
            assert!((back_clock.second() - clock.second()).abs() < 1e-8);
        }
    }

    #[test]
    fn iana_2026b_snapshot_has_full_history_and_fixed_expiration() {
        let schedule = UtcLeapSchedule::from_iana_2026b().unwrap();
        assert_eq!(schedule.transitions.len(), 27);
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(1972, 1, 1)), Ok(10));
        assert_eq!(schedule.utc_day_length_seconds(d(1972, 6, 30)), Ok(86_401));
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(1972, 7, 1)), Ok(11));
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(1980, 1, 1)), Ok(19));
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(2000, 1, 1)), Ok(32));
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(2017, 1, 1)), Ok(37));
        assert_eq!(schedule.utc_day_length_seconds(d(2016, 12, 31)), Ok(86_401));
        assert_eq!(schedule.tai_minus_utc_at_day_start(d(2026, 12, 27)), Ok(37));
        assert_eq!(
            schedule.tai_minus_utc_at_day_start(d(2026, 12, 28)),
            Err(UtcLeapError::OutsideCoverage)
        );
        // Construct the preceding day internally: the public constructor
        // correctly refuses UTC civil dates before the 1972 leap regime.
        let before_1972 = UtcDay {
            julian_day_number: d(1972, 1, 1).julian_day_number() - 1,
        };
        assert_eq!(
            schedule.tai_minus_utc_at_day_start(before_1972),
            Err(UtcLeapError::OutsideCoverage)
        );
    }

    #[test]
    fn iana_2026b_snapshot_roundtrips_historical_positive_leaps() {
        let schedule = UtcLeapSchedule::from_iana_2026b().unwrap();
        for leap_day in [
            d(1972, 6, 30),
            d(1972, 12, 31),
            d(1998, 12, 31),
            d(2008, 12, 31),
            d(2016, 12, 31),
        ] {
            assert_eq!(schedule.utc_day_length_seconds(leap_day), Ok(86_401));
            let leap = UtcClockTime::new(23, 59, 60.25).unwrap();
            let tai = schedule.utc_to_tai(leap_day, leap).unwrap();
            let (day, clock) = schedule.tai_to_utc(tai).unwrap();
            assert_eq!(day, leap_day);
            assert_eq!(clock.hour(), 23);
            assert_eq!(clock.minute(), 59);
            assert!((clock.second() - 60.25).abs() < 1e-8);
        }
    }

    #[test]
    fn iana_2026b_expired_schedule_rejects_both_directions() {
        let schedule = UtcLeapSchedule::from_iana_2026b().unwrap();
        let last = d(2026, 12, 27);
        let next = d(2026, 12, 28);
        let midnight = UtcClockTime::new(0, 0, 0.0).unwrap();
        assert_eq!(
            schedule.utc_to_tai(next, midnight),
            Err(UtcLeapError::OutsideCoverage)
        );
        let last_tai = schedule.utc_to_tai(last, midnight).unwrap();
        assert_eq!(schedule.tai_to_utc(last_tai).unwrap().0, last);
        let first_expired_tai =
            JulianDate::new(next.julian_day_number(), 37.0 / 86_400.0, TimeScale::Tai).unwrap();
        assert_eq!(
            schedule.tai_to_utc(first_expired_tai),
            Err(UtcLeapError::OutsideCoverage)
        );
    }

    #[test]
    fn versioned_2026b_matches_legacy_schedule_and_keeps_metadata() {
        let pinned = VersionedUtcLeapSchedule::iana_2026b().unwrap();
        assert_eq!(pinned.version(), "iana-tzdb-2026b");
        assert!(pinned.provenance().contains("tzdb-2026b"));
        assert_eq!(pinned.expires_utc_day(), d(2026, 12, 28));
        let legacy = UtcLeapSchedule::from_iana_2026b().unwrap();
        for day in [d(1972, 1, 1), d(1972, 7, 1), d(2017, 1, 1), d(2026, 12, 27)] {
            assert_eq!(
                pinned.schedule().tai_minus_utc_at_day_start(day),
                legacy.tai_minus_utc_at_day_start(day)
            );
        }
        assert_eq!(
            pinned
                .schedule()
                .tai_minus_utc_at_day_start(d(2026, 12, 28)),
            Err(UtcLeapError::OutsideCoverage)
        );
    }

    #[test]
    fn reviewed_new_expiry_keeps_original_version_immutable() {
        let old = VersionedUtcLeapSchedule::iana_2026b().unwrap();
        let source = include_str!("../data/iana-tzdb-2026b-leap-seconds.list")
            .replace("#@ 4007404800", "#@ 4023388800");
        let new = VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
            "reviewed-2027a",
            "synthetic-test-only; not an official IANA release",
            &source,
        )
        .unwrap();
        assert_eq!(new.expires_utc_day(), d(2027, 7, 1));
        assert_eq!(
            old.schedule().tai_minus_utc_at_day_start(d(2027, 1, 1)),
            Err(UtcLeapError::OutsideCoverage)
        );
        assert_eq!(
            new.schedule().tai_minus_utc_at_day_start(d(2027, 1, 1)),
            Ok(37)
        );
        assert_eq!(
            new.schedule().tai_minus_utc_at_day_start(d(2027, 7, 1)),
            Err(UtcLeapError::OutsideCoverage)
        );
        assert_eq!(old.version(), "iana-tzdb-2026b");
        assert_eq!(new.version(), "reviewed-2027a");
    }

    #[test]
    fn hypothetical_future_leap_is_explicit_and_does_not_change_old_data() {
        let old = VersionedUtcLeapSchedule::iana_2026b().unwrap();
        let source = include_str!("../data/iana-tzdb-2026b-leap-seconds.list")
            .replace("#@ 4007404800", "#@ 4023388800");
        let revised = format!("{source}4007750400 38\n");
        let future = VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
            "hypothetical-2027",
            "synthetic-test-only",
            &revised,
        )
        .unwrap();
        let leap_day = d(2026, 12, 31);
        assert_eq!(
            future.schedule().utc_day_length_seconds(leap_day),
            Ok(86_401)
        );
        let clock = UtcClockTime::new(23, 59, 60.5).unwrap();
        let tai = future.schedule().utc_to_tai(leap_day, clock).unwrap();
        let (back_day, back_clock) = future.schedule().tai_to_utc(tai).unwrap();
        assert_eq!(back_day, leap_day);
        assert!((back_clock.second() - 60.5).abs() < 1e-8);
        assert_eq!(
            old.schedule().utc_day_length_seconds(leap_day),
            Err(UtcLeapError::OutsideCoverage)
        );
    }

    #[test]
    fn reviewed_snapshot_rejects_malformed_dates_and_metadata() {
        let good = include_str!("../data/iana-tzdb-2026b-leap-seconds.list");
        let load = |data: &str| {
            VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
                "reviewed-test",
                "synthetic-test-only",
                data,
            )
        };
        for bad in [
            good.replace("#@ 4007404800", ""),
            good.replace("#@ 4007404800", "#@ 4007404800\n#@ 4007404800"),
            good.replace("2272060800 10", "2272060800 11"),
            good.replace("2287785600 11", "2287785601 11"),
            good.replace("2287785600 11", "2287785600 13"),
            good.replace("2287785600 11", "2272060800 11"),
            good.replace("#@ 4007404800", "#@ 3692217600"),
            good.replace("#@ 4007404800", "#@ 4007404801"),
        ] {
            assert_eq!(
                load(&bad).unwrap_err(),
                UtcLeapError::InvalidReviewedSnapshot
            );
        }
        for version in ["", "../bad", "bad version"] {
            assert_eq!(
                VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
                    version,
                    "reviewed-source",
                    good
                )
                .unwrap_err(),
                UtcLeapError::InvalidSnapshotProvenance
            );
        }
        assert_eq!(
            VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot("reviewed-2026b", "", good)
                .unwrap_err(),
            UtcLeapError::InvalidSnapshotProvenance
        );
    }

    #[test]
    fn reviewed_loader_rejects_raw_iana_integrity_markers_without_auditing_them() {
        let source = include_str!("../data/iana-tzdb-2026b-leap-seconds.list");
        let load = |text: &str| {
            VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
                "reviewed-fixture",
                "synthetic-test-only",
                text,
            )
        };
        // Source authenticity and #h integrity are *not* handled by Rust.
        // An official file must first go through the offline verifier,
        // then be converted to the curated numeric proposal.
        for raw_marker in [
            "#$ 3976686858",
            "#h 2e101270 4e6749f8 2f1792b7 14a0c188 36bb19d6",
            "  #$ 3976686858",
            "\t#h 2e101270 4e6749f8 2f1792b7 14a0c188 36bb19d6",
        ] {
            let raw = format!("{raw_marker}\n{source}");
            assert_eq!(
                load(&raw).unwrap_err(),
                UtcLeapError::InvalidReviewedSnapshot,
                "unverified official marker must be rejected: {raw_marker}"
            );
        }
        // Ordinary descriptive comments mentioning upstream metadata are
        // allowed: the pinned numeric snapshot uses these deliberately.
        let annotated = format!("# Manual review references #h and #$ fields separately\n{source}");
        let approved = load(&annotated).unwrap();
        assert_eq!(approved.expires_utc_day(), d(2026, 12, 28));
        assert_ne!(
            approved.source_sha256(),
            crate::fingerprint::sha256_hex(source.as_bytes())
        );
    }

    #[test]
    fn reviewed_loader_preserves_full_2026b_history_and_coverage() {
        let source = include_str!("../data/iana-tzdb-2026b-leap-seconds.list");
        let load = |text: &str| {
            VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
                "reviewed-future",
                "synthetic-only",
                text,
            )
        };

        // These cases are numerically and calendrically well formed;
        // only the comparison with approved 2026b detects their drift.
        let historic_shift = source.replace("2287785600 11", "2290464000 11");
        let final_shift = source.replace("3692217600 37", "3694896000 37");
        let truncated = source.replace("3692217600 37\n", "");
        let expired_early = source.replace("#@ 4007404800", "#@ 4007318400");
        let extra_past_leap = format!("{source}4005072000 38\n");
        for (reason, data) in [
            ("changed 1972 leap month", historic_shift),
            ("changed 2017 leap month", final_shift),
            ("omitted 2017 leap", truncated),
            ("coverage downgraded", expired_early),
            ("fabricated extra leap in approved 2026 history", extra_past_leap),
        ] {
            assert_eq!(
                load(&data).unwrap_err(),
                UtcLeapError::InvalidReviewedSnapshot,
                "must reject: {reason}"
            );
        }

        // Pinned data and unchanged expiry remain valid, as does a
        // *proposed* longer coverage interval pending separate review.
        let original = load(source).unwrap();
        assert_eq!(original.expires_utc_day(), d(2026, 12, 28));
        let extended = source.replace("#@ 4007404800", "#@ 4023388800");
        let updated = load(&extended).unwrap();
        assert_eq!(updated.expires_utc_day(), d(2027, 7, 1));
        assert_ne!(original.source_sha256(), updated.source_sha256());

        // A hypothetical post-expiration leap is still representable,
        // without rewriting the independently approved historic data.
        let future = format!("{extended}4007750400 38\n");
        assert!(load(&future).is_ok());
    }

    #[test]
    fn catalog_requires_explicit_selection_and_preserves_result_metadata() {
        let mut catalog = UtcLeapCatalog::new();
        assert_eq!(catalog.versions().count(), 0);
        assert!(matches!(
            catalog.select("iana-tzdb-2026b"),
            Err(UtcLeapError::UnknownVersion)
        ));
        catalog
            .register(VersionedUtcLeapSchedule::iana_2026b().unwrap())
            .unwrap();
        assert_eq!(
            catalog.versions().collect::<Vec<_>>(),
            vec!["iana-tzdb-2026b"]
        );
        let leap_day = d(2016, 12, 31);
        let result = catalog
            .utc_to_tai(
                "iana-tzdb-2026b",
                leap_day,
                UtcClockTime::new(23, 59, 60.5).unwrap(),
            )
            .unwrap();
        assert_eq!(result.version, "iana-tzdb-2026b");
        assert!(result.provenance.contains("tzdb-2026b"));
        assert_eq!(result.expires_utc_day, d(2026, 12, 28));
        assert_eq!(result.value.scale(), TimeScale::Tai);

        let reverse = catalog.tai_to_utc("iana-tzdb-2026b", result.value).unwrap();
        assert_eq!(reverse.version, result.version);
        assert_eq!(reverse.provenance, result.provenance);
        assert_eq!(reverse.expires_utc_day, result.expires_utc_day);
        assert_eq!(reverse.value.0, leap_day);
        assert!((reverse.value.1.second() - 60.5).abs() < 1e-8);
    }

    #[test]
    fn catalog_rejects_duplicate_or_unknown_versions_without_fallback() {
        let mut catalog = UtcLeapCatalog::new();
        catalog
            .register(VersionedUtcLeapSchedule::iana_2026b().unwrap())
            .unwrap();
        assert_eq!(
            catalog.register(VersionedUtcLeapSchedule::iana_2026b().unwrap()),
            Err(UtcLeapError::DuplicateVersion)
        );
        assert!(matches!(
            catalog.select("iana-tzdb-2026c"),
            Err(UtcLeapError::UnknownVersion)
        ));
        assert_eq!(
            catalog.utc_to_tai(
                "iana-tzdb-2026c",
                d(2017, 1, 1),
                UtcClockTime::new(0, 0, 0.0).unwrap()
            ),
            Err(UtcLeapError::UnknownVersion)
        );
        assert_eq!(
            catalog.tai_to_utc(
                "iana-tzdb-2026c",
                JulianDate::new(d(2017, 1, 1).julian_day_number(), 0.5, TimeScale::Tai).unwrap()
            ),
            Err(UtcLeapError::UnknownVersion)
        );
    }

    #[test]
    fn catalog_keeps_old_expiry_when_another_version_is_registered() {
        let mut catalog = UtcLeapCatalog::new();
        catalog
            .register(VersionedUtcLeapSchedule::iana_2026b().unwrap())
            .unwrap();
        let synthetic = include_str!("../data/iana-tzdb-2026b-leap-seconds.list")
            .replace("#@ 4007404800", "#@ 4023388800");
        catalog
            .register(
                VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
                    "synthetic-2027",
                    "test-only; not an official future IANA release",
                    &synthetic,
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(
            catalog.versions().collect::<Vec<_>>(),
            vec!["iana-tzdb-2026b", "synthetic-2027"]
        );
        let day = d(2027, 1, 1);
        let clock = UtcClockTime::new(0, 0, 0.25).unwrap();
        assert_eq!(
            catalog.utc_to_tai("iana-tzdb-2026b", day, clock),
            Err(UtcLeapError::OutsideCoverage)
        );
        let result = catalog.utc_to_tai("synthetic-2027", day, clock).unwrap();
        assert_eq!(result.version, "synthetic-2027");
        assert_eq!(result.expires_utc_day, d(2027, 7, 1));
        assert_eq!(
            catalog.tai_to_utc("iana-tzdb-2026b", result.value),
            Err(UtcLeapError::OutsideCoverage)
        );
        let back = catalog.tai_to_utc("synthetic-2027", result.value).unwrap();
        assert_eq!(back.value.0, day);
        assert!((back.value.1.second() - clock.second()).abs() < 1e-8);
    }

    #[test]
    fn result_metadata_is_owned_after_catalog_is_dropped() {
        let result = {
            let mut catalog = UtcLeapCatalog::new();
            catalog
                .register(VersionedUtcLeapSchedule::iana_2026b().unwrap())
                .unwrap();
            catalog
                .utc_to_tai(
                    "iana-tzdb-2026b",
                    d(2017, 1, 1),
                    UtcClockTime::new(0, 0, 0.0).unwrap(),
                )
                .unwrap()
        };
        assert_eq!(result.version, "iana-tzdb-2026b");
        assert!(result.provenance.contains("leap-seconds.list"));
        assert_eq!(result.expires_utc_day, d(2026, 12, 28));
        assert_eq!(result.value.scale(), TimeScale::Tai);
    }

    #[test]
    fn content_sha256_is_stable_and_attached_to_both_conversion_results() {
        let pinned = VersionedUtcLeapSchedule::iana_2026b().unwrap();
        let source = include_str!("../data/iana-tzdb-2026b-leap-seconds.list");
        assert_eq!(
            pinned.source_sha256(),
            crate::fingerprint::sha256_hex(source.as_bytes())
        );
        // Independent SHA-256 over the exact 989-byte pinned text,
        // checked with Python hashlib (not the Rust implementation).
        assert_eq!(
            pinned.source_sha256(),
            "e6120e71c65ff1969a4f63c285d6b99df8b02d9bdb90331669384d63534c41dd"
        );
        assert_eq!(pinned.source_sha256().len(), 64);
        assert!(pinned
            .source_sha256()
            .chars()
            .all(|c| c.is_ascii_hexdigit()));

        let mut catalog = UtcLeapCatalog::new();
        catalog.register(pinned).unwrap();
        let expected = catalog
            .select("iana-tzdb-2026b")
            .unwrap()
            .source_sha256()
            .to_owned();
        assert!(catalog
            .select_matching_sha256("iana-tzdb-2026b", &expected)
            .is_ok());
        let day = d(2016, 12, 31);
        let result = catalog
            .utc_to_tai(
                "iana-tzdb-2026b",
                day,
                UtcClockTime::new(23, 59, 60.5).unwrap(),
            )
            .unwrap();
        let reverse = catalog.tai_to_utc("iana-tzdb-2026b", result.value).unwrap();
        assert_eq!(result.source_sha256, expected);
        assert_eq!(reverse.source_sha256, expected);
        assert_eq!(reverse.value.0, day);
        assert!((reverse.value.1.second() - 60.5).abs() < 1e-8);
    }

    #[test]
    fn same_version_label_with_different_source_bytes_has_different_sha256() {
        let source = include_str!("../data/iana-tzdb-2026b-leap-seconds.list");
        let changed = format!("{source}# reviewer-specific comment; rows unchanged\n");
        let original = VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
            "identical-name",
            "reviewed-source-A",
            source,
        )
        .unwrap();
        let different = VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
            "identical-name",
            "reviewed-source-B",
            &changed,
        )
        .unwrap();
        assert_eq!(original.version(), different.version());
        assert_ne!(original.source_sha256(), different.source_sha256());
        assert_eq!(
            original
                .schedule()
                .tai_minus_utc_at_day_start(d(2017, 1, 1)),
            different
                .schedule()
                .tai_minus_utc_at_day_start(d(2017, 1, 1))
        );

        let mut catalog = UtcLeapCatalog::new();
        catalog.register(different).unwrap();
        assert!(matches!(
            catalog.select_matching_sha256("identical-name", original.source_sha256()),
            Err(UtcLeapError::SourceDigestMismatch)
        ));
        let actual = catalog
            .select("identical-name")
            .unwrap()
            .source_sha256()
            .to_owned();
        assert!(catalog
            .select_matching_sha256("identical-name", &actual)
            .is_ok());
    }

    #[test]
    fn modified_expiry_cannot_reuse_pinned_source_digest() {
        let source = include_str!("../data/iana-tzdb-2026b-leap-seconds.list");
        let updated = source.replace("#@ 4007404800", "#@ 4023388800");
        let pinned = VersionedUtcLeapSchedule::iana_2026b().unwrap();
        let future = VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
            "iana-tzdb-2026b",
            "synthetic-test-only",
            &updated,
        )
        .unwrap();
        assert_eq!(future.version(), pinned.version());
        assert_ne!(future.source_sha256(), pinned.source_sha256());
        assert_eq!(pinned.expires_utc_day(), d(2026, 12, 28));
        assert_eq!(future.expires_utc_day(), d(2027, 7, 1));
    }

    #[test]
    fn checked_bidirectional_conversion_roundtrips_with_expected_source_digest() {
        let mut catalog = UtcLeapCatalog::new();
        catalog
            .register(VersionedUtcLeapSchedule::iana_2026b().unwrap())
            .unwrap();
        let version = "iana-tzdb-2026b";
        let hash = "e6120e71c65ff1969a4f63c285d6b99df8b02d9bdb90331669384d63534c41dd";
        let day = d(2016, 12, 31);
        let tai = catalog
            .utc_to_tai_checked(
                version,
                hash,
                day,
                UtcClockTime::new(23, 59, 60.25).unwrap(),
            )
            .unwrap();
        assert_eq!(tai.version, version);
        assert_eq!(tai.source_sha256, hash);
        let utc = catalog
            .tai_to_utc_checked(version, hash, tai.value)
            .unwrap();
        assert_eq!(utc.source_sha256, hash);
        assert_eq!(utc.value.0, day);
        assert_eq!(utc.value.1.hour(), 23);
        assert_eq!(utc.value.1.minute(), 59);
        assert!((utc.value.1.second() - 60.25).abs() < 1e-8);
    }

    #[test]
    fn checked_conversions_fail_closed_on_wrong_digest_and_unknown_version() {
        let mut catalog = UtcLeapCatalog::new();
        catalog
            .register(VersionedUtcLeapSchedule::iana_2026b().unwrap())
            .unwrap();
        let version = "iana-tzdb-2026b";
        let day = d(2017, 1, 1);
        let zero = UtcClockTime::new(0, 0, 0.0).unwrap();
        let tai = catalog.utc_to_tai(version, day, zero).unwrap().value;
        for wrong_hash in [
            "",
            "f",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ] {
            assert_eq!(
                catalog.utc_to_tai_checked(version, wrong_hash, day, zero),
                Err(UtcLeapError::SourceDigestMismatch)
            );
            assert_eq!(
                catalog.tai_to_utc_checked(version, wrong_hash, tai),
                Err(UtcLeapError::SourceDigestMismatch)
            );
        }
        assert_eq!(
            catalog.utc_to_tai_checked("unknown", "wrong", day, zero),
            Err(UtcLeapError::UnknownVersion)
        );
        assert_eq!(
            catalog.tai_to_utc_checked("unknown", "wrong", tai),
            Err(UtcLeapError::UnknownVersion)
        );
    }

    #[test]
    fn checked_conversion_rejects_reused_version_name_with_changed_bytes() {
        let mut catalog = UtcLeapCatalog::new();
        let pinned = VersionedUtcLeapSchedule::iana_2026b().unwrap();
        let old_hash = pinned.source_sha256().to_owned();
        let source = include_str!("../data/iana-tzdb-2026b-leap-seconds.list");
        let altered = format!("{source}# harmless extra comment\n");
        catalog
            .register(
                VersionedUtcLeapSchedule::from_reviewed_numeric_snapshot(
                    "iana-tzdb-2026b",
                    "synthetic-test-only; same label, changed bytes",
                    &altered,
                )
                .unwrap(),
            )
            .unwrap();
        let day = d(2017, 1, 1);
        assert_eq!(
            catalog.utc_to_tai_checked(
                "iana-tzdb-2026b",
                &old_hash,
                day,
                UtcClockTime::new(0, 0, 0.0).unwrap()
            ),
            Err(UtcLeapError::SourceDigestMismatch)
        );
    }

    #[test]
    fn final_boundary_transition_and_no_extrapolation_are_explicit() {
        let last = d(2016, 12, 31);
        let schedule = UtcLeapSchedule::new(
            last,
            last,
            36,
            vec![UtcLeapTransition {
                effective_utc_day: d(2017, 1, 1),
                tai_minus_utc_seconds: 37,
            }],
        )
        .unwrap();
        assert_eq!(schedule.utc_day_length_seconds(last), Ok(86_401));
        assert_eq!(
            schedule.utc_day_length_seconds(d(2017, 1, 1)),
            Err(UtcLeapError::OutsideCoverage)
        );
    }
}
