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

/// A reader that opens normally but fails on later reads, modelling removable
/// media or a damaged stream without requiring a real DE441 file.
struct FailAfterOpen {
    inner: Cursor<Vec<u8>>,
    fail_reads: std::rc::Rc<std::cell::Cell<bool>>,
}

impl std::io::Read for FailAfterOpen {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.fail_reads.get() {
            return Err(std::io::Error::other("injected record read failure"));
        }
        std::io::Read::read(&mut self.inner, buf)
    }
}

impl std::io::Seek for FailAfterOpen {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.inner, pos)
    }
}

#[test]
fn propagates_record_io_failure_without_returning_a_partial_state() {
    let fail_reads = std::rc::Rc::new(std::cell::Cell::new(false));
    let reader = FailAfterOpen {
        inner: Cursor::new(fixture()),
        fail_reads: fail_reads.clone(),
    };
    let mut eph = De441::new(reader).expect("synthetic file should open");
    fail_reads.set(true);
    let err = eph
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1010.0),
        )
        .expect_err("record read failure must not return a state");
    assert!(matches!(err, De441Error::Io(ref e) if e.kind() == std::io::ErrorKind::Other));

    // A failed read must not poison the reader's ability to serve later epochs.
    fail_reads.set(false);
    let recovered = eph
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1010.0),
        )
        .expect("reader should recover once the injected failure is removed");
    assert!(recovered.position_km.iter().all(|x| x.is_finite()));
    assert!(recovered.velocity_km_per_day.iter().all(|x| x.is_finite()));
}
