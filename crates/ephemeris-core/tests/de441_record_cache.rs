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

use std::cell::Cell;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

fn f64_field(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn i32_field(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn fixture() -> Vec<u8> {
    const RECORD_BYTES: usize = 3000;
    let mut bytes = vec![0u8; RECORD_BYTES * 4];
    bytes[..29].copy_from_slice(b"JPL Planetary Ephemeris DE441");
    f64_field(&mut bytes, 2652, 1000.0);
    f64_field(&mut bytes, 2660, 1064.0);
    f64_field(&mut bytes, 2668, 32.0);
    f64_field(&mut bytes, 2688, 81.0);
    i32_field(&mut bytes, 2840, 441);
    // Mercury T0 only, and a final pointer that fixes record size.
    for (index, first) in [(0, 3), (12, 373)] {
        let offset = if index == 12 { 2844 } else { 2696 };
        i32_field(&mut bytes, offset, first);
        i32_field(&mut bytes, offset + 4, 1);
        i32_field(&mut bytes, offset + 8, 1);
    }
    for record in 0..2 {
        let offset = (record + 2) * RECORD_BYTES;
        f64_field(&mut bytes, offset, 1000.0 + record as f64 * 32.0);
        f64_field(&mut bytes, offset + 8, 1032.0 + record as f64 * 32.0);
        f64_field(&mut bytes, offset + 16, 100.0 + record as f64 * 100.0);
    }
    bytes
}

struct MeasuredCursor {
    inner: Cursor<Vec<u8>>,
    reads: Rc<Cell<usize>>,
}

impl Read for MeasuredCursor {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.reads.set(self.reads.get() + 1);
        self.inner.read(bytes)
    }
}

impl Seek for MeasuredCursor {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(position)
    }
}

#[test]
fn repeated_queries_in_same_record_do_not_read_from_source_again() {
    let reads = Rc::new(Cell::new(0));
    let source = MeasuredCursor {
        inner: Cursor::new(fixture()),
        reads: reads.clone(),
    };
    let mut eph = De441::new(source).unwrap();
    let header_reads = reads.get();
    assert!(header_reads > 0);

    for (jd, expected) in [(1000.0, 100.0), (1008.0, 100.0), (1031.0, 100.0)] {
        let state = eph
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd),
            )
            .unwrap();
        assert_eq!(state.position_km[0], expected);
    }
    let first_record_reads = reads.get();
    assert_eq!(first_record_reads, header_reads + 1);

    let second = eph
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1032.0),
        )
        .unwrap();
    assert_eq!(second.position_km[0], 200.0);
    assert_eq!(reads.get(), first_record_reads + 1);

    eph.state(
        Body::Mercury,
        Body::SolarSystemBarycenter,
        JulianDateTdb(1064.0),
    )
    .unwrap();
    assert_eq!(reads.get(), first_record_reads + 1);

    let back = eph
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1016.0),
        )
        .unwrap();
    assert_eq!(back.position_km[0], 100.0);
    assert_eq!(reads.get(), first_record_reads + 2);
}
