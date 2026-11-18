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

//! Synthetic record-layout tests: this is not independent JPL validation.

use std::io::{self, Cursor, Write};
use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};

const SIZE: usize = 3000;
const START: f64 = 1000.0;
const STEP: f64 = 32.0;

fn fixture(records: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; (records + 2) * SIZE];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    bytes[2652..2660].copy_from_slice(&START.to_le_bytes());
    bytes[2660..2668].copy_from_slice(&(START + records as f64 * STEP).to_le_bytes());
    bytes[2668..2676].copy_from_slice(&STEP.to_le_bytes());
    bytes[2688..2696].copy_from_slice(&81.0f64.to_le_bytes());
    bytes[2840..2844].copy_from_slice(&441i32.to_le_bytes());
    // One Mercury polynomial and a last dummy series to derive 3000 bytes.
    for (index, first) in [(0, 3i32), (12, 373i32)] {
        let offset = if index == 12 { 2844 } else { 2696 + index * 12 };
        for (at, value) in [(offset, first), (offset + 4, 1), (offset + 8, 1)] {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    for index in 0..records {
        let offset = (index + 2) * SIZE;
        bytes[offset..offset + 8]
            .copy_from_slice(&(START + index as f64 * STEP).to_le_bytes());
        bytes[offset + 8..offset + 16]
            .copy_from_slice(&(START + (index + 1) as f64 * STEP).to_le_bytes());
        for axis in 0..3 {
            let value = (index * 100 + axis * 3) as f64;
            let at = offset + (2 + axis) * 8;
            bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

#[test]
fn slices_whole_source_records_and_preserves_geometric_states() {
    let original = fixture(5);
    let mut source = De441::new(Cursor::new(original.clone())).unwrap();
    assert_eq!(source.record_step_days(), STEP);
    assert_eq!(source.coefficient_record_count(), 5);
    let mut output = Vec::new();
    let summary = source
        .write_slice(JulianDateTdb(1033.0), JulianDateTdb(1095.0), &mut output)
        .unwrap();
    assert_eq!(summary.coefficient_records, 2);
    assert_eq!(summary.stored_start_tdb, JulianDateTdb(1032.0));
    assert_eq!(summary.stored_end_tdb, JulianDateTdb(1096.0));
    assert_eq!(summary.bytes_written, 4 * SIZE as u64);
    assert_eq!(output.len(), 4 * SIZE);
    assert_eq!(&output[SIZE..SIZE * 2], &original[SIZE..SIZE * 2]);
    assert_eq!(&output[SIZE * 2..], &original[SIZE * 3..SIZE * 5]);

    let mut slice = De441::new(Cursor::new(output)).unwrap();
    assert_eq!(slice.record_step_days(), STEP);
    assert_eq!(slice.coefficient_record_count(), 2);
    assert_eq!(
        slice.coverage(),
        (JulianDateTdb(1032.0), JulianDateTdb(1096.0))
    );
    for jd in [1032.0, 1033.0, 1063.9, 1064.0, 1095.0, 1096.0] {
        let target = Body::Mercury;
        let center = Body::SolarSystemBarycenter;
        assert_eq!(
            source.state(target, center, JulianDateTdb(jd)).unwrap(),
            slice.state(target, center, JulianDateTdb(jd)).unwrap()
        );
    }
    assert!(matches!(
        slice.state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1031.9)
        ),
        Err(De441Error::OutsideCoverage { .. })
    ));
}

#[test]
fn exact_boundary_selects_following_record_only() {
    let mut source = De441::new(Cursor::new(fixture(5))).unwrap();
    let mut output = Vec::new();
    let summary = source
        .write_slice(JulianDateTdb(1064.0), JulianDateTdb(1096.0), &mut output)
        .unwrap();
    assert_eq!(summary.coefficient_records, 1);
    assert_eq!(output.len(), 3 * SIZE);
    assert_eq!(
        De441::new(Cursor::new(output)).unwrap().coverage(),
        (JulianDateTdb(1064.0), JulianDateTdb(1096.0))
    );
}

#[test]
fn rejects_invalid_or_outside_ranges_before_writing() {
    let mut source = De441::new(Cursor::new(fixture(5))).unwrap();
    for (start, end) in [
        (1000.0, 1000.0),
        (1050.0, 1040.0),
        (f64::NAN, 1040.0),
        (1020.0, f64::INFINITY),
        (999.0, 1001.0),
        (1159.0, 1161.0),
    ] {
        let mut bytes = Vec::new();
        assert!(matches!(
            source.write_slice(JulianDateTdb(start), JulianDateTdb(end), &mut bytes),
            Err(De441Error::InvalidSliceRange(_))
        ));
        assert!(bytes.is_empty());
    }
}

struct FailWriter;
impl Write for FailWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "synthetic failure"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn propagates_writer_io_failure() {
    let mut source = De441::new(Cursor::new(fixture(5))).unwrap();
    assert!(matches!(
        source.write_slice(JulianDateTdb(1032.0), JulianDateTdb(1064.0), &mut FailWriter),
        Err(De441Error::Io(err)) if err.kind() == io::ErrorKind::BrokenPipe
    ));
}
