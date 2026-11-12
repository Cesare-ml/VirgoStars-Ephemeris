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

//! Explicit civil timestamp to six **geocentric fixed-axis** quantities.
//!
//! This small adapter reuses the existing approximate UTC/TAI/TT/TDB
//! conversion, classic DE441 reader, reception light-time solver, and
//! fixed-axis Cartesian-to-spherical position/rate projection.
//!
//! This is **not** a Swiss Ephemeris or apparent-of-date calculation.
//! No annual aberration, gravitational deflection, precession, nutation,
//! topocentric parallax, or moving-frame derivative is applied.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::coordinates::{project_fixed_axes, ProjectionError, SphericalState};
use crate::de441::{Body, De441, JulianDateTdb, StateVector};
use crate::first_sky::{CivilTimestamp, SkyError};
use crate::light_time::{geocentric_light_time, LightTimeError};

/// Reception-time spherical state after **one-way light-time only**.
///
/// `spherical` contains six position/rate fields in fixed axes:
/// longitude/latitude (radians), range (km), longitude/latitude rates
/// (radians/reception TDB day), radial rate (km/reception TDB day).
/// The exact target body identifier, Earth center and reception TDB
/// Julian date are retained in `spherical`.
///
/// The target emission epoch, light-time and its reception-time rate are
/// preserved. The civil conversion uses the limited IERS table and a
/// truncated TDB-TT approximation; not full relativistic TDB accuracy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CivilGeocentricFixedSpherical {
    pub spherical: SphericalState,
    pub emission_epoch: JulianDateTdb,
    pub light_time_seconds: f64,
    pub light_time_rate: f64,
}

#[derive(Debug)]
pub enum CivilGeocentricError {
    CivilTime(SkyError),
    LightTime(LightTimeError),
    Projection(ProjectionError),
}

impl fmt::Display for CivilGeocentricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CivilTime(error) => write!(f, "civil time to approximate TDB: {error}"),
            Self::LightTime(error) => write!(f, "geocentric reception light-time: {error}"),
            Self::Projection(error) => write!(f, "fixed-axis spherical projection: {error}"),
        }
    }
}

impl Error for CivilGeocentricError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CivilTime(error) => Some(error),
            Self::LightTime(error) => Some(error),
            Self::Projection(error) => Some(error),
        }
    }
}

impl From<SkyError> for CivilGeocentricError {
    fn from(error: SkyError) -> Self {
        Self::CivilTime(error)
    }
}

impl From<LightTimeError> for CivilGeocentricError {
    fn from(error: LightTimeError) -> Self {
        Self::LightTime(error)
    }
}

impl From<ProjectionError> for CivilGeocentricError {
    fn from(error: ProjectionError) -> Self {
        Self::Projection(error)
    }
}

/// Compute six light-time-only geocentric spherical fields from civil time.
///
/// `civil` must have an explicit UTC offset and be inside the limited
/// 1972-01-01..2027-06-27 UTC conversion coverage (IERS Bulletin C 72).
/// The conversion uses a **truncated TDB-TT series** and returns one f64
/// TDB JD; no high-precision TDB or UT1 semantics may be inferred.
///
/// The caller supplies a readable/seekable **classic DE441** binary.
/// Target is observed relative to Earth's center at reception TDB, with
/// the target sampled at emission TDB. Outer DE441 planet-body variants
/// such as `JupiterBarycenter` remain system barycenters.
/// `fixed_tilt_rad` is an arbitrary **time-constant** passive X rotation:
/// zero means ICRF equatorial fixed axes, nonzero does NOT imply an
/// IAU-defined ecliptic or a moving true ecliptic of date.
///
/// Returns six **geometric + one-way light-time** quantities, not
/// aberrated/apparent positions or a `swe_calc_ut` substitute.
pub fn geocentric_fixed_spherical_from_civil<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    civil: CivilTimestamp,
    fixed_tilt_rad: f64,
) -> Result<CivilGeocentricFixedSpherical, CivilGeocentricError> {
    let reception_epoch = civil.to_approximate_tdb_julian_date()?;
    let state = geocentric_light_time(ephemeris, target, reception_epoch)?;

    let cartesian = StateVector {
        target: state.target,
        center: Body::Earth,
        epoch: reception_epoch,
        position_km: state.position_km,
        velocity_km_per_day: state.velocity_km_per_day,
    };
    let spherical = project_fixed_axes(&cartesian, fixed_tilt_rad)?;

    Ok(CivilGeocentricFixedSpherical {
        spherical,
        emission_epoch: state.emission_epoch,
        light_time_seconds: state.light_time_seconds,
        light_time_rate: state.light_time_rate,
    })
}
