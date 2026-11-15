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

//! Experimental physical planetary centers using explicitly supplied SPK offsets.
//! Added 2026-11-15. No kernel fallback, solar deflection or Swiss parity.

use crate::aberration_state::{aberrate_light_time_state, AberratedStateError};
use crate::de441::{Body, De441, De441Error, JulianDateTdb, StateVector};
use crate::light_time::{solve_from_observer_custom, LightTimeError};
use crate::spk_kernel_set::SpkKernelSet;
use crate::spk_relative_state::{SpkRelativeError, SpkRelativeState};
use crate::true_ecliptic::{project_true_ecliptic, TrueEclipticError};
use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalPlanet {
    Mars,
    Jupiter,
    Saturn,
}
impl PhysicalPlanet {
    fn identifiers(self) -> (Body, i32, i32) {
        match self {
            Self::Mars => (Body::MarsBarycenter, 499, 4),
            Self::Jupiter => (Body::JupiterBarycenter, 599, 5),
            Self::Saturn => (Body::SaturnBarycenter, 699, 6),
        }
    }
}

/// Physical center, Earth-centered, light-time and annual-aberration corrected
/// in true ecliptic of date; degrees, AU and derivatives per reception TDB day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalCenterPosition {
    pub planet: PhysicalPlanet,
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
pub enum PhysicalCenterError {
    InvalidEpoch,
    InvalidOffset,
    Ephemeris(De441Error),
    Spk(SpkRelativeError),
    LightTime(LightTimeError),
    Aberration(AberratedStateError),
    Projection(TrueEclipticError),
}
impl fmt::Display for PhysicalCenterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEpoch => write!(f, "TT and TDB must describe the same finite instant"),
            Self::InvalidOffset => write!(f, "invalid physical-center offset geometry"),
            Self::Ephemeris(e) => write!(f, "DE441: {e}"),
            Self::Spk(e) => write!(f, "supplemental SPK: {e}"),
            Self::LightTime(e) => write!(f, "light-time: {e}"),
            Self::Aberration(e) => write!(f, "aberration: {e}"),
            Self::Projection(e) => write!(f, "projection: {e}"),
        }
    }
}
impl Error for PhysicalCenterError {}

/// Return a geometric physical-center minus system-barycenter SPK offset in
/// km and km/TDB second, at the supplied TDB JD. Missing coverage is an error.
pub fn center_offset<R: Read + Seek>(
    kernels: &mut SpkKernelSet<R>,
    planet: PhysicalPlanet,
    tdb: JulianDateTdb,
) -> Result<SpkRelativeState, PhysicalCenterError> {
    if !tdb.0.is_finite() {
        return Err(PhysicalCenterError::InvalidEpoch);
    }
    let (_, target, center) = planet.identifiers();
    let et = (tdb.0 - 2_451_545.0) * 86_400.0;
    if !et.is_finite() {
        return Err(PhysicalCenterError::InvalidEpoch);
    }
    kernels
        .relative_state(target, center, et)
        .map_err(PhysicalCenterError::Spk)
}

fn add_offset(
    mut bary: StateVector,
    offset: &SpkRelativeState,
) -> Result<StateVector, PhysicalCenterError> {
    for i in 0..3 {
        bary.position_km[i] += offset.position_km[i];
        bary.velocity_km_per_day[i] += offset.velocity_km_per_second[i] * 86_400.0;
    }
    if bary
        .position_km
        .iter()
        .chain(bary.velocity_km_per_day.iter())
        .any(|v| !v.is_finite())
    {
        return Err(PhysicalCenterError::InvalidOffset);
    }
    Ok(bary)
}

/// Combine DE441 system barycenters with supplemental physical-center SPK
/// offsets at each iterated emission time, then apply the existing annual
/// aberration and moving true-ecliptic projection. The classic ephemeris and
/// supplemental kernels may come from different solutions; the caller must
/// assess that combination and provenance. Missing/corrupt SPK never falls back.
pub fn geocentric_physical_center<R: Read + Seek, S: Read + Seek>(
    ephemeris: &mut De441<R>,
    kernels: &mut SpkKernelSet<S>,
    planet: PhysicalPlanet,
    tdb: JulianDateTdb,
    tt_jd: f64,
) -> Result<PhysicalCenterPosition, PhysicalCenterError> {
    if !tt_jd.is_finite() || !tdb.0.is_finite() || (tt_jd - tdb.0).abs() > 1.0 / 86_400.0 {
        return Err(PhysicalCenterError::InvalidEpoch);
    }
    let (bary, _, _) = planet.identifiers();
    let earth = ephemeris
        .state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, tdb)
        .map_err(PhysicalCenterError::Ephemeris)?;
    // `bary` identifies the classic series within this private composition.
    // Every target sample is corrected to the physical center before use;
    // only `PhysicalPlanet`, never this temporary label, is publicly returned.
    let light = solve_from_observer_custom(bary, Body::Earth, tdb, |body, epoch| {
        let state = ephemeris
            .state(body, Body::SolarSystemBarycenter, epoch)
            .map_err(LightTimeError::Ephemeris)?;
        if body == Body::Earth {
            return Ok(state);
        }
        let (_, target, center) = planet.identifiers();
        let et = (epoch.0 - 2_451_545.0) * 86_400.0;
        let offset = kernels
            .relative_state(target, center, et)
            .map_err(LightTimeError::SupplementalSpk)?;
        add_offset(state, &offset).map_err(|_| LightTimeError::InvalidGeometry)
    })
    .map_err(PhysicalCenterError::LightTime)?;
    let aberrated = aberrate_light_time_state(
        &light,
        earth.velocity_km_per_day,
        earth.acceleration_km_per_day2,
    )
    .map_err(PhysicalCenterError::Aberration)?;
    let state =
        project_true_ecliptic(&aberrated, tt_jd).map_err(PhysicalCenterError::Projection)?;
    Ok(PhysicalCenterPosition {
        planet,
        tt_jd,
        tdb,
        longitude_deg: state.longitude_deg,
        latitude_deg: state.latitude_deg,
        distance_au: state.distance_au,
        longitude_speed_deg_per_tdb_day: state.longitude_speed_deg_per_tdb_day,
        latitude_speed_deg_per_tdb_day: state.latitude_speed_deg_per_tdb_day,
        distance_speed_au_per_tdb_day: state.distance_speed_au_per_tdb_day,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_offset_adds_position_and_converts_velocity_units() {
        let bary = StateVector {
            target: Body::JupiterBarycenter,
            center: Body::SolarSystemBarycenter,
            epoch: JulianDateTdb(2_451_545.0),
            position_km: [10.0, 20.0, 30.0],
            velocity_km_per_day: [1.0, 2.0, 3.0],
        };
        let offset = SpkRelativeState {
            target_id: 599,
            center_id: 5,
            common_ancestor_id: 5,
            tdb_seconds_past_j2000: 0.0,
            position_km: [2.0, -3.0, 4.0],
            velocity_km_per_second: [0.5, -0.25, 0.0],
        };
        let combined = add_offset(bary, &offset).unwrap();
        assert_eq!(combined.position_km, [12.0, 17.0, 34.0]);
        assert_eq!(combined.velocity_km_per_day, [43_201.0, -21_598.0, 3.0]);
    }
    #[test]
    fn no_supplied_kernel_is_an_error() {
        let mut set = SpkKernelSet::<std::io::Cursor<Vec<u8>>>::new();
        assert!(center_offset(&mut set, PhysicalPlanet::Mars, JulianDateTdb(2_451_545.0)).is_err());
    }
    #[test]
    #[ignore = "requires DE441_PATH, MARS_SPK_PATH, JUPITER_SPK_PATH and SATURN_SPK_PATH"]
    fn external_kernels_compare_to_horizons_and_compose() {
        let mut eph = De441::open(std::env::var_os("DE441_PATH").expect("DE441_PATH")).unwrap();
        let mut set = SpkKernelSet::new();
        for key in ["MARS_SPK_PATH", "JUPITER_SPK_PATH", "SATURN_SPK_PATH"] {
            set.load(std::fs::File::open(std::env::var_os(key).expect(key)).unwrap())
                .unwrap();
        }
        let tdb = JulianDateTdb(2_461_354.500_800_725);
        let tt = 2_461_354.500_800_741;
        // Horizons geometric ICRF KM-S, TDB JD 2461354.500800725, targets
        // 499/4, 599/5, 699/6; Mars source is mar099 vs local mar097.
        let cases = [
            (
                PhysicalPlanet::Mars,
                [
                    -1.039_015_645_376_957e-4,
                    -1.702_197_945_957_843e-4,
                    -3.213_371_137_099_707e-5,
                ],
                [
                    3.000_382_866_517_237e-8,
                    -1.099_946_956_722_097e-8,
                    -2.148_112_686_019_174e-8,
                ],
            ),
            (
                PhysicalPlanet::Jupiter,
                [
                    -103.154_700_843_238_9,
                    93.782_113_399_814_78,
                    43.259_179_434_800_52,
                ],
                [
                    -0.001_661_002_961_418_669,
                    -0.000_825_722_664_935_605_9,
                    -0.000_416_697_628_052_389_1,
                ],
            ),
            (
                PhysicalPlanet::Saturn,
                [
                    264.929_386_305_900_1,
                    113.557_241_042_754,
                    -33.951_812_444_883_84,
                ],
                [
                    -0.000_557_688_985_441_889_3,
                    0.001_097_886_025_637_879,
                    -0.000_024_300_492_957_060_57,
                ],
            ),
        ];
        for (planet, reference, velocity_reference) in cases {
            let offset = center_offset(&mut set, planet, tdb).unwrap();
            for (actual, expected) in offset.position_km.into_iter().zip(reference) {
                assert!((actual - expected).abs() < 1e-6);
            }
            for (actual, expected) in offset
                .velocity_km_per_second
                .into_iter()
                .zip(velocity_reference)
            {
                assert!((actual - expected).abs() < 1e-10);
            }
            let result = geocentric_physical_center(&mut eph, &mut set, planet, tdb, tt).unwrap();
            assert_eq!(result.planet, planet);
            assert!((0.0..360.0).contains(&result.longitude_deg));
            assert!(result.distance_au > 0.0);
            let before = geocentric_physical_center(
                &mut eph,
                &mut set,
                planet,
                JulianDateTdb(tdb.0 - 0.05),
                tt - 0.05,
            )
            .unwrap();
            let after = geocentric_physical_center(
                &mut eph,
                &mut set,
                planet,
                JulianDateTdb(tdb.0 + 0.05),
                tt + 0.05,
            )
            .unwrap();
            let mut dl = after.longitude_deg - before.longitude_deg;
            if dl >= 180.0 {
                dl -= 360.0;
            }
            if dl < -180.0 {
                dl += 360.0;
            }
            let numerical = dl / ((tdb.0 + 0.05) - (tdb.0 - 0.05));
            assert!((result.longitude_speed_deg_per_tdb_day - numerical).abs() < 0.0001);
        }
    }
}
