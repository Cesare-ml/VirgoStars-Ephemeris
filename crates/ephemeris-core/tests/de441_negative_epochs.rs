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
use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};

const RECORD_BYTES: usize = 3000;
const START_TDB_JD: f64 = -64.0;
const END_TDB_JD: f64 = 64.0;

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// 2026-11-14: Four entirely synthetic 32-day classic records that straddle
// negative JD, JD zero and positive JD. Mercury has two 16-day granules in
// each record, with T0 = 100*record + 10*granule + axis, T1 = 2.
// This fixture is not an independent NASA/JPL astronomical reference.
fn fixture() -> Vec<u8> {
    let mut bytes = vec![0; RECORD_BYTES * 6];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    put_f64(&mut bytes, 2652, START_TDB_JD);
    put_f64(&mut bytes, 2660, END_TDB_JD);
    put_f64(&mut bytes, 2668, 32.0);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);

    // Mercury pointer: 3 axes, 2 coefficients, 2 granules.
    put_i32(&mut bytes, 2696, 3);
    put_i32(&mut bytes, 2700, 2);
    put_i32(&mut bytes, 2704, 2);

    // Unused last pointer forces a record size of 3,000 bytes.
    put_i32(&mut bytes, 2844, 373);
    put_i32(&mut bytes, 2848, 1);
    put_i32(&mut bytes, 2852, 1);

    for record in 0..4 {
        let base = RECORD_BYTES * (record + 2);
        let record_start = START_TDB_JD + record as f64 * 32.0;
        put_f64(&mut bytes, base, record_start);
        put_f64(&mut bytes, base + 8, record_start + 32.0);
        for granule in 0..2 {
            for axis in 0..3 {
                let offset = base + (2 + (granule * 3 + axis) * 2) * 8;
                put_f64(
                    &mut bytes,
                    offset,
                    (record * 100 + granule * 10 + axis) as f64,
                );
                put_f64(&mut bytes, offset + 8, 2.0);
            }
        }
    }
    bytes
}

fn mercury(eph: &mut De441<Cursor<Vec<u8>>>, jd: f64) -> [f64; 3] {
    let state = eph
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(jd),
        )
        .unwrap();
    assert_eq!(state.epoch, JulianDateTdb(jd));
    assert_eq!(state.velocity_km_per_day, [0.25; 3]);
    state.position_km
}

#[test]
fn covers_negative_zero_and_positive_tdb_julian_dates() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    assert_eq!(
        eph.coverage(),
        (JulianDateTdb(START_TDB_JD), JulianDateTdb(END_TDB_JD))
    );
    // At each granule start Chebyshev x=-1, so position=T0-2.
    // At the inclusive last endpoint x=+1, so position=T0+2.
    for (jd, expected) in [
        (-64.0, [-2.0, -1.0, 0.0]),
        (-48.0, [8.0, 9.0, 10.0]),
        (-32.0, [98.0, 99.0, 100.0]),
        (-16.0, [108.0, 109.0, 110.0]),
        (0.0, [198.0, 199.0, 200.0]),
        (16.0, [208.0, 209.0, 210.0]),
        (32.0, [298.0, 299.0, 300.0]),
        (48.0, [308.0, 309.0, 310.0]),
        (64.0, [312.0, 313.0, 314.0]),
    ] {
        assert_eq!(mercury(&mut eph, jd), expected, "unexpected JD {jd}");
    }
}

#[test]
fn negative_record_boundary_is_left_open_and_right_selected() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();

    // At JD -32.25 the previous record's last granule has x=0.96875.
    assert_eq!(mercury(&mut eph, -32.25), [11.9375, 12.9375, 13.9375]);
    // The exact boundary chooses the next record's first granule.
    assert_eq!(mercury(&mut eph, -32.0), [98.0, 99.0, 100.0]);
    // Moving backwards after reading a positive JD must reload the first
    // record rather than reusing the wrong cached granule.
    assert_eq!(mercury(&mut eph, 33.0), [298.25, 299.25, 300.25]);
    assert_eq!(mercury(&mut eph, -63.0), [-1.75, -0.75, 0.25]);
}

#[test]
fn negative_out_of_coverage_does_not_clamp_to_first_record() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    for jd in [-64.25, 64.25, f64::NEG_INFINITY, f64::NAN] {
        assert!(
            matches!(
                eph.state(
                    Body::Mercury,
                    Body::SolarSystemBarycenter,
                    JulianDateTdb(jd)
                ),
                Err(De441Error::OutsideCoverage { .. })
            ),
            "unexpected acceptance of JD {jd}"
        );
    }
    // An invalid lookup must not poison a later valid lookup.
    assert_eq!(mercury(&mut eph, -64.0), [-2.0, -1.0, 0.0]);
}
