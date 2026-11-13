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

//! Ordered, caller-supplied SPK kernel set in the canonical Rust engine.
//!
//! Last loaded file wins for a given target and TDB/ET, and last covering
//! segment wins within that file. Selection does not pre-filter center, frame
//! or SPK type. Per-link evaluation delegates to the existing type-2/type-3
//! reader, and relative-state composition shares the single-kernel solver.
//!
//! NAIF precedence: <https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html>

use std::io::{Read, Seek};

use crate::spk_direct_state::{
    direct_spk_state, select_covering_segment, SpkDirectState, SpkStateError,
};
use crate::spk_index::{inspect_spk, SpkIndex, SpkIndexError};
use crate::spk_relative_state::{
    resolve_relative, SpkRelativeError, SpkRelativeState, SpkStateResolver,
};

struct LoadedSpk<R> {
    source: R,
    index: SpkIndex,
}

/// Ordered collection of validated SPK index/reader pairs.
///
/// The caller provides every seekable source; the library never downloads
/// kernels or redistributes data. Newer entries have higher priority.
/// Different sources must use the same Rust \`R: Read + Seek\` type (for example
/// \`File\`); mixed little/big-endian kernels are supported independently.
///
/// The only scientific capabilities are type 2/3 direct geometric states and
/// frame 1 (J2000/ICRF) center-chain composition. No non-J2000 frame support,
/// apparent positions, multi-file time conversion or real-kernel parity claim.
pub struct SpkKernelSet<R> {
    loaded: Vec<LoadedSpk<R>>,
}

impl<R> Default for SpkKernelSet<R> {
    fn default() -> Self {
        Self { loaded: Vec::new() }
    }
}

impl<R: Read + Seek> SpkKernelSet<R> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inspect and append one kernel as the new highest-priority source.
    ///
    /// The stream and its index remain paired for their entire lifetime.
    /// On invalid headers/summary chains no entry is added.
    pub fn load(&mut self, mut source: R) -> Result<(), SpkIndexError> {
        let index = inspect_spk(&mut source)?;
        self.loaded.push(LoadedSpk { source, index });
        Ok(())
    }

    /// Unload and return the most recently loaded source.
    ///
    /// Earlier sources regain priority for their covered targets/epochs.
    pub fn unload_last(&mut self) -> Option<R> {
        self.loaded.pop().map(|entry| entry.source)
    }

    pub fn kernel_count(&self) -> usize {
        self.loaded.len()
    }

    pub fn is_empty(&self) -> bool {
        self.loaded.is_empty()
    }

    /// Direct target-minus-selected-segment-center state, km and km/s.
    ///
    /// A higher-priority covering descriptor masks all older ones, even if
    /// it has an unsupported SPK type/frame or corrupt coefficient record.
    /// Only an *absence of time coverage* allows older files to be searched.
    pub fn direct_state(
        &mut self,
        target_id: i32,
        et: f64,
    ) -> Result<SpkDirectState, SpkStateError> {
        if !et.is_finite() {
            return Err(SpkStateError::InvalidEpoch);
        }
        let entry = self
            .loaded
            .iter_mut()
            .rev()
            .find(|entry| select_covering_segment(&entry.index, target_id, et).is_some())
            .ok_or(SpkStateError::MissingSegment { target_id })?;
        direct_spk_state(&mut entry.source, &entry.index, target_id, et)
    }

    /// Geometric state of target relative to center at one TDB/ET epoch.
    ///
    /// Each required body-to-center link is selected independently using
    /// file load order and in-file segment order. Center chains can therefore
    /// cross kernels, with no extra Chebyshev or vector-composition algorithm.
    pub fn relative_state(
        &mut self,
        target_id: i32,
        center_id: i32,
        et: f64,
    ) -> Result<SpkRelativeState, SpkRelativeError> {
        resolve_relative(self, target_id, center_id, et)
    }
}

impl<R: Read + Seek> SpkStateResolver for SpkKernelSet<R> {
    fn selected_center(&self, body: i32, et: f64) -> Option<i32> {
        self.loaded.iter().rev().find_map(|entry| {
            select_covering_segment(&entry.index, body, et)
                .map(|segment| segment.center_id)
        })
    }

    fn direct_state(&mut self, body: i32, et: f64) -> Result<SpkDirectState, SpkStateError> {
        SpkKernelSet::direct_state(self, body, et)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spk_index::SpkByteOrder;
    use std::io::Cursor;

    const RECORD_BYTES: usize = 1024;
    const INITIAL_WORD: usize = 513;

    #[derive(Clone, Copy)]
    struct Segment {
        target: i32,
        center: i32,
        frame: i32,
        spk_type: i32,
        coverage_start: f64,
        coverage_end: f64,
        position: [f64; 2],
        velocity: [f64; 2],
    }

    fn seg(
        target: i32,
        center: i32,
        spk_type: i32,
        position: [f64; 2],
        velocity: [f64; 2],
    ) -> Segment {
        Segment {
            target,
            center,
            frame: 1,
            spk_type,
            coverage_start: 0.0,
            coverage_end: 10.0,
            position,
            velocity,
        }
    }

    fn int(bytes: &mut [u8], at: usize, value: i32, endian: SpkByteOrder) {
        let payload = match endian {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[at..at + 4].copy_from_slice(&payload);
    }

    fn double(bytes: &mut [u8], at: usize, value: f64, endian: SpkByteOrder) {
        let payload = match endian {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[at..at + 8].copy_from_slice(&payload);
    }

    fn kernel(endian: SpkByteOrder, segments: &[Segment]) -> Vec<u8> {
        // Fully synthetic, one-record Type 2/3 segments, no JPL data.
        assert!(!segments.is_empty() && segments.len() <= 6);
        let mut bytes = vec![0u8; 5 * RECORD_BYTES];
        bytes[..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(match endian {
            SpkByteOrder::LittleEndian => b"LTL-IEEE",
            SpkByteOrder::BigEndian => b"BIG-IEEE",
        });
        int(&mut bytes, 8, 2, endian);
        int(&mut bytes, 12, 6, endian);
        int(&mut bytes, 76, 3, endian);
        int(&mut bytes, 80, 3, endian);
        let summary = 2 * RECORD_BYTES;
        double(&mut bytes, summary + 16, segments.len() as f64, endian);
        let mut next = INITIAL_WORD;
        for (slot, segment) in segments.iter().enumerate() {
            let component_count = if segment.spk_type == 2 { 3 } else { 6 };
            let record_words = 2 + component_count * 2;
            let first = next;
            let last = first + record_words + 3;
            let descriptor = summary + (3 + 5 * slot) * 8;
            double(&mut bytes, descriptor, segment.coverage_start, endian);
            double(&mut bytes, descriptor + 8, segment.coverage_end, endian);
            for (i, number) in [
                segment.target,
                segment.center,
                segment.frame,
                segment.spk_type,
                first as i32,
                last as i32,
            ]
            .into_iter()
            .enumerate()
            {
                int(&mut bytes, descriptor + 16 + i * 4, number, endian);
            }
            let record = (first - 1) * 8;
            double(&mut bytes, record, 5.0, endian);
            double(&mut bytes, record + 8, 5.0, endian);
            let position = segment.position;
            let velocity = segment.velocity;
            let coefficients: Vec<[f64; 2]> = if segment.spk_type == 2 {
                vec![
                    [position[0], velocity[0] * 5.0],
                    [position[1], velocity[1] * 5.0],
                    [0.0, 0.0],
                ]
            } else {
                vec![
                    [position[0], 0.0],
                    [position[1], 0.0],
                    [0.0, 0.0],
                    [velocity[0], 0.0],
                    [velocity[1], 0.0],
                    [0.0, 0.0],
                ]
            };
            for (axis, pair) in coefficients.iter().enumerate() {
                double(&mut bytes, record + (2 + axis * 2) * 8, pair[0], endian);
                double(&mut bytes, record + (3 + axis * 2) * 8, pair[1], endian);
            }
            let directory = (last - 4) * 8;
            for (i, value) in [0.0, 10.0, record_words as f64, 1.0]
                .into_iter()
                .enumerate()
            {
                double(&mut bytes, directory + i * 8, value, endian);
            }
            next = last + 1;
        }
        assert!(next <= 640);
        int(&mut bytes, 84, next as i32, endian);
        bytes
    }

    fn with_kernels(
        specs: &[(SpkByteOrder, Vec<Segment>)],
    ) -> SpkKernelSet<Cursor<Vec<u8>>> {
        let mut set = SpkKernelSet::new();
        for (endian, segments) in specs {
            set.load(Cursor::new(kernel(*endian, segments))).unwrap();
        }
        set
    }

    fn near(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn chains_span_two_kernels_in_mixed_endian_and_mixed_types() {
        let old = vec![
            seg(399, 3, 2, [10.0, 0.0], [1.0, 0.0]),
            seg(3, 0, 2, [100.0, 20.0], [3.0, 1.0]),
            seg(4, 0, 3, [-200.0, 50.0], [-2.0, 0.5]),
        ];
        let new = vec![seg(301, 399, 3, [2.0, 4.0], [-0.2, 0.1])];
        let mut set = with_kernels(&[
            (SpkByteOrder::LittleEndian, old),
            (SpkByteOrder::BigEndian, new),
        ]);
        assert_eq!(set.kernel_count(), 2);
        assert_eq!(set.direct_state(301, 5.0).unwrap().center_id, 399);
        let moon_mars = set.relative_state(301, 4, 5.0).unwrap();
        assert_eq!(moon_mars.common_ancestor_id, 0);
        assert_eq!(moon_mars.position_km, [312.0, -26.0, 0.0]);
        near(moon_mars.velocity_km_per_second[0], 5.8);
        near(moon_mars.velocity_km_per_second[1], 0.6);
        let reverse = set.relative_state(4, 301, 5.0).unwrap();
        assert_eq!(reverse.position_km, [-312.0, 26.0, 0.0]);
        near(reverse.velocity_km_per_second[0], -5.8);
    }

    #[test]
    fn newer_file_overrides_segment_center_and_entire_chain() {
        let old = vec![
            seg(399, 3, 2, [10.0, 0.0], [1.0, 0.0]),
            seg(3, 0, 2, [100.0, 0.0], [3.0, 0.0]),
            seg(4, 0, 2, [-200.0, 0.0], [-2.0, 0.0]),
        ];
        let new = vec![seg(399, 4, 3, [7.0, 0.0], [0.5, 0.0])];
        let mut set = with_kernels(&[
            (SpkByteOrder::LittleEndian, old),
            (SpkByteOrder::LittleEndian, new),
        ]);
        let selected = set.direct_state(399, 5.0).unwrap();
        assert_eq!(selected.center_id, 4);
        assert_eq!(selected.position_km[0], 7.0);
        assert_eq!(set.relative_state(399, 0, 5.0).unwrap().position_km[0], -193.0);
        assert_eq!(set.relative_state(399, 4, 5.0).unwrap().position_km[0], 7.0);
        set.unload_last().unwrap();
        assert_eq!(set.kernel_count(), 1);
        let restored = set.relative_state(399, 0, 5.0).unwrap();
        assert_eq!(restored.position_km[0], 110.0);
        assert_eq!(restored.velocity_km_per_second[0], 4.0);
    }

    #[test]
    fn earlier_kernel_is_eligible_only_when_newer_has_no_time_coverage() {
        let old = vec![seg(301, 399, 2, [2.0, 4.0], [0.0, 0.0])];
        let mut later = seg(301, 4, 2, [50.0, 0.0], [0.0, 0.0]);
        later.coverage_start = 6.0;
        let mut set = with_kernels(&[
            (SpkByteOrder::BigEndian, old),
            (SpkByteOrder::LittleEndian, vec![later]),
        ]);
        let at_five = set.direct_state(301, 5.0).unwrap();
        assert_eq!((at_five.center_id, at_five.position_km[0]), (399, 2.0));
        assert_eq!(set.relative_state(301, 399, 5.0).unwrap().position_km[0], 2.0);
        let at_six = set.direct_state(301, 6.0).unwrap();
        assert_eq!(at_six.center_id, 4);
        assert_eq!(at_six.position_km[0], 50.0);
        assert!(matches!(
            set.relative_state(301, 399, 6.0),
            Err(SpkRelativeError::Disconnected { .. })
        ));
    }

    #[test]
    fn unsupported_newer_segment_blocks_older_valid_data() {
        let valid = vec![seg(301, 399, 2, [1.0, 0.0], [0.0, 0.0])];
        for (frame, kind) in [(17, 2), (1, 9)] {
            let mut bad = seg(301, 399, kind, [10.0, 0.0], [0.0, 0.0]);
            bad.frame = frame;
            let mut set = with_kernels(&[
                (SpkByteOrder::LittleEndian, valid.clone()),
                (SpkByteOrder::BigEndian, vec![bad]),
            ]);
            assert!(matches!(
                set.relative_state(301, 399, 5.0),
                Err(SpkRelativeError::Segment(
                    SpkStateError::UnsupportedFrame(17) | SpkStateError::UnsupportedType(9)
                ))
            ));
        }
    }

    #[test]
    fn corrupt_newer_record_is_not_replaced_by_older_state() {
        let old = kernel(
            SpkByteOrder::LittleEndian,
            &[seg(301, 399, 2, [1.0, 0.0], [0.0, 0.0])],
        );
        let mut corrupt = kernel(
            SpkByteOrder::BigEndian,
            &[seg(301, 399, 2, [10.0, 0.0], [0.0, 0.0])],
        );
        // Footer RSIZE occupies directory word 3 after the 8-word Type-2 record.
        let footer_rsize = (INITIAL_WORD + 8 + 1) * 8;
        double(&mut corrupt, footer_rsize, 7.0, SpkByteOrder::BigEndian);
        let mut set = SpkKernelSet::new();
        set.load(Cursor::new(old)).unwrap();
        set.load(Cursor::new(corrupt)).unwrap();
        assert!(matches!(
            set.direct_state(301, 5.0),
            Err(SpkStateError::InvalidSegment(_))
        ));
    }

    #[test]
    fn cycles_created_across_files_are_rejected() {
        let mut set = with_kernels(&[
            (
                SpkByteOrder::LittleEndian,
                vec![
                    seg(301, 399, 2, [1.0, 0.0], [0.0, 0.0]),
                    seg(399, 3, 2, [2.0, 0.0], [0.0, 0.0]),
                ],
            ),
            (
                SpkByteOrder::BigEndian,
                vec![seg(3, 399, 3, [3.0, 0.0], [0.0, 0.0])],
            ),
        ]);
        assert!(matches!(
            set.relative_state(301, 0, 5.0),
            Err(SpkRelativeError::Cycle { body_id: 399 })
        ));
    }

    #[test]
    fn last_covering_segment_within_newest_file_wins() {
        let earlier = vec![seg(301, 399, 2, [1.0, 0.0], [0.0, 0.0])];
        let newer = vec![
            seg(301, 399, 2, [2.0, 0.0], [0.0, 0.0]),
            seg(301, 4, 3, [3.0, 0.0], [0.5, 0.0]),
        ];
        let mut set = with_kernels(&[
            (SpkByteOrder::LittleEndian, earlier),
            (SpkByteOrder::BigEndian, newer),
        ]);
        let selected = set.direct_state(301, 5.0).unwrap();
        assert_eq!((selected.center_id, selected.position_km[0]), (4, 3.0));
        assert_eq!(selected.velocity_km_per_second[0], 0.5);
    }

    #[test]
    fn invalid_load_keeps_existing_set_and_empty_has_explicit_errors() {
        let mut set: SpkKernelSet<Cursor<Vec<u8>>> = SpkKernelSet::new();
        assert!(set.is_empty());
        assert!(matches!(
            set.direct_state(301, 5.0),
            Err(SpkStateError::MissingSegment { target_id: 301 })
        ));
        let invalid = Cursor::new(vec![0u8; 1024]);
        assert!(set.load(invalid).is_err());
        assert!(set.is_empty());
        set.load(Cursor::new(kernel(
            SpkByteOrder::LittleEndian,
            &[seg(301, 399, 3, [2.0, 1.0], [0.0, 0.0])],
        )))
        .unwrap();
        assert_eq!(set.kernel_count(), 1);
        assert_eq!(set.relative_state(399, 399, 5.0).unwrap().position_km, [0.0; 3]);
        assert!(matches!(
            set.relative_state(301, 399, f64::INFINITY),
            Err(SpkRelativeError::InvalidEpoch)
        ));
        assert!(set.unload_last().is_some());
        assert!(set.unload_last().is_none());
    }
}
