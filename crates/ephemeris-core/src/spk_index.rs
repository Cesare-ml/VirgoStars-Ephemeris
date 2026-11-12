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

//! Read-only NAIF DAF/SPK segment indexing for an externally supplied .bsp.
//!
//! This module reads file-record and summary-record metadata only. It does NOT
//! evaluate SPK type-2/type-3 Chebyshev polynomials or produce astronomical
//! states. Segment order is preserved for later priority selection.
//!
//! Format: <https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/daf.html>
//! SPK: <https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html>

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};

const RECORD_BYTES: usize = 1024;
const WORD_BYTES: usize = 8;
const SUMMARY_WORDS: usize = 5;
const MAX_SUMMARIES: usize = 25;

/// Byte ordering as declared in the DAF file record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpkByteOrder {
    LittleEndian,
    BigEndian,
}

/// One DAF/SPK segment descriptor, without interpolated position data.
///
/// TDB epochs are seconds past J2000; addresses are 1-based DAF 64-bit words.
/// These fields record what the file declares, not independently verified
/// physical data or support for the segment's reference frame / SPK type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpkSegmentDescriptor {
    pub start_tdb_seconds: f64,
    pub end_tdb_seconds: f64,
    pub target_id: i32,
    pub center_id: i32,
    pub frame_id: i32,
    pub spk_type: i32,
    pub initial_address: u64,
    pub final_address: u64,
}

/// Index of SPK metadata in the original, forward file order.
///
/// A later resolver must select the *last covering* eligible segment and
/// reject unsupported types/frames rather than silently falling back.
#[derive(Clone, Debug, PartialEq)]
pub struct SpkIndex {
    pub byte_order: SpkByteOrder,
    pub segments: Vec<SpkSegmentDescriptor>,
}

#[derive(Debug)]
pub enum SpkIndexError {
    Io(io::Error),
    InvalidFile(&'static str),
}

impl fmt::Display for SpkIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "SPK index I/O error: {error}"),
            Self::InvalidFile(message) => write!(f, "invalid DAF/SPK file: {message}"),
        }
    }
}

impl Error for SpkIndexError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::InvalidFile(_) => None,
        }
    }
}

impl From<io::Error> for SpkIndexError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn integer(bytes: &[u8], offset: usize, order: SpkByteOrder) -> i32 {
    let word: [u8; 4] = bytes[offset..offset + 4].try_into().expect("fixed word");
    match order {
        SpkByteOrder::LittleEndian => i32::from_le_bytes(word),
        SpkByteOrder::BigEndian => i32::from_be_bytes(word),
    }
}

fn double(bytes: &[u8], offset: usize, order: SpkByteOrder) -> f64 {
    let word: [u8; 8] = bytes[offset..offset + 8].try_into().expect("fixed double");
    match order {
        SpkByteOrder::LittleEndian => f64::from_le_bytes(word),
        SpkByteOrder::BigEndian => f64::from_be_bytes(word),
    }
}

/// A DAF summary-list pointer/count stored as an exact integer in f64.
/// The caller decides whether zero is allowed and checks record bounds.
fn exact_nonnegative_integer(value: f64) -> Option<u64> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value >= 9_007_199_254_740_992.0
    {
        None
    } else {
        Some(value as u64)
    }
}

/// Read and validate the DAF/SPK metadata index, without loading SPK data.
///
/// - Supports NAIF \`DAF/SPK \` with ND=2, NI=6 and IEEE little/big endian.
/// - Traverses forward summary records, checking links, counts and bounds.
/// - Does not assume any particular segment frame or type is computable.
/// - Preserves descriptor ordering for later NAIF priority semantics.
/// - Rejects partial records, malformed pointers, nonfinite epochs and
///   segment address spans beyond the occupied DAF data.
///
/// This function makes no network requests and needs no bundled kernels.
pub fn inspect_spk<R: Read + Seek>(source: &mut R) -> Result<SpkIndex, SpkIndexError> {
    let size = source.seek(SeekFrom::End(0))?;
    if size < RECORD_BYTES as u64 * 3 || size % RECORD_BYTES as u64 != 0 {
        return Err(SpkIndexError::InvalidFile(
            "invalid DAF record alignment or size",
        ));
    }
    let file_records = size / RECORD_BYTES as u64;
    let file_words = size / WORD_BYTES as u64;
    source.seek(SeekFrom::Start(0))?;
    let mut file_record = [0u8; RECORD_BYTES];
    source.read_exact(&mut file_record)?;
    if &file_record[..8] != b"DAF/SPK " {
        return Err(SpkIndexError::InvalidFile(
            "expected DAF/SPK identification word",
        ));
    }
    let order = match &file_record[88..96] {
        b"LTL-IEEE" => SpkByteOrder::LittleEndian,
        b"BIG-IEEE" => SpkByteOrder::BigEndian,
        _ => return Err(SpkIndexError::InvalidFile("unsupported DAF binary format")),
    };
    if integer(&file_record, 8, order) != 2 || integer(&file_record, 12, order) != 6 {
        return Err(SpkIndexError::InvalidFile("SPK requires ND=2 and NI=6"));
    }
    let first = integer(&file_record, 76, order);
    let last = integer(&file_record, 80, order);
    let free = integer(&file_record, 84, order);
    if first < 2
        || last < 2
        || free < 2
        || first as u64 >= file_records
        || last as u64 >= file_records
        || free as u64 > file_words + 1
    {
        return Err(SpkIndexError::InvalidFile(
            "invalid DAF summary or free pointers",
        ));
    }
    let mut visited = HashSet::new();
    let mut segments = Vec::new();
    let mut current = first as u64;
    let mut previous = 0u64;
    loop {
        if current < 2 || current >= file_records || !visited.insert(current) {
            return Err(SpkIndexError::InvalidFile(
                "cyclic or invalid DAF summary link",
            ));
        }
        // DAF summary records are immediately followed by name records.
        let offset = (current - 1) * RECORD_BYTES as u64;
        source.seek(SeekFrom::Start(offset))?;
        let mut record = [0u8; RECORD_BYTES];
        source.read_exact(&mut record)?;
        let next = exact_nonnegative_integer(double(&record, 0, order))
            .ok_or(SpkIndexError::InvalidFile("invalid next summary pointer"))?;
        let prev = exact_nonnegative_integer(double(&record, 8, order)).ok_or(
            SpkIndexError::InvalidFile("invalid previous summary pointer"),
        )?;
        let count = exact_nonnegative_integer(double(&record, 16, order))
            .ok_or(SpkIndexError::InvalidFile("invalid DAF summary count"))?;
        if prev != previous || count > MAX_SUMMARIES as u64 {
            return Err(SpkIndexError::InvalidFile(
                "inconsistent DAF summary chain or count",
            ));
        }
        if count == 0 && (next != 0 || segments.is_empty()) {
            return Err(SpkIndexError::InvalidFile(
                "unexpected empty DAF summary record",
            ));
        }
        for index in 0..count as usize {
            let offset = (3 + SUMMARY_WORDS * index) * WORD_BYTES;
            let start_tdb_seconds = double(&record, offset, order);
            let end_tdb_seconds = double(&record, offset + 8, order);
            let ints = offset + 16;
            let initial = integer(&record, ints + 16, order);
            let final_address = integer(&record, ints + 20, order);
            if !start_tdb_seconds.is_finite()
                || !end_tdb_seconds.is_finite()
                || end_tdb_seconds < start_tdb_seconds
                || initial < 129
                || final_address < initial
                || final_address as u64 >= free as u64
                || final_address as u64 > file_words
            {
                return Err(SpkIndexError::InvalidFile("invalid SPK segment descriptor"));
            }
            let spk_type = integer(&record, ints + 12, order);
            if spk_type <= 0 {
                return Err(SpkIndexError::InvalidFile("invalid SPK segment type"));
            }
            segments.push(SpkSegmentDescriptor {
                start_tdb_seconds,
                end_tdb_seconds,
                target_id: integer(&record, ints, order),
                center_id: integer(&record, ints + 4, order),
                frame_id: integer(&record, ints + 8, order),
                spk_type,
                initial_address: initial as u64,
                final_address: final_address as u64,
            });
        }
        if next == 0 {
            if current != last as u64 {
                return Err(SpkIndexError::InvalidFile("last summary pointer mismatch"));
            }
            break;
        }
        previous = current;
        current = next;
    }
    Ok(SpkIndex {
        byte_order: order,
        segments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn put_i32(buf: &mut [u8], offset: usize, value: i32, order: SpkByteOrder) {
        let bytes = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        buf[offset..offset + 4].copy_from_slice(&bytes);
    }

    fn put_f64(buf: &mut [u8], offset: usize, value: f64, order: SpkByteOrder) {
        let bytes = match order {
            SpkByteOrder::LittleEndian => value.to_le_bytes(),
            SpkByteOrder::BigEndian => value.to_be_bytes(),
        };
        buf[offset..offset + 8].copy_from_slice(&bytes);
    }

    fn write_segment(
        buf: &mut [u8],
        record: usize,
        order: SpkByteOrder,
        target: i32,
        start: f64,
        end: f64,
        data: i32,
    ) {
        let offset = (record - 1) * RECORD_BYTES + 3 * WORD_BYTES;
        put_f64(buf, offset, start, order);
        put_f64(buf, offset + 8, end, order);
        for (i, value) in [target, 0, 1, 2, data, data + 3].into_iter().enumerate() {
            put_i32(buf, offset + 16 + i * 4, value, order);
        }
    }

    fn file(order: SpkByteOrder, chained: bool) -> Vec<u8> {
        // Record 1: file header; record 2: reserved; record 3: summary;
        // record 4: names; record 5: synthetic coefficient area.
        // With chained summaries: record 6 summary, 7 names, 8 data.
        let mut b = vec![
            0;
            if chained {
                8 * RECORD_BYTES
            } else {
                5 * RECORD_BYTES
            }
        ];
        b[0..8].copy_from_slice(b"DAF/SPK ");
        b[88..96].copy_from_slice(match order {
            SpkByteOrder::LittleEndian => b"LTL-IEEE",
            SpkByteOrder::BigEndian => b"BIG-IEEE",
        });
        put_i32(&mut b, 8, 2, order);
        put_i32(&mut b, 12, 6, order);
        put_i32(&mut b, 76, 3, order);
        put_i32(&mut b, 80, if chained { 6 } else { 3 }, order);
        put_i32(&mut b, 84, if chained { 901 } else { 517 }, order);
        put_f64(
            &mut b,
            2 * RECORD_BYTES,
            if chained { 6.0 } else { 0.0 },
            order,
        );
        put_f64(&mut b, 2 * RECORD_BYTES + 8, 0.0, order);
        put_f64(&mut b, 2 * RECORD_BYTES + 16, 1.0, order);
        write_segment(&mut b, 3, order, 399, -10.0, 100.0, 513);
        if chained {
            put_f64(&mut b, 5 * RECORD_BYTES, 0.0, order);
            put_f64(&mut b, 5 * RECORD_BYTES + 8, 3.0, order);
            put_f64(&mut b, 5 * RECORD_BYTES + 16, 1.0, order);
            write_segment(&mut b, 6, order, 301, 0.0, 300.0, 897);
        }
        b
    }

    fn parse(bytes: Vec<u8>) -> Result<SpkIndex, SpkIndexError> {
        inspect_spk(&mut Cursor::new(bytes))
    }

    #[test]
    fn indexes_both_ieee_endiannesses_without_reordering_segments() {
        for order in [SpkByteOrder::LittleEndian, SpkByteOrder::BigEndian] {
            let result = parse(file(order, true)).unwrap();
            assert_eq!(result.byte_order, order);
            assert_eq!(result.segments.len(), 2);
            let earth = result.segments[0];
            assert_eq!(earth.target_id, 399);
            assert_eq!(earth.center_id, 0);
            assert_eq!(earth.frame_id, 1);
            assert_eq!(earth.spk_type, 2);
            assert_eq!(
                (earth.start_tdb_seconds, earth.end_tdb_seconds),
                (-10.0, 100.0)
            );
            assert_eq!((earth.initial_address, earth.final_address), (513, 516));
            let moon = result.segments[1];
            assert_eq!(moon.target_id, 301);
            assert_eq!((moon.initial_address, moon.final_address), (897, 900));
        }
    }

    #[test]
    fn rejects_bad_magic_binary_format_dimensions_and_incomplete_files() {
        let order = SpkByteOrder::LittleEndian;
        let mut b = file(order, false);
        b[0..8].copy_from_slice(b"DAF/PCK ");
        assert!(parse(b).is_err());
        let mut b = file(order, false);
        b[88..96].copy_from_slice(b"VAX-GFLT");
        assert!(parse(b).is_err());
        let mut b = file(order, false);
        put_i32(&mut b, 12, 5, order);
        assert!(parse(b).is_err());
        let mut b = file(order, false);
        b.pop();
        assert!(parse(b).is_err());
    }

    #[test]
    fn rejects_corrupt_record_pointers_and_cycles() {
        let order = SpkByteOrder::LittleEndian;
        for (offset, value) in [(76, -1), (80, 5), (84, 999_999)] {
            let mut b = file(order, false);
            put_i32(&mut b, offset, value, order);
            assert!(parse(b).is_err());
        }
        let mut b = file(order, true);
        put_f64(&mut b, 5 * RECORD_BYTES, 3.0, order);
        assert!(parse(b).is_err(), "cyclic summary link must fail");
        let mut b = file(order, true);
        put_f64(&mut b, 5 * RECORD_BYTES + 8, 0.0, order);
        assert!(parse(b).is_err(), "broken previous summary link must fail");
    }

    #[test]
    fn rejects_noninteger_summary_controls_and_overfull_counts() {
        let order = SpkByteOrder::BigEndian;
        for (offset, value) in [(0, f64::NAN), (0, 3.5), (16, 26.0), (16, f64::INFINITY)] {
            let mut b = file(order, true);
            put_f64(&mut b, 2 * RECORD_BYTES + offset, value, order);
            assert!(parse(b).is_err());
        }
    }

    #[test]
    fn rejects_invalid_coverage_data_address_and_type() {
        let order = SpkByteOrder::LittleEndian;
        let record = 2 * RECORD_BYTES + 3 * WORD_BYTES;
        let mut b = file(order, false);
        put_f64(&mut b, record, f64::NAN, order);
        assert!(parse(b).is_err());
        let mut b = file(order, false);
        put_f64(&mut b, record + 8, -20.0, order);
        assert!(parse(b).is_err());
        let mut b = file(order, false);
        put_i32(&mut b, record + 16 + 16, 9000, order);
        assert!(parse(b).is_err());
        let mut b = file(order, false);
        put_i32(&mut b, record + 16 + 12, 0, order);
        assert!(parse(b).is_err());
    }

    #[test]
    fn accepts_metadata_without_claiming_to_evaluate_unsupported_types_or_frames() {
        let order = SpkByteOrder::LittleEndian;
        let mut b = file(order, false);
        let offset = 2 * RECORD_BYTES + 3 * WORD_BYTES + 16;
        put_i32(&mut b, offset + 8, 17, order);
        put_i32(&mut b, offset + 12, 9, order);
        let result = parse(b).unwrap();
        assert_eq!(result.segments[0].frame_id, 17);
        assert_eq!(result.segments[0].spk_type, 9);
    }
}
