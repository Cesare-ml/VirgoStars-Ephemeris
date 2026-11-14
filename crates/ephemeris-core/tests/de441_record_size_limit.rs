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

// 2026-11-14: synthetic classic header, not a NASA/JPL numerical fixture.
fn fixture() -> Vec<u8> {
    let mut bytes = vec![0; RECORD_BYTES * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    put_f64(&mut bytes, 2652, 1000.0);
    put_f64(&mut bytes, 2660, 1032.0);
    put_f64(&mut bytes, 2668, 32.0);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);
    // The trailing pointer fixes the classic record length at 3000 bytes.
    put_i32(&mut bytes, 2844, 373);
    put_i32(&mut bytes, 2848, 1);
    put_i32(&mut bytes, 2852, 1);
    put_f64(&mut bytes, RECORD_BYTES * 2, 1000.0);
    put_f64(&mut bytes, RECORD_BYTES * 2 + 8, 1032.0);
    bytes
}

#[test]
fn normally_sized_classic_record_still_opens() {
    let reader = De441::new(Cursor::new(fixture())).unwrap();
    let (start, end) = reader.coverage();
    assert_eq!((start.0, end.0), (1000.0, 1032.0));
}

#[test]
fn oversized_pointer_is_rejected_before_file_length_or_cache_allocation() {
    // A forged pointer can request 16.8 GB if set to i32::MAX. The
    // record-size guard must run before checking a claimed file length.
    for first in [2_100_000, i32::MAX] {
        let mut bytes = fixture();
        put_i32(&mut bytes, 2844, first);
        assert!(matches!(
            De441::new(Cursor::new(bytes)),
            Err(De441Error::InvalidFile(
                "coefficient record exceeds supported size"
            ))
        ));
    }
}
