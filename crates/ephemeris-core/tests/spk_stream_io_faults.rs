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

//! Synthetic read/seek failure regressions for the public DAF/SPK interfaces.
//!
//! Modification (2026-11-14): verify interrupted/short binary reads, EOF,
//! and an I/O error from the selected coefficient record. No JPL data is used.
//! Modification (2026-11-14): multi-kernel load failure and selected-source
//! I/O errors must preserve priority, provenance and rollback guarantees.

use std::error::Error;
use std::io::{self, Cursor, Read, Seek, SeekFrom};

use virgostars_ephemeris_core::spk_direct_state::{direct_spk_state, SpkStateError};
use virgostars_ephemeris_core::spk_index::{inspect_spk, SpkByteOrder, SpkIndexError};
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;
use virgostars_ephemeris_core::spk_relative_state::SpkRelativeError;

const RECORD_BYTES: usize = 1024;
const FIRST_DATA_WORD: usize = 513;

fn write_i32(bytes: &mut [u8], at: usize, value: i32, order: SpkByteOrder) {
    let word = match order {
        SpkByteOrder::LittleEndian => value.to_le_bytes(),
        SpkByteOrder::BigEndian => value.to_be_bytes(),
    };
    bytes[at..at + 4].copy_from_slice(&word);
}

fn write_f64(bytes: &mut [u8], at: usize, value: f64, order: SpkByteOrder) {
    let word = match order {
        SpkByteOrder::LittleEndian => value.to_le_bytes(),
        SpkByteOrder::BigEndian => value.to_be_bytes(),
    };
    bytes[at..at + 8].copy_from_slice(&word);
}

/// A single well-formed, synthetic SPK Type 2 or 3 segment in a 5-record DAF.
/// The coefficient bytes are NOT derived from a NASA/JPL ephemeris.
fn synthetic_spk(order: SpkByteOrder, kind: i32) -> Vec<u8> {
    assert!(kind == 2 || kind == 3);
    let components = if kind == 2 { 3 } else { 6 };
    let record_words = 2 + 2 * components;
    let final_address = FIRST_DATA_WORD + record_words + 3;
    let mut bytes = vec![0; 5 * RECORD_BYTES];
    bytes[0..8].copy_from_slice(b"DAF/SPK ");
    bytes[88..96].copy_from_slice(match order {
        SpkByteOrder::LittleEndian => b"LTL-IEEE",
        SpkByteOrder::BigEndian => b"BIG-IEEE",
    });
    write_i32(&mut bytes, 8, 2, order);
    write_i32(&mut bytes, 12, 6, order);
    write_i32(&mut bytes, 76, 3, order);
    write_i32(&mut bytes, 80, 3, order);
    write_i32(&mut bytes, 84, (final_address + 1) as i32, order);

    let summary = 2 * RECORD_BYTES;
    write_f64(&mut bytes, summary + 16, 1.0, order);
    write_f64(&mut bytes, summary + 24, 0.0, order);
    write_f64(&mut bytes, summary + 32, 10.0, order);
    for (i, n) in [
        301,
        399,
        1,
        kind,
        FIRST_DATA_WORD as i32,
        final_address as i32,
    ]
    .into_iter()
    .enumerate()
    {
        write_i32(&mut bytes, summary + 40 + 4 * i, n, order);
    }

    let record_start = (FIRST_DATA_WORD - 1) * 8;
    write_f64(&mut bytes, record_start, 5.0, order);
    write_f64(&mut bytes, record_start + 8, 5.0, order);
    let coefficients = [
        [100.0, 10.0],
        [20.0, 0.0],
        [30.0, 0.0],
        [4.0, 0.0],
        [5.0, 0.0],
        [6.0, 0.0],
    ];
    for (component, pair) in coefficients.iter().take(components).enumerate() {
        for (degree, &coefficient) in pair.iter().enumerate() {
            write_f64(
                &mut bytes,
                record_start + (2 + 2 * component + degree) * 8,
                coefficient,
                order,
            );
        }
    }
    let directory = (final_address - 4) * 8;
    for (index, value) in [0.0, 10.0, record_words as f64, 1.0]
        .into_iter()
        .enumerate()
    {
        write_f64(&mut bytes, directory + 8 * index, value, order);
    }
    bytes
}

/// A seekable stream which can fragment reads, inject one Interrupted, hide a
/// file tail, or deny reads from a byte region while allowing independent seeks.
struct InjectedReader {
    inner: Cursor<Vec<u8>>,
    chunk: usize,
    interrupt_once: bool,
    eof_at: Option<u64>,
    deny: Option<(u64, u64)>,
}

impl InjectedReader {
    fn new(data: Vec<u8>, chunk: usize) -> Self {
        assert!(chunk > 0);
        Self {
            inner: Cursor::new(data),
            chunk,
            interrupt_once: false,
            eof_at: None,
            deny: None,
        }
    }
}

impl Read for InjectedReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.interrupt_once {
            self.interrupt_once = false;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        let pos = self.inner.position();
        if let Some((start, end)) = self.deny {
            if start <= pos && pos < end {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
        }
        let mut amount = buf.len().min(self.chunk);
        if let Some(cutoff) = self.eof_at {
            if pos >= cutoff {
                return Ok(0);
            }
            amount = amount.min((cutoff - pos) as usize);
        }
        if let Some((start, _)) = self.deny {
            if pos < start {
                amount = amount.min((start - pos) as usize);
            }
        }
        self.inner.read(&mut buf[..amount])
    }
}

impl Seek for InjectedReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.inner.seek(from)
    }
}

#[test]
fn interrupted_and_chunked_reads_preserve_index_and_direct_xyz() {
    for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
        for kind in [2, 3] {
            let mut reader = InjectedReader::new(synthetic_spk(order, kind), 7);
            reader.interrupt_once = true;
            let index = inspect_spk(&mut reader).unwrap();
            assert_eq!(index.byte_order, order);
            assert_eq!(index.segments.len(), 1);
            assert_eq!(index.segments[0].target_id, 301);
            let state = direct_spk_state(&mut reader, &index, 301, 5.0).unwrap();
            assert_eq!(state.center_id, 399);
            assert_eq!(state.tdb_seconds_past_j2000, 5.0);
            assert_eq!(state.position_km, [100.0, 20.0, 30.0]);
            assert_eq!(
                state.velocity_km_per_second,
                if kind == 2 {
                    [2.0, 0.0, 0.0]
                } else {
                    [4.0, 5.0, 6.0]
                }
            );
        }
    }
}

#[test]
fn summary_read_permission_failure_preserves_typed_io_error_source() {
    for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
        let mut reader = InjectedReader::new(synthetic_spk(order, 2), 7);
        reader.deny = Some((2 * RECORD_BYTES as u64, 3 * RECORD_BYTES as u64));
        let failure = inspect_spk(&mut reader).unwrap_err();
        assert!(matches!(
            &failure,
            SpkIndexError::Io(error) if error.kind() == io::ErrorKind::PermissionDenied
        ));
        let cause = failure.source().expect("I/O source must be exposed");
        assert_eq!(
            cause.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::PermissionDenied
        );
    }
}

#[test]
fn incomplete_summary_read_returns_unexpected_eof_not_a_partial_index() {
    for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
        let mut reader = InjectedReader::new(synthetic_spk(order, 2), 17);
        reader.eof_at = Some(2 * RECORD_BYTES as u64 + 120);
        assert!(matches!(
            inspect_spk(&mut reader),
            Err(SpkIndexError::Io(ref error))
                if error.kind() == io::ErrorKind::UnexpectedEof
        ));
    }
}

#[test]
fn failed_selected_coefficient_read_returns_error_not_a_state() {
    for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
        for kind in [2, 3] {
            let data = synthetic_spk(order, kind);
            let index = inspect_spk(&mut Cursor::new(data.clone())).unwrap();
            let record_words = if kind == 2 { 8 } else { 14 };
            let start = (FIRST_DATA_WORD - 1) as u64 * 8;
            // Directory reads occur after this region, while the coefficient
            // record read must fail. Never substitute another state.
            let mut reader = InjectedReader::new(data, 13);
            reader.deny = Some((start, start + record_words * 8));
            let failure = direct_spk_state(&mut reader, &index, 301, 5.0).unwrap_err();
            assert!(matches!(
                &failure,
                SpkStateError::Io(error) if error.kind() == io::ErrorKind::PermissionDenied
            ));
            let cause = failure.source().expect("selected-record I/O cause");
            assert_eq!(
                cause.downcast_ref::<io::Error>().unwrap().kind(),
                io::ErrorKind::PermissionDenied
            );
        }
    }
}

#[test]
fn failed_kernel_load_from_io_keeps_existing_state_and_index() {
    for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
        for kind in [2, 3] {
            let mut kernels = SpkKernelSet::<InjectedReader>::new();
            kernels
                .load(InjectedReader::new(synthetic_spk(order, kind), 11))
                .unwrap();
            let baseline = kernels.relative_state(301, 399, 5.0).unwrap();

            // The second file's summary cannot be read. The candidate must
            // not enter the set and alter either the load count or priority.
            let mut denied = InjectedReader::new(synthetic_spk(order, kind), 7);
            denied.deny = Some((2 * RECORD_BYTES as u64, 3 * RECORD_BYTES as u64));
            let failure = kernels.load(denied).unwrap_err();
            assert!(matches!(
                &failure,
                SpkIndexError::Io(error)
                    if error.kind() == io::ErrorKind::PermissionDenied
            ));
            assert_eq!(
                failure
                    .source()
                    .unwrap()
                    .downcast_ref::<io::Error>()
                    .unwrap()
                    .kind(),
                io::ErrorKind::PermissionDenied
            );
            assert_eq!(kernels.kernel_count(), 1);
            assert_eq!(kernels.selected_segment(301, 5.0).unwrap().unwrap().0, 0);
            assert_eq!(kernels.relative_state(301, 399, 5.0).unwrap(), baseline);
        }
    }
}

#[test]
fn unreadable_high_priority_coefficient_never_falls_back_to_older_kernel() {
    for old_order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
        for kind in [2, 3] {
            let newer_order = match old_order {
                SpkByteOrder::LittleEndian => SpkByteOrder::BigEndian,
                SpkByteOrder::BigEndian => SpkByteOrder::LittleEndian,
            };
            let mut kernels = SpkKernelSet::<InjectedReader>::new();
            kernels
                .load(InjectedReader::new(synthetic_spk(old_order, kind), 31))
                .unwrap();
            let baseline = kernels.relative_state(301, 399, 5.0).unwrap();

            // Indexing the newer mixed-endian kernel succeeds: only its
            // selected coefficient record is unreadable. A valid older
            // record with identical target/coverage must NOT be substituted.
            let mut denied = InjectedReader::new(synthetic_spk(newer_order, kind), 13);
            let start = (FIRST_DATA_WORD - 1) as u64 * 8;
            let words = if kind == 2 { 8 } else { 14 };
            denied.deny = Some((start, start + words * 8));
            kernels.load(denied).unwrap();
            assert_eq!(kernels.kernel_count(), 2);
            assert_eq!(kernels.selected_segment(301, 5.0).unwrap().unwrap().0, 1);

            let direct_failure = kernels.direct_state(301, 5.0).unwrap_err();
            assert!(matches!(
                &direct_failure,
                SpkStateError::Io(error)
                    if error.kind() == io::ErrorKind::PermissionDenied
            ));
            assert_eq!(
                direct_failure
                    .source()
                    .unwrap()
                    .downcast_ref::<io::Error>()
                    .unwrap()
                    .kind(),
                io::ErrorKind::PermissionDenied
            );
            let relative_failure = kernels.relative_state(301, 399, 5.0).unwrap_err();
            assert!(matches!(
                &relative_failure,
                SpkRelativeError::Segment(SpkStateError::Io(error))
                    if error.kind() == io::ErrorKind::PermissionDenied
            ));
            assert_eq!(
                relative_failure
                    .source()
                    .unwrap()
                    .source()
                    .unwrap()
                    .downcast_ref::<io::Error>()
                    .unwrap()
                    .kind(),
                io::ErrorKind::PermissionDenied
            );

            // Unloading the failed overlay is an explicit caller action.
            // Only then should the originally valid state be restored.
            assert!(kernels.unload_last().is_some());
            assert_eq!(kernels.kernel_count(), 1);
            assert_eq!(kernels.relative_state(301, 399, 5.0).unwrap(), baseline);
        }
    }
}
