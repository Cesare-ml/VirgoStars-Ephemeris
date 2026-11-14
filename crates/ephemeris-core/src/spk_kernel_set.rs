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
use crate::spk_index::{inspect_spk, SpkIndex, SpkIndexError, SpkSegmentDescriptor};
use crate::spk_relative_state::{
    resolve_relative, SpkRelativeError, SpkRelativeState, SpkStateResolver,
};

/// One selected SPK descriptor and the outcome of evaluating it at one ET.
///
/// This distinguishes *declared* coverage from a genuinely evaluated direct
/// geometric state at exactly one TDB/ET epoch. The selected descriptor is
/// never replaced by a lower-priority candidate when its frame, type, records
/// or data stream are invalid. A successful sample does not certify the
/// entire coverage interval or any multi-link target/center relationship.
///
/// Modification (2026-11-14): retain selection provenance when evaluation
/// fails, without suppressing the underlying SPK error.
#[derive(Debug)]
pub struct SpkDirectStateProbe {
    /// Zero-based kernel load index; later loads have higher priority.
    pub kernel_index: usize,
    /// The actual winning DAF descriptor, including its target and center.
    pub descriptor: SpkSegmentDescriptor,
    /// Evaluated state in frame 1 (J2000/ICRF), km and km/s, or its error.
    pub evaluation: Result<SpkDirectState, SpkStateError>,
}

/// Failure while indexing one input in an atomic SPK kernel load.
///
/// `input_index` is zero-based within the attempted batch, not within the
/// previously loaded stack. Earlier successfully indexed batch inputs are
/// deliberately not made visible on failure.
#[derive(Debug)]
pub struct SpkKernelLoadError {
    pub input_index: usize,
    pub error: SpkIndexError,
}

impl std::fmt::Display for SpkKernelLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SPK batch input {} failed indexing: {}",
            self.input_index, self.error
        )
    }
}

impl std::error::Error for SpkKernelLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

// Modification (2026-11-14): stage all input indexes before committing an
// ordered multi-kernel update; reject partial updates with an indexed error.

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

    /// Atomically add multiple caller-supplied kernels in their given load order.
    ///
    /// Every input is indexed before any of them is made visible. If an index
    /// fails, the existing stack (including source cursors) is unchanged and
    /// no input from this batch remains loaded. The zero-based failing input
    /// index is returned alongside the original `SpkIndexError`.
    ///
    /// Like `load`, this only verifies DAF/SPK indexes; coefficient records
    /// may fail on later state evaluation. Empty batches succeed unchanged.
    /// The attempted input readers are consumed, including on failure.
    ///
    /// Geometric states are still ET/TDB seconds past J2000, frame 1
    /// J2000/ICRF, km and km/TDB-second. No network or kernel fallback.
    pub fn load_batch_atomic(
        &mut self,
        sources: impl IntoIterator<Item = R>,
    ) -> Result<(), SpkKernelLoadError> {
        let mut staged = Vec::new();
        for (input_index, mut source) in sources.into_iter().enumerate() {
            let index = inspect_spk(&mut source)
                .map_err(|error| SpkKernelLoadError { input_index, error })?;
            staged.push(LoadedSpk { source, index });
        }
        self.loaded.extend(staged);
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

    /// Inspect which SPK segment wins for a target at a finite TDB/ET epoch.
    ///
    /// Returns its **zero-based kernel load index** (0 is the oldest loaded
    /// file) and a copy of its declared DAF descriptor. Higher-priority
    /// files and later segments within each file win, *without* prefiltering
    /// unsupported frames/types or inspecting coefficient bytes.
    ///
    /// This is metadata-only diagnostics, not proof that a state can be
    /// evaluated or that a JPL vector is accurate. `None` means no segment
    /// covers the target/epoch; nonfinite ET is always an error.
    ///
    /// Modification (2026-11-14): make the effective SPK source traceable
    /// without evaluating any ephemeris state.
    pub fn selected_segment(
        &self,
        target_id: i32,
        et: f64,
    ) -> Result<Option<(usize, SpkSegmentDescriptor)>, SpkStateError> {
        if !et.is_finite() {
            return Err(SpkStateError::InvalidEpoch);
        }
        Ok(self
            .loaded
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, entry)| {
                select_covering_segment(&entry.index, target_id, et).map(|segment| (index, segment))
            }))
    }

    /// List every declared descriptor for one target in kernel load order.
    ///
    /// Results are ordered from oldest to newest loaded file and, within
    /// each file, in the original DAF summary order. Duplicate/overlapping
    /// descriptors are deliberately retained so callers can diagnose
    /// precedence; `selected_segment` still determines the winner at ET.
    ///
    /// This metadata-only inventory does not filter by time coverage,
    /// supported frame/type or readable coefficients. An entry is not proof
    /// of a computable position or of scientific accuracy. No kernel bytes
    /// are read after the initial `load`.
    pub fn declared_segments_for_target(
        &self,
        target_id: i32,
    ) -> Vec<(usize, SpkSegmentDescriptor)> {
        self.loaded
            .iter()
            .enumerate()
            .flat_map(|(kernel_index, entry)| {
                entry
                    .index
                    .segments
                    .iter()
                    .copied()
                    .filter_map(move |segment| {
                        (segment.target_id == target_id).then_some((kernel_index, segment))
                    })
            })
            .collect()
    }

    /// Sorted, distinct NAIF target IDs declared across all loaded kernels.
    ///
    /// This is a read-only metadata inventory: duplicates across files or
    /// segments are removed, but targets in unsupported frames/types and
    /// completely shadowed segments are retained. Center-only IDs are not
    /// included unless they also occur as a segment target. Signed NAIF IDs,
    /// including negative IDs, are preserved. An empty kernel set returns an
    /// empty vector. This does not establish current-epoch coverage or that a
    /// geometric state can be evaluated for any listed target.
    ///
    /// Modification (2026-11-14): expose target discovery for offline tools
    /// without a separate DAF reader or a second astronomy implementation.
    pub fn declared_target_ids(&self) -> Vec<i32> {
        let mut targets: Vec<i32> = self
            .loaded
            .iter()
            .flat_map(|entry| entry.index.segments.iter().map(|segment| segment.target_id))
            .collect();
        targets.sort_unstable();
        targets.dedup();
        targets
    }

    /// Sorted, unique TDB/ET coverage endpoints declared for one target.
    ///
    /// These boundaries are useful when planning discontinuity/priority
    /// diagnostics across loaded kernels, including overlapping or masked
    /// segments. They do **not** imply contiguous coverage or that a valid
    /// state exists between endpoints: selected segments may be unsupported
    /// or corrupt, and endpoints are inclusive. Use `selected_segment` at
    /// the actual epoch to determine the winner.
    ///
    /// Epochs are TDB seconds past J2000, not Julian days. The DAF index
    /// validation guarantees finite endpoints. Positive and negative zero
    /// are normalized into a single +0.0 boundary.
    pub fn declared_coverage_boundaries_for_target(&self, target_id: i32) -> Vec<f64> {
        let mut boundaries = self
            .declared_segments_for_target(target_id)
            .into_iter()
            .flat_map(|(_, segment)| [segment.start_tdb_seconds, segment.end_tdb_seconds])
            .map(|epoch| if epoch == 0.0 { 0.0 } else { epoch })
            .collect::<Vec<_>>();
        boundaries.sort_by(f64::total_cmp);
        boundaries.dedup();
        boundaries
    }

    /// Sorted union of inclusive, declared TDB/ET coverage spans for a target.
    ///
    /// Overlapping intervals and intervals sharing an endpoint are coalesced.
    /// Disjoint intervals remain separate even when their endpoints differ
    /// by only one representable `f64` step: this is not a tolerance-based
    /// estimate. Negative and positive zero are both returned as +0.0.
    /// SPK index validation guarantees finite, ordered descriptor endpoints.
    ///
    /// This is metadata-only and retains unsupported or shadowed segments.
    /// It cannot certify computable coverage, supported frames/types, valid
    /// coefficients or center-chain connectivity; use `selected_segment`
    /// and actual state evaluation for those checks. No kernel I/O is needed
    /// after `load`. All times are TDB seconds past J2000.
    ///
    /// Modification (2026-11-14): report time gaps without merging across
    /// uncovered epochs or silently filtering higher-priority declarations.
    pub fn declared_coverage_intervals_for_target(&self, target_id: i32) -> Vec<(f64, f64)> {
        let mut spans: Vec<(f64, f64)> = self
            .declared_segments_for_target(target_id)
            .into_iter()
            .map(|(_, segment)| {
                let start = segment.start_tdb_seconds;
                let end = segment.end_tdb_seconds;
                (
                    if start == 0.0 { 0.0 } else { start },
                    if end == 0.0 { 0.0 } else { end },
                )
            })
            .collect();
        spans.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));

        let mut merged: Vec<(f64, f64)> = Vec::with_capacity(spans.len());
        for (start, end) in spans {
            if let Some((_, last_end)) = merged.last_mut() {
                if start <= *last_end {
                    if end > *last_end {
                        *last_end = end;
                    }
                    continue;
                }
            }
            merged.push((start, end));
        }
        merged
    }

    /// Inspect and evaluate the winning direct SPK segment at one epoch.
    ///
    /// An invalid/nonfinite TDB/ET instant is an outer error. When no segment
    /// covers the target it returns `Ok(None)`; when a segment is declared it
    /// returns its load index and descriptor together with the *actual*
    /// interpolation result (or its exact error). An unsupported or damaged
    /// highest-priority segment is never bypassed for a lower-priority one.
    ///
    /// Unlike metadata-only inventory APIs, this reads coefficient records
    /// and may encounter I/O errors. A successful direct state proves only
    /// that this particular target-center sample was evaluated, **not**
    /// whole-interval coverage, a resolved center chain or JPL numerical
    /// accuracy. Units are TDB seconds past J2000, km and km/s.
    ///
    /// Modification (2026-11-14): pair the selected descriptor with the
    /// same evaluation path used by `direct_state`.
    pub fn probe_direct_state(
        &mut self,
        target_id: i32,
        et: f64,
    ) -> Result<Option<SpkDirectStateProbe>, SpkStateError> {
        let Some((kernel_index, descriptor)) = self.selected_segment(target_id, et)? else {
            return Ok(None);
        };
        let entry = &mut self.loaded[kernel_index];
        let evaluation = direct_spk_state(&mut entry.source, &entry.index, target_id, et);
        Ok(Some(SpkDirectStateProbe {
            kernel_index,
            descriptor,
            evaluation,
        }))
    }

    /// Evaluate every declared target covered at one TDB/ET instant.
    ///
    /// Results are sorted by signed NAIF target ID, with one entry per
    /// distinct target. Targets with no selected descriptor at this instant
    /// are omitted; see `declared_target_ids` for the unfiltered inventory.
    /// Each `SpkDirectStateProbe` retains the selected kernel and the
    /// independent result/error of its actual Type-2/3 data evaluation.
    ///
    /// An invalid ET is an outer error even for an empty kernel set.
    /// Evaluation failures stay inside individual probes: a corrupt or
    /// unsupported high-priority segment is never replaced by older data,
    /// and does not hide independent successes for other targets. This
    /// performs coefficient I/O and does not guarantee a whole interval,
    /// target-center chain connectivity, or real JPL/NAIF numerical parity.
    /// Units: TDB seconds past J2000; geometric frame 1 state km and km/s.
    ///
    /// Modification (2026-11-14): compose existing canonical inventory
    /// and single-target probes without a parallel segment-selection path.
    pub fn probe_direct_states_at(
        &mut self,
        et: f64,
    ) -> Result<Vec<(i32, SpkDirectStateProbe)>, SpkStateError> {
        if !et.is_finite() {
            return Err(SpkStateError::InvalidEpoch);
        }
        let mut results = Vec::new();
        for target_id in self.declared_target_ids() {
            if let Some(probe) = self.probe_direct_state(target_id, et)? {
                results.push((target_id, probe));
            }
        }
        Ok(results)
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
        self.probe_direct_state(target_id, et)?
            .ok_or(SpkStateError::MissingSegment { target_id })?
            .evaluation
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
            select_covering_segment(&entry.index, body, et).map(|segment| segment.center_id)
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

    fn with_kernels(specs: &[(SpkByteOrder, Vec<Segment>)]) -> SpkKernelSet<Cursor<Vec<u8>>> {
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
        assert_eq!(
            set.relative_state(399, 0, 5.0).unwrap().position_km[0],
            -193.0
        );
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
        assert_eq!(
            set.relative_state(301, 399, 5.0).unwrap().position_km[0],
            2.0
        );
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
    fn unrelated_higher_priority_target_is_ignored_for_selected_chain() {
        let old = vec![seg(301, 399, 2, [2.0, 4.0], [0.0, 0.0])];
        let mut unrelated = seg(999, 0, 9, [9.0, 0.0], [0.0, 0.0]);
        unrelated.frame = 17;
        let mut set = with_kernels(&[
            (SpkByteOrder::LittleEndian, old),
            (SpkByteOrder::BigEndian, vec![unrelated]),
        ]);
        // Unsupported data for another body cannot mask a valid Moon link.
        let moon = set.relative_state(301, 399, 5.0).unwrap();
        assert_eq!(moon.position_km, [2.0, 4.0, 0.0]);
        assert!(matches!(
            set.direct_state(999, 5.0),
            Err(SpkStateError::UnsupportedFrame(17))
        ));
    }

    // Modification (2026-11-14): test public kernel-stack rollback and
    // reloading behavior with independently encoded little/big-endian fixtures.
    #[test]
    fn unloading_unsupported_overlay_restores_older_state_and_reloading_masks_it() {
        let older = seg(301, 399, 2, [2.0, -3.0], [0.0, 0.0]);
        let mut unsupported = seg(301, 4, 3, [8.0, 9.0], [1.0, 0.0]);
        unsupported.frame = 17;

        let mut set = with_kernels(&[
            (SpkByteOrder::LittleEndian, vec![older]),
            (SpkByteOrder::BigEndian, vec![unsupported]),
        ]);
        assert_eq!(set.kernel_count(), 2);
        assert!(matches!(
            set.direct_state(301, 5.0),
            Err(SpkStateError::UnsupportedFrame(17))
        ));

        // The bad high-priority link must mask the older file, but unloading
        // that file must restore exactly the original type-2 result.
        let overlay = set.unload_last().expect("newer kernel is loaded");
        assert_eq!(set.kernel_count(), 1);
        let restored = set.direct_state(301, 5.0).unwrap();
        assert_eq!(restored.center_id, 399);
        assert_eq!(restored.position_km, [2.0, -3.0, 0.0]);
        assert_eq!(restored.velocity_km_per_second, [0.0; 3]);
        assert_eq!(
            set.relative_state(301, 399, 5.0).unwrap().position_km,
            [2.0, -3.0, 0.0]
        );

        // The returned reader remains reusable, even if it was previously
        // positioned by failed state evaluation.
        set.load(overlay).unwrap();
        assert_eq!(set.kernel_count(), 2);
        assert!(matches!(
            set.direct_state(301, 5.0),
            Err(SpkStateError::UnsupportedFrame(17))
        ));
        assert!(set.unload_last().is_some());
        assert_eq!(set.direct_state(301, 5.0).unwrap(), restored);
    }

    #[test]
    fn failed_kernel_load_never_changes_existing_selected_state() {
        let mut set = with_kernels(&[
            (
                SpkByteOrder::LittleEndian,
                vec![seg(301, 399, 2, [1.0, 2.0], [0.1, 0.2])],
            ),
            (
                SpkByteOrder::BigEndian,
                vec![seg(301, 399, 3, [3.0, 4.0], [0.3, 0.4])],
            ),
        ]);
        let before = set.direct_state(301, 5.0).unwrap();
        assert_eq!(before.position_km, [3.0, 4.0, 0.0]);
        assert_eq!(before.velocity_km_per_second, [0.3, 0.4, 0.0]);

        // A malformed later file must not replace, clear, or partially
        // reorder the validated kernel set.
        let bad_header = Cursor::new(vec![0u8; 5 * RECORD_BYTES]);
        assert!(set.load(bad_header).is_err());
        assert_eq!(set.kernel_count(), 2);
        assert_eq!(set.direct_state(301, 5.0).unwrap(), before);

        // Unloading after a failed load must still remove the actual newest
        // valid kernel, exposing the earlier type-2 source.
        assert!(set.unload_last().is_some());
        assert_eq!(set.kernel_count(), 1);
        let earlier = set.direct_state(301, 5.0).unwrap();
        assert_eq!(earlier.center_id, 399);
        assert_eq!(earlier.position_km, [1.0, 2.0, 0.0]);
        assert_eq!(earlier.velocity_km_per_second, [0.1, 0.2, 0.0]);
    }

    // Modification (2026-11-14): a later DAF summary/name pair must be
    // allocated before FREE, even if all segment data fits below FREE.
    // A rejected higher-priority file must not evict valid loaded kernels.
    #[test]
    fn corrupt_free_pointer_in_chained_metadata_does_not_replace_loaded_kernel() {
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            let mut set = with_kernels(&[(
                SpkByteOrder::LittleEndian,
                vec![seg(301, 399, 2, [2.0, 0.0], [0.0, 0.0])],
            )]);

            let mut corrupt = kernel(order, &[seg(301, 399, 2, [99.0, 0.0], [0.0, 0.0])]);
            // The original data ends at word 524, with FREE=525. Append
            // summary record 6 and its name record 7 *without* advancing
            // FREE. The metadata link is otherwise consistent.
            corrupt.resize(7 * RECORD_BYTES, 0);
            // Link the new LAST summary from the header and first summary.
            int(&mut corrupt, 80, 6, order);
            double(&mut corrupt, 2 * RECORD_BYTES, 6.0, order);
            double(&mut corrupt, 5 * RECORD_BYTES + 8, 3.0, order);
            // Last summary COUNT=0, allowed when an earlier summary has data.
            assert!(matches!(
                set.load(Cursor::new(corrupt)),
                Err(SpkIndexError::InvalidFile(
                    "DAF free pointer precedes summary or name allocation"
                ))
            ));
            assert_eq!(set.kernel_count(), 1);
            assert_eq!(set.direct_state(301, 5.0).unwrap().position_km[0], 2.0);
            assert_eq!(
                set.relative_state(301, 399, 5.0).unwrap().position_km[0],
                2.0
            );
        }
    }

    // Modification (2026-11-14): a declared winner must remain identifiable
    // even if its actual frame or coefficients make evaluation impossible.
    #[test]
    fn direct_probe_preserves_selected_source_and_evaluation_errors() {
        let older = seg(301, 399, 2, [2.0, -1.0], [0.25, -0.5]);
        let mut unsupported = seg(301, 4, 3, [40.0, 50.0], [0.0, 0.0]);
        unsupported.frame = 17;
        let mut set = with_kernels(&[
            (SpkByteOrder::LittleEndian, vec![older]),
            (SpkByteOrder::BigEndian, vec![unsupported]),
        ]);

        let probe = set.probe_direct_state(301, 5.0).unwrap().unwrap();
        assert_eq!(probe.kernel_index, 1);
        assert_eq!(probe.descriptor.target_id, 301);
        assert_eq!(probe.descriptor.center_id, 4);
        assert_eq!(probe.descriptor.frame_id, 17);
        assert!(matches!(
            probe.evaluation,
            Err(SpkStateError::UnsupportedFrame(17))
        ));
        assert!(matches!(
            set.direct_state(301, 5.0),
            Err(SpkStateError::UnsupportedFrame(17))
        ));

        set.unload_last().unwrap();
        let restored = set.probe_direct_state(301, 5.0).unwrap().unwrap();
        assert_eq!(restored.kernel_index, 0);
        assert_eq!(restored.descriptor.center_id, 399);
        assert_eq!(
            restored.evaluation.unwrap(),
            set.direct_state(301, 5.0).unwrap()
        );
        assert!(set.probe_direct_state(301, 11.0).unwrap().is_none());
        assert!(set.probe_direct_state(999, 5.0).unwrap().is_none());
        assert!(matches!(
            set.probe_direct_state(301, f64::NAN),
            Err(SpkStateError::InvalidEpoch)
        ));
        assert!(matches!(
            set.direct_state(301, f64::NAN),
            Err(SpkStateError::InvalidEpoch)
        ));

        let empty: SpkKernelSet<Cursor<Vec<u8>>> = SpkKernelSet::new();
        let mut empty = empty;
        assert!(empty.probe_direct_state(301, 5.0).unwrap().is_none());
        assert!(matches!(
            empty.direct_state(301, 5.0),
            Err(SpkStateError::MissingSegment { target_id: 301 })
        ));
    }

    #[test]
    fn direct_probe_keeps_corrupt_record_error_with_winning_descriptor() {
        let mut set = with_kernels(&[(
            SpkByteOrder::LittleEndian,
            vec![seg(301, 399, 2, [2.0, 0.0], [0.0, 0.0])],
        )]);
        let mut corrupt = kernel(
            SpkByteOrder::BigEndian,
            &[seg(301, 4, 2, [9.0, 0.0], [0.0, 0.0])],
        );
        // Preserve the valid indexed descriptor while corrupting the actual
        // record footer RSIZE (third word): the loader checks DAF metadata,
        // not coefficients.
        let record_words = 2 + 3 * 2;
        let last_address = INITIAL_WORD + record_words + 3;
        double(
            &mut corrupt,
            (last_address - 4) * 8 + 16,
            0.0,
            SpkByteOrder::BigEndian,
        );
        set.load(Cursor::new(corrupt)).unwrap();

        let probe = set.probe_direct_state(301, 5.0).unwrap().unwrap();
        assert_eq!(probe.kernel_index, 1);
        assert_eq!(probe.descriptor.center_id, 4);
        assert!(matches!(
            probe.evaluation,
            Err(SpkStateError::InvalidSegment("invalid SPK record size"))
        ));
        assert!(matches!(
            set.direct_state(301, 5.0),
            Err(SpkStateError::InvalidSegment("invalid SPK record size"))
        ));
        set.unload_last().unwrap();
        assert_eq!(
            set.probe_direct_state(301, 5.0)
                .unwrap()
                .unwrap()
                .evaluation
                .unwrap(),
            set.direct_state(301, 5.0).unwrap()
        );
    }

    // Modification (2026-11-14): one evaluation failure must not hide
    // valid states of other targets in a whole-kernel diagnostic snapshot.
    #[test]
    fn direct_probe_snapshot_is_sorted_and_preserves_individual_failures() {
        let mut later_coverage = seg(777, 0, 2, [77.0, 0.0], [0.0, 0.0]);
        later_coverage.coverage_start = 7.0;
        let older = vec![
            seg(301, 399, 2, [2.0, -1.0], [0.25, -0.5]),
            seg(-82, 0, 3, [8.0, 1.0], [0.0, 0.0]),
            later_coverage,
        ];
        let mut unsupported = seg(301, 4, 2, [40.0, 50.0], [0.0, 0.0]);
        unsupported.frame = 17;
        let newer = vec![seg(399, 0, 2, [3.0, 4.0], [0.0, 0.0]), unsupported];
        let mut set = with_kernels(&[
            (SpkByteOrder::LittleEndian, older),
            (SpkByteOrder::BigEndian, newer),
        ]);

        let probes = set.probe_direct_states_at(5.0).unwrap();
        assert_eq!(
            probes.iter().map(|(target, _)| *target).collect::<Vec<_>>(),
            vec![-82, 301, 399]
        );
        assert_eq!(probes[0].1.kernel_index, 0);
        assert_eq!(probes[0].1.descriptor.center_id, 0);
        assert_eq!(
            probes[0].1.evaluation.as_ref().unwrap().position_km,
            [8.0, 1.0, 0.0]
        );
        assert_eq!(probes[1].1.kernel_index, 1);
        assert_eq!(probes[1].1.descriptor.center_id, 4);
        assert!(matches!(
            &probes[1].1.evaluation,
            Err(SpkStateError::UnsupportedFrame(17))
        ));
        assert_eq!(probes[2].1.kernel_index, 1);
        assert_eq!(
            probes[2].1.evaluation.as_ref().unwrap(),
            &set.direct_state(399, 5.0).unwrap()
        );
        assert!(matches!(
            set.direct_state(301, 5.0),
            Err(SpkStateError::UnsupportedFrame(17))
        ));

        assert_eq!(
            set.probe_direct_states_at(7.0)
                .unwrap()
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>(),
            vec![-82, 301, 399, 777]
        );
        assert!(set.probe_direct_states_at(11.0).unwrap().is_empty());
        for epoch in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                set.probe_direct_states_at(epoch),
                Err(SpkStateError::InvalidEpoch)
            ));
        }

        set.unload_last().unwrap();
        let restored = set.probe_direct_states_at(5.0).unwrap();
        assert_eq!(
            restored.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![-82, 301]
        );
        assert_eq!(restored[1].1.kernel_index, 0);
        assert_eq!(restored[1].1.descriptor.center_id, 399);
        assert_eq!(
            restored[1].1.evaluation.as_ref().unwrap(),
            &set.direct_state(301, 5.0).unwrap()
        );

        let mut empty: SpkKernelSet<Cursor<Vec<u8>>> = SpkKernelSet::new();
        assert!(empty.probe_direct_states_at(5.0).unwrap().is_empty());
        assert!(matches!(
            empty.probe_direct_states_at(f64::NAN),
            Err(SpkStateError::InvalidEpoch)
        ));
    }

    // Modification (2026-11-14): target inventory is sorted, unique, signed
    // and metadata-only even for unsupported or shadowed declarations.
    #[test]
    fn declared_target_ids_include_shadowed_and_unsupported_targets_without_centers() {
        let mut set: SpkKernelSet<Cursor<Vec<u8>>> = SpkKernelSet::new();
        assert!(set.declared_target_ids().is_empty());

        set.load(Cursor::new(kernel(
            SpkByteOrder::LittleEndian,
            &[
                seg(301, 72, 2, [1.0, 0.0], [0.0, 0.0]),
                seg(-82, 72, 3, [2.0, 0.0], [0.0, 0.0]),
                seg(301, 72, 2, [3.0, 0.0], [0.0, 0.0]),
            ],
        )))
        .unwrap();
        assert_eq!(set.declared_target_ids(), vec![-82, 301]);

        let mut unsupported = seg(399, 12, 9, [5.0, 0.0], [0.0, 0.0]);
        unsupported.frame = 17;
        set.load(Cursor::new(kernel(
            SpkByteOrder::BigEndian,
            &[unsupported, seg(301, 0, 2, [4.0, 0.0], [0.0, 0.0])],
        )))
        .unwrap();
        assert_eq!(set.declared_target_ids(), vec![-82, 301, 399]);
        assert!(!set.declared_target_ids().contains(&72));
        assert!(!set.declared_target_ids().contains(&12));

        // The highest-priority segment for ID 399 is unsupported, but
        // its declared target must still be discoverable without evaluation.
        assert!(matches!(
            set.direct_state(399, 5.0),
            Err(SpkStateError::UnsupportedFrame(17))
        ));
        assert_eq!(set.declared_target_ids(), vec![-82, 301, 399]);

        set.unload_last().unwrap();
        assert_eq!(set.declared_target_ids(), vec![-82, 301]);
        set.unload_last().unwrap();
        assert!(set.declared_target_ids().is_empty());
    }

    // Modification (2026-11-14): interval union is based on declared
    // inclusive SPK coverage, not whether a segment is executable.
    #[test]
    fn declared_coverage_intervals_preserve_gaps_and_union_mixed_endian_overlays() {
        let make = |target: i32, start: f64, end: f64, spk_type: i32| {
            let mut segment = seg(target, 0, spk_type, [0.0, 0.0], [0.0, 0.0]);
            segment.coverage_start = start;
            segment.coverage_end = end;
            segment
        };
        let mut set: SpkKernelSet<Cursor<Vec<u8>>> = SpkKernelSet::new();
        assert!(set.declared_coverage_intervals_for_target(301).is_empty());
        set.load(Cursor::new(kernel(
            SpkByteOrder::LittleEndian,
            &[
                make(301, -10.0, -7.0, 2),
                make(301, -5.0, -0.0, 2),
                make(301, 3.0, 3.0, 3),
                make(301, 6.0, 9.0, 2),
                make(399, -100.0, 100.0, 2),
            ],
        )))
        .unwrap();
        assert_eq!(
            set.declared_coverage_intervals_for_target(301),
            vec![(-10.0, -7.0), (-5.0, 0.0), (3.0, 3.0), (6.0, 9.0)]
        );
        assert_eq!(
            set.declared_coverage_intervals_for_target(301)[1]
                .1
                .to_bits(),
            0.0_f64.to_bits(),
        );

        // Newer segments bridge two earlier gaps and touch existing spans.
        // The last segment is unsupported for evaluation but still declared.
        set.load(Cursor::new(kernel(
            SpkByteOrder::BigEndian,
            &[
                make(301, -8.0, -4.0, 3),
                make(301, -0.0, 3.0, 2),
                make(301, 9.0, 12.0, 9),
            ],
        )))
        .unwrap();
        assert_eq!(
            set.declared_coverage_intervals_for_target(301),
            vec![(-10.0, 3.0), (6.0, 12.0)]
        );
        assert!(set.selected_segment(301, 5.0).unwrap().is_none());
        assert_eq!(
            set.declared_coverage_intervals_for_target(399),
            vec![(-100.0, 100.0)]
        );
        assert!(set.declared_coverage_intervals_for_target(999).is_empty());

        set.unload_last().unwrap();
        assert_eq!(
            set.declared_coverage_intervals_for_target(301),
            vec![(-10.0, -7.0), (-5.0, 0.0), (3.0, 3.0), (6.0, 9.0)]
        );
        set.unload_last().unwrap();
        assert!(set.declared_coverage_intervals_for_target(301).is_empty());
    }

    #[test]
    fn declared_coverage_intervals_do_not_bridge_one_ulp_gaps() {
        let after_one = f64::from_bits(1.0_f64.to_bits() + 1);
        let mut first = seg(301, 0, 2, [0.0, 0.0], [0.0, 0.0]);
        first.coverage_start = 0.0;
        first.coverage_end = 1.0;
        let mut second = seg(301, 0, 3, [0.0, 0.0], [0.0, 0.0]);
        second.coverage_start = after_one;
        second.coverage_end = 2.0;
        let set = with_kernels(&[(SpkByteOrder::BigEndian, vec![second, first])]);
        assert_eq!(
            set.declared_coverage_intervals_for_target(301),
            vec![(0.0, 1.0), (after_one, 2.0)]
        );
    }

    // Modification (2026-11-14): selection diagnostics must report the exact
    // same winning descriptor as the actual geometric state path.
    // Modification (2026-11-14): provide a lossless, read-only segment
    // inventory so diagnostic tooling can explain overlapping-kernel priority.
    #[test]
    fn declared_inventory_retains_duplicate_targets_and_unsupported_overrides() {
        let old = vec![
            seg(399, 3, 2, [10.0, 0.0], [0.0, 0.0]),
            seg(301, 399, 2, [1.0, 0.0], [0.0, 0.0]),
            seg(399, 0, 3, [20.0, 0.0], [0.0, 0.0]),
        ];
        let mut unsupported = seg(399, 4, 9, [30.0, 0.0], [0.0, 0.0]);
        unsupported.coverage_start = 6.0;
        let mut set = with_kernels(&[
            (SpkByteOrder::LittleEndian, old),
            (SpkByteOrder::BigEndian, vec![unsupported]),
        ]);

        let inventory = set.declared_segments_for_target(399);
        assert_eq!(
            inventory
                .iter()
                .map(|(index, descriptor)| (
                    *index,
                    descriptor.center_id,
                    descriptor.spk_type,
                    descriptor.start_tdb_seconds,
                    descriptor.end_tdb_seconds,
                ))
                .collect::<Vec<_>>(),
            vec![
                (0, 3, 2, 0.0, 10.0),
                (0, 0, 3, 0.0, 10.0),
                (1, 4, 9, 6.0, 10.0),
            ]
        );
        assert!(set.declared_segments_for_target(999).is_empty());
        assert_eq!(set.declared_segments_for_target(301).len(), 1);

        let before = set.selected_segment(399, 5.0).unwrap().unwrap();
        assert_eq!((before.0, before.1.center_id), (0, 0));
        let after = set.selected_segment(399, 7.0).unwrap().unwrap();
        assert_eq!((after.0, after.1.center_id), (1, 4));
        assert!(matches!(
            set.direct_state(399, 7.0),
            Err(SpkStateError::UnsupportedType(9))
        ));
        assert_eq!(set.declared_segments_for_target(399), inventory);

        set.unload_last().unwrap();
        assert_eq!(
            set.declared_segments_for_target(399),
            inventory[..2].to_vec()
        );
    }

    // Modification (2026-11-14): preserve all declared coverage boundaries
    // without making scientific claims about which segments are computable.
    #[test]
    fn declared_coverage_boundaries_are_sorted_unique_and_restore_after_unload() {
        let empty: SpkKernelSet<Cursor<Vec<u8>>> = SpkKernelSet::new();
        assert!(empty
            .declared_coverage_boundaries_for_target(301)
            .is_empty());

        let mut earliest = seg(301, 399, 2, [1.0, 0.0], [0.0, 0.0]);
        earliest.coverage_start = -0.0;
        let mut interior = seg(301, 4, 3, [2.0, 0.0], [0.0, 0.0]);
        interior.coverage_start = 3.0;
        interior.coverage_end = 7.0;
        let duplicate_span = seg(301, 3, 2, [3.0, 0.0], [0.0, 0.0]);
        let unrelated = seg(399, 0, 2, [4.0, 0.0], [0.0, 0.0]);
        let mut unsupported = seg(301, 0, 9, [5.0, 0.0], [0.0, 0.0]);
        unsupported.coverage_start = 6.0;
        unsupported.coverage_end = 8.0;
        let mut set = with_kernels(&[
            (
                SpkByteOrder::LittleEndian,
                vec![earliest, interior, duplicate_span, unrelated],
            ),
            (SpkByteOrder::BigEndian, vec![unsupported]),
        ]);

        let expected = vec![0.0, 3.0, 6.0, 7.0, 8.0, 10.0];
        assert_eq!(set.declared_coverage_boundaries_for_target(301), expected);
        assert_eq!(
            set.declared_coverage_boundaries_for_target(301)[0].to_bits(),
            0.0_f64.to_bits(),
        );
        assert!(set.declared_coverage_boundaries_for_target(999).is_empty());
        assert_eq!(
            set.declared_coverage_boundaries_for_target(399),
            vec![0.0, 10.0]
        );

        // Both start and end of an overlay are inclusive. The next
        // representable epoch after its end is no longer in that overlay.
        assert_eq!(set.selected_segment(301, 8.0).unwrap().unwrap().0, 1);
        let after_end = f64::from_bits(8.0_f64.to_bits() + 1);
        assert_eq!(set.selected_segment(301, after_end).unwrap().unwrap().0, 0);
        assert!(matches!(
            set.direct_state(301, 8.0),
            Err(SpkStateError::UnsupportedType(9))
        ));

        set.unload_last().unwrap();
        assert_eq!(
            set.declared_coverage_boundaries_for_target(301),
            vec![0.0, 3.0, 7.0, 10.0],
        );
    }

    #[test]
    fn selected_segment_reports_latest_file_and_last_covering_descriptor() {
        let earlier = vec![seg(301, 399, 2, [1.0, 0.0], [0.0, 0.0])];
        let later = vec![
            seg(301, 399, 2, [2.0, 0.0], [0.0, 0.0]),
            seg(301, 4, 3, [3.0, 0.0], [0.5, 0.0]),
        ];
        let mut set = with_kernels(&[
            (SpkByteOrder::LittleEndian, earlier),
            (SpkByteOrder::BigEndian, later),
        ]);
        let (kernel_index, segment) = set.selected_segment(301, 5.0).unwrap().unwrap();
        assert_eq!(kernel_index, 1);
        assert_eq!((segment.target_id, segment.center_id), (301, 4));
        assert_eq!((segment.frame_id, segment.spk_type), (1, 3));
        assert_eq!(
            (segment.start_tdb_seconds, segment.end_tdb_seconds),
            (0.0, 10.0)
        );
        let state = set.direct_state(301, 5.0).unwrap();
        assert_eq!(state.center_id, segment.center_id);
        assert_eq!(state.position_km, [3.0, 0.0, 0.0]);
    }

    #[test]
    fn selected_segment_respects_coverage_boundary_and_unload() {
        let old = vec![seg(301, 399, 2, [1.0, 0.0], [0.0, 0.0])];
        let mut newer = seg(301, 4, 3, [2.0, 0.0], [0.0, 0.0]);
        newer.coverage_start = 6.0;
        let mut set = with_kernels(&[
            (SpkByteOrder::LittleEndian, old),
            (SpkByteOrder::BigEndian, vec![newer]),
        ]);
        assert_eq!(set.selected_segment(301, 5.0).unwrap().unwrap().0, 0);
        assert_eq!(set.selected_segment(301, 6.0).unwrap().unwrap().0, 1);
        assert_eq!(set.selected_segment(301, 10.0).unwrap().unwrap().0, 1);
        assert!(set.selected_segment(301, 11.0).unwrap().is_none());
        assert!(set.selected_segment(999, 5.0).unwrap().is_none());
        assert!(set.unload_last().is_some());
        let (index, descriptor) = set.selected_segment(301, 6.0).unwrap().unwrap();
        assert_eq!(index, 0);
        assert_eq!(descriptor.center_id, 399);
    }

    #[test]
    fn selected_segment_preserves_unsupported_overlay_and_invalid_epoch_errors() {
        let older = seg(301, 399, 2, [1.0, 0.0], [0.0, 0.0]);
        let mut unsupported = seg(301, 4, 2, [3.0, 0.0], [0.0, 0.0]);
        unsupported.frame = 17;
        let mut set = with_kernels(&[
            (SpkByteOrder::LittleEndian, vec![older]),
            (SpkByteOrder::BigEndian, vec![unsupported]),
        ]);
        let (index, descriptor) = set.selected_segment(301, 5.0).unwrap().unwrap();
        assert_eq!(index, 1);
        assert_eq!(descriptor.frame_id, 17);
        assert!(matches!(
            set.direct_state(301, 5.0),
            Err(SpkStateError::UnsupportedFrame(17))
        ));
        for epoch in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                set.selected_segment(301, epoch),
                Err(SpkStateError::InvalidEpoch)
            ));
            assert!(matches!(
                set.direct_state(301, epoch),
                Err(SpkStateError::InvalidEpoch)
            ));
        }
        let empty: SpkKernelSet<Cursor<Vec<u8>>> = SpkKernelSet::new();
        assert!(empty.selected_segment(301, 0.0).unwrap().is_none());
    }

    // Modification (2026-11-14): the one-file path and ordered-kernel set
    // must invoke the same center-chain and Chebyshev semantics at both
    // inclusive record endpoints, not only at an interior epoch.
    #[test]
    fn single_kernel_and_one_element_set_agree_at_record_endpoints() {
        use crate::spk_relative_state::relative_spk_state;

        for byte_order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            for (moon_type, earth_type) in [(2, 3), (3, 2)] {
                let fixture = kernel(
                    byte_order,
                    &[
                        seg(301, 399, moon_type, [3.0, -2.0], [0.2, -0.1]),
                        seg(399, 0, earth_type, [100.0, 10.0], [-0.3, 0.05]),
                    ],
                );
                let mut single_source = Cursor::new(fixture.clone());
                let single_index = inspect_spk(&mut single_source).unwrap();
                let mut set = SpkKernelSet::new();
                set.load(Cursor::new(fixture)).unwrap();
                assert_eq!(set.kernel_count(), 1);

                for et in [0.0, 5.0, 10.0] {
                    for (target, center) in [(301, 399), (399, 301), (301, 0), (0, 301)] {
                        let single = relative_spk_state(
                            &mut single_source,
                            &single_index,
                            target,
                            center,
                            et,
                        )
                        .unwrap();
                        let loaded = set.relative_state(target, center, et).unwrap();
                        assert_eq!(
                            loaded, single,
                            "path mismatch for {byte_order:?}, types {moon_type}/{earth_type}, ET={et}, {target}->{center}"
                        );
                    }
                }
            }
        }
    }

    // Modification (2026-11-14): geometric position and velocity composition
    // must obey target/center transitivity across three separately loaded
    // SPKs, including the inclusive ET coverage endpoints.
    #[test]
    fn mixed_kernel_center_chains_preserve_transitivity_and_reverse_sign() {
        for (moon_kind, earth_kind, emb_kind) in [(2, 3, 2), (3, 2, 3)] {
            let mut set = with_kernels(&[
                (
                    SpkByteOrder::LittleEndian,
                    vec![seg(3, 0, emb_kind, [100.0, 20.0], [0.5, -0.25])],
                ),
                (
                    SpkByteOrder::BigEndian,
                    vec![seg(399, 3, earth_kind, [10.0, 4.0], [-0.25, 0.125])],
                ),
                (
                    SpkByteOrder::LittleEndian,
                    vec![seg(301, 399, moon_kind, [2.0, -1.0], [0.125, 0.0])],
                ),
            ]);
            assert_eq!(set.kernel_count(), 3);

            for et in [0.0, 5.0, 10.0] {
                let moon_earth = set.relative_state(301, 399, et).unwrap();
                let earth_emb = set.relative_state(399, 3, et).unwrap();
                let emb_ssb = set.relative_state(3, 0, et).unwrap();
                let moon_ssb = set.relative_state(301, 0, et).unwrap();
                let ssb_moon = set.relative_state(0, 301, et).unwrap();
                assert_eq!(moon_ssb.common_ancestor_id, 0);
                assert_eq!(ssb_moon.common_ancestor_id, 0);

                for axis in 0..3 {
                    near(
                        moon_ssb.position_km[axis],
                        moon_earth.position_km[axis]
                            + earth_emb.position_km[axis]
                            + emb_ssb.position_km[axis],
                    );
                    near(
                        moon_ssb.velocity_km_per_second[axis],
                        moon_earth.velocity_km_per_second[axis]
                            + earth_emb.velocity_km_per_second[axis]
                            + emb_ssb.velocity_km_per_second[axis],
                    );
                    near(moon_ssb.position_km[axis], -ssb_moon.position_km[axis]);
                    near(
                        moon_ssb.velocity_km_per_second[axis],
                        -ssb_moon.velocity_km_per_second[axis],
                    );
                }
            }
        }
    }

    // Modification (2026-11-14): metadata diagnostics must not perform
    // hidden I/O after load. Numerical lookups still fail closed if the
    // underlying data stream becomes temporarily unreadable.
    #[test]
    fn metadata_queries_never_reopen_unreadable_loaded_sources() {
        use std::cell::Cell;
        use std::io::{self, Read, Seek, SeekFrom};
        use std::rc::Rc;

        struct GuardedSource {
            cursor: Cursor<Vec<u8>>,
            block_reads: Rc<Cell<bool>>,
        }

        impl Read for GuardedSource {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                if self.block_reads.get() {
                    Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "injected post-index read denial",
                    ))
                } else {
                    self.cursor.read(buffer)
                }
            }
        }

        impl Seek for GuardedSource {
            fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
                self.cursor.seek(position)
            }
        }

        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            for spk_type in [2, 3] {
                let blocked = Rc::new(Cell::new(false));
                let source = GuardedSource {
                    cursor: Cursor::new(kernel(
                        order,
                        &[seg(301, 399, spk_type, [17.0, 2.0], [0.5, -0.25])],
                    )),
                    block_reads: Rc::clone(&blocked),
                };
                let mut set = SpkKernelSet::new();
                set.load(source).unwrap();
                assert_eq!(set.kernel_count(), 1);
                blocked.set(true);

                let inventory = set.declared_segments_for_target(301);
                assert_eq!(inventory.len(), 1);
                assert_eq!(inventory[0].0, 0);
                assert_eq!(inventory[0].1.center_id, 399);
                assert_eq!(
                    set.declared_coverage_boundaries_for_target(301),
                    vec![0.0, 10.0]
                );
                assert!(set.declared_segments_for_target(399).is_empty());
                assert_eq!(set.selected_segment(301, 5.0).unwrap(), Some(inventory[0]));
                assert!(set.selected_segment(301, 11.0).unwrap().is_none());

                // Metadata was available without a read, but the same
                // source must not produce a plausible geometric vector.
                assert!(matches!(
                    set.direct_state(301, 5.0),
                    Err(SpkStateError::Io(ref error))
                        if error.kind() == io::ErrorKind::PermissionDenied
                ));
                assert!(matches!(
                    set.relative_state(301, 399, 5.0),
                    Err(SpkRelativeError::Segment(SpkStateError::Io(ref error)))
                        if error.kind() == io::ErrorKind::PermissionDenied
                ));

                blocked.set(false);
                let recovered = set.relative_state(301, 399, 5.0).unwrap();
                assert_eq!(recovered.position_km, [17.0, 2.0, 0.0]);
                assert_eq!(recovered.velocity_km_per_second, [0.5, -0.25, 0.0]);
                assert_eq!(set.kernel_count(), 1);
            }
        }
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
        assert_eq!(
            set.relative_state(399, 399, 5.0).unwrap().position_km,
            [0.0; 3]
        );
        assert!(matches!(
            set.relative_state(301, 399, f64::INFINITY),
            Err(SpkRelativeError::InvalidEpoch)
        ));
        assert!(set.unload_last().is_some());
        assert!(set.unload_last().is_none());
    }

    #[test]
    fn atomic_batch_does_not_publish_valid_prefix_when_later_index_fails() {
        for (first_order, overlay_order) in [
            (SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian),
            (SpkByteOrder::BigEndian, SpkByteOrder::LittleEndian),
        ] {
            let mut set = with_kernels(&[(
                first_order,
                vec![seg(301, 399, 2, [2.0, 3.0], [0.5, -0.25])],
            )]);
            let previous = set.direct_state(301, 5.0).unwrap();
            let previously_selected = set.selected_segment(301, 5.0).unwrap();
            let overlay = Cursor::new(kernel(
                overlay_order,
                &[seg(301, 4, 3, [99.0, -17.0], [0.0, 1.0])],
            ));
            let invalid = Cursor::new(vec![0_u8; RECORD_BYTES]);
            let failure = set
                .load_batch_atomic(vec![overlay, invalid])
                .expect_err("invalid second input must reject entire batch");

            assert_eq!(failure.input_index, 1);
            assert!(matches!(
                &failure.error,
                SpkIndexError::InvalidFile(_) | SpkIndexError::Io(_)
            ));
            assert!(std::error::Error::source(&failure).is_some());
            assert_eq!(set.kernel_count(), 1);
            assert_eq!(set.selected_segment(301, 5.0).unwrap(), previously_selected);
            assert_eq!(set.direct_state(301, 5.0).unwrap(), previous);
            assert_eq!(
                set.relative_state(301, 399, 5.0).unwrap().position_km,
                previous.position_km
            );
        }
    }

    #[test]
    fn atomic_batch_preserves_load_priority_and_unload_order() {
        let first = Cursor::new(kernel(
            SpkByteOrder::LittleEndian,
            &[seg(301, 399, 2, [1.0, 0.0], [0.0, 0.0])],
        ));
        let second = Cursor::new(kernel(
            SpkByteOrder::BigEndian,
            &[seg(301, 4, 3, [4.0, 1.0], [0.25, 0.5])],
        ));
        let mut set = SpkKernelSet::new();
        set.load_batch_atomic(Vec::<Cursor<Vec<u8>>>::new())
            .unwrap();
        assert!(set.is_empty());
        set.load_batch_atomic(vec![first, second]).unwrap();

        assert_eq!(set.kernel_count(), 2);
        assert_eq!(set.selected_segment(301, 5.0).unwrap().unwrap().0, 1);
        let newest = set.direct_state(301, 5.0).unwrap();
        assert_eq!(newest.center_id, 4);
        assert_eq!(newest.position_km, [4.0, 1.0, 0.0]);
        assert_eq!(newest.velocity_km_per_second, [0.25, 0.5, 0.0]);

        set.unload_last().unwrap();
        assert_eq!(set.kernel_count(), 1);
        assert_eq!(set.selected_segment(301, 5.0).unwrap().unwrap().0, 0);
        let previous = set.direct_state(301, 5.0).unwrap();
        assert_eq!(previous.center_id, 399);
        assert_eq!(previous.position_km, [1.0, 0.0, 0.0]);
        assert_eq!(previous.velocity_km_per_second, [0.0; 3]);
        set.unload_last().unwrap();
        assert!(set.is_empty());
    }
}
