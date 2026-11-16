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

//! Civil-time front door for the geocentric apparent pipeline (MVP 1, rows 1–2).
//!
//! Converts a civil timestamp with an explicit UTC offset to a TDB Julian date
//! under a **declared, reported time policy**, then evaluates
//! [`crate::apparent_ecliptic_of_date`] for several bodies at that epoch.
//!
//! # Time policy (never silent)
//!
//! | Instant (UTC/UT) | Source reported in the result | Method |
//! |---|---|---|
//! | 1972-01-01 .. 2027-06-27 | [`CivilTimeSource::IersLeapSeconds`] | existing `CivilTimestamp::to_approximate_tdb_julian_date` (TAI−UTC table, TT, TDB series); UT1 is taken as UTC (|UT1−UTC| < 0.9 s) |
//! | 1900-01-01 .. 1972-01-01 | [`CivilTimeSource::DeltaTPolynomial`] | `TT = UT + ΔT` with the Espenak–Meeus polynomial, see [`crate::delta_t`]; UT is the clock minus its offset; TDB = TT + the leading periodic term |
//! | anything else | error | no extrapolation, no fallback |
//!
//! The pre-1972 path inherits the stated ΔT uncertainty, so the Moon (about
//! 0.55 arcsecond per second of time) is the body that suffers most; the
//! result reports the `delta_t_seconds` it used. Leap-second labels (`:60`)
//! are rejected, exactly as in the 1972+ path.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::apparent_ecliptic_of_date::{
    geocentric_apparent_six_fields_many_with_options, ApparentError, ApparentOptions,
    ApparentSixFieldsDegreesAu,
};
use crate::calendar::{Calendar, CalendarDateTime, CalendarError, TimeScale};
use crate::de441::{Body, De441, JulianDateTdb};
use crate::delta_t::{
    decimal_year_from_jd, delta_t_seconds_1900_1972, DeltaTError, DELTA_T_1900_1972_MODEL_ID,
};
use crate::first_sky::{CivilTimestamp, SkyError};

const JD_1900_01_01: f64 = 2_415_020.5;
const JD_1972_01_01: f64 = 2_441_317.5;
const J2000_JD: f64 = 2_451_545.0;

/// Which rule produced the TDB epoch of a civil result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CivilTimeSource {
    /// 1972-01-01 .. 2027-06-27 with the IERS TAI−UTC table.
    IersLeapSeconds,
    /// 1900-01-01 .. 1972-01-01 with the named ΔT model and its value.
    DeltaTPolynomial {
        model_id: &'static str,
        delta_t_seconds: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CivilApparentBatch {
    pub reception_epoch: JulianDateTdb,
    pub time_source: CivilTimeSource,
    pub results: Vec<ApparentSixFieldsDegreesAu>,
}

#[derive(Debug)]
pub enum CivilApparentError {
    /// Offset outside ±14:00 or another malformed civil field.
    InvalidCivil(&'static str),
    /// The instant is before 1900-01-01 UT or after the validated IERS table.
    UnsupportedInstant,
    Calendar(CalendarError),
    DeltaT(DeltaTError),
    CivilTime(SkyError),
    Apparent(ApparentError),
}

impl fmt::Display for CivilApparentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCivil(s) => write!(f, "invalid civil timestamp: {s}"),
            Self::UnsupportedInstant => write!(
                f,
                "instant outside the supported civil range (1900-01-01 through 2027-06-27 UT)"
            ),
            Self::Calendar(e) => write!(f, "calendar: {e}"),
            Self::DeltaT(e) => write!(f, "delta T: {e}"),
            Self::CivilTime(e) => write!(f, "civil time: {e}"),
            Self::Apparent(e) => write!(f, "apparent positions: {e}"),
        }
    }
}

impl Error for CivilApparentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Calendar(e) => Some(e),
            Self::DeltaT(e) => Some(e),
            Self::CivilTime(e) => Some(e),
            Self::Apparent(e) => Some(e),
            Self::InvalidCivil(_) | Self::UnsupportedInstant => None,
        }
    }
}

impl From<CalendarError> for CivilApparentError {
    fn from(e: CalendarError) -> Self {
        Self::Calendar(e)
    }
}
impl From<DeltaTError> for CivilApparentError {
    fn from(e: DeltaTError) -> Self {
        Self::DeltaT(e)
    }
}
impl From<SkyError> for CivilApparentError {
    fn from(e: SkyError) -> Self {
        Self::CivilTime(e)
    }
}
impl From<ApparentError> for CivilApparentError {
    fn from(e: ApparentError) -> Self {
        Self::Apparent(e)
    }
}

/// UT Julian date of the civil clock (offset removed), single `f64`.
fn julian_date_ut(civil: CivilTimestamp) -> Result<f64, CivilApparentError> {
    if !(-840..=840).contains(&civil.utc_offset_minutes) {
        return Err(CivilApparentError::InvalidCivil(
            "UTC offset must be within ±14:00",
        ));
    }
    let local = CalendarDateTime {
        year: civil.year,
        month: civil.month,
        day: civil.day,
        hour: civil.hour,
        minute: civil.minute,
        second: civil.second,
        scale: TimeScale::Ut1,
    }
    .to_julian_date(Calendar::ProlepticGregorian)?;
    let (whole, fraction) = local.parts()?;
    Ok(whole + fraction - f64::from(civil.utc_offset_minutes) / 1_440.0)
}

/// Convert a civil timestamp to a TDB Julian date under the declared policy.
pub fn civil_to_tdb_with_policy(
    civil: CivilTimestamp,
) -> Result<(JulianDateTdb, CivilTimeSource), CivilApparentError> {
    let jd_ut = julian_date_ut(civil)?;
    if jd_ut < JD_1900_01_01 {
        return Err(CivilApparentError::UnsupportedInstant);
    }
    if jd_ut >= JD_1972_01_01 {
        return match civil.to_approximate_tdb_julian_date() {
            Ok(tdb) => Ok((tdb, CivilTimeSource::IersLeapSeconds)),
            Err(SkyError::UnsupportedUtc) => Err(CivilApparentError::UnsupportedInstant),
            Err(e) => Err(e.into()),
        };
    }
    let delta_t = delta_t_seconds_1900_1972(decimal_year_from_jd(jd_ut))?;
    let jd_tt = jd_ut + delta_t / 86_400.0;
    // Leading term of TDB − TT (seconds), consistent with the apparent module.
    let g = (357.5291092 + 35_999.0502909 * ((jd_tt - J2000_JD) / 36_525.0)).to_radians();
    let jd_tdb = jd_tt + 0.001657 * g.sin() / 86_400.0;
    Ok((
        JulianDateTdb(jd_tdb),
        CivilTimeSource::DeltaTPolynomial {
            model_id: DELTA_T_1900_1972_MODEL_ID,
            delta_t_seconds: delta_t,
        },
    ))
}

/// Geocentric apparent six fields for several bodies at a civil time.
///
/// The epoch is converted once. An empty request still validates the civil
/// time and returns the epoch and time source without touching the kernel.
pub fn geocentric_apparent_six_fields_many_from_civil<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    targets: &[Body],
    civil: CivilTimestamp,
    options: ApparentOptions,
) -> Result<CivilApparentBatch, CivilApparentError> {
    let (reception_epoch, time_source) = civil_to_tdb_with_policy(civil)?;
    let results = geocentric_apparent_six_fields_many_with_options(
        ephemeris,
        targets,
        reception_epoch,
        options,
    )?;
    Ok(CivilApparentBatch {
        reception_epoch,
        time_source,
        results,
    })
}
