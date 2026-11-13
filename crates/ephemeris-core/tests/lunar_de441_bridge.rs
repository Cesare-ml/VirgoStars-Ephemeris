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

use std::cell::Cell;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;

use virgostars_ephemeris_core::de441::{De441, De441Error, JulianDateTdb};
use virgostars_ephemeris_core::lunar_phase::{
    geometric_lunar_illumination_from_de441, LunarDe441Error, LunarIlluminationError,
};
use virgostars_ephemeris_core::geometry::AngularSeparationError;

const RECORD_BYTES: usize = 3000;
const FIRST_RECORD: usize = RECORD_BYTES * 2;
const LUNAR_MASS_RATIO: f64 = 81.0;

fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// Two 32-day records with synthetic constant ICRF vectors. No real JPL data.
// The last pointer (libration) forces 3000-byte records. Positions are km.
// EMB=(0,0,0), geocentric Moon=(1,0,0) in both records.
// Sun barycenter record 0 -> geocentric Sun=(1,1,0) = quarter configuration;
// Sun barycenter record 1 -> geocentric Sun=(2,0,0) = new configuration.
fn fixture() -> Vec<u8> {
    let mut bytes = vec![0u8; RECORD_BYTES * 4];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    write_f64(&mut bytes, 2652, 1000.0);
    write_f64(&mut bytes, 2660, 1064.0);
    write_f64(&mut bytes, 2668, 32.0);
    write_f64(&mut bytes, 2688, LUNAR_MASS_RATIO);
    write_i32(&mut bytes, 2840, 441);
    for (index, first) in [(2, 3), (9, 6), (10, 9), (12, 373)] {
        let at = if index == 12 { 2844 } else { 2696 + index * 12 };
        write_i32(&mut bytes, at, first);
        write_i32(&mut bytes, at + 4, 1);
        write_i32(&mut bytes, at + 8, 1);
    }

    for record in 0..2 {
        let at = FIRST_RECORD + record * RECORD_BYTES;
        write_f64(&mut bytes, at, 1000.0 + record as f64 * 32.0);
        write_f64(&mut bytes, at + 8, 1032.0 + record as f64 * 32.0);
        // Earth-Moon barycenter (pointer 3): [0, 0, 0].
        // Geocentric Moon (pointer 6): [1, 0, 0].
        write_f64(&mut bytes, at + 5 * 8, 1.0);
        // Sun barycentric position must account for Earth =
        // EMB - Moon / (1 + mass ratio), so that Sun/Earth equals
        // exactly the expected synthetic geometry.
        let sun = if record == 0 {
            [1.0 - 1.0 / (1.0 + LUNAR_MASS_RATIO), 1.0, 0.0]
        } else {
            [2.0 - 1.0 / (1.0 + LUNAR_MASS_RATIO), 0.0, 0.0]
        };
        for (axis, value) in sun.into_iter().enumerate() {
            write_f64(&mut bytes, at + (8 + axis) * 8, value);
        }
    }
    bytes
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:?}, expected={expected:?}, tolerance={tolerance:?}"
    );
}

#[test]
fn direct_reader_bridge_reproduces_independent_synthetic_quarter_and_new_geometry() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    // The next record starts exactly at 1032.0 and the final epoch is inclusive.
    for jd in [1000.0, 1016.0, 1032.0 - 1.0 / 1024.0] {
        let sample = geometric_lunar_illumination_from_de441(&mut eph, JulianDateTdb(jd)).unwrap();
        assert_eq!(sample.epoch, JulianDateTdb(jd));
        near(sample.phase_angle_rad, std::f64::consts::FRAC_PI_2, 1e-14);
        near(sample.illuminated_fraction, 0.5, 1e-14);
    }
    for jd in [1032.0, 1048.0, 1064.0] {
        let sample = geometric_lunar_illumination_from_de441(&mut eph, JulianDateTdb(jd)).unwrap();
        assert_eq!(sample.epoch, JulianDateTdb(jd));
        near(sample.phase_angle_rad, std::f64::consts::PI, 1e-14);
        near(sample.illuminated_fraction, 0.0, 1e-14);
    }
}

#[test]
fn explicit_preflight_error_for_nonfinite_epoch_and_reader_coverage_error() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    for epoch in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            geometric_lunar_illumination_from_de441(&mut eph, JulianDateTdb(epoch)),
            Err(LunarDe441Error::NonFiniteEpoch)
        ));
    }
    for jd in [999.0, 1064.000_001] {
        assert!(matches!(
            geometric_lunar_illumination_from_de441(&mut eph, JulianDateTdb(jd)),
            Err(LunarDe441Error::Ephemeris(De441Error::OutsideCoverage { .. }))
        ));
    }
}

#[test]
fn explicitly_preserves_missing_sun_moon_and_emb_series_errors() {
    for index in [2usize, 9, 10] {
        let mut bytes = fixture();
        let at = 2696 + index * 12;
        for offset in [0, 4, 8] {
            write_i32(&mut bytes, at + offset, 0);
        }
        let mut eph = De441::new(Cursor::new(bytes)).unwrap();
        assert!(
            matches!(
                geometric_lunar_illumination_from_de441(&mut eph, JulianDateTdb(1016.0)),
                Err(LunarDe441Error::Ephemeris(De441Error::MissingSeries(_)))
            ),
            "missing series index {index} should not silently fall back"
        );
    }
}

#[test]
fn damaged_sun_coefficient_and_zero_moon_direction_fail_closed() {
    let mut bytes = fixture();
    write_f64(&mut bytes, FIRST_RECORD + 8 * 8, f64::NAN);
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    assert!(matches!(
        geometric_lunar_illumination_from_de441(&mut eph, JulianDateTdb(1016.0)),
        Err(LunarDe441Error::Ephemeris(De441Error::InvalidFile(
            "non-finite Chebyshev coefficient"
        )))
    ));

    let mut bytes = fixture();
    write_f64(&mut bytes, FIRST_RECORD + 5 * 8, 0.0);
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    assert!(matches!(
        geometric_lunar_illumination_from_de441(&mut eph, JulianDateTdb(1016.0)),
        Err(LunarDe441Error::Illumination(
            LunarIlluminationError::Geometry(AngularSeparationError::UndefinedDirection)
        ))
    ));
}

struct InstrumentedCursor {
    inner: Cursor<Vec<u8>>,
    reads: Rc<Cell<usize>>,
    fail: Rc<Cell<bool>>,
}

impl Read for InstrumentedCursor {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reads.set(self.reads.get() + 1);
        if self.fail.get() {
            return Err(io::Error::other("injected record read failure"));
        }
        self.inner.read(buf)
    }
}

impl Seek for InstrumentedCursor {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

#[test]
fn bridge_reuses_record_cache_and_recovers_after_injected_io_failure() {
    let reads = Rc::new(Cell::new(0));
    let fail = Rc::new(Cell::new(false));
    let mut eph = De441::new(InstrumentedCursor {
        inner: Cursor::new(fixture()),
        reads: reads.clone(),
        fail: fail.clone(),
    }).unwrap();

    fail.set(true);
    let err = geometric_lunar_illumination_from_de441(&mut eph, JulianDateTdb(1016.0))
        .expect_err("injected read error should be propagated");
    assert!(matches!(err, LunarDe441Error::Ephemeris(De441Error::Io(_))));
    assert!(std::error::Error::source(&err).is_some());
    fail.set(false);

    let before = reads.get();
    let first = geometric_lunar_illumination_from_de441(&mut eph, JulianDateTdb(1016.0)).unwrap();
    assert!(reads.get() > before, "reader must load first coefficient record");
    near(first.illuminated_fraction, 0.5, 1e-14);
    let cached_reads = reads.get();
    let second = geometric_lunar_illumination_from_de441(&mut eph, JulianDateTdb(1020.0)).unwrap();
    near(second.illuminated_fraction, 0.5, 1e-14);
    assert_eq!(reads.get(), cached_reads, "same record should reuse cache");
}
