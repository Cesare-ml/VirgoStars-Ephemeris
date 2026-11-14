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

// Modification 2026-11-14: combined cross-boundary regression for three
// independent classic DE441 guards: bounded record cache, direct record
// boundary selection, and direct Chebyshev granule boundary selection.

//! Two distinct two-granule records with synthetic constants and exact TDB
//! JD boundaries; NOT real DE441 coordinates or an independent NASA/JPL oracle.

use std::io::Cursor;
use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};

const RECORD_BYTES: usize = 3000;

fn put_f64(bytes: &mut [u8], at: usize, value: f64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut [u8], at: usize, value: i32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn two_records(start: f64) -> Vec<u8> {
    // Each 32-day record has two 16-day Mercury granules. Each Cartesian
    // axis carries its own deliberately discontinuous T0 coefficient.
    let mut bytes = vec![0u8; RECORD_BYTES * 4];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    put_f64(&mut bytes, 2652, start);
    put_f64(&mut bytes, 2660, start + 64.0);
    put_f64(&mut bytes, 2668, 32.0);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);
    put_i32(&mut bytes, 2696, 3); // Mercury starts at word 3
    put_i32(&mut bytes, 2700, 1); // one coefficient per axis
    put_i32(&mut bytes, 2704, 2); // two granules per principal record
    put_i32(&mut bytes, 2844, 373); // unused last series pads to 3000 bytes
    put_i32(&mut bytes, 2848, 1);
    put_i32(&mut bytes, 2852, 1);
    for record in 0..2 {
        let base = (record + 2) * RECORD_BYTES;
        let tdb_start = start + record as f64 * 32.0;
        put_f64(&mut bytes, base, tdb_start);
        put_f64(&mut bytes, base + 8, tdb_start + 32.0);
        for granule in 0..2 {
            for axis in 0..3 {
                let at = base + (2 + granule * 3 + axis) * 8;
                put_f64(
                    &mut bytes,
                    at,
                    (100 * (record + 1) + 10 * granule + axis) as f64,
                );
            }
        }
    }
    bytes
}

fn expect_mercury(reader: &mut De441<Cursor<Vec<u8>>>, jd: f64, value: f64) {
    let state = reader
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(jd),
        )
        .unwrap();
    assert_eq!(state.epoch, JulianDateTdb(jd));
    assert_eq!(state.position_km, [value, value + 1.0, value + 2.0]);
    assert_eq!(state.velocity_km_per_day, [0.0; 3]);
}

#[test]
fn signed_zero_neighborhood_obeys_both_record_and_granule_boundaries() {
    let mut reader = De441::new(Cursor::new(two_records(-32.0))).unwrap();
    let previous_neg16 = f64::from_bits((-16.0_f64).to_bits() + 1);
    let next_neg16 = f64::from_bits((-16.0_f64).to_bits() - 1);
    let previous_pos16 = f64::from_bits((16.0_f64).to_bits() - 1);
    let next_pos16 = f64::from_bits((16.0_f64).to_bits() + 1);
    let tiny = f64::from_bits(1);
    assert!(previous_neg16 < -16.0 && next_neg16 > -16.0);
    assert!(previous_pos16 < 16.0 && next_pos16 > 16.0);

    for (jd, expected) in [
        (-32.0, 100.0),
        (previous_neg16, 100.0),
        (-16.0, 110.0),
        (next_neg16, 110.0),
        (-tiny, 110.0), // rounds to 0 after subtracting -32
        (0.0, 200.0),   // exact principal record boundary
        (tiny, 200.0),
        (previous_pos16, 200.0),
        (16.0, 210.0),
        (next_pos16, 210.0),
        (32.0, 210.0),  // inclusive final endpoint
        (-tiny, 110.0), // backward record-cache invalidation
        (-16.0, 110.0),
        (0.0, 200.0),
    ] {
        expect_mercury(&mut reader, jd, expected);
    }
}

#[test]
fn distant_negative_tdb_record_and_granule_boundaries_are_distinct() {
    let start = -3_027_231.5;
    let mut reader = De441::new(Cursor::new(two_records(start))).unwrap();
    for (delta, value) in [
        (0.0, 100.0),
        (16.0, 110.0),
        (32.0, 200.0),
        (48.0, 210.0),
        (64.0, 210.0),
        (16.0, 110.0),
    ] {
        expect_mercury(&mut reader, start + delta, value);
    }

    for jd in [start - 1.0, start + 65.0] {
        assert!(matches!(
            reader.state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd)
            ),
            Err(De441Error::OutsideCoverage { .. })
        ));
    }
    expect_mercury(&mut reader, start + 32.0, 200.0);
}
