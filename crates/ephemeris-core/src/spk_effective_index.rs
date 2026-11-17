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

//! Read-only winning SPK descriptor inventory at one TDB/ET epoch.
//!
//! Uses the existing Rust kernel-set selection and target inventory only.
//! Modification (2026-11-14): ordered metadata snapshot, no new astronomy.

use crate::spk_direct_state::SpkStateError;
use crate::spk_index::SpkSegmentDescriptor;
use crate::spk_kernel_set::SpkKernelSet;
use std::io::{Read, Seek};

/// The selected descriptor for a single target at the requested instant.
/// Kernel indices start at zero (oldest loaded file). This is only metadata.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpkEffectiveSegment {
    pub target_id: i32,
    pub kernel_load_index: usize,
    pub descriptor: SpkSegmentDescriptor,
}

/// Inspect winning descriptors for all targets covered at finite TDB/ET.
///
/// Returned targets are sorted by signed NAIF ID, with no duplicates. Missing
/// coverage is omitted; compare with declared_target_ids to find gaps.
/// Last-loaded file and last-covering segment priority are delegated to the
/// canonical selected_segment method. Unsupported frames/types and corrupt
/// coefficient arrays are not silently bypassed or hidden by this inspector.
/// No coefficients are read, states interpolated or time scales converted.
/// Input ET is TDB seconds past J2000; even an empty set rejects nonfinite ET.
pub fn effective_segments_at<R: Read + Seek>(
    kernels: &SpkKernelSet<R>,
    et: f64,
) -> Result<Vec<SpkEffectiveSegment>, SpkStateError> {
    if !et.is_finite() {
        return Err(SpkStateError::InvalidEpoch);
    }
    let mut selected = Vec::new();
    for target_id in kernels.declared_target_ids() {
        if let Some((kernel_load_index, descriptor)) = kernels.selected_segment(target_id, et)? {
            selected.push(SpkEffectiveSegment {
                target_id,
                kernel_load_index,
                descriptor,
            });
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spk_index::SpkByteOrder;
    use std::io::Cursor;

    const RECORD_BYTES: usize = 1024;

    #[derive(Clone, Copy)]
    struct Segment {
        target: i32,
        center: i32,
        frame: i32,
        kind: i32,
        start: f64,
        end: f64,
    }

    fn put_i32(bytes: &mut [u8], at: usize, value: i32, order: SpkByteOrder) {
        let v = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[at..at + 4].copy_from_slice(&v);
    }

    fn put_f64(bytes: &mut [u8], at: usize, value: f64, order: SpkByteOrder) {
        let v = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[at..at + 8].copy_from_slice(&v);
    }

    fn kernel(order: SpkByteOrder, segments: &[Segment]) -> Cursor<Vec<u8>> {
        // Only metadata is inspected; these are invented in-memory DAF files.
        assert!(!segments.is_empty() && segments.len() <= 6);
        let mut bytes = vec![0u8; 5 * RECORD_BYTES];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(match order {
            SpkByteOrder::LittleEndian => b"LTL-IEEE",
            SpkByteOrder::BigEndian => b"BIG-IEEE",
        });
        for (at, value) in [(8, 2), (12, 6), (76, 3), (80, 3)] {
            put_i32(&mut bytes, at, value, order);
        }
        let summary = 2 * RECORD_BYTES;
        put_f64(&mut bytes, summary + 16, segments.len() as f64, order);
        let mut next = 513usize;
        for (slot, seg) in segments.iter().enumerate() {
            let first = next;
            let last = first + 11;
            let desc = summary + (3 + 5 * slot) * 8;
            put_f64(&mut bytes, desc, seg.start, order);
            put_f64(&mut bytes, desc + 8, seg.end, order);
            for (i, v) in [
                seg.target,
                seg.center,
                seg.frame,
                seg.kind,
                first as i32,
                last as i32,
            ]
            .into_iter()
            .enumerate()
            {
                put_i32(&mut bytes, desc + 16 + i * 4, v, order);
            }
            let at = (first - 1) * 8;
            for (i, v) in [5.0, 5.0, 10.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 10.0, 8.0, 1.0]
                .into_iter()
                .enumerate()
            {
                put_f64(&mut bytes, at + i * 8, v, order);
            }
            next = last + 1;
        }
        assert!(next <= 640);
        put_i32(&mut bytes, 84, next as i32, order);
        Cursor::new(bytes)
    }

    fn fixture() -> SpkKernelSet<Cursor<Vec<u8>>> {
        let mut set = SpkKernelSet::new();
        set.load(kernel(
            SpkByteOrder::LittleEndian,
            &[
                Segment {
                    target: 399,
                    center: 0,
                    frame: 1,
                    kind: 2,
                    start: 0.0,
                    end: 10.0,
                },
                Segment {
                    target: 301,
                    center: 399,
                    frame: 1,
                    kind: 2,
                    start: 0.0,
                    end: 10.0,
                },
                Segment {
                    target: -100,
                    center: 0,
                    frame: 1,
                    kind: 2,
                    start: -10.0,
                    end: -1.0,
                },
            ],
        ))
        .unwrap();
        set.load(kernel(
            SpkByteOrder::BigEndian,
            &[
                Segment {
                    target: 301,
                    center: 4,
                    frame: 17,
                    kind: 9,
                    start: 2.0,
                    end: 8.0,
                },
                Segment {
                    target: 500,
                    center: 0,
                    frame: 1,
                    kind: 2,
                    start: 8.0,
                    end: 12.0,
                },
            ],
        ))
        .unwrap();
        set
    }

    #[test]
    fn sorted_signed_targets_and_unsupported_overlay_match_canonical_selection() {
        let set = fixture();
        let selected = effective_segments_at(&set, 5.0).unwrap();
        assert_eq!(
            selected.iter().map(|s| s.target_id).collect::<Vec<_>>(),
            [301, 399]
        );
        assert_eq!(selected[0].kernel_load_index, 1);
        assert_eq!(
            (
                selected[0].descriptor.center_id,
                selected[0].descriptor.frame_id,
                selected[0].descriptor.spk_type
            ),
            (4, 17, 9)
        );
        assert_eq!(selected[1].kernel_load_index, 0);
        for item in &selected {
            assert_eq!(
                set.selected_segment(item.target_id, 5.0).unwrap(),
                Some((item.kernel_load_index, item.descriptor))
            );
        }
        assert!(set.selected_segment(-100, 5.0).unwrap().is_none());
        assert!(set.selected_segment(500, 5.0).unwrap().is_none());
    }

    #[test]
    fn coverage_boundaries_and_gaps_include_endpoints() {
        let set = fixture();
        let before = effective_segments_at(&set, -10.0).unwrap();
        assert_eq!(
            before.iter().map(|s| s.target_id).collect::<Vec<_>>(),
            [-100]
        );
        assert!(effective_segments_at(&set, -0.5).unwrap().is_empty());
        assert_eq!(
            effective_segments_at(&set, 2.0).unwrap()[0].kernel_load_index,
            1
        );
        let at_eight = effective_segments_at(&set, 8.0).unwrap();
        assert_eq!(
            at_eight.iter().map(|s| s.target_id).collect::<Vec<_>>(),
            [301, 399, 500]
        );
        assert_eq!(at_eight[0].kernel_load_index, 1);
        let at_ten = effective_segments_at(&set, 10.0).unwrap();
        assert_eq!(
            at_ten.iter().map(|s| s.target_id).collect::<Vec<_>>(),
            [301, 399, 500]
        );
        assert_eq!(at_ten[0].kernel_load_index, 0);
    }

    #[test]
    fn unloading_overlay_restores_previous_priority_without_reordering() {
        let mut set = fixture();
        assert_eq!(
            effective_segments_at(&set, 5.0).unwrap()[0].kernel_load_index,
            1
        );
        set.unload_last().unwrap();
        let restored = effective_segments_at(&set, 5.0).unwrap();
        assert_eq!(
            restored.iter().map(|s| s.target_id).collect::<Vec<_>>(),
            [301, 399]
        );
        assert_eq!(restored[0].kernel_load_index, 0);
        assert_eq!(restored[0].descriptor.center_id, 399);
        assert!(effective_segments_at(&set, 11.0).unwrap().is_empty());
    }

    #[test]
    fn rejects_nonfinite_even_on_empty_kernel_set() {
        let empty: SpkKernelSet<Cursor<Vec<u8>>> = SpkKernelSet::new();
        assert!(effective_segments_at(&empty, 0.0).unwrap().is_empty());
        for et in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                effective_segments_at(&empty, et),
                Err(SpkStateError::InvalidEpoch)
            ));
            assert!(matches!(
                effective_segments_at(&fixture(), et),
                Err(SpkStateError::InvalidEpoch)
            ));
        }
    }
}
