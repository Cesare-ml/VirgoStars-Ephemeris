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

//! Optional real classic-DE441 regression against an independently
//! recorded JPL Horizons light-time baseline, plus whole-pipeline rates.
//!
//! IMPORTANT: Horizons below is VEC_CORR=LT, not LT+S. The corrected
//! annual-aberration result is **deboosted** before comparison to these
//! independent LT vectors. This cannot certify independent LT+S parity.
//! File and astronomical source data are NOT redistributed.

use std::f64::consts::PI;

use virgostars_ephemeris_core::aberrated_spherical::{
    geocentric_aberrated_fixed_six_fields_many, geocentric_aberrated_fixed_spherical,
};
use virgostars_ephemeris_core::aberration::annual_aberration_icrf;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

const HORIZONS_TDB_JD: f64 = 2_461_354.5;
const DAY_SECONDS: f64 = 86_400.0;

fn local_kernel() -> De441<std::fs::File> {
    let path = std::env::var("DE441_PATH")
        .expect("set DE441_PATH to the external, legally acquired CLASSIC de441.eph file");
    De441::open(path).expect("cannot read the external classic DE441 file")
}

fn xyz(spherical: &virgostars_ephemeris_core::coordinates::SphericalState) -> [f64; 3] {
    let r_cos_lat = spherical.distance_km * spherical.latitude_rad.cos();
    [
        r_cos_lat * spherical.longitude_rad.cos(),
        r_cos_lat * spherical.longitude_rad.sin(),
        spherical.distance_km * spherical.latitude_rad.sin(),
    ]
}

fn magnitude(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

fn max_component_error(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3)
        .map(|i| (a[i] - b[i]).abs())
        .fold(0.0, f64::max)
}

/// The four numerical vectors were independently obtained from NASA/JPL
/// Horizons on 2026-11-10 as already recorded in the repository's
/// `tests/light_time_horizons.rs`.
///
/// Settings: JPL DE441, VECTORS, CENTER=500@399, TIME_TYPE=TDB,
/// TLIST=2461354.5, REF_PLANE=FRAME, REF_SYSTEM=ICRF,
/// OUT_UNITS=KM-S, VEC_CORR=LT, VEC_TABLE=3.
///
/// These are **LT** source positions, NOT reference LT+S apparent vectors.
/// The inverse boost below checks the combined output is consistent with
/// the independently measured light-time geometry after removing the
/// project-implemented SR aberration. It does NOT independently verify
/// the relativistic aberration formula itself.
#[test]
#[ignore = "requires separately supplied real classic DE441_PATH (Horizons LT reference)"]
fn real_de441_aberrated_pipeline_recovers_horizons_light_time_geometry() {
    let mut eph = local_kernel();
    let t = JulianDateTdb(HORIZONS_TDB_JD);
    let earth = eph
        .state(Body::Earth, Body::SolarSystemBarycenter, t)
        .unwrap();
    let reverse_earth_velocity = earth.velocity_km_per_day.map(|v| -v);

    // Do not silently substitute a planet center for a system barycenter.
    let cases = [
        (
            Body::Moon,
            [-2.271092990545779e5, -2.863237983404845e5, -1.620441867628545e5],
            1.333498431623768,
        ),
        (
            Body::Venus,
            [-4.238188819403984e7, -1.552509477906777e7, -9.024076156753547e6],
            153.5368740945382,
        ),
        (
            Body::MarsBarycenter,
            [-1.614080644210615e8, 1.105419500423874e8, 5.485717291371012e7],
            677.7296430371571,
        ),
        (
            Body::JupiterBarycenter,
            [-6.56483483498132e8, 4.186401692284813e8, 1.924872858802854e8],
            2675.345227915447,
        ),
    ];
    for (body, expected_lt_xyz, expected_delay_s) in cases {
        let corrected = geocentric_aberrated_fixed_spherical(&mut eph, body, t, 0.0)
            .unwrap();
        assert_eq!(corrected.spherical.center, Body::Earth);
        assert_eq!(corrected.spherical.target, body);
        assert_eq!(corrected.spherical.epoch, t);
        assert!(corrected.emission_epoch.0 < t.0);

        let apparent_xyz = xyz(&corrected.spherical);
        let recovered_lt_direction = annual_aberration_icrf(
            apparent_xyz,
            reverse_earth_velocity,
        )
        .unwrap();
        let recovered_lt_xyz = recovered_lt_direction.map(|d| d * corrected.spherical.distance_km);
        let vector_error_km = max_component_error(recovered_lt_xyz, expected_lt_xyz);
        let range_error_km =
            (corrected.spherical.distance_km - magnitude(expected_lt_xyz)).abs();
        let delay_error_s = (corrected.light_time_seconds - expected_delay_s).abs();

        println!(
            "{body:?}: recovered Horizons LT vector error {vector_error_km:.9} km,              LT range error {range_error_km:.9} km, delay error {delay_error_s:.12} s"
        );
        // Tolerances match the existing real-file Horizons LT test, with
        // extra geometric round-trip not exceeding the same conservative
        // 0.01 km limit. They do not imply LT+S numerical validation.
        assert!(vector_error_km < 0.01, "{body:?} recovered LT vector");
        assert!(range_error_km < 0.01, "{body:?} LT range");
        assert!(delay_error_s < 1.0e-6, "{body:?} light time");
    }
}

/// The output velocities are derivatives with respect to reception
/// **TDB**, not the uncorrected Horizons instantaneous velocity columns.
/// Compare them with finite differences of the **full** real-kernel
/// correction pipeline instead of mixing incompatible conventions.
///
/// The numeric tolerances here are intentionally conservative and must
/// be audited against independently acquired real scientific rate data
/// before promoting this to an astronomical accuracy gate.
#[test]
#[ignore = "requires real classic DE441_PATH; checks real-file rates vs finite differences"]
fn real_de441_aberrated_batch_rates_follow_neighboring_full_pipeline_states() {
    let mut eph = local_kernel();
    let targets = [
        Body::Moon,
        Body::Venus,
        Body::MarsBarycenter,
        Body::JupiterBarycenter,
    ];
    let step_tdb_days = 600.0 / DAY_SECONDS;
    let sample = |reader: &mut De441<std::fs::File>, offset: f64| {
        geocentric_aberrated_fixed_six_fields_many(
            reader,
            &targets,
            JulianDateTdb(HORIZONS_TDB_JD + offset),
            0.0,
        )
        .unwrap()
    };
    let left = sample(&mut eph, -step_tdb_days);
    let middle = sample(&mut eph, 0.0);
    let right = sample(&mut eph, step_tdb_days);
    let actual_dt = (HORIZONS_TDB_JD + step_tdb_days) - (HORIZONS_TDB_JD - step_tdb_days);
    assert!(actual_dt.is_finite() && actual_dt > 0.0);

    for i in 0..targets.len() {
        let before = &left[i].fields;
        let at = &middle[i].fields;
        let after = &right[i].fields;
        let signed_longitude_delta =
            (after.longitude_deg - before.longitude_deg + 540.0).rem_euclid(360.0) - 180.0;
        let measured_lon_speed = signed_longitude_delta / actual_dt;
        let measured_lat_speed = (after.latitude_deg - before.latitude_deg) / actual_dt;
        let measured_range_speed = (after.distance_au - before.distance_au) / actual_dt;
        let angular_error = (at.longitude_speed_deg_per_tdb_day - measured_lon_speed)
            .abs()
            .max((at.latitude_speed_deg_per_tdb_day - measured_lat_speed).abs());
        let radial_error = (at.distance_speed_au_per_tdb_day - measured_range_speed).abs();
        println!(
            "{:?}: max angular rate residual = {:.9} arcseconds/TDB-day;              radial rate residual = {:.10e} AU/TDB-day",
            targets[i],
            angular_error * 3600.0,
            radial_error,
        );
        assert!(
            angular_error < 2.0e-4,
            "{:?}: real-kernel angular speed mismatch",
            targets[i]
        );
        assert!(
            radial_error < 1.0e-7,
            "{:?}: real-kernel radial speed mismatch",
            targets[i]
        );
        assert!(at.longitude_deg >= 0.0 && at.longitude_deg < 2.0 * PI * 180.0 / PI);
        assert!(at.distance_au > 0.0);
    }
}
