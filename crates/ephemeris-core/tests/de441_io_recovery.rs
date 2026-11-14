// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

//! Fault-injection tests for seekable, on-demand classic DE441 input.
//!
//! Created 2026-11-14. Tests synthetic stream recovery and explicit I/O
//! errors only; none of these checks independently validates NASA/JPL states.

use std::cell::Cell;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;

use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};

const RECORD_BYTES: usize = 3000;
const RECORD_START: u64 = (2 * RECORD_BYTES) as u64;
const START_JD: f64 = 2_451_545.0;

#[derive(Clone, Copy)]
enum StopMode {
    IoError,
    EndOfFile,
}

#[derive(Clone, Default)]
struct Controls {
    stop_read_at: Rc<Cell<Option<(u64, StopMode)>>>,
    fail_next_seek: Rc<Cell<bool>>,
    interrupt_next_read: Rc<Cell<bool>>,
}

/// A seekable stream returning deliberately short reads and optional failures.
/// The controls remain accessible after the stream is moved into De441.
struct ControlledStream {
    inner: Cursor<Vec<u8>>,
    max_read_bytes: usize,
    controls: Controls,
}

impl ControlledStream {
    fn new(content: Vec<u8>, max_read_bytes: usize) -> (Self, Controls) {
        assert!(max_read_bytes > 0);
        let controls = Controls::default();
        (
            Self {
                inner: Cursor::new(content),
                max_read_bytes,
                controls: controls.clone(),
            },
            controls,
        )
    }
}

impl Read for ControlledStream {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.controls.interrupt_next_read.replace(false) {
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        let mut count = buffer.len().min(self.max_read_bytes);
        if let Some((stop, mode)) = self.controls.stop_read_at.get() {
            let position = self.inner.position();
            if position >= stop {
                return match mode {
                    StopMode::IoError => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
                    StopMode::EndOfFile => Ok(0),
                };
            }
            count = count.min((stop - position) as usize);
        }
        self.inner.read(&mut buffer[..count])
    }
}

impl Seek for ControlledStream {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if self.controls.fail_next_seek.replace(false) {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
        self.inner.seek(position)
    }
}

fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// One 32-TDB-day record, with linear Mercury XYZ Chebyshev coefficients.
/// Synthetic "DE441" header and empty filler only: no actual JPL data.
fn synthetic_classic_binary() -> Vec<u8> {
    let mut bytes = vec![0; RECORD_BYTES * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    write_f64(&mut bytes, 2652, START_JD);
    write_f64(&mut bytes, 2660, START_JD + 32.0);
    write_f64(&mut bytes, 2668, 32.0);
    write_f64(&mut bytes, 2688, 81.0);
    write_i32(&mut bytes, 2840, 441);

    // First series (Mercury), 1 granule, 2 coefficients per Cartesian axis.
    write_i32(&mut bytes, 2696, 3);
    write_i32(&mut bytes, 2700, 2);
    write_i32(&mut bytes, 2704, 1);

    // A last header pointer reserves the classic 3000-byte record size.
    write_i32(&mut bytes, 2844, 373);
    write_i32(&mut bytes, 2848, 1);
    write_i32(&mut bytes, 2852, 1);

    for axis in 0..3 {
        let offset = RECORD_START as usize + (2 + axis * 2) * 8;
        write_f64(&mut bytes, offset, 100.0 * (axis + 1) as f64);
        write_f64(&mut bytes, offset + 8, 16.0 * (axis + 1) as f64);
    }
    bytes
}

fn assert_known_state(ephemeris: &mut De441<ControlledStream>) {
    // Midpoint of the 32-day record, so T1(x=0) vanishes.
    let state = ephemeris
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(START_JD + 16.0),
        )
        .unwrap();
    assert_eq!(state.position_km, [100.0, 200.0, 300.0]);
    // d(T1)/d(JD) = 2/32 per day.
    assert_eq!(state.velocity_km_per_day, [1.0, 2.0, 3.0]);
}

#[test]
fn short_reads_still_reconstruct_complete_header_and_record() {
    // Read::read is allowed to return fewer bytes than requested.
    let (stream, _) = ControlledStream::new(synthetic_classic_binary(), 3);
    let mut ephemeris = De441::new(stream).unwrap();
    assert_eq!(
        ephemeris.coverage(),
        (
            JulianDateTdb(START_JD),
            JulianDateTdb(START_JD + 32.0)
        )
    );
    assert_known_state(&mut ephemeris);
}

#[test]
fn interrupted_read_is_retried_without_changing_the_ephemeris_state() {
    let (stream, controls) = ControlledStream::new(synthetic_classic_binary(), 11);
    let mut ephemeris = De441::new(stream).unwrap();
    controls.interrupt_next_read.set(true);
    assert_known_state(&mut ephemeris);
    assert_known_state(&mut ephemeris);
}

#[test]
fn mid_record_io_error_and_early_eof_fail_closed_then_recover_after_retry() {
    for mode in [StopMode::IoError, StopMode::EndOfFile] {
        let (stream, controls) = ControlledStream::new(synthetic_classic_binary(), 13);
        let mut ephemeris = De441::new(stream).unwrap();

        // Let exactly 101 bytes of the coefficient record be read, then stop.
        controls.stop_read_at.set(Some((RECORD_START + 101, mode)));
        let error = ephemeris
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(START_JD + 16.0),
            )
            .unwrap_err();
        let expected_kind = match mode {
            StopMode::IoError => io::ErrorKind::BrokenPipe,
            StopMode::EndOfFile => io::ErrorKind::UnexpectedEof,
        };
        assert!(matches!(
            error,
            De441Error::Io(ref underlying) if underlying.kind() == expected_kind
        ));

        // A fresh request must seek and re-read; failed partial reads cannot
        // leave an invalid or stale state cached.
        controls.stop_read_at.set(None);
        assert_known_state(&mut ephemeris);
    }
}

#[test]
fn failed_seek_is_reported_and_does_not_poison_following_state_queries() {
    let (stream, controls) = ControlledStream::new(synthetic_classic_binary(), 31);
    let mut ephemeris = De441::new(stream).unwrap();
    controls.fail_next_seek.set(true);
    let error = ephemeris
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(START_JD + 16.0),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        De441Error::Io(ref underlying) if underlying.kind() == io::ErrorKind::PermissionDenied
    ));
    assert_known_state(&mut ephemeris);
}

#[test]
fn truncated_header_reports_unexpected_eof_instead_of_panicking() {
    let bytes = synthetic_classic_binary();
    let (stream, _) = ControlledStream::new(bytes[..2000].to_vec(), 5);
    let error = match De441::new(stream) {
        Ok(_) => panic!("truncated header must not be accepted"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        De441Error::Io(ref underlying) if underlying.kind() == io::ErrorKind::UnexpectedEof
    ));
}
