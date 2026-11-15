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

//! Experimental catalog-star propagation; no bundled catalog. Added 2026-11-15.
//! Constant rectilinear motion, Earth parallax and annual aberration only.
//! Not a complete apparent place or Swiss fixed-star replacement.

use crate::aberration::{annual_aberration_icrf, AberrationError};
use crate::de441::{Body, De441, De441Error, JulianDateTdb};
use crate::earth_orientation::{
    matrix_vector_multiply, true_ecliptic_of_date_matrix, OrientationError,
};
use crate::six_field_units::IAU_AU_KM;
use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

const DAYS_PER_YEAR: f64 = 365.25;
const RAD_PER_MAS: f64 = std::f64::consts::PI / (180.0 * 3600.0 * 1000.0);
const STEP_DAYS: f64 = 0.01;

/// Barycentric ICRS catalog values at an explicit TDB epoch. Proper motion in
/// RA is μ_α* = dα/dt cos(dec), not raw dα/dt. No values are inferred.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedStarCatalogEntry {
    pub epoch_tdb: JulianDateTdb,
    pub right_ascension_deg: f64,
    pub declination_deg: f64,
    pub proper_motion_ra_cos_dec_mas_per_year: f64,
    pub proper_motion_dec_mas_per_year: f64,
    pub parallax_mas: f64,
    pub radial_velocity_km_per_second: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExperimentalStarPosition {
    pub tt_jd: f64,
    pub tdb: JulianDateTdb,
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub distance_au: f64,
    pub longitude_speed_deg_per_tdb_day: f64,
    pub latitude_speed_deg_per_tdb_day: f64,
    pub distance_speed_au_per_tdb_day: f64,
}

#[derive(Debug)]
pub enum FixedStarError {
    InvalidCatalog,
    IncoherentEpoch,
    InvalidGeometry,
    Ephemeris(De441Error),
    Orientation(OrientationError),
    Aberration(AberrationError),
}
impl fmt::Display for FixedStarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCatalog => write!(f, "invalid or unsupported star catalog values"),
            Self::IncoherentEpoch => write!(f, "TT and TDB must describe the same instant"),
            Self::InvalidGeometry => write!(f, "invalid or singular star geometry"),
            Self::Ephemeris(e) => write!(f, "ephemeris: {e}"),
            Self::Orientation(e) => write!(f, "orientation: {e}"),
            Self::Aberration(e) => write!(f, "aberration: {e}"),
        }
    }
}
impl Error for FixedStarError {}

/// Geometric observer-to-star ICRS vector in AU. The observer is supplied as
/// a barycentric position in AU, at reception. Rectilinear propagation uses
/// the catalog epoch and TDB Julian years; no stellar light-time correction.
pub fn geometric_star_vector(
    entry: FixedStarCatalogEntry,
    tdb: JulianDateTdb,
    observer_au: [f64; 3],
) -> Result<[f64; 3], FixedStarError> {
    if ![
        entry.epoch_tdb.0,
        entry.right_ascension_deg,
        entry.declination_deg,
        entry.proper_motion_ra_cos_dec_mas_per_year,
        entry.proper_motion_dec_mas_per_year,
        entry.parallax_mas,
        entry.radial_velocity_km_per_second,
        tdb.0,
    ]
    .iter()
    .chain(observer_au.iter())
    .all(|v| v.is_finite())
        || !(0.0..360.0).contains(&entry.right_ascension_deg)
        || entry.declination_deg.abs() >= 90.0
        || !(0.001..=10_000.0).contains(&entry.parallax_mas)
        || entry.radial_velocity_km_per_second.abs() >= 299_792.458
    {
        return Err(FixedStarError::InvalidCatalog);
    }
    let (sa, ca) = entry.right_ascension_deg.to_radians().sin_cos();
    let (sd, cd) = entry.declination_deg.to_radians().sin_cos();
    let direction = [cd * ca, cd * sa, sd];
    let east = [-sa, ca, 0.0];
    let north = [-sd * ca, -sd * sa, cd];
    let distance_au = 1.0 / (entry.parallax_mas * RAD_PER_MAS);
    let mu_a = entry.proper_motion_ra_cos_dec_mas_per_year * RAD_PER_MAS / DAYS_PER_YEAR;
    let mu_d = entry.proper_motion_dec_mas_per_year * RAD_PER_MAS / DAYS_PER_YEAR;
    let radial = entry.radial_velocity_km_per_second * 86_400.0 / IAU_AU_KM;
    let velocity: [f64; 3] = std::array::from_fn(|i| {
        distance_au * (mu_a * east[i] + mu_d * north[i]) + radial * direction[i]
    });
    let speed = velocity[0].hypot(velocity[1]).hypot(velocity[2]);
    if !speed.is_finite() || speed >= 299_792.458 * 86_400.0 / IAU_AU_KM {
        return Err(FixedStarError::InvalidCatalog);
    }
    let dt = tdb.0 - entry.epoch_tdb.0;
    let position: [f64; 3] =
        std::array::from_fn(|i| distance_au * direction[i] + dt * velocity[i] - observer_au[i]);
    let norm = position[0].hypot(position[1]).hypot(position[2]);
    if !norm.is_finite() || norm <= 0.0 {
        return Err(FixedStarError::InvalidGeometry);
    }
    Ok(position)
}

fn sample<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    entry: FixedStarCatalogEntry,
    tt: f64,
    tdb: JulianDateTdb,
) -> Result<(f64, f64, f64), FixedStarError> {
    if !tt.is_finite() || !tdb.0.is_finite() || (tt - tdb.0).abs() > 1.0 / 86_400.0 {
        return Err(FixedStarError::IncoherentEpoch);
    }
    let earth = ephemeris
        .state(Body::Earth, Body::SolarSystemBarycenter, tdb)
        .map_err(FixedStarError::Ephemeris)?;
    let relative = geometric_star_vector(entry, tdb, earth.position_km.map(|v| v / IAU_AU_KM))?;
    let distance = relative[0].hypot(relative[1]).hypot(relative[2]);
    let direction = annual_aberration_icrf(
        relative.map(|value| value * IAU_AU_KM),
        earth.velocity_km_per_day,
    )
    .map_err(FixedStarError::Aberration)?;
    let matrix = true_ecliptic_of_date_matrix(tt).map_err(FixedStarError::Orientation)?;
    let rotated = matrix_vector_multiply(matrix, direction);
    let xy = rotated[0].hypot(rotated[1]);
    if !xy.is_finite() || xy <= 32.0 * f64::EPSILON {
        return Err(FixedStarError::InvalidGeometry);
    }
    let longitude = rotated[1].atan2(rotated[0]).to_degrees().rem_euclid(360.0);
    Ok((
        if longitude >= 360.0 { 0.0 } else { longitude },
        rotated[2].atan2(xy).to_degrees(),
        distance,
    ))
}

/// DE441 observer + rectilinear star + annual aberration, projected to the
/// IAU 2006/2000B true ecliptic of date. Speeds use a centered 0.01-TDB-day
/// difference, advancing TT and TDB equally. Gravitational bending, Roemer
/// adjustment, nonlinear stellar motion and complete nutation are omitted.
pub fn calculate_star<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    entry: FixedStarCatalogEntry,
    tt: f64,
    tdb: JulianDateTdb,
) -> Result<ExperimentalStarPosition, FixedStarError> {
    let now = sample(ephemeris, entry, tt, tdb)?;
    let before = sample(
        ephemeris,
        entry,
        tt - STEP_DAYS,
        JulianDateTdb(tdb.0 - STEP_DAYS),
    )?;
    let after = sample(
        ephemeris,
        entry,
        tt + STEP_DAYS,
        JulianDateTdb(tdb.0 + STEP_DAYS),
    )?;
    let mut delta = after.0 - before.0;
    if delta >= 180.0 {
        delta -= 360.0;
    }
    if delta < -180.0 {
        delta += 360.0;
    }
    let span = (tdb.0 + STEP_DAYS) - (tdb.0 - STEP_DAYS);
    Ok(ExperimentalStarPosition {
        tt_jd: tt,
        tdb,
        longitude_deg: now.0,
        latitude_deg: now.1,
        distance_au: now.2,
        longitude_speed_deg_per_tdb_day: delta / span,
        latitude_speed_deg_per_tdb_day: (after.1 - before.1) / span,
        distance_speed_au_per_tdb_day: (after.2 - before.2) / span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn example() -> FixedStarCatalogEntry {
        FixedStarCatalogEntry {
            epoch_tdb: JulianDateTdb(2_451_545.0),
            right_ascension_deg: 0.0,
            declination_deg: 0.0,
            proper_motion_ra_cos_dec_mas_per_year: 1000.0,
            proper_motion_dec_mas_per_year: 0.0,
            parallax_mas: 1000.0,
            radial_velocity_km_per_second: 0.0,
        }
    }
    #[test]
    fn motion_and_parallax_have_expected_direction_and_scale() {
        let e = example();
        let p = geometric_star_vector(e, JulianDateTdb(e.epoch_tdb.0 + 3652.5), [0.0; 3]).unwrap();
        let angle = p[1].atan2(p[0]).to_degrees();
        assert!((angle - 10.0 / 3600.0).abs() < 1e-10);
        let shifted = geometric_star_vector(e, e.epoch_tdb, [0.0, 1.0, 0.0]).unwrap();
        assert!((shifted[1].atan2(shifted[0]).to_degrees() + 1.0 / 3600.0).abs() < 1e-10);
    }
    #[test]
    fn erfa_pmpx_published_fixture_is_consistent_to_one_nanoradian() {
        // Independent ERFA pmpx test: small expected difference from omitted
        // Roemer correction; source arguments use dRA/dt rather than μ_RA*.
        let mas_per_rad = 1.0 / RAD_PER_MAS;
        let entry = FixedStarCatalogEntry {
            epoch_tdb: JulianDateTdb(2_451_545.0),
            right_ascension_deg: 1.234_f64.to_degrees(),
            declination_deg: 0.789_f64.to_degrees(),
            proper_motion_ra_cos_dec_mas_per_year: 1e-5 * 0.789_f64.cos() * mas_per_rad,
            proper_motion_dec_mas_per_year: -2e-5 * mas_per_rad,
            parallax_mas: 10.0,
            radial_velocity_km_per_second: 10.0,
        };
        let p = geometric_star_vector(
            entry,
            JulianDateTdb(2_451_545.0 + 8.75 * 365.25),
            [0.9, 0.4, 0.1],
        )
        .unwrap();
        let n = p[0].hypot(p[1]).hypot(p[2]);
        for (v, reference) in p.into_iter().zip([
            0.232_813_762_396_030_84,
            0.665_109_708_539_785_5,
            0.709_525_776_589_636,
        ]) {
            assert!((v / n - reference).abs() < 1e-9);
        }
    }
    #[test]
    fn invalid_parallax_and_poles_are_explicit_errors() {
        let mut e = example();
        e.parallax_mas = 0.0;
        assert!(geometric_star_vector(e, e.epoch_tdb, [0.0; 3]).is_err());
        e = example();
        e.declination_deg = 90.0;
        assert!(geometric_star_vector(e, e.epoch_tdb, [0.0; 3]).is_err());
    }
}
