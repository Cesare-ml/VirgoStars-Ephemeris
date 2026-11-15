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

//! Opt-in geocentric planetary solar deflection, added 2026-11-15.
//! Finite source, light time, first-order point-mass solar bending, annual
//! aberration and IAU 2006/2000B date-frame projection. This is not Swiss
//! parity or a fully validated apparent-place model.

use crate::aberration_state::{aberrate_light_time_state, AberratedStateError};
use crate::de441::{Body, De441, De441Error, JulianDateTdb};
use crate::light_time::{solve_from_observer_custom, GeocentricLightTimeState, LightTimeError};
use crate::physical_center::{add_offset, PhysicalPlanet};
use crate::six_field_units::IAU_AU_KM;
use crate::solar_deflection::{deflect_finite_source, SolarDeflectionError};
use crate::spk_kernel_set::SpkKernelSet;
use crate::true_ecliptic::{project_true_ecliptic, project_true_equatorial, TrueEclipticError};
use std::{
    error::Error,
    fmt,
    io::{Read, Seek},
};

const STEP_DAYS: f64 = 0.005;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetTarget {
    Sun,
    Moon,
    Mercury,
    Venus,
    Physical(PhysicalPlanet),
}
impl PlanetTarget {
    fn body(self) -> Body {
        match self {
            Self::Sun => Body::Sun,
            Self::Moon => Body::Moon,
            Self::Mercury => Body::Mercury,
            Self::Venus => Body::Venus,
            Self::Physical(p) => p.identifiers().0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetFrame {
    TrueEcliptic,
    TrueEquatorial,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetDeflectedPosition {
    pub target: PlanetTarget,
    pub frame: PlanetFrame,
    pub tt_jd: f64,
    pub tdb: JulianDateTdb,
    /// Ecliptic longitude or equatorial right ascension, degrees.
    pub first_angle_deg: f64,
    /// Ecliptic latitude or equatorial declination, degrees.
    pub second_angle_deg: f64,
    pub distance_au: f64,
    pub first_angle_speed_deg_per_tdb_day: f64,
    pub second_angle_speed_deg_per_tdb_day: f64,
    pub distance_speed_au_per_tdb_day: f64,
}

#[derive(Debug)]
pub enum PlanetaryDeflectionError {
    InvalidEpoch,
    MissingCenterKernel,
    Ephemeris(De441Error),
    LightTime(LightTimeError),
    Solar(SolarDeflectionError),
    Aberration(AberratedStateError),
    Projection(TrueEclipticError),
}
impl fmt::Display for PlanetaryDeflectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEpoch => write!(f, "TT and TDB must describe the same finite instant"),
            Self::MissingCenterKernel => write!(f, "an explicit physical-center SPK is required"),
            Self::Ephemeris(e) => write!(f, "ephemeris: {e}"),
            Self::LightTime(e) => write!(f, "light time: {e}"),
            Self::Solar(e) => write!(f, "solar deflection: {e}"),
            Self::Aberration(e) => write!(f, "aberration: {e}"),
            Self::Projection(e) => write!(f, "projection: {e}"),
        }
    }
}
impl Error for PlanetaryDeflectionError {}

fn sample<R: Read + Seek, S: Read + Seek>(
    ephemeris: &mut De441<R>,
    kernels: &mut Option<&mut SpkKernelSet<S>>,
    target: PlanetTarget,
    tdb: JulianDateTdb,
) -> Result<(GeocentricLightTimeState, [f64; 3]), PlanetaryDeflectionError> {
    let body = target.body();
    let light = solve_from_observer_custom(body, Body::Earth, tdb, |requested, epoch| {
        let state = ephemeris
            .state(requested, Body::SolarSystemBarycenter, epoch)
            .map_err(LightTimeError::Ephemeris)?;
        if requested == Body::Earth {
            return Ok(state);
        }
        if let PlanetTarget::Physical(planet) = target {
            let (_, target_id, center_id) = planet.identifiers();
            let set = kernels
                .as_deref_mut()
                .ok_or(LightTimeError::InvalidGeometry)?;
            let offset = set
                .relative_state(target_id, center_id, (epoch.0 - 2_451_545.0) * 86_400.0)
                .map_err(LightTimeError::SupplementalSpk)?;
            return add_offset(state, &offset).map_err(|_| LightTimeError::InvalidGeometry);
        }
        Ok(state)
    })
    .map_err(PlanetaryDeflectionError::LightTime)?;
    if target == PlanetTarget::Sun {
        return Ok((light, light.position_km));
    }
    let earth = ephemeris
        .state(Body::Earth, Body::SolarSystemBarycenter, tdb)
        .map_err(PlanetaryDeflectionError::Ephemeris)?;
    let sun = ephemeris
        .state(Body::Sun, Body::SolarSystemBarycenter, tdb)
        .map_err(PlanetaryDeflectionError::Ephemeris)?;
    let sun_from_earth =
        std::array::from_fn(|i| (sun.position_km[i] - earth.position_km[i]) / IAU_AU_KM);
    let source_from_earth = light.position_km.map(|v| v / IAU_AU_KM);
    let deflected = deflect_finite_source(source_from_earth, sun_from_earth)
        .map_err(PlanetaryDeflectionError::Solar)?;
    let distance = light.position_km[0]
        .hypot(light.position_km[1])
        .hypot(light.position_km[2]);
    Ok((light, deflected.direction.map(|v| v * distance)))
}

/// Geocentric light-time, finite-source point-mass solar deflection and annual
/// aberration. Solar deflection is not applied to the Sun itself. Physical
/// Mars–Pluto require an explicit SPK center kernel. Corrected ICRF velocity
/// uses a centered 0.005-TDB-day difference of corrected position samples;
/// date-frame motion is added separately by the projection. Solar limb
/// occultations or missing coverage at any sample are errors.
pub fn calculate_deflected_planet<R: Read + Seek, S: Read + Seek>(
    ephemeris: &mut De441<R>,
    mut kernels: Option<&mut SpkKernelSet<S>>,
    target: PlanetTarget,
    tdb: JulianDateTdb,
    tt_jd: f64,
    frame: PlanetFrame,
) -> Result<PlanetDeflectedPosition, PlanetaryDeflectionError> {
    if !tdb.0.is_finite() || !tt_jd.is_finite() || (tt_jd - tdb.0).abs() > 1.0 / 86_400.0 {
        return Err(PlanetaryDeflectionError::InvalidEpoch);
    }
    if matches!(target, PlanetTarget::Physical(_)) && kernels.is_none() {
        return Err(PlanetaryDeflectionError::MissingCenterKernel);
    }
    let (mut light, now) = sample(ephemeris, &mut kernels, target, tdb)?;
    if target != PlanetTarget::Sun {
        let (_, before) = sample(
            ephemeris,
            &mut kernels,
            target,
            JulianDateTdb(tdb.0 - STEP_DAYS),
        )?;
        let (_, after) = sample(
            ephemeris,
            &mut kernels,
            target,
            JulianDateTdb(tdb.0 + STEP_DAYS),
        )?;
        let span = (tdb.0 + STEP_DAYS) - (tdb.0 - STEP_DAYS);
        light.position_km = now;
        light.velocity_km_per_day = std::array::from_fn(|i| (after[i] - before[i]) / span);
    }
    let earth = ephemeris
        .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, tdb)
        .map_err(PlanetaryDeflectionError::Ephemeris)?;
    let aberrated = aberrate_light_time_state(
        &light,
        earth.velocity_km_per_day,
        earth.acceleration_km_per_day2,
    )
    .map_err(PlanetaryDeflectionError::Aberration)?;
    let (first, second, distance, first_speed, second_speed, distance_speed) = match frame {
        PlanetFrame::TrueEcliptic => {
            let s = project_true_ecliptic(&aberrated, tt_jd)
                .map_err(PlanetaryDeflectionError::Projection)?;
            (
                s.longitude_deg,
                s.latitude_deg,
                s.distance_au,
                s.longitude_speed_deg_per_tdb_day,
                s.latitude_speed_deg_per_tdb_day,
                s.distance_speed_au_per_tdb_day,
            )
        }
        PlanetFrame::TrueEquatorial => {
            let s = project_true_equatorial(&aberrated, tt_jd)
                .map_err(PlanetaryDeflectionError::Projection)?;
            (
                s.right_ascension_deg,
                s.declination_deg,
                s.distance_au,
                s.right_ascension_speed_deg_per_tdb_day,
                s.declination_speed_deg_per_tdb_day,
                s.distance_speed_au_per_tdb_day,
            )
        }
    };
    Ok(PlanetDeflectedPosition {
        target,
        frame,
        tt_jd,
        tdb,
        first_angle_deg: first,
        second_angle_deg: second,
        distance_au: distance,
        first_angle_speed_deg_per_tdb_day: first_speed,
        second_angle_speed_deg_per_tdb_day: second_speed,
        distance_speed_au_per_tdb_day: distance_speed,
    })
}
