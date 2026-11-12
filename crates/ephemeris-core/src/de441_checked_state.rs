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

//! Guarded split-TDB state lookup through the existing classic DE441 reader.
//!
//! This adapter validates the caller's precision budget before calling into
//! the reader. The reader still evaluates a single-f64 JD; no new numerical
//! ephemeris algorithm, time-scale conversion, or apparent corrections exist.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::calendar::JulianDate;
use crate::de441::{Body, De441, De441Error, StateVector};
use crate::de441_epoch_bridge::{checked_de441_epoch, EpochBridgeError};

/// Geometric target-minus-center ICRF state, with the TDB JD rounding loss.
///
/// The rounding loss is *only* a bound on this adapter's input conversion,
/// not a bound on astronomical position, velocity or model accuracy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckedDe441State {
    pub state: StateVector,
    pub quantization_error_seconds: f64,
}

/// Separates invalid TDB/precision input from DE441 file and coverage errors.
#[derive(Debug)]
pub enum CheckedStateError {
    Epoch(EpochBridgeError),
    Ephemeris(De441Error),
}

impl fmt::Display for CheckedStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Epoch(error) => write!(f, "DE441 split-epoch error: {error}"),
            Self::Ephemeris(error) => write!(f, "DE441 state lookup error: {error}"),
        }
    }
}

impl Error for CheckedStateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Epoch(error) => Some(error),
            Self::Ephemeris(error) => Some(error),
        }
    }
}

impl From<EpochBridgeError> for CheckedStateError {
    fn from(error: EpochBridgeError) -> Self {
        Self::Epoch(error)
    }
}

impl From<De441Error> for CheckedStateError {
    fn from(error: De441Error) -> Self {
        Self::Ephemeris(error)
    }
}

/// Obtain a geometric DE441 state at an already-TDB split Julian date.
///
/// The precision budget is a finite, nonnegative number of seconds and
/// constrains *only* conversion of the split Julian date into the reader's
/// single-f64 JD input. The conversion is checked before any state I/O.
/// The result is target minus center, in fixed ICRF axes: km and km/TDB day.
///
/// No UTC/TT-to-TDB conversion, light-time, annual aberration, refraction,
/// precession or nutation is performed. Out-of-coverage and missing-series
/// errors from the existing DE441 reader are propagated without fallback.
pub fn checked_state_at_split_tdb<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    center: Body,
    split_epoch: JulianDate,
    max_quantization_error_seconds: f64,
) -> Result<CheckedDe441State, CheckedStateError> {
    let checked = checked_de441_epoch(split_epoch, max_quantization_error_seconds)?;
    let state = ephemeris.state(target, center, checked.epoch)?;
    Ok(CheckedDe441State {
        state,
        quantization_error_seconds: checked.quantization_error_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::TimeScale;
    use crate::de441::JulianDateTdb;
    use std::cell::Cell;
    use std::io::{self, Cursor, SeekFrom};
    use std::rc::Rc;

    const START: f64 = 2_451_544.0;
    const END: f64 = START + 32.0;
    const RECORD_SIZE: usize = 3_000;

    fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn fixture() -> Vec<u8> {
        // Original, synthetic single-record classic-DE441 structure.
        // No JPL coefficients, physical ephemeris, or Horizons data.
        let mut bytes = vec![0_u8; RECORD_SIZE * 3];
        let title = b"JPL Planetary Ephemeris DE441";
        bytes[..title.len()].copy_from_slice(title);
        write_f64(&mut bytes, 2652, START);
        write_f64(&mut bytes, 2660, END);
        write_f64(&mut bytes, 2668, 32.0);
        write_f64(&mut bytes, 2688, 81.0);
        write_i32(&mut bytes, 2840, 441);

        // Mercury: three T0 position coefficients; last padding pointer
        // establishes the record size without contributing to this lookup.
        write_i32(&mut bytes, 2696, 3);
        write_i32(&mut bytes, 2700, 1);
        write_i32(&mut bytes, 2704, 1);
        write_i32(&mut bytes, 2844, 373);
        write_i32(&mut bytes, 2848, 1);
        write_i32(&mut bytes, 2852, 1);

        write_f64(&mut bytes, RECORD_SIZE * 2, START);
        write_f64(&mut bytes, RECORD_SIZE * 2 + 8, END);
        for (axis, value) in [100.0, 200.0, 300.0].into_iter().enumerate() {
            write_f64(&mut bytes, RECORD_SIZE * 2 + (2 + axis) * 8, value);
        }
        bytes
    }

    #[derive(Debug)]
    struct CountingReader {
        inner: Cursor<Vec<u8>>,
        coefficient_reads: Rc<Cell<usize>>,
    }

    impl Read for CountingReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.inner.position() >= (RECORD_SIZE * 2) as u64 {
                self.coefficient_reads.set(self.coefficient_reads.get() + 1);
            }
            self.inner.read(buf)
        }
    }

    impl Seek for CountingReader {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.inner.seek(pos)
        }
    }

    fn reader() -> (De441<CountingReader>, Rc<Cell<usize>>) {
        let reads = Rc::new(Cell::new(0));
        let input = CountingReader {
            inner: Cursor::new(fixture()),
            coefficient_reads: reads.clone(),
        };
        (De441::new(input).unwrap(), reads)
    }

    fn exact_tdb() -> JulianDate {
        // JulianDate day_number is the noon JDN, with a fraction since midnight.
        JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap()
    }

    fn fractional_tdb() -> JulianDate {
        JulianDate::new(2_451_545, 0.5 + 0.25 / 86_400.0, TimeScale::Tdb).unwrap()
    }

    #[test]
    fn exact_split_jd_returns_identical_geometric_state_without_rounding() {
        let (mut ephemeris, reads) = reader();
        let checked = checked_state_at_split_tdb(
            &mut ephemeris,
            Body::Mercury,
            Body::SolarSystemBarycenter,
            exact_tdb(),
            0.0,
        )
        .unwrap();
        assert_eq!(checked.quantization_error_seconds, 0.0);
        assert_eq!(checked.state.target, Body::Mercury);
        assert_eq!(checked.state.center, Body::SolarSystemBarycenter);
        assert_eq!(checked.state.epoch, JulianDateTdb(2_451_545.0));
        assert_eq!(checked.state.position_km, [100.0, 200.0, 300.0]);
        assert_eq!(checked.state.velocity_km_per_day, [0.0; 3]);
        assert_eq!(reads.get(), 1);
    }

    #[test]
    fn budget_applies_before_reader_io_and_permissive_budget_allows_lookup() {
        let (mut ephemeris, reads) = reader();
        assert!(matches!(
            checked_state_at_split_tdb(
                &mut ephemeris,
                Body::Mercury,
                Body::SolarSystemBarycenter,
                fractional_tdb(),
                0.000_001
            ),
            Err(CheckedStateError::Epoch(
                EpochBridgeError::PrecisionLoss { .. }
            ))
        ));
        assert_eq!(
            reads.get(),
            0,
            "rejected epoch must not read coefficient records"
        );

        let accepted = checked_state_at_split_tdb(
            &mut ephemeris,
            Body::Mercury,
            Body::SolarSystemBarycenter,
            fractional_tdb(),
            0.001,
        )
        .unwrap();
        assert!(accepted.quantization_error_seconds > 0.000_001);
        assert!(accepted.quantization_error_seconds <= 0.001);
        assert_eq!(accepted.state.position_km, [100.0, 200.0, 300.0]);
        assert_eq!(reads.get(), 1);
    }

    #[test]
    fn wrong_scale_and_invalid_budget_fail_before_record_access() {
        let (mut ephemeris, reads) = reader();
        for scale in [TimeScale::Tt, TimeScale::Tai, TimeScale::Ut1] {
            let input = JulianDate::new(2_451_545, 0.5, scale).unwrap();
            assert!(matches!(
                checked_state_at_split_tdb(
                    &mut ephemeris,
                    Body::Mercury,
                    Body::SolarSystemBarycenter,
                    input,
                    1.0
                ),
                Err(CheckedStateError::Epoch(EpochBridgeError::WrongTimeScale))
            ));
        }
        assert!(matches!(
            checked_state_at_split_tdb(
                &mut ephemeris,
                Body::Mercury,
                Body::SolarSystemBarycenter,
                exact_tdb(),
                f64::NAN
            ),
            Err(CheckedStateError::Epoch(
                EpochBridgeError::InvalidErrorBudget
            ))
        ));
        assert_eq!(reads.get(), 0);
    }

    #[test]
    fn preserves_file_coverage_error_without_approximate_fallback() {
        let (mut ephemeris, _) = reader();
        let beyond = JulianDate::new(2_451_580, 0.5, TimeScale::Tdb).unwrap();
        assert!(matches!(
            checked_state_at_split_tdb(
                &mut ephemeris,
                Body::Mercury,
                Body::SolarSystemBarycenter,
                beyond,
                0.0
            ),
            Err(CheckedStateError::Ephemeris(
                De441Error::OutsideCoverage { .. }
            ))
        ));
    }

    #[test]
    fn preserves_missing_series_error_and_does_not_invent_body_values() {
        let (mut ephemeris, reads) = reader();
        assert!(matches!(
            checked_state_at_split_tdb(
                &mut ephemeris,
                Body::Venus,
                Body::SolarSystemBarycenter,
                exact_tdb(),
                0.0
            ),
            Err(CheckedStateError::Ephemeris(De441Error::MissingSeries(_)))
        ));
        assert_eq!(reads.get(), 1);
    }
}
