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
//! Constant rectilinear motion, Earth parallax, annual aberration and an
//! explicitly selected optional point-mass solar light deflection.
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
// IAU nominal solar mass parameter, km^3/s^2; optical solar radius, km.
const NOMINAL_SOLAR_GM_KM3_S2: f64 = 132_712_440_000.0;
const SOLAR_RADIUS_KM: f64 = 695_700.0;
const SPEED_OF_LIGHT_KM_S: f64 = 299_792.458;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StellarSolarDeflection {
    Omit,
    PointMass,
}

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
    OccultedBySun,
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
            Self::OccultedBySun => write!(f, "stellar ray intersects the adopted solar disk"),
            Self::Ephemeris(e) => write!(f, "ephemeris: {e}"),
            Self::Orientation(e) => write!(f, "orientation: {e}"),
            Self::Aberration(e) => write!(f, "aberration: {e}"),
        }
    }
}
impl Error for FixedStarError {}

/// First-order point-mass solar deflection for a distant source. Both inputs
/// are ICRS vectors: observer-to-source and Sun-to-observer, in AU. The
/// source must be at least 1000 times the Sun-observer distance and outside
/// the adopted solar disk; an occulted ray is an error. No planetary
/// deflection or higher-order solar terms are included.
/// The vector expression follows the distant-source limit of equation (1)
/// in Kaplan, USNO Circular 179 (1998), https://aa.usno.navy.mil/downloads/reports/Kaplan1998.pdf.
pub fn solar_deflect_distant_star(
    observer_to_source_au: [f64; 3],
    sun_to_observer_au: [f64; 3],
) -> Result<[f64; 3], FixedStarError> {
    if observer_to_source_au
        .iter()
        .chain(sun_to_observer_au.iter())
        .any(|v| !v.is_finite())
    {
        return Err(FixedStarError::InvalidGeometry);
    }
    let norm = |v: [f64; 3]| v[0].hypot(v[1]).hypot(v[2]);
    let source_distance = norm(observer_to_source_au);
    let solar_distance = norm(sun_to_observer_au);
    if !source_distance.is_finite()
        || !solar_distance.is_finite()
        || solar_distance * IAU_AU_KM <= SOLAR_RADIUS_KM
        || source_distance < 1000.0 * solar_distance
    {
        return Err(FixedStarError::InvalidGeometry);
    }
    let p = observer_to_source_au.map(|v| v / source_distance);
    let e = sun_to_observer_au.map(|v| v / solar_distance);
    let dot = (0..3).map(|i| p[i] * e[i]).sum::<f64>().clamp(-1.0, 1.0);
    let angular_radius_sin = SOLAR_RADIUS_KM / (solar_distance * IAU_AU_KM);
    if dot <= -(1.0 - angular_radius_sin * angular_radius_sin).sqrt() {
        return Err(FixedStarError::OccultedBySun);
    }
    let scale = 2.0 * NOMINAL_SOLAR_GM_KM3_S2
        / (SPEED_OF_LIGHT_KM_S.powi(2) * solar_distance * IAU_AU_KM * (1.0 + dot));
    let vector: [f64; 3] = std::array::from_fn(|i| p[i] + scale * (e[i] - dot * p[i]));
    let magnitude = norm(vector);
    if !magnitude.is_finite() || magnitude <= 0.0 {
        return Err(FixedStarError::InvalidGeometry);
    }
    Ok(vector.map(|v| v / magnitude))
}

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

/// SSB-centered, geometric ICRS astrometric direction at an explicit TDB epoch.
/// No DE441 kernel, observer parallax, annual aberration, solar deflection,
/// precession, nutation or stellar light-time is applied. This is a native
/// experimental comparison path, **not** the apparent ecliptic star API.
/// The source six-vector is propagated rectilinearly; ERFA pmsafe uses its
/// own safe distance handling and may differ for pathological inputs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IcrsAstrometricStarPosition {
    pub tdb_julian_day: f64,
    pub right_ascension_deg: f64,
    pub declination_deg: f64,
    pub distance_au: f64,
}

pub fn propagate_star_icrs_astrometric(
    entry: FixedStarCatalogEntry,
    tdb: JulianDateTdb,
) -> Result<IcrsAstrometricStarPosition, FixedStarError> {
    let p = geometric_star_vector(entry, tdb, [0.0; 3])?;
    let xy = p[0].hypot(p[1]);
    let distance = xy.hypot(p[2]);
    if !xy.is_finite() || xy <= 32.0 * f64::EPSILON || !distance.is_finite() || distance <= 0.0 {
        return Err(FixedStarError::InvalidGeometry);
    }
    let ra = p[1].atan2(p[0]).to_degrees().rem_euclid(360.0);
    let dec = p[2].atan2(xy).to_degrees();
    if !ra.is_finite() || !dec.is_finite() || dec.abs() >= 90.0 {
        return Err(FixedStarError::InvalidGeometry);
    }
    Ok(IcrsAstrometricStarPosition {
        tdb_julian_day: tdb.0,
        right_ascension_deg: if ra >= 360.0 { 0.0 } else { ra },
        declination_deg: dec,
        distance_au: distance,
    })
}

fn sample<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    entry: FixedStarCatalogEntry,
    tt: f64,
    tdb: JulianDateTdb,
    solar_deflection: StellarSolarDeflection,
) -> Result<(f64, f64, f64), FixedStarError> {
    if !tt.is_finite() || !tdb.0.is_finite() || (tt - tdb.0).abs() > 1.0 / 86_400.0 {
        return Err(FixedStarError::IncoherentEpoch);
    }
    let earth = ephemeris
        .state(Body::Earth, Body::SolarSystemBarycenter, tdb)
        .map_err(FixedStarError::Ephemeris)?;
    let relative = geometric_star_vector(entry, tdb, earth.position_km.map(|v| v / IAU_AU_KM))?;
    let distance = relative[0].hypot(relative[1]).hypot(relative[2]);
    let incoming = match solar_deflection {
        StellarSolarDeflection::Omit => relative,
        StellarSolarDeflection::PointMass => {
            let sun = ephemeris
                .state(Body::Sun, Body::SolarSystemBarycenter, tdb)
                .map_err(FixedStarError::Ephemeris)?;
            let earth_from_sun =
                std::array::from_fn(|i| (earth.position_km[i] - sun.position_km[i]) / IAU_AU_KM);
            solar_deflect_distant_star(relative, earth_from_sun)?.map(|v| v * distance)
        }
    };
    let direction = annual_aberration_icrf(
        incoming.map(|value| value * IAU_AU_KM),
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
    calculate_star_with_solar_deflection(ephemeris, entry, tt, tdb, StellarSolarDeflection::Omit)
}

/// Calculate with an explicit choice of stellar solar-deflection model.
/// Rates are centered differences of the complete selected pipeline.
pub fn calculate_star_with_solar_deflection<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    entry: FixedStarCatalogEntry,
    tt: f64,
    tdb: JulianDateTdb,
    solar_deflection: StellarSolarDeflection,
) -> Result<ExperimentalStarPosition, FixedStarError> {
    let now = sample(ephemeris, entry, tt, tdb, solar_deflection)?;
    let before = sample(
        ephemeris,
        entry,
        tt - STEP_DAYS,
        JulianDateTdb(tdb.0 - STEP_DAYS),
        solar_deflection,
    )?;
    let after = sample(
        ephemeris,
        entry,
        tt + STEP_DAYS,
        JulianDateTdb(tdb.0 + STEP_DAYS),
        solar_deflection,
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
    fn icrs_astrometric_propagation_is_barycentric_and_kernel_free() {
        let e = example();
        let epoch = propagate_star_icrs_astrometric(e, e.epoch_tdb).unwrap();
        assert!(epoch.right_ascension_deg.abs() < 1e-12);
        assert!(epoch.declination_deg.abs() < 1e-12);
        let future =
            propagate_star_icrs_astrometric(e, JulianDateTdb(e.epoch_tdb.0 + 3652.5)).unwrap();
        // 1000 mas/year over ten Julian years = 10 arcsec, to first order.
        assert!((future.right_ascension_deg - 10.0 / 3600.0).abs() < 1e-10);
        assert!(future.distance_au.is_finite());
        assert_eq!(future.tdb_julian_day, e.epoch_tdb.0 + 3652.5);
    }
    #[test]
    fn icrs_astrometric_ra_wraps_without_ecliptic_projection() {
        let mut e = example();
        e.right_ascension_deg = 359.999;
        let p = propagate_star_icrs_astrometric(e, JulianDateTdb(e.epoch_tdb.0 + 3652.5)).unwrap();
        assert!(p.right_ascension_deg > 0.0 && p.right_ascension_deg < 0.01);
        e.parallax_mas = 0.0;
        assert!(matches!(
            propagate_star_icrs_astrometric(e, e.epoch_tdb),
            Err(FixedStarError::InvalidCatalog)
        ));
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
    fn solar_point_mass_matches_erfa_and_rejects_occulted_rays() {
        // ERFA 2.0.1.5 ldsun reference at the independent near-Sun fixture.
        let p = [
            -0.894_627_630_753_756_1,
            0.411_883_651_612_139_97,
            0.173_185_622_459_170_88,
        ];
        let earth = [
            0.903_358_544_130_430_2,
            -0.415_395_237_027_994_9,
            -0.180_084_014_143_265_77,
        ];
        let expected = [
            -0.894_628_540_209_116_8,
            0.411_884_815_971_075_5,
            0.173_178_155_301_756_68,
        ];
        let actual = solar_deflect_distant_star(p.map(|v| v * 2_000_000.0), earth).unwrap();
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-10);
        }
        let quadrant = solar_deflect_distant_star([0.0, 100_000.0, 0.0], [1.0, 0.0, 0.0]).unwrap();
        assert!(quadrant[0] > 1.9e-8 && quadrant[0] < 2.1e-8);
        assert!(matches!(
            solar_deflect_distant_star([-100_000.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
            Err(FixedStarError::OccultedBySun)
        ));
        assert!(matches!(
            solar_deflect_distant_star([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
            Err(FixedStarError::InvalidGeometry)
        ));
    }
    #[test]
    #[ignore = "requires DE441_PATH for an independent ERFA pipeline comparison"]
    fn erfa_pipeline_exposes_near_sun_deflection_omission() {
        // ERFA t_atci13 synthetic ICRS catalog and TT date. ERFA input pr is
        // raw dRA/dt, whereas our API takes dRA/dt * cos(dec). The comparison
        // fixture with solar deflection intentionally disabled was generated
        // independently using ERFA 2.0.1.5 apci13 -> pmpx -> ab -> pnm06a.
        // ERFA epv00 and DE441 do not use exactly the same Earth solution.
        let tt = 2_456_165.901_182_685;
        let mas_per_rad = 1.0 / RAD_PER_MAS;
        let entry = FixedStarCatalogEntry {
            epoch_tdb: JulianDateTdb(2_451_545.0),
            right_ascension_deg: 2.71_f64.to_degrees(),
            declination_deg: 0.174_f64.to_degrees(),
            proper_motion_ra_cos_dec_mas_per_year: 1e-5 * 0.174_f64.cos() * mas_per_rad,
            proper_motion_dec_mas_per_year: 5e-6 * mas_per_rad,
            parallax_mas: 100.0,
            radial_velocity_km_per_second: 55.0,
        };
        let mut eph = De441::open(std::env::var_os("DE441_PATH").expect("DE441_PATH")).unwrap();
        // Treat TT and TDB as equal for this fixture only; their actual offset
        // is under two milliseconds and is not derived by this test.
        let result = calculate_star(&mut eph, entry, tt, JulianDateTdb(tt)).unwrap();
        let eps = crate::earth_orientation::ecliptic_nutation(tt)
            .unwrap()
            .true_obliquity_rad;
        let (sl, cl) = result.longitude_deg.to_radians().sin_cos();
        let (sb, cb) = result.latitude_deg.to_radians().sin_cos();
        let (se, ce) = eps.sin_cos();
        let x = cb * cl;
        let y = cb * sl * ce - sb * se;
        let z = cb * sl * se + sb * ce;
        let ra = y.atan2(x).rem_euclid(std::f64::consts::TAU);
        let dec = z.atan2(x.hypot(y));
        let sep_mas = |ref_ra: f64, ref_dec: f64| {
            ((ra - ref_ra) * ref_dec.cos()).hypot(dec - ref_dec) * mas_per_rad
        };
        let without_solar_deflection = sep_mas(2.713_022_883_083_276_8, 0.172_944_718_098_265_34);
        assert!(
            without_solar_deflection < 10.0,
            "{without_solar_deflection} mas"
        );
        // ERFA's published full t_atci13 result is CIRS; subtract its equation
        // of origins to obtain equinox-based RA before comparison. This
        // deliberately near-Sun fixture includes solar gravitational bending.
        let full = sep_mas(
            2.710_121_572_968_697 + 0.002_900_618_712_657_376,
            0.172_937_136_721_953_9,
        );
        assert!((1_000.0..2_000.0).contains(&full), "{full} mas");
        let corrected = calculate_star_with_solar_deflection(
            &mut eph,
            entry,
            tt,
            JulianDateTdb(tt),
            StellarSolarDeflection::PointMass,
        )
        .unwrap();
        let (sl, cl) = corrected.longitude_deg.to_radians().sin_cos();
        let (sb, cb) = corrected.latitude_deg.to_radians().sin_cos();
        let x = cb * cl;
        let y = cb * sl * ce - sb * se;
        let z = cb * sl * se + sb * ce;
        let ra = y.atan2(x).rem_euclid(std::f64::consts::TAU);
        let dec = z.atan2(x.hypot(y));
        let reference_ra = 2.710_121_572_968_697 + 0.002_900_618_712_657_376;
        let reference_dec: f64 = 0.172_937_136_721_953_9;
        let corrected_difference =
            ((ra - reference_ra) * reference_dec.cos()).hypot(dec - reference_dec) * mas_per_rad;
        assert!(corrected_difference < 10.0, "{corrected_difference} mas");
        assert!(corrected.longitude_speed_deg_per_tdb_day.is_finite());
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
