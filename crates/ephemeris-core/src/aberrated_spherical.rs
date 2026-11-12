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

//! Composed observer-aberrated **fixed-axis** spherical positions and rates.
//!
//! This is a scientific intermediate for MVP 1, not an apparent ecliptic
//! of date or a drop-in Swiss Ephemeris `swe_calc_ut` implementation.
//! It reuses the original DE441/light-time/aberration implementation and
//! existing Cartesian-to-spherical projection, with no duplicated math.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::aberration_state::{
    geocentric_light_time_annual_aberration_analytic,
    geocentric_light_time_annual_aberration_with_earth, AberratedGeocentricState,
    AberratedStateError, ObserverAccelerationSource,
};
use crate::coordinates::{project_fixed_axes, ProjectionError, SphericalState};
use crate::de441::{Body, De441, JulianDateTdb, StateVector};
use crate::six_field_units::{
    fixed_spherical_to_degrees_au, FixedSixFieldsDegreesAu, UnitConversionError,
};

/// A spherical result computed *after* one-way reception light-time and
/// Earth's special-relativistic annual aberration. Source position/rate
/// are expressed in **fixed ICRF axes**, optionally rotated by a caller
/// chosen **constant** passive X tilt; not an ecliptic-of-date model.
///
/// `spherical` has **six fields**, longitude/latitude in radians,
/// range in km, angular rates in radians per reception **TDB day**, and
/// radial rate in km/TDB day. Its center is `Body::Earth`.
///
/// The exact `target` variant is retained: e.g. JupiterBarycenter is
/// not mislabeled Jupiter's physical center.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AberratedFixedSpherical {
    pub spherical: SphericalState,
    pub emission_epoch: JulianDateTdb,
    pub light_time_seconds: f64,
    pub light_time_rate: f64,
    pub observer_acceleration_source: ObserverAccelerationSource,
}

/// The same **fixed-axis** aberrated spherical six quantities in degrees/AU.
///
/// Not a Swiss `swe_calc_ut` result: the frame, input time-scale, target
/// (including DE441 planet-system barycenters) and applied physical
/// corrections are unchanged. `fields` always retains those labels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AberratedFixedSixFieldsDegreesAu {
    pub fields: FixedSixFieldsDegreesAu,
    pub emission_epoch: JulianDateTdb,
    pub light_time_seconds: f64,
    pub light_time_rate: f64,
    pub observer_acceleration_source: ObserverAccelerationSource,
}

/// A bad intermediate cannot be passed off as a computed sky position.
#[derive(Debug)]
pub enum AberratedSphericalError {
    InvalidIntermediate,
    Aberration(AberratedStateError),
    Projection(ProjectionError),
    Units(UnitConversionError),
}

impl fmt::Display for AberratedSphericalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIntermediate => write!(f, "invalid light-time and aberration metadata"),
            Self::Aberration(error) => write!(f, "aberration: {error}"),
            Self::Projection(error) => write!(f, "fixed-axis spherical projection: {error}"),
            Self::Units(error) => write!(f, "fixed-axis six-field units: {error}"),
        }
    }
}

impl Error for AberratedSphericalError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Aberration(error) => Some(error),
            Self::Projection(error) => Some(error),
            Self::Units(error) => Some(error),
            Self::InvalidIntermediate => None,
        }
    }
}

impl From<AberratedStateError> for AberratedSphericalError {
    fn from(error: AberratedStateError) -> Self {
        Self::Aberration(error)
    }
}

impl From<ProjectionError> for AberratedSphericalError {
    fn from(error: ProjectionError) -> Self {
        Self::Projection(error)
    }
}

impl From<UnitConversionError> for AberratedSphericalError {
    fn from(error: UnitConversionError) -> Self {
        Self::Units(error)
    }
}

/// Convert a **previously aberrated** Cartesian state and rate to six
/// fixed-axis spherical quantities, keeping reception and emission
/// metadata. No second light-time correction or second aberration is
/// applied; both position and velocity rotate through the same
/// constant X tilt.
///
/// The caller must not interpret a chosen tilt as the moving true
/// ecliptic-of-date frame; this function has no `Rdot × r` term.
pub fn spherical_from_aberrated_state(
    state: &AberratedGeocentricState,
    fixed_tilt_rad: f64,
) -> Result<AberratedFixedSpherical, AberratedSphericalError> {
    if state.target == Body::Earth
        || !state.observation_epoch.0.is_finite()
        || !state.emission_epoch.0.is_finite()
        || state.emission_epoch.0 > state.observation_epoch.0
        || !state.light_time_seconds.is_finite()
        || state.light_time_seconds <= 0.0
        || !state.light_time_rate.is_finite()
    {
        return Err(AberratedSphericalError::InvalidIntermediate);
    }
    let cartesian = StateVector {
        target: state.target,
        center: Body::Earth,
        epoch: state.observation_epoch,
        position_km: state.position_km,
        velocity_km_per_day: state.velocity_km_per_tdb_day,
    };
    let spherical = project_fixed_axes(&cartesian, fixed_tilt_rad)?;
    Ok(AberratedFixedSpherical {
        spherical,
        emission_epoch: state.emission_epoch,
        light_time_seconds: state.light_time_seconds,
        light_time_rate: state.light_time_rate,
        observer_acceleration_source: state.observer_acceleration_source,
    })
}

/// One-call **classic-DE441 → geocentric light-time → observer aberration
/// with analytic Earth acceleration → six fixed-frame spherical fields**
/// and reception-time speeds.
///
/// The caller supplies a separately obtained classic JPL DE441 `.eph`
/// and a **TDB** Julian epoch. No hidden data file, UTC/TT conversion,
/// actual moving-date-frame precession/nutation, gravitational light
/// deflection or outer-system-barycenter-to-planet-center correction
/// is performed. Output is NOT Swiss `swe_calc_ut` parity.
pub fn geocentric_aberrated_fixed_spherical<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
    fixed_tilt_rad: f64,
) -> Result<AberratedFixedSpherical, AberratedSphericalError> {
    let state =
        geocentric_light_time_annual_aberration_analytic(ephemeris, target, reception_epoch)?;
    spherical_from_aberrated_state(&state, fixed_tilt_rad)
}

/// Convert an already projected, aberrated fixed-frame `SphericalState`
/// to degrees/AU, retaining emission/reception and observer-acceleration
/// metadata. This is **only unit conversion** and does not redo aberration.
pub fn aberrated_spherical_to_degrees_au(
    result: &AberratedFixedSpherical,
) -> Result<AberratedFixedSixFieldsDegreesAu, AberratedSphericalError> {
    Ok(AberratedFixedSixFieldsDegreesAu {
        fields: fixed_spherical_to_degrees_au(&result.spherical)?,
        emission_epoch: result.emission_epoch,
        light_time_seconds: result.light_time_seconds,
        light_time_rate: result.light_time_rate,
        observer_acceleration_source: result.observer_acceleration_source,
    })
}

/// One-call classic-DE441 → Earth reception light-time → analytic
/// Earth-acceleration annual aberration → six **fixed-frame** fields in
/// degrees/AU, including all three signed reception-TDB-day rates.
///
/// This MUST NOT be described as apparent ecliptic-of-date or Swiss
/// `swe_calc_ut` parity: frame bias/precession/nutation, moving-frame
/// derivatives, solar deflection and several time/body models are absent.
pub fn geocentric_aberrated_fixed_six_fields<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    reception_epoch: JulianDateTdb,
    fixed_tilt_rad: f64,
) -> Result<AberratedFixedSixFieldsDegreesAu, AberratedSphericalError> {
    let spherical =
        geocentric_aberrated_fixed_spherical(ephemeris, target, reception_epoch, fixed_tilt_rad)?;
    aberrated_spherical_to_degrees_au(&spherical)
}

/// Calculate observer-aberrated, fixed-axis geocentric six fields for
/// several target bodies at **one already-TDB** reception Julian date.
///
/// This batch retains the scalar pipeline's light-time solution,
/// Lorentz boost, spherical projection and unit conversion. It evaluates
/// Earth's barycentric Chebyshev position/velocity/acceleration **once**
/// at reception TDB, then reuses that validated observer state for each
/// target's independent light-time and aberration calculation.
/// Input order, duplicates and per-target metadata are preserved.
/// This avoids repeated Earth-state interpolation but does **not**
/// claim a measured runtime speedup; target evaluation still occurs
/// separately for every requested body.
///
/// All-or-error: a failed target (missing series, out-of-coverage
/// emission epoch, Earth as target, pole singularity, etc.) rejects
/// the whole batch and exposes the original typed error. An empty
/// target list returns an empty vector **only after** checking finite
/// TDB epoch and finite constant tilt, without probing kernel coverage.
/// No partial output vector is returned on error.
///
/// Output angles/rates use degrees, AU and **reception TDB days**;
/// frame is fixed ICRF or a caller-selected constant X tilt, never
/// the true ecliptic of date. The correction set includes one-way
/// light-time and annual aberration **only**: there is no solar
/// gravitational deflection, date-frame rotation, observer parallax,
/// or Swiss Ephemeris parity claim.
pub fn geocentric_aberrated_fixed_six_fields_many<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    targets: &[Body],
    reception_epoch: JulianDateTdb,
    fixed_tilt_rad: f64,
) -> Result<Vec<AberratedFixedSixFieldsDegreesAu>, AberratedSphericalError> {
    if !reception_epoch.0.is_finite() {
        return Err(AberratedSphericalError::InvalidIntermediate);
    }
    if !fixed_tilt_rad.is_finite() {
        return Err(AberratedSphericalError::Projection(
            ProjectionError::NonFinite,
        ));
    }

    // Preserve the documented empty-batch contract without probing
    // kernel coverage or performing an Earth interpolation.
    if targets.is_empty() {
        return Ok(Vec::new());
    }

    let earth = ephemeris
        .state_with_acceleration(
            Body::Earth,
            Body::SolarSystemBarycenter,
            reception_epoch,
        )
        .map_err(AberratedStateError::from)?;

    targets
        .iter()
        .copied()
        .map(|target| {
            let aberrated = geocentric_light_time_annual_aberration_with_earth(
                ephemeris,
                target,
                reception_epoch,
                &earth,
            )?;
            let spherical = spherical_from_aberrated_state(&aberrated, fixed_tilt_rad)?;
            aberrated_spherical_to_degrees_au(&spherical)
        })
        .collect()
}
