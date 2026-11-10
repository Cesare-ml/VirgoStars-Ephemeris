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

use std::io::Cursor;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// Synthetic two-record classic DE441 header with disjoint coefficient ranges.
// EMB coordinates exceed the f64 spacing for the geocentric lunar offsets.
fn fixture() -> Vec<u8> {
    const RECORD_SIZE: usize = 3000;
    let mut bytes = vec![0u8; RECORD_SIZE * 4];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    write_f64(&mut bytes, 2652, 1000.0);
    write_f64(&mut bytes, 2660, 1064.0);
    write_f64(&mut bytes, 2668, 32.0);
    write_f64(&mut bytes, 2688, 81.0);
    write_i32(&mut bytes, 2840, 441);
    // The last pointer keeps this synthetic record the required 3000 bytes.
    for (index, first, coefficients, granules) in
        [(2, 3, 1, 1), (9, 6, 2, 1), (12, 373, 1, 1)]
    {
        let at = if index == 12 { 2844 } else { 2696 + index * 12 };
        write_i32(&mut bytes, at, first);
        write_i32(&mut bytes, at + 4, coefficients);
        write_i32(&mut bytes, at + 8, granules);
    }
    for record in 0..2 {
        let base = (record + 2) * RECORD_SIZE;
        write_f64(&mut bytes, base, 1000.0 + record as f64 * 32.0);
        write_f64(&mut bytes, base + 8, 1032.0 + record as f64 * 32.0);
        for axis in 0..3 {
            let component = (axis + 1) as f64;
            write_f64(&mut bytes, base + (2 + axis) * 8, 1.0e20);
            // Moon geo series: T0 = 82*i km, T1 = 16*i km.
            // At the midpoint, the velocity is i km/day (granule width 32).
            write_f64(&mut bytes, base + (5 + axis * 2) * 8, 82.0 * component);
            write_f64(&mut bytes, base + (6 + axis * 2) * 8, 16.0 * component);
        }
    }
    bytes
}

fn assert_close(actual: [f64; 3], expected: [f64; 3]) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!((a - e).abs() < 1.0e-10, "{a} differs from {e}");
    }
}

#[test]
fn earth_moon_emb_relative_states_preserve_small_positions_and_velocities() {
    let mut ephemeris = De441::new(Cursor::new(fixture())).unwrap();
    let ratio = 81.0 / 82.0;
    for epoch in [1016.0, 1048.0] {
        let epoch = JulianDateTdb(epoch);
        for (target, center, scale) in [
            (Body::Moon, Body::EarthMoonBarycenter, ratio),
            (Body::EarthMoonBarycenter, Body::Moon, -ratio),
            (Body::Earth, Body::EarthMoonBarycenter, -1.0 / 82.0),
            (Body::EarthMoonBarycenter, Body::Earth, 1.0 / 82.0),
        ] {
            let state = ephemeris.state(target, center, epoch).unwrap();
            assert_close(
                state.position_km,
                [82.0 * scale, 164.0 * scale, 246.0 * scale],
            );
            assert_close(
                state.velocity_km_per_day,
                [scale, 2.0 * scale, 3.0 * scale],
            );
        }
    }
}

#[test]
fn emb_relative_offsets_only_need_the_geocentric_moon_series() {
    let mut bytes = fixture();
    // An EMB absolute series is not required to derive relative EMB offsets.
    for offset in [2720, 2724, 2728] {
        write_i32(&mut bytes, offset, 0);
    }
    let mut ephemeris = De441::new(Cursor::new(bytes)).unwrap();
    let state = ephemeris
        .state(
            Body::Moon,
            Body::EarthMoonBarycenter,
            JulianDateTdb(1016.0),
        )
        .unwrap();
    assert_close(state.position_km, [81.0, 162.0, 243.0]);
}
