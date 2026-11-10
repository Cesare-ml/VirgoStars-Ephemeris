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

const RECORD_BYTES: usize = 3000;

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// One 32-day coefficient record, plus the two mandatory header records.
// The unused libration pointer forces a 3000-byte record without depending
// on a downloaded DE441 file. This is a format test, not an astronomy oracle.
fn valid_binary() -> Vec<u8> {
    let mut bytes = vec![0; RECORD_BYTES * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    put_f64(&mut bytes, 2652, 1000.0);
    put_f64(&mut bytes, 2660, 1032.0);
    put_f64(&mut bytes, 2668, 32.0);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);
    put_i32(&mut bytes, 2844, 373);
    put_i32(&mut bytes, 2848, 1);
    put_i32(&mut bytes, 2852, 1);
    put_f64(&mut bytes, RECORD_BYTES * 2, 1000.0);
    put_f64(&mut bytes, RECORD_BYTES * 2 + 8, 1032.0);
    bytes
}

fn assert_invalid(bytes: Vec<u8>) {
    assert!(matches!(
        De441::new(Cursor::new(bytes)),
        Err(De441Error::InvalidFile(_))
    ));
}

#[test]
fn accepts_minimal_consistent_header_and_inclusive_coverage() {
    let ephemeris = De441::new(Cursor::new(valid_binary())).unwrap();
    let (start, end) = ephemeris.coverage();
    assert_eq!((start.0, end.0), (1000.0, 1032.0));
}

#[test]
fn rejects_non_finite_and_non_positive_header_fields() {
    for (offset, value) in [
        (2652, f64::NAN),
        (2660, f64::INFINITY),
        (2668, 0.0),
        (2668, -32.0),
        (2688, f64::NAN),
        (2688, 0.0),
        (2688, -1.0),
    ] {
        let mut bytes = valid_binary();
        put_f64(&mut bytes, offset, value);
        assert_invalid(bytes);
    }
}

#[test]
fn rejects_fractional_or_reversed_record_coverage() {
    for (offset, value) in [
        (2660, 1031.0), // 31 days is not an integer multiple of 32.
        (2660, 1000.0),
        (2660, 999.0),
        (2668, 64.0), // Fewer than one complete coefficient record.
    ] {
        let mut bytes = valid_binary();
        put_f64(&mut bytes, offset, value);
        assert_invalid(bytes);
    }
}

#[test]
fn rejects_short_header_and_mismatched_file_lengths() {
    let mut bytes = valid_binary();
    bytes.truncate(2855);
    assert!(matches!(
        De441::new(Cursor::new(bytes)),
        Err(De441Error::Io(_))
    ));

    let mut bytes = valid_binary();
    bytes.truncate(RECORD_BYTES * 3 - 1);
    assert_invalid(bytes);

    let mut bytes = valid_binary();
    bytes.push(0);
    assert_invalid(bytes);
}

#[test]
fn rejects_invalid_pointer_shape_and_de_version() {
    for (offset, value) in [
        (2840, 440), // Not DE441.
        (2844, 2),   // Coefficients cannot overlap record date words.
        (2848, 0),   // Partial pointer is invalid.
        (2852, 0),
        (2852, 101), // Granule count outside supported bound.
    ] {
        let mut bytes = valid_binary();
        put_i32(&mut bytes, offset, value);
        assert_invalid(bytes);
    }
}
