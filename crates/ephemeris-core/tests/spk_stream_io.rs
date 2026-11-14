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

// Added 2026-11-14: synthetic SPK read-error and retry regressions.
// No NASA/JPL kernel data or independent astronomical reference is included.

use std::io::{self, Cursor, Read, Seek, SeekFrom};

use virgostars_ephemeris_core::spk_direct_state::{direct_spk_state, SpkStateError};
use virgostars_ephemeris_core::spk_index::{inspect_spk, SpkByteOrder};

const RECORD_BYTES: usize = 1024;
const FIRST_DATA_WORD: usize = 513;
const LAST_DATA_WORD: usize = FIRST_DATA_WORD + 8;
const FOOTER_OFFSET: usize = (FIRST_DATA_WORD - 1 + 5) * 8;

fn write_i32(bytes: &mut [u8], offset: usize, value: i32, order: SpkByteOrder) {
    let encoded = match order {
        SpkByteOrder::LittleEndian => value.to_le_bytes(),
        SpkByteOrder::BigEndian => value.to_be_bytes(),
    };
    bytes[offset..offset + 4].copy_from_slice(&encoded);
}

fn write_f64(bytes: &mut [u8], offset: usize, value: f64, order: SpkByteOrder) {
    let encoded = match order {
        SpkByteOrder::LittleEndian => value.to_le_bytes(),
        SpkByteOrder::BigEndian => value.to_be_bytes(),
    };
    bytes[offset..offset + 8].copy_from_slice(&encoded);
}

fn synthetic_type2_kernel(order: SpkByteOrder) -> Vec<u8> {
    // DAF records: file header, reserved/comment, summary, names, data.
    // One Type-2 constant-position record: MID=5, RADIUS=5, XYZ=(42,3,-2).
    // RSIZE=5 words; the four-word directory follows the record.
    let mut bytes = vec![0_u8; 5 * RECORD_BYTES];
    bytes[..8].copy_from_slice(b"DAF/SPK ");
    bytes[88..96].copy_from_slice(match order {
        SpkByteOrder::LittleEndian => b"LTL-IEEE",
        SpkByteOrder::BigEndian => b"BIG-IEEE",
    });
    for (offset, value) in [
        (8, 2),
        (12, 6),
        (76, 3),
        (80, 3),
        (84, (LAST_DATA_WORD + 1) as i32),
    ] {
        write_i32(&mut bytes, offset, value, order);
    }

    let summary = 2 * RECORD_BYTES;
    write_f64(&mut bytes, summary + 16, 1.0, order);
    write_f64(&mut bytes, summary + 24, 0.0, order);
    write_f64(&mut bytes, summary + 32, 10.0, order);
    for (index, value) in [
        499,
        10,
        1,
        2,
        FIRST_DATA_WORD as i32,
        LAST_DATA_WORD as i32,
    ]
    .into_iter()
    .enumerate()
    {
        write_i32(&mut bytes, summary + 40 + 4 * index, value, order);
    }
    let data_start = (FIRST_DATA_WORD - 1) * 8;
    for (index, value) in [5.0, 5.0, 42.0, 3.0, -2.0, 0.0, 10.0, 5.0, 1.0]
        .into_iter()
        .enumerate()
    {
        write_f64(&mut bytes, data_start + 8 * index, value, order);
    }
    bytes
}

struct FaultOnce {
    source: Cursor<Vec<u8>>,
    fault: Option<(u64, io::ErrorKind)>,
}

impl Read for FaultOnce {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if let Some((offset, kind)) = self.fault {
            if self.source.position() == offset {
                self.fault = None;
                return Err(io::Error::new(kind, "synthetic SPK read fault"));
            }
        }
        self.source.read(buffer)
    }
}

impl Seek for FaultOnce {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.source.seek(position)
    }
}

fn verify_constant_state(
    state: virgostars_ephemeris_core::spk_direct_state::SpkDirectState,
) {
    assert_eq!(state.target_id, 499);
    assert_eq!(state.center_id, 10);
    assert_eq!(state.frame_id, 1);
    assert_eq!(state.tdb_seconds_past_j2000, 5.0);
    assert_eq!(state.position_km, [42.0, 3.0, -2.0]);
    assert_eq!(state.velocity_km_per_second, [0.0; 3]);
}

#[test]
fn truncated_directory_fails_closed_and_restoring_bytes_recovers() {
    for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
        let original = synthetic_type2_kernel(order);
        let mut source = Cursor::new(original.clone());
        let index = inspect_spk(&mut source).expect("valid synthetic index");
        verify_constant_state(direct_spk_state(&mut source, &index, 499, 5.0).unwrap());

        // Simulate file truncation AFTER indexing, partway through the
        // four-word Type-2 directory. A stale index is never proof of bytes.
        source.get_mut().truncate(FOOTER_OFFSET + 15);
        let error = direct_spk_state(&mut source, &index, 499, 5.0).unwrap_err();
        assert!(matches!(
            error,
            SpkStateError::Io(ref cause)
                if cause.kind() == io::ErrorKind::UnexpectedEof
        ));

        // Every lookup seeks to its own directory/record offsets; a partial
        // prior read must not poison the next call when bytes are restored.
        *source.get_mut() = original;
        verify_constant_state(direct_spk_state(&mut source, &index, 499, 5.0).unwrap());
    }
}

#[test]
fn one_off_record_io_error_is_reported_and_subsequent_lookup_recovers() {
    for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
        let mut source = FaultOnce {
            source: Cursor::new(synthetic_type2_kernel(order)),
            fault: None,
        };
        let index = inspect_spk(&mut source).expect("valid synthetic index");
        source.fault = Some(((FIRST_DATA_WORD - 1) as u64 * 8, io::ErrorKind::BrokenPipe));

        let error = direct_spk_state(&mut source, &index, 499, 5.0).unwrap_err();
        assert!(matches!(
            error,
            SpkStateError::Io(ref cause)
                if cause.kind() == io::ErrorKind::BrokenPipe
        ));
        verify_constant_state(direct_spk_state(&mut source, &index, 499, 5.0).unwrap());
    }
}

#[test]
fn interrupted_read_is_retried_by_read_exact_without_state_corruption() {
    for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
        let mut source = FaultOnce {
            source: Cursor::new(synthetic_type2_kernel(order)),
            fault: None,
        };
        let index = inspect_spk(&mut source).expect("valid synthetic index");
        source.fault = Some(((FIRST_DATA_WORD - 1) as u64 * 8, io::ErrorKind::Interrupted));
        verify_constant_state(direct_spk_state(&mut source, &index, 499, 5.0).unwrap());
        assert!(source.fault.is_none(), "the synthetic interruption was injected");
    }
}
