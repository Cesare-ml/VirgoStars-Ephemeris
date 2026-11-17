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

//! Civil-time adapter to the existing multi-target annual-aberration
//! six-field pipeline. Astronomy is delegated to the original Rust core.
//!
//! This is **not** an apparent true-ecliptic-of-date or Swiss-compatible
//! calculator. The civil UTC→TDB step is a bounded approximation.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::aberrated_spherical::{
    geocentric_aberrated_fixed_six_fields_many, AberratedFixedSixFieldsDegreesAu,
    AberratedSphericalError,
};
use crate::de441::{Body, De441, JulianDateTdb};
use crate::first_sky::{CivilTimestamp, SkyError};

/// A shared reception epoch and all requested geocentric target results.
///
/// The epoch is a **single-f64 approximate TDB Julian date** derived from
/// the explicit UTC offset on `CivilTimestamp`. Even an empty request
/// retains its converted epoch. The results preserve input ordering and
/// duplicates. Target and Earth center are retained individually.
#[derive(Clone, Debug, PartialEq)]
pub struct CivilAberratedSixFieldsBatch {
    pub reception_epoch: JulianDateTdb,
    pub results: Vec<AberratedFixedSixFieldsDegreesAu>,
}

/// Keeps UTC conversion failures distinct from ephemeris/projection errors.
#[derive(Debug)]
pub enum CivilAberratedError {
    CivilTime(SkyError),
    Aberrated(AberratedSphericalError),
}

impl fmt::Display for CivilAberratedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CivilTime(error) => write!(f, "civil time to approximate TDB: {error}"),
            Self::Aberrated(error) => write!(f, "geocentric annual-aberration six fields: {error}"),
        }
    }
}

impl Error for CivilAberratedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CivilTime(error) => Some(error),
            Self::Aberrated(error) => Some(error),
        }
    }
}

impl From<SkyError> for CivilAberratedError {
    fn from(error: SkyError) -> Self {
        Self::CivilTime(error)
    }
}

impl From<AberratedSphericalError> for CivilAberratedError {
    fn from(error: AberratedSphericalError) -> Self {
        Self::Aberrated(error)
    }
}

/// Compute six signed fixed-axis geocentric values per target from a civil
/// time with an **explicit UTC offset** (including `Z`).
///
/// Converts the clock into approximate TDB **once** using the existing
/// `CivilTimestamp::to_approximate_tdb_julian_date` from dependency #96.
/// That conversion accepts UTC instants from 1972-01-01 through
/// 2027-06-27 (IERS Bulletin C 72) and a truncated USNO TDB−TT series.
/// Leap-second `:60` labels and high-precision TDB claims are excluded.
///
/// Delegates without reimplementing astronomy to
/// `geocentric_aberrated_fixed_six_fields_many` from dependency #107:
/// a caller-supplied **classic DE441** binary is evaluated at one reception
/// TDB epoch; each body's one-way emission light time and annual
/// aberration (including analytic Earth Chebyshev acceleration) are
/// computed independently by that existing pipeline.
///
/// Results contain degrees/AU and their signed rates per **reception
/// TDB day** in **fixed ICRF equatorial axes** when `fixed_tilt_rad=0`.
/// Any nonzero tilt is a constant, caller-supplied passive X rotation:
/// it is NOT a moving ecliptic of date and has no frame derivative.
/// Outer planet `Body` variants remain *system barycenters*.
///
/// Preserves duplicate targets and caller order. Any failed body rejects
/// the whole call, with the original typed error nested in
/// `CivilAberratedError::Aberrated`; nothing is silently skipped.
/// An empty request validates civil time and finite fixed tilt and
/// returns an empty vector with the converted reception epoch; it does
/// **not** check whether a particular body exists in the data file.
///
/// No gravitational light deflection, complete precession/nutation,
/// topocentric parallax, physical outer-planet centers or independent
/// real JPL/Horizons/Swiss numerical validation is implied.
pub fn geocentric_aberrated_fixed_six_fields_many_from_civil<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    targets: &[Body],
    civil: CivilTimestamp,
    fixed_tilt_rad: f64,
) -> Result<CivilAberratedSixFieldsBatch, CivilAberratedError> {
    let reception_epoch = civil.to_approximate_tdb_julian_date()?;
    let results = geocentric_aberrated_fixed_six_fields_many(
        ephemeris,
        targets,
        reception_epoch,
        fixed_tilt_rad,
    )?;
    Ok(CivilAberratedSixFieldsBatch {
        reception_epoch,
        results,
    })
}
