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

//! Strict split-TDB Julian-date entry points for geometric SPK state queries.
//!
//! This module converts an already-TDB split Julian date into ET only once,
//! then delegates to the canonical SPK relative-state and batch solvers.
//! Other time scales must never be mislabeled as TDB.
//!
//! Modification (2026-11-14): introduce typed calendar-to-SPK adapters,
//! without reproducing astronomical interpolation or time-scale models.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::calendar::{CalendarError, JulianDate};
use crate::spk_batch::{relative_states_at, SpkBatchError};
use crate::spk_kernel_set::SpkKernelSet;
use crate::spk_relative_state::{SpkRelativeError, SpkRelativeState};

/// Failure of a split-JD geometric SPK query.
///
/// Calendar errors are returned before any kernel is queried. The nested
/// relative-state/batch errors preserve their exact underlying causes.
#[derive(Debug)]
pub enum SpkDatedQueryError {
    Calendar(CalendarError),
    Relative(SpkRelativeError),
    Batch(SpkBatchError),
}

impl fmt::Display for SpkDatedQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Calendar(error) => write!(f, "invalid split TDB date for SPK: {error}"),
            Self::Relative(error) => write!(f, "SPK relative state failed: {error}"),
            Self::Batch(error) => write!(f, "SPK relative state batch failed: {error}"),
        }
    }
}

impl Error for SpkDatedQueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Calendar(error) => Some(error),
            Self::Relative(error) => Some(error),
            Self::Batch(error) => Some(error),
        }
    }
}

impl From<CalendarError> for SpkDatedQueryError {
    fn from(error: CalendarError) -> Self {
        Self::Calendar(error)
    }
}

/// Evaluate a geometric SPK target-minus-center state at a split TDB date.
///
/// The date must already have TimeScale::Tdb: TT, TAI and UT1 are rejected
/// rather than silently interpreted as TDB. The two JD parts are converted
/// directly to TDB seconds from J2000; they are never collapsed into a
/// large single-f64 Julian day. Existing per-kernel priority and supported
/// J2000/ICRF SPK type-2/3 center-chain semantics are unchanged.
///
/// Position: km. Velocity: km per TDB second. No light-time, aberration,
/// precession, nutation, frame conversion or civil-time processing occurs.
/// A same-body request returns the core's mathematical zero, even without
/// kernel coverage, after the date has been validated.
pub fn relative_state_at_tdb_date<R: Read + Seek>(
    kernels: &mut SpkKernelSet<R>,
    target_id: i32,
    center_id: i32,
    date: JulianDate,
) -> Result<SpkRelativeState, SpkDatedQueryError> {
    let et = date.to_tdb_seconds_past_j2000()?.value();
    kernels
        .relative_state(target_id, center_id, et)
        .map_err(SpkDatedQueryError::Relative)
}

/// Evaluate ordered SPK target-minus-center pairs at one split TDB date.
///
/// Converts the supplied split JD exactly once and passes one ET to every
/// pair. The canonical batch API preserves order and duplicates and fails
/// without a partial result when any link is unresolvable or unsupported.
/// Even with no requests, a wrong-scale or unrepresentable date is rejected.
pub fn relative_states_at_tdb_date<R: Read + Seek>(
    kernels: &mut SpkKernelSet<R>,
    requests: &[(i32, i32)],
    date: JulianDate,
) -> Result<Vec<SpkRelativeState>, SpkDatedQueryError> {
    let et = date.to_tdb_seconds_past_j2000()?.value();
    relative_states_at(kernels, requests, et).map_err(SpkDatedQueryError::Batch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::{CalendarError, TimeScale};
    use crate::spk_direct_state::SpkStateError;
    use std::io::Cursor;

    #[derive(Clone, Copy)]
    enum Endian {
        Little,
        Big,
    }

    fn put_i32(bytes: &mut [u8], offset: usize, value: i32, endian: Endian) {
        let payload = match endian {
            Endian::Little => value.to_le_bytes(),
            Endian::Big => value.to_be_bytes(),
        };
        bytes[offset..offset + 4].copy_from_slice(&payload);
    }

    fn put_f64(bytes: &mut [u8], offset: usize, value: f64, endian: Endian) {
        let payload = match endian {
            Endian::Little => value.to_le_bytes(),
            Endian::Big => value.to_be_bytes(),
        };
        bytes[offset..offset + 8].copy_from_slice(&payload);
    }

    fn synthetic_kernel(
        target: i32,
        center: i32,
        position: [f64; 2],
        velocity: [f64; 2],
        endian: Endian,
        frame: i32,
    ) -> Cursor<Vec<u8>> {
        // No JPL data: one DAF Type-2 Chebyshev record in ET [0, 10].
        // Coordinates have constant + linear coefficients; their velocity
        // is the exact time derivative in km/TDB-second.
        const RECORD_BYTES: usize = 1024;
        let mut bytes = vec![0_u8; 5 * RECORD_BYTES];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(match endian {
            Endian::Little => b"LTL-IEEE",
            Endian::Big => b"BIG-IEEE",
        });
        for (offset, value) in [(8, 2), (12, 6), (76, 3), (80, 3), (84, 525)] {
            put_i32(&mut bytes, offset, value, endian);
        }
        let summary = 2 * RECORD_BYTES;
        for (offset, value) in [(16, 1.0), (24, 0.0), (32, 10.0)] {
            put_f64(&mut bytes, summary + offset, value, endian);
        }
        for (i, value) in [target, center, frame, 2, 513, 524].into_iter().enumerate() {
            put_i32(&mut bytes, summary + 40 + i * 4, value, endian);
        }
        let record = 4 * RECORD_BYTES;
        for (i, value) in [
            5.0,
            5.0,
            position[0],
            velocity[0] * 5.0,
            position[1],
            velocity[1] * 5.0,
            0.0,
            0.0,
            0.0,
            10.0,
            8.0,
            1.0,
        ]
        .into_iter()
        .enumerate()
        {
            put_f64(&mut bytes, record + i * 8, value, endian);
        }
        Cursor::new(bytes)
    }

    fn two_kernels() -> SpkKernelSet<Cursor<Vec<u8>>> {
        let mut kernels = SpkKernelSet::new();
        kernels
            .load(synthetic_kernel(
                399,
                0,
                [100.0, 20.0],
                [3.0, -1.0],
                Endian::Little,
                1,
            ))
            .unwrap();
        kernels
            .load(synthetic_kernel(
                301,
                399,
                [2.0, 4.0],
                [0.1, 0.2],
                Endian::Big,
                1,
            ))
            .unwrap();
        kernels
    }

    #[test]
    fn split_tdb_quarter_second_preserves_fractional_et_and_units() {
        let date = JulianDate::new(2_451_545, 0.5 + 0.25 / 86_400.0, TimeScale::Tdb).unwrap();
        let et = date.to_tdb_seconds_past_j2000().unwrap().value();
        assert!((et - 0.25).abs() < 1e-9);
        let mut kernels = two_kernels();
        let dated = relative_state_at_tdb_date(&mut kernels, 301, 0, date).unwrap();
        let direct = kernels.relative_state(301, 0, et).unwrap();
        assert_eq!(dated, direct);
        assert_eq!(dated.common_ancestor_id, 0);
        assert!((dated.position_km[0] - (102.0 + 3.1 * (et - 5.0))).abs() < 1e-12);
        assert!((dated.velocity_km_per_second[0] - 3.1).abs() < 1e-12);
        assert_eq!(dated.tdb_seconds_past_j2000, et);
    }

    #[test]
    fn batch_one_epoch_preserves_request_order_multiplicity_and_velocities() {
        let date = JulianDate::new(2_451_545, 0.5 + 0.125 / 86_400.0, TimeScale::Tdb).unwrap();
        let et = date.to_tdb_seconds_past_j2000().unwrap().value();
        let requests = [(301, 0), (399, 0), (301, 399), (399, 301), (301, 0)];
        let mut kernels = two_kernels();
        let batch = relative_states_at_tdb_date(&mut kernels, &requests, date).unwrap();
        assert_eq!(batch.len(), requests.len());
        for (index, &(target, center)) in requests.iter().enumerate() {
            assert_eq!(
                batch[index],
                kernels.relative_state(target, center, et).unwrap()
            );
            assert_eq!(
                (batch[index].target_id, batch[index].center_id),
                (target, center)
            );
            assert_eq!(batch[index].tdb_seconds_past_j2000, et);
        }
        assert_eq!(batch[0], batch[4]);
        assert!((batch[0].velocity_km_per_second[1] + 0.8).abs() < 1e-12);
    }

    #[test]
    fn wrong_scales_fail_before_any_query_even_for_empty_batch_and_same_body() {
        let mut kernels = two_kernels();
        for scale in [TimeScale::Tt, TimeScale::Tai, TimeScale::Ut1] {
            let date = JulianDate::new(2_451_545, 0.5, scale).unwrap();
            let err = relative_state_at_tdb_date(&mut kernels, 301, 301, date).unwrap_err();
            assert!(matches!(
                err,
                SpkDatedQueryError::Calendar(CalendarError::WrongTimeScale)
            ));
            assert!(err
                .source()
                .unwrap()
                .downcast_ref::<CalendarError>()
                .is_some());
            let batch_err = relative_states_at_tdb_date(&mut kernels, &[], date).unwrap_err();
            assert!(matches!(
                batch_err,
                SpkDatedQueryError::Calendar(CalendarError::WrongTimeScale)
            ));
        }
    }

    #[test]
    fn unrepresentable_julian_days_are_rejected_before_empty_batch() {
        let mut kernels = two_kernels();
        let bad = JulianDate::new(1_i64 << 52, 0.5, TimeScale::Tdb).unwrap();
        let err = relative_states_at_tdb_date(&mut kernels, &[], bad).unwrap_err();
        assert!(matches!(
            err,
            SpkDatedQueryError::Calendar(CalendarError::YearOutOfRange)
        ));
        assert_eq!(kernels.kernel_count(), 2);
        assert!(relative_states_at_tdb_date(
            &mut kernels,
            &[],
            JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap(),
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn failing_pair_retains_nested_index_and_original_error_cause() {
        let mut kernels = two_kernels();
        let date = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap();
        let requests = [(301, 0), (999, 0), (399, 0)];
        let err = relative_states_at_tdb_date(&mut kernels, &requests, date).unwrap_err();
        assert!(matches!(
            &err,
            SpkDatedQueryError::Batch(SpkBatchError::Request {
                index: 1,
                target_id: 999,
                center_id: 0,
                cause: SpkRelativeError::Disconnected { .. },
            })
        ));
        assert!(err.source().unwrap().source().is_some());
        assert_eq!(
            kernels.relative_state(301, 0, 0.0).unwrap().position_km,
            [86.5, 28.0, 0.0]
        );
    }

    #[test]
    fn unsupported_overlay_fails_without_fallback_and_zero_identity_is_typed() {
        let mut kernels = two_kernels();
        kernels
            .load(synthetic_kernel(
                301,
                399,
                [1.0, 0.0],
                [0.0, 0.0],
                Endian::Little,
                17,
            ))
            .unwrap();
        let date = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap();
        let err = relative_state_at_tdb_date(&mut kernels, 301, 399, date).unwrap_err();
        assert!(matches!(
            err,
            SpkDatedQueryError::Relative(SpkRelativeError::Segment(
                SpkStateError::UnsupportedFrame(17)
            ))
        ));
        assert!(err.source().is_some());
        kernels.unload_last().unwrap();
        assert_eq!(
            relative_state_at_tdb_date(&mut kernels, 301, 301, date)
                .unwrap()
                .position_km,
            [0.0; 3]
        );
    }
}
