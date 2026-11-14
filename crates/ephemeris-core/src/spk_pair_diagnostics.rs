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

//! Per-pair, best-effort diagnostic probes of geometric SPK relative states.
//!
//! Unlike the atomic batch API, every pair produces its own success/error.
//! This is a diagnostic wrapper around the canonical Rust relative solver,
//! not a second astronomical implementation or a production accuracy claim.
//!
//! Modification (2026-11-14): add per-pair result preservation for coverage
//! and unsupported-SPK investigations using caller-provided kernel sets.

use std::io::{Read, Seek};

use crate::spk_kernel_set::SpkKernelSet;
use crate::spk_relative_state::{SpkRelativeError, SpkRelativeState};

/// Diagnostic result for one requested target-minus-center pair.
///
/// Each failure contains the exact solver error. Unlike an all-or-error
/// batch, an individual failure does not erase successes for other pairs.
#[derive(Debug)]
pub struct SpkRelativeProbe {
    pub request_index: usize,
    pub target_id: i32,
    pub center_id: i32,
    pub result: Result<SpkRelativeState, SpkRelativeError>,
}

/// Probe independent geometric pairs at the same explicit TDB/ET epoch.
///
/// Every pair is evaluated by the *existing* SpkKernelSet::relative_state.
/// The output preserves original pair order, indices and duplicates, and is
/// always the same length as requests for a finite epoch. A missing center
/// chain, cycle, unsupported frame/type or invalid coefficient is retained
/// as a per-pair error; other pairs are still examined. Failed pairs never
/// fall back to lower-priority kernels or generate fabricated zero vectors.
///
/// A nonfinite ET is a global input error, including for empty requests.
/// Finite ET with no requests succeeds with an empty vector. A same-body
/// pair delegates to the solver's mathematical zero identity and does not
/// establish actual data coverage for that body.
///
/// All successful states are geometric J2000/ICRF frame-1 positions (km)
/// and velocities (km per TDB second). The epoch is TDB seconds since J2000.
/// No apparent corrections, UTC/TT conversion, independent NAIF/SPICE
/// numerical validation, data downloads or source authentication occur.
///
/// For workflows that require atomic all-or-error output instead, use
/// spk_batch::relative_states_at, not this deliberately diagnostic API.
pub fn probe_relative_states_at<R: Read + Seek>(
    kernels: &mut SpkKernelSet<R>,
    requests: &[(i32, i32)],
    et: f64,
) -> Result<Vec<SpkRelativeProbe>, SpkRelativeError> {
    if !et.is_finite() {
        return Err(SpkRelativeError::InvalidEpoch);
    }
    let mut probes = Vec::with_capacity(requests.len());
    for (request_index, &(target_id, center_id)) in requests.iter().enumerate() {
        probes.push(SpkRelativeProbe {
            request_index,
            target_id,
            center_id,
            result: kernels.relative_state(target_id, center_id, et),
        });
    }
    Ok(probes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spk_direct_state::SpkStateError;
    use std::io::Cursor;

    #[derive(Clone, Copy)]
    enum Order {
        Little,
        Big,
    }

    fn put_i32(file: &mut [u8], at: usize, value: i32, order: Order) {
        let bytes = match order {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        };
        file[at..at + 4].copy_from_slice(&bytes);
    }

    fn put_f64(file: &mut [u8], at: usize, value: f64, order: Order) {
        let bytes = match order {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        };
        file[at..at + 8].copy_from_slice(&bytes);
    }

    fn kernel(
        target: i32,
        center: i32,
        position: [f64; 2],
        velocity: [f64; 2],
        order: Order,
        frame: i32,
    ) -> Cursor<Vec<u8>> {
        // Entirely synthetic, one Type-2 Chebyshev record over ET [0,10].
        // This tests diagnostic routing, NOT numerical parity with NASA.
        let mut bytes = vec![0u8; 5 * 1024];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(match order {
            Order::Little => b"LTL-IEEE",
            Order::Big => b"BIG-IEEE",
        });
        for (at, value) in [(8, 2), (12, 6), (76, 3), (80, 3), (84, 525)] {
            put_i32(&mut bytes, at, value, order);
        }
        let summary = 2 * 1024;
        put_f64(&mut bytes, summary + 16, 1.0, order);
        put_f64(&mut bytes, summary + 24, 0.0, order);
        put_f64(&mut bytes, summary + 32, 10.0, order);
        for (i, value) in [target, center, frame, 2, 513, 524].into_iter().enumerate() {
            put_i32(&mut bytes, summary + 40 + i * 4, value, order);
        }
        let record = 4 * 1024;
        put_f64(&mut bytes, record, 5.0, order);
        put_f64(&mut bytes, record + 8, 5.0, order);
        for (i, value) in [
            position[0],
            velocity[0] * 5.0,
            position[1],
            velocity[1] * 5.0,
            0.0,
            0.0,
        ]
        .into_iter()
        .enumerate()
        {
            put_f64(&mut bytes, record + 16 + i * 8, value, order);
        }
        for (i, value) in [0.0, 10.0, 8.0, 1.0].into_iter().enumerate() {
            put_f64(&mut bytes, record + 64 + i * 8, value, order);
        }
        Cursor::new(bytes)
    }

    fn two_kernels() -> SpkKernelSet<Cursor<Vec<u8>>> {
        let mut kernels = SpkKernelSet::new();
        kernels
            .load(kernel(399, 0, [100.0, 5.0], [3.0, -1.0], Order::Little, 1))
            .unwrap();
        kernels
            .load(kernel(301, 399, [2.0, 4.0], [0.1, 0.2], Order::Big, 1))
            .unwrap();
        kernels
    }

    #[test]
    fn mixed_endian_good_bad_good_pairs_keep_order_and_nested_errors() {
        let mut kernels = two_kernels();
        let pairs = [(301, 0), (999, 0), (399, 0), (301, 399), (301, 0)];
        let rows = probe_relative_states_at(&mut kernels, &pairs, 5.0).unwrap();
        assert_eq!(rows.len(), pairs.len());
        for (i, &(target, center)) in pairs.iter().enumerate() {
            assert_eq!(rows[i].request_index, i);
            assert_eq!((rows[i].target_id, rows[i].center_id), (target, center));
        }
        let first = rows[0].result.as_ref().unwrap();
        assert_eq!(first.common_ancestor_id, 0);
        assert_eq!(first.position_km, [102.0, 9.0, 0.0]);
        assert!((first.velocity_km_per_second[0] - 3.1).abs() < 1e-12);
        assert!((first.velocity_km_per_second[1] + 0.8).abs() < 1e-12);
        assert!(matches!(
            rows[1].result,
            Err(SpkRelativeError::Disconnected {
                target_id: 999,
                center_id: 0
            })
        ));
        for i in [0, 2, 3, 4] {
            assert_eq!(
                rows[i].result.as_ref().unwrap(),
                &kernels.relative_state(pairs[i].0, pairs[i].1, 5.0).unwrap()
            );
        }
        assert_eq!(
            rows[0].result.as_ref().unwrap(),
            rows[4].result.as_ref().unwrap()
        );
    }

    #[test]
    fn unsupported_high_priority_overlay_is_not_skipped_and_other_pairs_survive() {
        let mut kernels = two_kernels();
        kernels
            .load(kernel(301, 399, [7.0, 7.0], [0.0, 0.0], Order::Little, 17))
            .unwrap();
        let pairs = [(301, 399), (399, 0), (301, 0)];
        let probes = probe_relative_states_at(&mut kernels, &pairs, 5.0).unwrap();
        assert!(matches!(
            probes[0].result,
            Err(SpkRelativeError::Segment(SpkStateError::UnsupportedFrame(
                17
            )))
        ));
        assert!(probes[1].result.is_ok());
        assert!(matches!(
            probes[2].result,
            Err(SpkRelativeError::Segment(SpkStateError::UnsupportedFrame(
                17
            )))
        ));
        assert_eq!(kernels.kernel_count(), 3);
        kernels.unload_last().unwrap();
        let restored = probe_relative_states_at(&mut kernels, &pairs, 5.0).unwrap();
        assert!(restored.iter().all(|probe| probe.result.is_ok()));
    }

    #[test]
    fn finite_empty_set_and_same_body_identity_are_explicit() {
        let mut empty: SpkKernelSet<Cursor<Vec<u8>>> = SpkKernelSet::new();
        assert!(probe_relative_states_at(&mut empty, &[], 0.0)
            .unwrap()
            .is_empty());
        let pairs = [(777, 777), (777, 0), (777, 777)];
        let result = probe_relative_states_at(&mut empty, &pairs, -100.0).unwrap();
        assert_eq!(result.len(), 3);
        for index in [0, 2] {
            let state = result[index].result.as_ref().unwrap();
            assert_eq!((state.target_id, state.center_id), (777, 777));
            assert_eq!(state.position_km, [0.0; 3]);
            assert_eq!(state.velocity_km_per_second, [0.0; 3]);
        }
        assert!(matches!(
            result[1].result,
            Err(SpkRelativeError::Disconnected {
                target_id: 777,
                center_id: 0
            })
        ));
        assert_eq!(empty.kernel_count(), 0);
    }

    #[test]
    fn invalid_global_epoch_is_rejected_even_for_empty_request_list() {
        let mut kernels = two_kernels();
        for et in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                probe_relative_states_at(&mut kernels, &[], et),
                Err(SpkRelativeError::InvalidEpoch)
            ));
            assert!(matches!(
                probe_relative_states_at(&mut kernels, &[(301, 301)], et),
                Err(SpkRelativeError::InvalidEpoch)
            ));
        }
        assert_eq!(kernels.kernel_count(), 2);
    }
}
