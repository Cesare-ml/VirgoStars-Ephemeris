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

//! Reception light-time through a precision-checked split-TDB observation epoch.
//!
//! This is a thin Rust-only adapter to the existing DE441 light-time solver.
//! It rejects unacceptable *observation-input* JD rounding before ephemeris
//! record I/O. It cannot eliminate the solver's own single-f64 JD quantization
//! of iterated emission epochs, or validate its astronomical accuracy.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::calendar::JulianDate;
use crate::de441::{Body, De441};
use crate::de441_epoch_bridge::{checked_de441_epoch, EpochBridgeError};
use crate::light_time::{geocentric_light_time, GeocentricLightTimeState, LightTimeError};

/// Earth-centered reception light-time state plus observation-input JD loss.
///
/// Position is in km, velocity in km per TDB day, in fixed ICRF axes.
/// The quantization metric applies *only* to the observation input. The
/// emission epoch is iterated by the existing single-f64-JD solver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckedGeocentricLightTimeState {
    pub state: GeocentricLightTimeState,
    pub observation_quantization_error_seconds: f64,
}

/// Distinguishes time-input rejection from a light-time solver/reader error.
#[derive(Debug)]
pub enum CheckedLightTimeError {
    Epoch(EpochBridgeError),
    LightTime(LightTimeError),
}

impl fmt::Display for CheckedLightTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Epoch(error) => write!(f, "DE441 observation TDB epoch error: {error}"),
            Self::LightTime(error) => write!(f, "DE441 reception light-time error: {error}"),
        }
    }
}

impl Error for CheckedLightTimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Epoch(error) => Some(error),
            Self::LightTime(error) => Some(error),
        }
    }
}

impl From<EpochBridgeError> for CheckedLightTimeError {
    fn from(error: EpochBridgeError) -> Self {
        Self::Epoch(error)
    }
}

impl From<LightTimeError> for CheckedLightTimeError {
    fn from(error: LightTimeError) -> Self {
        Self::LightTime(error)
    }
}

/// Compute Newtonian geocentric reception light-time at an already-TDB split JD.
///
/// The caller's finite, nonnegative budget in seconds is a limit on
/// *input-JD conversion*, checked before reading coefficient records. It
/// is not a bound on the solver's emission-epoch precision, planet position,
/// light-time accuracy or rate accuracy.
///
/// Only the existing Newtonian reception light-time is applied. No annual
/// aberration, gravitational deflection, topocentric correction, precession,
/// nutation, refraction, or UTC/TT/TDB conversion occurs. Missing coefficients,
/// out-of-coverage emission epochs and undefined Earth directions are errors.
pub fn checked_geocentric_light_time_at_split_tdb<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    observation_epoch: JulianDate,
    max_observation_quantization_error_seconds: f64,
) -> Result<CheckedGeocentricLightTimeState, CheckedLightTimeError> {
    let checked = checked_de441_epoch(
        observation_epoch,
        max_observation_quantization_error_seconds,
    )?;
    let state = geocentric_light_time(ephemeris, target, checked.epoch)?;
    Ok(CheckedGeocentricLightTimeState {
        state,
        observation_quantization_error_seconds: checked.quantization_error_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::TimeScale;
    use crate::de441::{De441Error, JulianDateTdb};
    use crate::light_time::SPEED_OF_LIGHT_KM_PER_SECOND;
    use std::cell::Cell;
    use std::io::{self, Cursor, SeekFrom};
    use std::rc::Rc;

    const START: f64 = 2_451_544.0;
    const END: f64 = START + 32.0;
    const RECORD_SIZE: usize = 3_000;
    const TARGET_DISTANCE_KM: f64 = 150_000_000.0;

    fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn fixture() -> Vec<u8> {
        // In-memory classic-DE441 *layout*, with artificial constant vectors.
        // EMB and geocentric Moon are zero; Mars system barycenter is fixed at
        // +150 million km on X. No NASA data or real orbit is used.
        let mut bytes = vec![0_u8; RECORD_SIZE * 3];
        let title = b"JPL Planetary Ephemeris DE441";
        bytes[..title.len()].copy_from_slice(title);
        put_f64(&mut bytes, 2652, START);
        put_f64(&mut bytes, 2660, END);
        put_f64(&mut bytes, 2668, 32.0);
        put_f64(&mut bytes, 2688, 81.0);
        put_i32(&mut bytes, 2840, 441);

        // Each series is three T0 coefficients. Last pointer pads 375 words.
        for (index, first) in [(2, 3), (3, 6), (9, 9), (12, 373)] {
            let at = if index == 12 { 2844 } else { 2696 + index * 12 };
            put_i32(&mut bytes, at, first);
            put_i32(&mut bytes, at + 4, 1);
            put_i32(&mut bytes, at + 8, 1);
        }

        let record = RECORD_SIZE * 2;
        put_f64(&mut bytes, record, START);
        put_f64(&mut bytes, record + 8, END);
        // Mars barycenter coefficient word 6 (1-based); other axes zero.
        put_f64(&mut bytes, record + (6 - 1) * 8, TARGET_DISTANCE_KM);
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

    fn observation() -> JulianDate {
        JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap()
    }

    fn fractional_observation() -> JulianDate {
        JulianDate::new(2_451_545, 0.5 + 0.25 / 86_400.0, TimeScale::Tdb).unwrap()
    }

    #[test]
    fn constant_geocentric_vector_has_expected_light_time_and_zero_rate() {
        let (mut eph, reads) = reader();
        let checked = checked_geocentric_light_time_at_split_tdb(
            &mut eph,
            Body::MarsBarycenter,
            observation(),
            0.0,
        )
        .unwrap();
        assert_eq!(checked.observation_quantization_error_seconds, 0.0);
        assert_eq!(checked.state.target, Body::MarsBarycenter);
        assert_eq!(checked.state.observation_epoch, JulianDateTdb(2_451_545.0));
        assert!(checked.state.emission_epoch.0 < checked.state.observation_epoch.0);
        assert_eq!(checked.state.position_km, [TARGET_DISTANCE_KM, 0.0, 0.0]);
        assert_eq!(checked.state.velocity_km_per_day, [0.0; 3]);
        assert_eq!(checked.state.light_time_rate, 0.0);
        let delay = TARGET_DISTANCE_KM / SPEED_OF_LIGHT_KM_PER_SECOND;
        assert!((checked.state.light_time_seconds - delay).abs() < 1.0e-10);
        assert_eq!(reads.get(), 1, "reused record should be cached after first read");
    }

    #[test]
    fn rejects_excessive_observation_quantization_before_ephemeris_io() {
        let (mut eph, reads) = reader();
        assert!(matches!(
            checked_geocentric_light_time_at_split_tdb(
                &mut eph,
                Body::MarsBarycenter,
                fractional_observation(),
                0.000_001,
            ),
            Err(CheckedLightTimeError::Epoch(
                EpochBridgeError::PrecisionLoss { .. }
            ))
        ));
        assert_eq!(reads.get(), 0);
        let accepted = checked_geocentric_light_time_at_split_tdb(
            &mut eph,
            Body::MarsBarycenter,
            fractional_observation(),
            0.001,
        )
        .unwrap();
        assert!(accepted.observation_quantization_error_seconds > 0.000_001);
        assert!(accepted.observation_quantization_error_seconds < 0.001);
        assert_eq!(accepted.state.position_km, [TARGET_DISTANCE_KM, 0.0, 0.0]);
        assert_eq!(reads.get(), 1);
    }

    #[test]
    fn non_tdb_observation_and_invalid_budget_do_not_read_coefficients() {
        let (mut eph, reads) = reader();
        for scale in [TimeScale::Tt, TimeScale::Tai, TimeScale::Ut1] {
            let input = JulianDate::new(2_451_545, 0.5, scale).unwrap();
            assert!(matches!(
                checked_geocentric_light_time_at_split_tdb(
                    &mut eph,
                    Body::MarsBarycenter,
                    input,
                    1.0,
                ),
                Err(CheckedLightTimeError::Epoch(EpochBridgeError::WrongTimeScale))
            ));
        }
        for invalid in [f64::NAN, f64::INFINITY, -0.0001] {
            assert!(matches!(
                checked_geocentric_light_time_at_split_tdb(
                    &mut eph,
                    Body::MarsBarycenter,
                    observation(),
                    invalid,
                ),
                Err(CheckedLightTimeError::Epoch(
                    EpochBridgeError::InvalidErrorBudget
                ))
            ));
        }
        assert_eq!(reads.get(), 0);
    }

    #[test]
    fn earth_as_geocentric_target_is_explicitly_undefined() {
        let (mut eph, reads) = reader();
        assert!(matches!(
            checked_geocentric_light_time_at_split_tdb(
                &mut eph,
                Body::Earth,
                observation(),
                0.0,
            ),
            Err(CheckedLightTimeError::LightTime(
                LightTimeError::UndefinedDirection
            ))
        ));
        assert_eq!(reads.get(), 0);
    }

    #[test]
    fn emission_earlier_than_kernel_start_is_not_silently_extrapolated() {
        let (mut eph, _) = reader();
        let start = JulianDate::from_f64(START, TimeScale::Tdb).unwrap();
        assert!(matches!(
            checked_geocentric_light_time_at_split_tdb(
                &mut eph,
                Body::MarsBarycenter,
                start,
                0.0,
            ),
            Err(CheckedLightTimeError::LightTime(
                LightTimeError::Ephemeris(De441Error::OutsideCoverage { .. })
            ))
        ));
    }

    #[test]
    fn missing_target_series_is_propagated_without_fallback() {
        let (mut eph, _) = reader();
        assert!(matches!(
            checked_geocentric_light_time_at_split_tdb(
                &mut eph,
                Body::Venus,
                observation(),
                0.0,
            ),
            Err(CheckedLightTimeError::LightTime(
                LightTimeError::Ephemeris(De441Error::MissingSeries(_))
            ))
        ));
    }

    #[test]
    fn output_matches_existing_single_jd_solver_for_same_rounded_epoch() {
        let (mut eph, _) = reader();
        let checked = checked_geocentric_light_time_at_split_tdb(
            &mut eph,
            Body::MarsBarycenter,
            fractional_observation(),
            0.001,
        )
        .unwrap();
        let legacy_epoch = fractional_observation().to_de441_epoch().unwrap();
        let existing = geocentric_light_time(&mut eph, Body::MarsBarycenter, legacy_epoch).unwrap();
        assert_eq!(checked.state, existing);
        assert!(checked.observation_quantization_error_seconds > 0.0);
    }
}
