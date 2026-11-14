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

//! Same-epoch batch queries through the canonical loaded SPK kernel set.
//!
//! A batch is an ordered list of target-center NAIF body ID pairs evaluated
//! at one explicit finite ET (TDB seconds past J2000). The existing relative
//! state solver handles all kernel priority, center-chain and interpolation.
//! This adapter never substitutes missing data or returns a partial batch.
//!
//! Modified 2026-11-14: initial batch API, no new astronomical algorithms.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::spk_kernel_set::SpkKernelSet;
use crate::spk_relative_state::{SpkRelativeError, SpkRelativeState};

/// Failure of an entire geometric SPK batch.
///
/// The failing pair's zero-based position is preserved for downstream
/// diagnostics; no successful-prefix vector is returned.
#[derive(Debug)]
pub enum SpkBatchError {
    InvalidEpoch,
    Request {
        index: usize,
        target_id: i32,
        center_id: i32,
        cause: SpkRelativeError,
    },
}

impl fmt::Display for SpkBatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEpoch => write!(f, "SPK batch ET must be finite TDB seconds past J2000"),
            Self::Request {
                index,
                target_id,
                center_id,
                cause,
            } => write!(
                f,
                "SPK batch request {index} (target {target_id}, center {center_id}) failed: {cause}"
            ),
        }
    }
}

impl Error for SpkBatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidEpoch => None,
            Self::Request { cause, .. } => Some(cause),
        }
    }
}

/// Evaluate multiple target-minus-center geometric states at one explicit ET.
///
/// Results preserve the exact order and multiplicity of caller-supplied
/// (target_id, center_id) tuples; no deduplication or sorting is done.
/// Success returns a vector of the same length. The first failing pair
/// instead returns SpkBatchError::Request and no partial results.
///
/// Contract: ET in TDB seconds since J2000, NAIF frame 1 (J2000/ICRF),
/// geometric positions in km and velocities in km per TDB second. Requires
/// only supported SPK type-2/3 coverage for the requested center chains.
/// A (body, body) pair preserves the base solver's exact-zero identity;
/// it is not proof that a kernel contains that body.
///
/// Finite ET with an empty request list succeeds with an empty result, even
/// for an empty kernel set. NaN or infinity is always rejected, including
/// for an empty request list. Calls only SpkKernelSet::relative_state,
/// so loaded-file precedence, unsupported-frame/type rejection and cycle
/// detection are identical to individual queries.
///
/// It does NOT convert UTC/TT, implement date-frame rotation or apparent
/// corrections, download kernels, validate kernel provenance, or establish
/// independent numerical equivalence with NAIF/SPICE.
pub fn relative_states_at<R: Read + Seek>(
    kernels: &mut SpkKernelSet<R>,
    requests: &[(i32, i32)],
    et: f64,
) -> Result<Vec<SpkRelativeState>, SpkBatchError> {
    if !et.is_finite() {
        return Err(SpkBatchError::InvalidEpoch);
    }
    let mut states = Vec::with_capacity(requests.len());
    for (index, &(target_id, center_id)) in requests.iter().enumerate() {
        let state = kernels
            .relative_state(target_id, center_id, et)
            .map_err(|cause| SpkBatchError::Request {
                index,
                target_id,
                center_id,
                cause,
            })?;
        states.push(state);
    }
    Ok(states)
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
        // Fully synthetic, exactly one Type-2 Chebyshev record (midpoint ET=5,
        // radius=5s); never a downloaded or redistributed NASA/JPL kernel.
        let mut bytes = vec![0_u8; 5 * 1024];
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
        let mut set = SpkKernelSet::new();
        set.load(kernel(399, 0, [100.0, 5.0], [3.0, -1.0], Order::Little, 1))
            .unwrap();
        set.load(kernel(301, 399, [2.0, 4.0], [0.1, 0.2], Order::Big, 1))
            .unwrap();
        set
    }

    #[test]
    fn cross_kernel_batch_preserves_request_order_duplicates_and_units() {
        let mut set = two_kernels();
        let requests = [(301, 0), (399, 0), (301, 399), (399, 301), (301, 0)];
        let actual = relative_states_at(&mut set, &requests, 5.0).unwrap();
        assert_eq!(actual.len(), requests.len());
        for (index, &(target, center)) in requests.iter().enumerate() {
            let individually = set.relative_state(target, center, 5.0).unwrap();
            assert_eq!(actual[index], individually);
            assert_eq!(actual[index].target_id, target);
            assert_eq!(actual[index].center_id, center);
            assert_eq!(actual[index].tdb_seconds_past_j2000, 5.0);
        }
        assert_eq!(actual[0], actual[4]);
        assert_eq!(actual[0].common_ancestor_id, 0);
        assert_eq!(actual[0].position_km, [102.0, 9.0, 0.0]);
        assert!((actual[0].velocity_km_per_second[0] - 3.1).abs() < 1e-12);
        assert!((actual[0].velocity_km_per_second[1] + 0.8).abs() < 1e-12);
        assert_eq!(actual[3].position_km, [-2.0, -4.0, 0.0]);
        assert!((actual[3].velocity_km_per_second[0] + 0.1).abs() < 1e-12);
    }

    #[test]
    fn failures_report_pair_index_with_source_and_never_return_partial_states() {
        let mut set = two_kernels();
        let requests = [(301, 0), (999, 0), (399, 0)];
        let failure = relative_states_at(&mut set, &requests, 5.0).unwrap_err();
        assert!(matches!(
            failure,
            SpkBatchError::Request {
                index: 1,
                target_id: 999,
                center_id: 0,
                cause: SpkRelativeError::Disconnected { .. }
            }
        ));
        assert!(failure.source().is_some());
        let retry = relative_states_at(&mut set, &requests[..1], 5.0).unwrap();
        assert_eq!(retry[0].position_km, [102.0, 9.0, 0.0]);
    }

    #[test]
    fn unsupported_newest_kernel_must_not_fall_back_to_older() {
        let mut set = two_kernels();
        set.load(kernel(
            301,
            399,
            [1000.0, 0.0],
            [0.0, 0.0],
            Order::Little,
            17,
        ))
        .unwrap();
        let failure = relative_states_at(&mut set, &[(399, 0), (301, 399)], 5.0).unwrap_err();
        assert!(matches!(
            failure,
            SpkBatchError::Request {
                index: 1,
                cause: SpkRelativeError::Segment(SpkStateError::UnsupportedFrame(17)),
                ..
            }
        ));
        set.unload_last().unwrap();
        assert_eq!(
            relative_states_at(&mut set, &[(301, 399)], 5.0).unwrap()[0].position_km,
            [2.0, 4.0, 0.0]
        );
    }

    // Modification (2026-11-14): fail the entire ordered batch when a later
    // kernel becomes unreadable, without losing the original I/O cause or
    // poisoning a subsequent retry on the same loaded sources.
    #[test]
    fn failed_later_kernel_read_aborts_entire_batch_and_recovers() {
        use std::cell::Cell;
        use std::io::{self, Read, Seek, SeekFrom};
        use std::rc::Rc;

        struct ReadSwitch {
            inner: Cursor<Vec<u8>>,
            denied: Rc<Cell<bool>>,
        }

        impl Read for ReadSwitch {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                if self.denied.get() {
                    Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "synthetic source became unavailable",
                    ))
                } else {
                    self.inner.read(buffer)
                }
            }
        }

        impl Seek for ReadSwitch {
            fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
                self.inner.seek(pos)
            }
        }

        let deny_earth = Rc::new(Cell::new(false));
        let deny_moon = Rc::new(Cell::new(false));
        let mut set = SpkKernelSet::new();
        set.load(ReadSwitch {
            inner: kernel(399, 0, [100.0, 5.0], [3.0, -1.0], Order::Little, 1),
            denied: Rc::clone(&deny_earth),
        })
        .unwrap();
        set.load(ReadSwitch {
            inner: kernel(301, 399, [2.0, 4.0], [0.1, 0.2], Order::Big, 1),
            denied: Rc::clone(&deny_moon),
        })
        .unwrap();

        // The first request is valid. The second needs the newer Moon
        // kernel, where the injected BrokenPipe must abort the whole batch.
        let requests = [(399, 0), (301, 399), (301, 0)];
        deny_moon.set(true);
        let failure = relative_states_at(&mut set, &requests, 5.0).unwrap_err();
        assert!(matches!(
            failure,
            SpkBatchError::Request {
                index: 1,
                target_id: 301,
                center_id: 399,
                cause: SpkRelativeError::Segment(SpkStateError::Io(ref error))
            } if error.kind() == io::ErrorKind::BrokenPipe
        ));

        // The standard Error::source chain retains the native I/O cause.
        let relative = failure.source().unwrap();
        let segment = relative.source().unwrap();
        let io_cause = segment.source().unwrap();
        assert_eq!(
            io_cause.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::BrokenPipe
        );
        assert_eq!(set.kernel_count(), 2);

        // No reindexing/reloading should be required after the fault clears.
        deny_moon.set(false);
        let recovered = relative_states_at(&mut set, &requests, 5.0).unwrap();
        assert_eq!(recovered.len(), 3);
        assert_eq!(recovered[0].position_km, [100.0, 5.0, 0.0]);
        assert_eq!(recovered[1].position_km, [2.0, 4.0, 0.0]);
        assert_eq!(recovered[2].position_km, [102.0, 9.0, 0.0]);
        assert!((recovered[2].velocity_km_per_second[0] - 3.1).abs() < 1e-12);
        assert!((recovered[2].velocity_km_per_second[1] + 0.8).abs() < 1e-12);
        assert_eq!(set.kernel_count(), 2);
    }

    #[test]
    fn nonfinite_et_fails_even_for_empty_batch_and_empty_kernels() {
        let mut empty: SpkKernelSet<Cursor<Vec<u8>>> = SpkKernelSet::new();
        assert!(relative_states_at(&mut empty, &[], 5.0).unwrap().is_empty());
        assert_eq!(
            relative_states_at(&mut empty, &[], f64::NAN)
                .unwrap_err()
                .to_string(),
            "SPK batch ET must be finite TDB seconds past J2000"
        );
        for et in [f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                relative_states_at(&mut empty, &[(301, 0)], et),
                Err(SpkBatchError::InvalidEpoch)
            ));
        }
        assert_eq!(
            relative_states_at(&mut empty, &[(301, 301)], 5.0).unwrap()[0].position_km,
            [0.0; 3]
        );
    }
}
