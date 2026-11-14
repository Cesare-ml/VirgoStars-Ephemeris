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

//! One-kernel, fixed-frame SPK center-chain composition.
//!
//! Resolve the last covering segment for each body at one geometric TDB/ET
//! epoch, then subtract the target and center paths to their *nearest shared*
//! ancestor. Only required links are evaluated by the existing Rust SPK
//! type-2/type-3 Chebyshev reader; no orbital algorithm is duplicated.
//!
//! This is not a multi-kernel SPICE equivalent, apparent state or a validated
//! numerical JPL implementation.
//! NAIF SPK background: <https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html>

//! Modification (2026-11-14): accumulate signed target/observer links with
//! compensated sums, preserving small offsets during large-vector cancellation.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::spk_direct_state::{
    direct_spk_state, select_covering_segment, SpkDirectState, SpkStateError,
};
use crate::spk_index::SpkIndex;

/// Geometric target-minus-center state in fixed NAIF frame 1 (J2000/ICRF).
///
/// Position is in kilometres, velocity in kilometres per TDB second and
/// epoch is ET/TDB seconds past J2000. The shared ancestor is diagnostic,
/// not necessarily the solar-system barycenter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpkRelativeState {
    pub target_id: i32,
    pub center_id: i32,
    pub common_ancestor_id: i32,
    pub tdb_seconds_past_j2000: f64,
    pub position_km: [f64; 3],
    pub velocity_km_per_second: [f64; 3],
}

#[derive(Debug)]
pub enum SpkRelativeError {
    InvalidEpoch,
    Cycle { body_id: i32 },
    Disconnected { target_id: i32, center_id: i32 },
    NumericalOverflow,
    Segment(SpkStateError),
}

impl fmt::Display for SpkRelativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEpoch => write!(f, "SPK relative-state TDB/ET must be finite"),
            Self::Cycle { body_id } => write!(f, "SPK center chain contains a cycle at body {body_id}"),
            Self::Disconnected { target_id, center_id } => write!(
                f,
                "SPK target {target_id} and center {center_id} have no shared ancestor at this epoch"
            ),
            Self::NumericalOverflow => write!(f, "SPK relative-state vector overflow"),
            Self::Segment(error) => write!(f, "SPK relative-state segment error: {error}"),
        }
    }
}

impl Error for SpkRelativeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Self::Segment(error) = self {
            Some(error)
        } else {
            None
        }
    }
}

impl From<SpkStateError> for SpkRelativeError {
    fn from(error: SpkStateError) -> Self {
        Self::Segment(error)
    }
}

/// Selected-link interface shared by one-kernel and ordered multi-kernel sets.
///
/// Centers are inspected from index metadata; only links required for a
/// requested result are evaluated through the existing Rust SPK solver.
pub(crate) trait SpkStateResolver {
    fn selected_center(&self, body: i32, et: f64) -> Option<i32>;
    fn direct_state(&mut self, body: i32, et: f64) -> Result<SpkDirectState, SpkStateError>;
}

struct SingleKernel<'a, R: Read + Seek> {
    source: &'a mut R,
    index: &'a SpkIndex,
}

impl<R: Read + Seek> SpkStateResolver for SingleKernel<'_, R> {
    fn selected_center(&self, body: i32, et: f64) -> Option<i32> {
        select_covering_segment(self.index, body, et).map(|segment| segment.center_id)
    }

    fn direct_state(&mut self, body: i32, et: f64) -> Result<SpkDirectState, SpkStateError> {
        direct_spk_state(self.source, self.index, body, et)
    }
}

/// Discover declared center ancestry, stopping at SSB, missing data or a cycle.
///
/// Modification (2026-11-14): defer the cycle error until the nearest common
/// ancestor is known. A cycle *above* that ancestor cannot affect the required
/// target-minus-center state, whose paths stop at the common ancestor. Cycles
/// still fail closed if no shared ancestor is reachable without repeating a
/// node. Only metadata is inspected; no unused coefficients are evaluated.
fn ancestors<S: SpkStateResolver>(resolver: &S, body: i32, et: f64) -> (Vec<i32>, Option<i32>) {
    let mut path = Vec::new();
    let mut visited = HashSet::new();
    let mut node = body;
    loop {
        if !visited.insert(node) {
            return (path, Some(node));
        }
        path.push(node);
        if node == 0 {
            break;
        }
        match resolver.selected_center(node, et) {
            Some(parent) => node = parent,
            None => break,
        }
    }
    (path, None)
}

/// Neumaier-compensated summation of a geometric SPK state component.
///
/// A high-magnitude link can round away a smaller but valid contribution.
/// Accumulate both the target and *negative* observer links in one sequence,
/// so cancellation across the two paths does not erase low-order components.
/// This is roundoff control, not an independent astronomy implementation.
///
/// Modification (2026-11-14): preserve small signed link contributions
/// across center-chain and target-minus-observer cancellation.
#[derive(Clone, Copy, Default)]
struct CompensatedSum {
    value: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, term: f64) -> Result<(), SpkRelativeError> {
        let next = self.value + term;
        if !next.is_finite() {
            return Err(SpkRelativeError::NumericalOverflow);
        }
        let roundoff = if self.value.abs() >= term.abs() {
            (self.value - next) + term
        } else {
            (term - next) + self.value
        };
        self.correction += roundoff;
        if !self.correction.is_finite() {
            return Err(SpkRelativeError::NumericalOverflow);
        }
        self.value = next;
        Ok(())
    }

    fn finish(self) -> Result<f64, SpkRelativeError> {
        let value = self.value + self.correction;
        if !value.is_finite() {
            return Err(SpkRelativeError::NumericalOverflow);
        }
        Ok(value)
    }
}

/// Accumulate existing direct SPK links up to (but not including) the shared
/// ancestor. The sign is +1 for target links and -1 for observer links.
///
/// Only links required by the chosen path are evaluated. Any unsupported
/// selected segment or corrupt coefficients still fail without fallback.
fn accumulate_to_ancestor<S: SpkStateResolver>(
    resolver: &mut S,
    path: &[i32],
    common: i32,
    et: f64,
    sign: f64,
    sums: &mut [CompensatedSum; 6],
) -> Result<(), SpkRelativeError> {
    for &body in path {
        if body == common {
            break;
        }
        let state = resolver.direct_state(body, et)?;
        for axis in 0..3 {
            sums[axis].add(sign * state.position_km[axis])?;
            sums[axis + 3].add(sign * state.velocity_km_per_second[axis])?;
        }
    }
    Ok(())
}

/// Compute one geometric target-minus-center SPK state at one TDB/ET epoch.
///
/// The supplied stream and index must refer to the *same* externally provided
/// SPK file. Segment priority is **last covering segment per target** within
/// this one kernel, regardless of declared center/frame/type. At each required
/// link the evaluator supports only SPK type 2/3 and frame 1 J2000/ICRF.
///
/// Paths are composed through their first shared ancestor in the target's
/// ancestry; a full solar-system-barycenter chain is *not* required if the
/// bodies meet earlier. Unconnected paths and cycles that prevent reaching
/// any shared ancestor fail closed. A cycle strictly beyond the shared ancestor
/// is not evaluated and cannot invalidate the necessary finite paths.
///
/// This does not implement multiple loaded kernels, non-J2000 frame rotations,
/// planetary aberration, light time, or civil-time conversion. No velocities
/// are differentiated a second time: the direct Rust evaluator provides km/s.
///
/// A target relative to itself is the zero vector by definition for finite
/// ET, even when no segment for that body exists.
pub fn relative_spk_state<R: Read + Seek>(
    source: &mut R,
    index: &SpkIndex,
    target_id: i32,
    center_id: i32,
    et: f64,
) -> Result<SpkRelativeState, SpkRelativeError> {
    resolve_relative(
        &mut SingleKernel { source, index },
        target_id,
        center_id,
        et,
    )
}

/// Share one center-chain algorithm between single and ordered multi-kernel sets.
pub(crate) fn resolve_relative<S: SpkStateResolver>(
    resolver: &mut S,
    target_id: i32,
    center_id: i32,
    et: f64,
) -> Result<SpkRelativeState, SpkRelativeError> {
    if !et.is_finite() {
        return Err(SpkRelativeError::InvalidEpoch);
    }
    if target_id == center_id {
        return Ok(SpkRelativeState {
            target_id,
            center_id,
            common_ancestor_id: target_id,
            tdb_seconds_past_j2000: et,
            position_km: [0.0; 3],
            velocity_km_per_second: [0.0; 3],
        });
    }

    let (target_path, target_cycle) = ancestors(resolver, target_id, et);
    let (center_path, center_cycle) = ancestors(resolver, center_id, et);
    let common = match target_path
        .iter()
        .copied()
        .find(|node| center_path.contains(node))
    {
        Some(common) => common,
        None => {
            // A disconnected request must never turn a required cycle into
            // a misleading "no data" result. Keep the original fail-closed
            // cycle diagnostics, prioritizing the target side as before.
            if let Some(body_id) = target_cycle.or(center_cycle) {
                return Err(SpkRelativeError::Cycle { body_id });
            }
            return Err(SpkRelativeError::Disconnected {
                target_id,
                center_id,
            });
        }
    };

    let mut sums = [CompensatedSum::default(); 6];
    accumulate_to_ancestor(resolver, &target_path, common, et, 1.0, &mut sums)?;
    accumulate_to_ancestor(resolver, &center_path, common, et, -1.0, &mut sums)?;
    let position_km = [sums[0].finish()?, sums[1].finish()?, sums[2].finish()?];
    let velocity_km_per_second = [sums[3].finish()?, sums[4].finish()?, sums[5].finish()?];
    Ok(SpkRelativeState {
        target_id,
        center_id,
        common_ancestor_id: common,
        tdb_seconds_past_j2000: et,
        position_km,
        velocity_km_per_second,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spk_index::{inspect_spk, SpkByteOrder};
    use std::io::Cursor;

    const RECORD_BYTES: usize = 1024;

    #[derive(Clone, Copy)]
    struct Segment {
        target: i32,
        center: i32,
        frame: i32,
        kind: i32,
        position: [f64; 2],
        velocity: [f64; 2],
    }

    fn put_i32(bytes: &mut [u8], offset: usize, value: i32, order: SpkByteOrder) {
        let word = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[offset..offset + 4].copy_from_slice(&word);
    }

    fn put_f64(bytes: &mut [u8], offset: usize, value: f64, order: SpkByteOrder) {
        let word = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        bytes[offset..offset + 8].copy_from_slice(&word);
    }

    fn segment(target: i32, center: i32, kind: i32, pos: [f64; 2], vel: [f64; 2]) -> Segment {
        Segment {
            target,
            center,
            frame: 1,
            kind,
            position: pos,
            velocity: vel,
        }
    }

    fn synthetic_kernel(order: SpkByteOrder, specs: &[Segment]) -> Vec<u8> {
        assert!(!specs.is_empty() && specs.len() <= 6);
        let mut bytes = vec![0u8; 5 * RECORD_BYTES];
        bytes[0..8].copy_from_slice(b"DAF/SPK ");
        bytes[88..96].copy_from_slice(match order {
            SpkByteOrder::LittleEndian => b"LTL-IEEE",
            SpkByteOrder::BigEndian => b"BIG-IEEE",
        });
        put_i32(&mut bytes, 8, 2, order);
        put_i32(&mut bytes, 12, 6, order);
        put_i32(&mut bytes, 76, 3, order);
        put_i32(&mut bytes, 80, 3, order);
        let summary = 2 * RECORD_BYTES;
        put_f64(&mut bytes, summary + 16, specs.len() as f64, order);

        let mut next_word = 513usize;
        for (i, spec) in specs.iter().enumerate() {
            let components = if spec.kind == 2 { 3 } else { 6 };
            let rsize = 2 + components * 2;
            let first = next_word;
            let last = first + rsize + 3;
            let descriptor = summary + (3 + 5 * i) * 8;
            put_f64(&mut bytes, descriptor, 0.0, order);
            put_f64(&mut bytes, descriptor + 8, 10.0, order);
            for (j, value) in [
                spec.target,
                spec.center,
                spec.frame,
                spec.kind,
                first as i32,
                last as i32,
            ]
            .into_iter()
            .enumerate()
            {
                put_i32(&mut bytes, descriptor + 16 + 4 * j, value, order);
            }
            let record = (first - 1) * 8;
            put_f64(&mut bytes, record, 5.0, order);
            put_f64(&mut bytes, record + 8, 5.0, order);
            let coeffs: Vec<[f64; 2]> = if spec.kind == 2 {
                vec![
                    [spec.position[0], spec.velocity[0] * 5.0],
                    [spec.position[1], spec.velocity[1] * 5.0],
                    [0.0, 0.0],
                ]
            } else {
                vec![
                    [spec.position[0], 0.0],
                    [spec.position[1], 0.0],
                    [0.0, 0.0],
                    [spec.velocity[0], 0.0],
                    [spec.velocity[1], 0.0],
                    [0.0, 0.0],
                ]
            };
            for (j, coefficients) in coeffs.iter().enumerate() {
                put_f64(&mut bytes, record + (2 + j * 2) * 8, coefficients[0], order);
                put_f64(&mut bytes, record + (3 + j * 2) * 8, coefficients[1], order);
            }
            let directory = (last - 4) * 8;
            for (j, value) in [0.0, 10.0, rsize as f64, 1.0].into_iter().enumerate() {
                put_f64(&mut bytes, directory + j * 8, value, order);
            }
            next_word = last + 1;
        }
        assert!(next_word < 640, "synthetic data must fit in record five");
        put_i32(&mut bytes, 84, next_word as i32, order);
        bytes
    }

    fn base_segments() -> Vec<Segment> {
        vec![
            segment(399, 3, 2, [10.0, 0.0], [1.0, 0.0]),
            segment(301, 399, 3, [2.0, 4.0], [-0.2, 0.1]),
            segment(3, 0, 2, [100.0, 20.0], [3.0, 1.0]),
            segment(4, 0, 3, [-200.0, 50.0], [-2.0, 0.5]),
        ]
    }

    fn compute(
        order: SpkByteOrder,
        specs: &[Segment],
        target: i32,
        center: i32,
        et: f64,
    ) -> Result<SpkRelativeState, SpkRelativeError> {
        let mut reader = Cursor::new(synthetic_kernel(order, specs));
        let index = inspect_spk(&mut reader).unwrap();
        relative_spk_state(&mut reader, &index, target, center, et)
    }

    #[test]
    fn moon_earth_relatives_need_no_ssb_chain_and_are_antisymmetric() {
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            // The Earth's parent (EMB) is absent, but Moon -> Earth is enough.
            let specs = [segment(301, 399, 3, [2.0, 4.0], [-0.2, 0.1])];
            let moon_earth = compute(order, &specs, 301, 399, 5.0).unwrap();
            assert_eq!(moon_earth.common_ancestor_id, 399);
            assert_eq!(moon_earth.position_km, [2.0, 4.0, 0.0]);
            assert_eq!(moon_earth.velocity_km_per_second, [-0.2, 0.1, 0.0]);
            let earth_moon = compute(order, &specs, 399, 301, 5.0).unwrap();
            assert_eq!(earth_moon.position_km, [-2.0, -4.0, 0.0]);
            assert_eq!(earth_moon.velocity_km_per_second, [0.2, -0.1, 0.0]);
        }
    }

    #[test]
    fn composes_two_branches_through_earth_emb_and_ssb() {
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            let specs = base_segments();
            let moon_ssb = compute(order, &specs, 301, 0, 5.0).unwrap();
            assert_eq!(moon_ssb.common_ancestor_id, 0);
            assert_eq!(moon_ssb.position_km, [112.0, 24.0, 0.0]);
            assert_eq!(moon_ssb.velocity_km_per_second[0], 3.8);
            assert_eq!(moon_ssb.velocity_km_per_second[1], 1.1);

            let moon_mars = compute(order, &specs, 301, 4, 5.0).unwrap();
            assert_eq!(moon_mars.position_km, [312.0, -26.0, 0.0]);
            assert_eq!(moon_mars.velocity_km_per_second[0], 5.8);
            assert!((moon_mars.velocity_km_per_second[1] - 0.6).abs() < 1e-12);
            let mars_moon = compute(order, &specs, 4, 301, 5.0).unwrap();
            assert_eq!(mars_moon.position_km, [-312.0, 26.0, 0.0]);
            assert!((mars_moon.velocity_km_per_second[0] + 5.8).abs() < 1e-12);
        }
    }

    #[test]
    fn same_body_is_zero_even_without_a_segment_but_et_must_be_valid() {
        let specs = base_segments();
        let zero = compute(SpkByteOrder::LittleEndian, &specs, 12345, 12345, 5.0).unwrap();
        assert_eq!(zero.common_ancestor_id, 12345);
        assert_eq!(zero.position_km, [0.0; 3]);
        assert_eq!(zero.velocity_km_per_second, [0.0; 3]);
        assert!(matches!(
            compute(SpkByteOrder::LittleEndian, &specs, 301, 301, f64::NAN),
            Err(SpkRelativeError::InvalidEpoch)
        ));
    }

    #[test]
    fn unconnected_branches_are_explicit_errors_not_zero_or_fallback() {
        let specs = [
            segment(301, 399, 2, [2.0, 0.0], [0.0, 0.0]),
            segment(4, 0, 2, [9.0, 0.0], [0.0, 0.0]),
        ];
        assert!(matches!(
            compute(SpkByteOrder::LittleEndian, &specs, 301, 4, 5.0),
            Err(SpkRelativeError::Disconnected {
                target_id: 301,
                center_id: 4
            })
        ));
    }

    // Modification (2026-11-14): upstream cycles are irrelevant if the
    // requested target and center meet before reaching those cycles.
    #[test]
    fn shared_ancestor_before_cycle_keeps_required_relative_state() {
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            let specs = [
                segment(301, 399, 2, [3.0, 4.0], [0.25, -0.5]),
                segment(302, 399, 3, [1.0, 1.0], [-0.125, 0.75]),
                segment(399, 3, 2, [5.0, 0.0], [1.0, 0.0]),
                segment(3, 399, 3, [7.0, 0.0], [0.0, 0.0]),
            ];

            // Direct child-to-parent and the reverse need exactly one link.
            let moon_earth = compute(order, &specs, 301, 399, 5.0).unwrap();
            assert_eq!(moon_earth.common_ancestor_id, 399);
            assert_eq!(moon_earth.position_km, [3.0, 4.0, 0.0]);
            assert_eq!(moon_earth.velocity_km_per_second, [0.25, -0.5, 0.0]);
            let earth_moon = compute(order, &specs, 399, 301, 5.0).unwrap();
            assert_eq!(earth_moon.position_km, [-3.0, -4.0, 0.0]);
            assert_eq!(earth_moon.velocity_km_per_second, [-0.25, 0.5, 0.0]);

            // Siblings also meet at 399; neither path needs 399 -> 3 -> 399.
            let siblings = compute(order, &specs, 301, 302, 5.0).unwrap();
            assert_eq!(siblings.common_ancestor_id, 399);
            assert_eq!(siblings.position_km, [2.0, 3.0, 0.0]);
            assert_eq!(siblings.velocity_km_per_second, [0.375, -1.25, 0.0]);

            // The SSB request really needs an upstream connection: no shared
            // ancestor precedes the cycle and so the operation must fail.
            assert!(matches!(
                compute(order, &specs, 301, 0, 5.0),
                Err(SpkRelativeError::Cycle { body_id: 399 })
            ));
        }
    }

    #[test]
    fn cycles_in_selected_center_chains_fail_closed() {
        let specs = [
            segment(301, 399, 2, [1.0, 0.0], [0.0, 0.0]),
            segment(399, 3, 2, [1.0, 0.0], [0.0, 0.0]),
            segment(3, 399, 2, [1.0, 0.0], [0.0, 0.0]),
        ];
        assert!(matches!(
            compute(SpkByteOrder::BigEndian, &specs, 301, 0, 5.0),
            Err(SpkRelativeError::Cycle { body_id: 399 })
        ));
    }

    #[test]
    fn later_covering_center_wins_even_when_it_changes_ancestry() {
        let mut specs = base_segments();
        // The new Moon -> Mars edge masks earlier Moon -> Earth. The winner
        // determines both its data and which ancestor path is searched.
        specs.push(segment(301, 4, 2, [7.0, -5.0], [0.0, 0.0]));
        let moon_earth = compute(SpkByteOrder::LittleEndian, &specs, 301, 399, 5.0).unwrap();
        assert_eq!(moon_earth.position_km, [-303.0, 25.0, 0.0]);
        assert_eq!(moon_earth.common_ancestor_id, 0);
    }

    #[test]
    fn required_unsupported_frame_or_type_blocks_composition() {
        let mut specs = base_segments();
        specs[1].frame = 17;
        assert!(matches!(
            compute(SpkByteOrder::LittleEndian, &specs, 301, 399, 5.0),
            Err(SpkRelativeError::Segment(SpkStateError::UnsupportedFrame(
                17
            )))
        ));
        specs[1].frame = 1;
        specs[1].kind = 9;
        assert!(matches!(
            compute(SpkByteOrder::LittleEndian, &specs, 301, 399, 5.0),
            Err(SpkRelativeError::Segment(SpkStateError::UnsupportedType(9)))
        ));
    }

    #[test]
    fn cannot_silently_overflow_vector_sums() {
        let specs = [
            segment(301, 399, 2, [f64::MAX, 0.0], [0.0, 0.0]),
            segment(399, 0, 2, [f64::MAX, 0.0], [0.0, 0.0]),
        ];
        assert!(matches!(
            compute(SpkByteOrder::LittleEndian, &specs, 301, 0, 5.0),
            Err(SpkRelativeError::NumericalOverflow)
        ));
    }

    #[test]
    fn selected_segment_requires_its_own_declared_coverage() {
        let specs = base_segments();
        assert!(matches!(
            compute(SpkByteOrder::LittleEndian, &specs, 301, 0, 11.0),
            Err(SpkRelativeError::Disconnected { .. })
        ));
    }
}


#[cfg(test)]
mod compensated_chain_tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Link {
        target: i32,
        center: i32,
        position: [f64; 3],
        velocity: [f64; 3],
    }

    struct MockResolver {
        links: Vec<Link>,
    }

    impl SpkStateResolver for MockResolver {
        fn selected_center(&self, body: i32, _et: f64) -> Option<i32> {
            self.links
                .iter()
                .find(|link| link.target == body)
                .map(|link| link.center)
        }

        fn direct_state(&mut self, body: i32, et: f64) -> Result<SpkDirectState, SpkStateError> {
            let link = self
                .links
                .iter()
                .find(|link| link.target == body)
                .ok_or(SpkStateError::MissingSegment { target_id: body })?;
            Ok(SpkDirectState {
                target_id: link.target,
                center_id: link.center,
                frame_id: 1,
                tdb_seconds_past_j2000: et,
                position_km: link.position,
                velocity_km_per_second: link.velocity,
            })
        }
    }

    fn link(
        target: i32,
        center: i32,
        position: [f64; 3],
        velocity: [f64; 3],
    ) -> Link {
        Link {
            target,
            center,
            position,
            velocity,
        }
    }

    #[test]
    fn tiny_links_survive_large_target_and_center_cancellation() {
        // Independent exact arithmetic: (1e16 + 1) - 1e16 = 1.
        // Each input is exactly representable, but ordinary f64 summation
        // rounds 1e16 + 1 back to 1e16 before subtracting the observer.
        let mut resolver = MockResolver {
            links: vec![
                link(100, 300, [1e16, -1e16, 0.0], [1e16, -1e16, 0.0]),
                link(300, 0, [1.0, 2.0, 0.0], [1.0, 2.0, 0.0]),
                link(200, 0, [1e16, -1e16, 0.0], [1e16, -1e16, 0.0]),
            ],
        };
        let state = resolve_relative(&mut resolver, 100, 200, -123.0).unwrap();
        assert_eq!(state.common_ancestor_id, 0);
        assert_eq!(state.tdb_seconds_past_j2000, -123.0);
        assert_eq!(state.position_km, [1.0, 2.0, 0.0]);
        assert_eq!(state.velocity_km_per_second, [1.0, 2.0, 0.0]);

        let reverse = resolve_relative(&mut resolver, 200, 100, -123.0).unwrap();
        assert_eq!(reverse.position_km, [-1.0, -2.0, 0.0]);
        assert_eq!(reverse.velocity_km_per_second, [-1.0, -2.0, 0.0]);
    }

    #[test]
    fn signed_three_link_path_cancellation_keeps_unit_offset() {
        // (1e16 + 1 - 1e16) is exactly one kilometre along x and
        // one kilometre per second along y, despite losing the unit
        // contribution with naive left-to-right binary64 accumulation.
        let mut resolver = MockResolver {
            links: vec![
                link(100, 101, [1e16, 0.0, 0.0], [0.0, 1e16, 0.0]),
                link(101, 102, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
                link(102, 0, [-1e16, 0.0, 0.0], [0.0, -1e16, 0.0]),
            ],
        };
        let state = resolve_relative(&mut resolver, 100, 0, 0.0).unwrap();
        assert_eq!(state.position_km, [1.0, 0.0, 0.0]);
        assert_eq!(state.velocity_km_per_second, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn true_overflow_still_fails_closed() {
        let mut resolver = MockResolver {
            links: vec![
                link(100, 0, [f64::MAX, 0.0, 0.0], [0.0; 3]),
                link(200, 0, [-f64::MAX, 0.0, 0.0], [0.0; 3]),
            ],
        };
        assert!(matches!(
            resolve_relative(&mut resolver, 100, 200, 0.0),
            Err(SpkRelativeError::NumericalOverflow)
        ));
        assert!(matches!(
            resolve_relative(&mut resolver, 100, 200, f64::NAN),
            Err(SpkRelativeError::InvalidEpoch)
        ));
    }
}
