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

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::spk_direct_state::{direct_spk_state, select_covering_segment, SpkDirectState, SpkStateError};
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

/// Discover declared center ancestry, stopping at SSB (ID 0) or missing data.
///
/// Selects the last covering descriptor for each link *without* evaluating
/// coefficients. A missing path above a shared ancestor is harmless; missing
/// connections between the requested bodies are not. A cycle is never followed.
fn ancestors<S: SpkStateResolver>(resolver: &S, body: i32, et: f64) -> Result<Vec<i32>, SpkRelativeError> {
    let mut path = Vec::new();
    let mut visited = HashSet::new();
    let mut node = body;
    loop {
        if !visited.insert(node) {
            return Err(SpkRelativeError::Cycle { body_id: node });
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
    Ok(path)
}

/// Sum the existing direct SPK states from the start body to the ancestor.
///
/// Each link is retrieved through the canonical evaluator; any unsupported
/// selected frame/type or invalid coefficients therefore fail explicitly.
fn to_ancestor<S: SpkStateResolver>(
    resolver: &mut S,
    path: &[i32],
    common: i32,
    et: f64,
) -> Result<([f64; 3], [f64; 3]), SpkRelativeError> {
    let mut position = [0.0_f64; 3];
    let mut velocity = [0.0_f64; 3];
    for &body in path {
        if body == common {
            break;
        }
        let state = resolver.direct_state(body, et)?;
        for axis in 0..3 {
            position[axis] += state.position_km[axis];
            velocity[axis] += state.velocity_km_per_second[axis];
            if !position[axis].is_finite() || !velocity[axis].is_finite() {
                return Err(SpkRelativeError::NumericalOverflow);
            }
        }
    }
    Ok((position, velocity))
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
/// bodies meet earlier. Unconnected paths and cycles fail closed, rather than
/// returning approximate positions or falling back to lower-priority segments.
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

    let target_path = ancestors(resolver, target_id, et)?;
    let center_path = ancestors(resolver, center_id, et)?;
    let common = target_path
        .iter()
        .copied()
        .find(|node| center_path.contains(node))
        .ok_or(SpkRelativeError::Disconnected {
            target_id,
            center_id,
        })?;

    let (target_position, target_velocity) = to_ancestor(resolver, &target_path, common, et)?;
    let (center_position, center_velocity) = to_ancestor(resolver, &center_path, common, et)?;
    let mut position_km = [0.0; 3];
    let mut velocity_km_per_second = [0.0; 3];
    for axis in 0..3 {
        position_km[axis] = target_position[axis] - center_position[axis];
        velocity_km_per_second[axis] = target_velocity[axis] - center_velocity[axis];
        if !position_km[axis].is_finite() || !velocity_km_per_second[axis].is_finite() {
            return Err(SpkRelativeError::NumericalOverflow);
        }
    }
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
