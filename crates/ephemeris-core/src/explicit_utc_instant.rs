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

//! One UTC instant to UT1/TT/TDB using caller-supplied offsets, 2026-11-15.
//! No leap-second table, Earth-orientation model, or TDB model is inferred.
use std::{error::Error, fmt};

const MS_PER_DAY: i64 = 86_400_000;
const UNIX_EPOCH_JD: f64 = 2_440_587.5;
const UTC_1972_UNIX_MS: i64 = 63_072_000_000;
const UTC_2100_UNIX_MS: i64 = 4_102_444_800_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExplicitUtcOffsets {
    pub ut1_minus_utc_seconds: f64,
    pub tai_minus_utc_seconds: i32,
    pub tdb_minus_tt_seconds: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExplicitInstant {
    pub unix_milliseconds: i64,
    pub ut1_jd: f64,
    pub tt_jd: f64,
    pub tdb_jd: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplicitInstantError {
    UnsupportedInstant,
    InvalidOffsets,
    InvalidWindow,
    Ut1OutsideWindow,
}
impl fmt::Display for ExplicitInstantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedInstant => write!(
                f,
                "UTC instant must be between 1972 and 2100, excluding leap-second labels"
            ),
            Self::InvalidOffsets => write!(
                f,
                "caller-supplied time offsets are invalid or outside supported bounds"
            ),
            Self::InvalidWindow => write!(f, "UTC window must be ordered and at most 31 days"),
            Self::Ut1OutsideWindow => write!(f, "UT1 instant is outside the explicit window"),
        }
    }
}

/// Convert an arbitrary binary64 UT1 Julian date in a short, inclusive UTC
/// window. The caller asserts all three offsets remain constant and that the
/// window contains no leap-second label. Neither assertion is verified here.
/// UTC is rounded to a POSIX millisecond; UT1, TT and TDB remain binary64 JDs.
pub fn explicit_ut1_in_window(
    ut1_jd: f64,
    start_unix_milliseconds: i64,
    end_unix_milliseconds: i64,
    offsets: ExplicitUtcOffsets,
) -> Result<ExplicitInstant, ExplicitInstantError> {
    let start = explicit_utc_instant(start_unix_milliseconds, offsets)?;
    let end = explicit_utc_instant(end_unix_milliseconds, offsets)?;
    if end_unix_milliseconds <= start_unix_milliseconds
        || end_unix_milliseconds - start_unix_milliseconds > 31 * MS_PER_DAY
    {
        return Err(ExplicitInstantError::InvalidWindow);
    }
    if !ut1_jd.is_finite() || ut1_jd < start.ut1_jd || ut1_jd > end.ut1_jd {
        return Err(ExplicitInstantError::Ut1OutsideWindow);
    }
    let utc_ms = ((ut1_jd - UNIX_EPOCH_JD) * MS_PER_DAY as f64
        - offsets.ut1_minus_utc_seconds * 1000.0)
        .round() as i64;
    let tt_minus_ut1 =
        f64::from(offsets.tai_minus_utc_seconds) + 32.184 - offsets.ut1_minus_utc_seconds;
    Ok(ExplicitInstant {
        unix_milliseconds: utc_ms,
        ut1_jd,
        tt_jd: ut1_jd + tt_minus_ut1 / 86_400.0,
        tdb_jd: ut1_jd + (tt_minus_ut1 + offsets.tdb_minus_tt_seconds) / 86_400.0,
    })
}
impl Error for ExplicitInstantError {}

/// Convert POSIX milliseconds representing a normal UTC civil label. The
/// caller must supply time offsets applicable at exactly that instant and is
/// responsible for their provenance, leap-second changes and interpolation.
/// Leap-second labels themselves cannot be encoded by POSIX milliseconds.
/// Returned JDs are binary64 and have approximately 40 µs granularity near
/// the present; they are not a high-resolution split-date representation.
pub fn explicit_utc_instant(
    unix_milliseconds: i64,
    offsets: ExplicitUtcOffsets,
) -> Result<ExplicitInstant, ExplicitInstantError> {
    if !(UTC_1972_UNIX_MS..UTC_2100_UNIX_MS).contains(&unix_milliseconds) {
        return Err(ExplicitInstantError::UnsupportedInstant);
    }
    if !offsets.ut1_minus_utc_seconds.is_finite()
        || offsets.ut1_minus_utc_seconds.abs() > 0.9
        || !(10..=100).contains(&offsets.tai_minus_utc_seconds)
        || !offsets.tdb_minus_tt_seconds.is_finite()
        || offsets.tdb_minus_tt_seconds.abs() > 0.01
    {
        return Err(ExplicitInstantError::InvalidOffsets);
    }
    let day = unix_milliseconds.div_euclid(MS_PER_DAY);
    let seconds = (unix_milliseconds.rem_euclid(MS_PER_DAY) as f64) / 1000.0;
    let midnight = UNIX_EPOCH_JD + day as f64;
    let tt_seconds = seconds + f64::from(offsets.tai_minus_utc_seconds) + 32.184;
    Ok(ExplicitInstant {
        unix_milliseconds,
        ut1_jd: midnight + (seconds + offsets.ut1_minus_utc_seconds) / 86_400.0,
        tt_jd: midnight + tt_seconds / 86_400.0,
        tdb_jd: midnight + (tt_seconds + offsets.tdb_minus_tt_seconds) / 86_400.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_independent_erfa_fixtures_including_leap_boundary() {
        // pyerfa 2.0.1.5 dtf2d(UTC)->utctai->taitt and utcut1, normal labels.
        let cases = [
            (
                946_728_000_000,
                0.355,
                32,
                2_451_545.000_004_109,
                2_451_545.000_742_870_4,
            ),
            (
                1_483_228_799_000,
                -0.4087,
                36,
                2_457_754.499_983_696,
                2_457_754.500_777_592_4,
            ),
            (
                1_483_228_800_000,
                0.5913,
                37,
                2_457_754.500_006_844,
                2_457_754.500_800_741,
            ),
            (
                1_794_268_800_000,
                0.1,
                37,
                2_461_354.500_001_157_6,
                2_461_354.500_800_741,
            ),
        ];
        for (millis, dut1, tai, ut1_ref, tt_ref) in cases {
            let value = explicit_utc_instant(
                millis,
                ExplicitUtcOffsets {
                    ut1_minus_utc_seconds: dut1,
                    tai_minus_utc_seconds: tai,
                    tdb_minus_tt_seconds: 0.0,
                },
            )
            .unwrap();
            assert!((value.ut1_jd - ut1_ref).abs() < 1e-9, "{millis} UT1");
            assert!((value.tt_jd - tt_ref).abs() < 1e-9, "{millis} TT");
            assert_eq!(value.tt_jd, value.tdb_jd);
        }
    }
    #[test]
    fn invalid_instant_and_offsets_are_errors() {
        let good = ExplicitUtcOffsets {
            ut1_minus_utc_seconds: 0.0,
            tai_minus_utc_seconds: 37,
            tdb_minus_tt_seconds: 0.0,
        };
        assert_eq!(
            explicit_utc_instant(0, good),
            Err(ExplicitInstantError::UnsupportedInstant)
        );
        assert_eq!(
            explicit_utc_instant(UTC_2100_UNIX_MS, good),
            Err(ExplicitInstantError::UnsupportedInstant)
        );
        assert_eq!(
            explicit_utc_instant(
                UTC_1972_UNIX_MS,
                ExplicitUtcOffsets {
                    ut1_minus_utc_seconds: f64::NAN,
                    ..good
                }
            ),
            Err(ExplicitInstantError::InvalidOffsets)
        );
        assert_eq!(
            explicit_utc_instant(
                UTC_1972_UNIX_MS,
                ExplicitUtcOffsets {
                    tai_minus_utc_seconds: 0,
                    ..good
                }
            ),
            Err(ExplicitInstantError::InvalidOffsets)
        );
        assert_eq!(
            explicit_utc_instant(
                UTC_1972_UNIX_MS,
                ExplicitUtcOffsets {
                    tdb_minus_tt_seconds: 0.02,
                    ..good
                }
            ),
            Err(ExplicitInstantError::InvalidOffsets)
        );
    }

    #[test]
    fn ut1_window_matches_independent_erfa_anchors_and_rejects_outside() {
        // pyerfa 2.0.1.5 utcut1 and utctai->taitt, normal UTC label at J2000.
        let offsets = ExplicitUtcOffsets {
            ut1_minus_utc_seconds: 0.355,
            tai_minus_utc_seconds: 32,
            tdb_minus_tt_seconds: 0.0,
        };
        let start = 946_684_800_000;
        let end = 946_857_600_000;
        let converted = explicit_ut1_in_window(2_451_545.000_004_109, start, end, offsets).unwrap();
        assert_eq!(converted.unix_milliseconds, 946_728_000_000);
        assert!((converted.tt_jd - 2_451_545.000_742_870_4).abs() < 1e-9);
        assert_eq!(converted.tt_jd, converted.tdb_jd);
        assert_eq!(
            explicit_ut1_in_window(f64::NAN, start, end, offsets),
            Err(ExplicitInstantError::Ut1OutsideWindow)
        );
        assert_eq!(
            explicit_ut1_in_window(2_451_540.0, start, end, offsets),
            Err(ExplicitInstantError::Ut1OutsideWindow)
        );
        assert_eq!(
            explicit_ut1_in_window(2_451_545.0, end, start, offsets),
            Err(ExplicitInstantError::InvalidWindow)
        );
        assert_eq!(
            explicit_ut1_in_window(2_451_545.0, start, start + 32 * MS_PER_DAY, offsets),
            Err(ExplicitInstantError::InvalidWindow)
        );
        for ms in [start, end] {
            let anchor = explicit_utc_instant(ms, offsets).unwrap();
            assert_eq!(
                explicit_ut1_in_window(anchor.ut1_jd, start, end, offsets)
                    .unwrap()
                    .unix_milliseconds,
                ms
            );
        }
    }
}
