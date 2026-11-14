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

//! Synthetic cubic SPK center-chain kinematics far from the J2000 epoch.
//!
//! Modified 2026-11-14: independent ordinary-power polynomial oracles and
//! Richardson finite differences for nonlinear mixed-type, mixed-endian
//! SPK target-minus-center states at +/- one billion TDB/ET seconds.
//! No external kernel, NAIF vector, or independent JPL parity is claimed.

use std::io::Cursor;

use virgostars_ephemeris_core::spk_index::SpkByteOrder;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

const RECORD_BYTES: usize = 1024;
const FIRST_WORD: usize = 513;
const RADIUS_S: f64 = 8.0;

/// Ordinary-power polynomial p(t) = a + b*t + c*t^2 + d*t^3;
/// t is a TDB-second offset from the *record midpoint*, not from J2000.
type Polynomial = [f64; 4];

#[derive(Clone, Copy)]
struct Segment {
    target: i32,
    center: i32,
    kind: i32,
    xyz: [Polynomial; 3],
}

fn write_i32(bytes: &mut [u8], offset: usize, n: i32, order: SpkByteOrder) {
    let value = match order {
        SpkByteOrder::LittleEndian => n.to_le_bytes(),
        SpkByteOrder::BigEndian => n.to_be_bytes(),
    };
    bytes[offset..offset + 4].copy_from_slice(&value);
}

fn write_f64(bytes: &mut [u8], offset: usize, n: f64, order: SpkByteOrder) {
    let value = match order {
        SpkByteOrder::LittleEndian => n.to_le_bytes(),
        SpkByteOrder::BigEndian => n.to_be_bytes(),
    };
    bytes[offset..offset + 8].copy_from_slice(&value);
}

/// Transform a cubic in ordinary powers of physical seconds to Chebyshev T0..T3.
/// The test oracle below deliberately does *not* use these coefficients.
fn position_chebyshev([a, b, c, d]: Polynomial) -> [f64; 4] {
    let r2 = RADIUS_S * RADIUS_S;
    let r3 = r2 * RADIUS_S;
    [
        a + c * r2 / 2.0,
        b * RADIUS_S + 3.0 * d * r3 / 4.0,
        c * r2 / 2.0,
        d * r3 / 4.0,
    ]
}

/// SPK Type 3 stores its velocity series separately, in km/TDB-second.
fn type3_velocity_chebyshev([_a, b, c, d]: Polynomial) -> [f64; 4] {
    let r2 = RADIUS_S * RADIUS_S;
    [b + 1.5 * d * r2, 2.0 * c * RADIUS_S, 1.5 * d * r2, 0.0]
}

fn kernel(order: SpkByteOrder, midpoint: f64, segments: &[Segment]) -> Cursor<Vec<u8>> {
    assert!(!segments.is_empty() && segments.len() <= 5);
    let mut bytes = vec![0u8; 5 * RECORD_BYTES];
    bytes[..8].copy_from_slice(b"DAF/SPK ");
    bytes[88..96].copy_from_slice(match order {
        SpkByteOrder::LittleEndian => b"LTL-IEEE",
        SpkByteOrder::BigEndian => b"BIG-IEEE",
    });
    write_i32(&mut bytes, 8, 2, order);
    write_i32(&mut bytes, 12, 6, order);
    write_i32(&mut bytes, 76, 3, order);
    write_i32(&mut bytes, 80, 3, order);

    let mut first = FIRST_WORD;
    let summary = 2 * RECORD_BYTES;
    write_f64(&mut bytes, summary + 16, segments.len() as f64, order);
    for (segment_index, segment) in segments.iter().enumerate() {
        assert!(segment.kind == 2 || segment.kind == 3);
        let component_count = if segment.kind == 2 { 3 } else { 6 };
        let record_words = 2 + 4 * component_count;
        let last = first + record_words + 3;
        let descriptor = summary + (3 + 5 * segment_index) * 8;
        write_f64(&mut bytes, descriptor, midpoint - RADIUS_S, order);
        write_f64(&mut bytes, descriptor + 8, midpoint + RADIUS_S, order);
        for (index, value) in [
            segment.target,
            segment.center,
            1, // J2000/ICRF frame
            segment.kind,
            first as i32,
            last as i32,
        ]
        .into_iter()
        .enumerate()
        {
            write_i32(&mut bytes, descriptor + 16 + index * 4, value, order);
        }

        let record = (first - 1) * 8;
        write_f64(&mut bytes, record, midpoint, order);
        write_f64(&mut bytes, record + 8, RADIUS_S, order);
        for (axis, polynomial) in segment.xyz.iter().enumerate() {
            for (index, coeff) in position_chebyshev(*polynomial).into_iter().enumerate() {
                write_f64(
                    &mut bytes,
                    record + (2 + 4 * axis + index) * 8,
                    coeff,
                    order,
                );
            }
            if segment.kind == 3 {
                for (index, coeff) in type3_velocity_chebyshev(*polynomial)
                    .into_iter()
                    .enumerate()
                {
                    write_f64(
                        &mut bytes,
                        record + (2 + 4 * (axis + 3) + index) * 8,
                        coeff,
                        order,
                    );
                }
            }
        }
        let directory = (last - 4) * 8;
        for (index, value) in [
            midpoint - RADIUS_S,
            2.0 * RADIUS_S,
            record_words as f64,
            1.0,
        ]
        .into_iter()
        .enumerate()
        {
            write_f64(&mut bytes, directory + index * 8, value, order);
        }
        first = last + 1;
    }
    assert!(first <= 640);
    write_i32(&mut bytes, 84, first as i32, order);
    Cursor::new(bytes)
}

fn segment(target: i32, center: i32, kind: i32, xyz: [Polynomial; 3]) -> Segment {
    Segment {
        target,
        center,
        kind,
        xyz,
    }
}

// The same physics is encoded in two disjoint DAF/SPK byte orders. A
// conflicting older target 399->0 deliberately tests overlay precedence.
const EARTH_BARYCENTER: [Polynomial; 3] = [
    [100.0, 0.5, 0.02, 0.0005],
    [-40.0, -0.3, 0.01, -0.0002],
    [20.0, 0.4, -0.015, 0.0001],
];
const EARTH_OFFSET: [Polynomial; 3] = [
    [10.0, -0.2, 0.004, 0.0003],
    [8.0, 0.15, -0.003, 0.0002],
    [-3.0, 0.07, 0.005, -0.0004],
];
const MOON_OFFSET: [Polynomial; 3] = [
    [2.0, 0.8, -0.007, 0.0006],
    [-6.0, 0.45, 0.006, 0.0003],
    [1.0, -0.1, 0.008, -0.0002],
];
const MARS_BARYCENTER: [Polynomial; 3] = [
    [-80.0, -0.1, 0.003, -0.0003],
    [30.0, 0.2, -0.009, 0.0001],
    [-15.0, 0.16, 0.011, 0.0005],
];

fn loaded(midpoint: f64) -> SpkKernelSet<Cursor<Vec<u8>>> {
    let mut set = SpkKernelSet::new();
    set.load(kernel(
        SpkByteOrder::BigEndian,
        midpoint,
        &[
            segment(3, 0, 2, EARTH_BARYCENTER),
            segment(4, 0, 3, MARS_BARYCENTER),
            segment(399, 0, 2, [[999.0, 0.0, 0.0, 0.0]; 3]),
        ],
    ))
    .unwrap();
    set.load(kernel(
        SpkByteOrder::LittleEndian,
        midpoint,
        &[
            segment(399, 3, 2, EARTH_OFFSET),
            segment(301, 399, 3, MOON_OFFSET),
        ],
    ))
    .unwrap();
    set
}

/// The independent position/derivative oracle uses ordinary powers of seconds.
fn oracle([a, b, c, d]: Polynomial, seconds: f64) -> (f64, f64) {
    (
        a + seconds * (b + seconds * (c + seconds * d)),
        b + seconds * (2.0 * c + seconds * 3.0 * d),
    )
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected:.15}, observed {actual:.15}; tolerance {tolerance}"
    );
}

#[test]
fn nonlinear_mixed_type_chain_matches_physical_second_oracle_at_large_signed_et() {
    for midpoint in [-1_000_000_000.0, 1_000_000_000.0] {
        let mut set = loaded(midpoint);
        assert_eq!(set.selected_segment(399, midpoint).unwrap().unwrap().0, 1);
        for seconds in [-RADIUS_S, -5.25, -1.0, 0.0, 3.75, RADIUS_S] {
            let et = midpoint + seconds;
            let state = set.relative_state(301, 4, et).unwrap();
            assert_eq!((state.target_id, state.center_id), (301, 4));
            assert_eq!(state.common_ancestor_id, 0);
            assert_eq!(state.tdb_seconds_past_j2000, et);
            for axis in 0..3 {
                let parts = [
                    MOON_OFFSET[axis],
                    EARTH_OFFSET[axis],
                    EARTH_BARYCENTER[axis],
                ];
                let (mars_position, mars_velocity) = oracle(MARS_BARYCENTER[axis], seconds);
                let mut position = -mars_position;
                let mut velocity = -mars_velocity;
                for part in parts {
                    let (p, v) = oracle(part, seconds);
                    position += p;
                    velocity += v;
                }
                near(state.position_km[axis], position, 2.0e-11);
                near(state.velocity_km_per_second[axis], velocity, 1.0e-12);
            }
        }
        let old = set.unload_last().unwrap();
        assert_eq!(set.direct_state(399, midpoint).unwrap().center_id, 0);
        set.load(old).unwrap();
        assert_eq!(set.direct_state(399, midpoint).unwrap().center_id, 3);
    }
}

#[test]
fn nonlinear_chain_velocity_matches_fourth_order_position_difference() {
    for midpoint in [-1_000_000_000.0, 1_000_000_000.0] {
        let mut set = loaded(midpoint);
        let h = 0.25; // Representable exactly at both large ET magnitudes.
        let sampled = |set: &mut SpkKernelSet<Cursor<Vec<u8>>>, offset: f64| {
            set.relative_state(301, 4, midpoint + offset)
                .unwrap()
                .position_km
        };
        let minus_h = sampled(&mut set, -h);
        let plus_h = sampled(&mut set, h);
        let minus_half = sampled(&mut set, -h / 2.0);
        let plus_half = sampled(&mut set, h / 2.0);
        let center = set.relative_state(301, 4, midpoint).unwrap();
        for axis in 0..3 {
            let coarse = (plus_h[axis] - minus_h[axis]) / (2.0 * h);
            let fine = (plus_half[axis] - minus_half[axis]) / h;
            // Richardson cancellation of the O(h^2) term is exact for cubics.
            let extrapolated = (4.0 * fine - coarse) / 3.0;
            near(extrapolated, center.velocity_km_per_second[axis], 2.0e-12);
        }
    }
}
