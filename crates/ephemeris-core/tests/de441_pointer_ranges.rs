/* EXHIBIT A. Common Public Attribution License Version 1.0.
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
use virgostars_ephemeris_core::de441::{De441, De441Error};

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// Two 32-day records. Mercury has two granules with T0, T1 and T2 terms;
// EMB and Moon use constant vectors, so Earth/Moon reconstruction is analytical.
fn fixture() -> Vec<u8> {
    let size = 3000;
    let mut b = vec![0u8; size * 4];
    let title = b"JPL Planetary Ephemeris DE441";
    b[..title.len()].copy_from_slice(title);
    put_f64(&mut b, 2652, 1000.0);
    put_f64(&mut b, 2660, 1064.0);
    put_f64(&mut b, 2668, 32.0);
    put_f64(&mut b, 2688, 81.0);
    put_i32(&mut b, 2840, 441);
    for (index, first, coefficients, granules) in
        [(0, 3, 3, 2), (2, 21, 1, 1), (9, 24, 1, 1), (12, 373, 1, 1)]
    {
        let at = if index == 12 { 2844 } else { 2696 + index * 12 };
        put_i32(&mut b, at, first);
        put_i32(&mut b, at + 4, coefficients);
        put_i32(&mut b, at + 8, granules);
    }
    for r in 0..2 {
        let base = size * (r + 2);
        put_f64(&mut b, base, 1000.0 + r as f64 * 32.0);
        put_f64(&mut b, base + 8, 1032.0 + r as f64 * 32.0);
        for granule in 0..2 {
            for axis in 0..3 {
                let at = base + (2 + (granule * 3 + axis) * 3) * 8;
                put_f64(&mut b, at, (r * 1000 + granule * 100 + axis * 10) as f64);
                put_f64(&mut b, at + 8, 2.0);
                put_f64(&mut b, at + 16, 3.0);
            }
        }
        for axis in 0..3 {
            put_f64(&mut b, base + (20 + axis) * 8, 1000.0 + axis as f64);
            put_f64(&mut b, base + (23 + axis) * 8, 82.0 * (axis + 1) as f64);
        }
    }
    b
}

#[test]
fn rejects_overlapping_coefficient_series_in_header() {
    // The valid fixture has Mercury [3,21), EMB [21,24), Moon [24,27).
    // Move EMB to word 20 so it overlaps Mercury's final coefficient.
    let mut bad = fixture();
    put_i32(&mut bad, 2696 + 2 * 12, 20);
    assert!(matches!(
        De441::new(Cursor::new(bad)),
        Err(De441Error::InvalidFile("overlapping coefficient pointers"))
    ));
}

#[test]
fn accepts_adjacent_non_overlapping_coefficient_series() {
    // The original synthetic fixture deliberately uses adjacent intervals.
    assert!(De441::new(Cursor::new(fixture())).is_ok());
}
