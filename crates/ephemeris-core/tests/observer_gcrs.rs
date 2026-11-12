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

//! Integrated observer WGS84->ITRS->TIRS->CIRS->GCRS regression tests.
//!
//! Published SOFA/ERFA c2tcio and gd2gc numerical fixture data are
//! composed, but their epoch/realization assumptions and interpolated
//! EOP/CIP provenance remain externally supplied. Not a JPL parity test.

use std::f64::consts::TAU;

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::cio_orientation::cirs_to_gcrs_position;
use virgostars_ephemeris_core::earth_rotation::EarthRotationError;
use virgostars_ephemeris_core::observer_gcrs::{
    wgs84_observer_to_gcrs, ObserverGcrsError, ObserverOrientationInputs, Wgs84ObserverSite,
};
use virgostars_ephemeris_core::polar_motion::itrs_to_tirs_position;
use virgostars_ephemeris_core::tirs_cirs_era::tirs_to_cirs_earth_spin;
use virgostars_ephemeris_core::wgs84_ecef::Wgs84Error;

fn site() -> Wgs84ObserverSite {
    Wgs84ObserverSite {
        longitude_rad_east: 3.1,
        latitude_rad_north: -0.5,
        ellipsoidal_height_m: 2_500.0,
    }
}

fn orientation() -> ObserverOrientationInputs {
    // UT1 MJD=53736.0 / JD=2453736.5. This date corresponds to the
    // published ERFA c2tcio ERA=1.75283325530307 test fixture.
    // CIO X/Y are the last row of the published c2tcio rc2i matrix.
    // The CIO s was recovered geometrically from that same *published*
    // matrix, not computed from an independent physical TT series.
    ObserverOrientationInputs {
        ut1_epoch: JulianDate::new(2_453_737, 0.0, TimeScale::Ut1).unwrap(),
        polar_xp_rad: 2.550_602_38e-7,
        polar_yp_rad: 1.860_359_247e-6,
        tio_locator_sp_rad: -0.136_717_458_072_889_15e-10,
        cip_x: 0.579_130_847_216_815_3e-3,
        cip_y: 0.402_059_566_159_399_4e-4,
        cio_locator_s_rad: -1.220_040_848_124_797_4e-8,
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, observed {actual}, tolerance {tolerance}"
    );
}

fn near_xyz(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (a, b) in actual.into_iter().zip(expected) {
        near(a, b, tolerance);
    }
}

#[test]
fn independently_published_erfa_earth_orientation_and_wgs84_site_compose() {
    // Independent reference #1, ERFA t_gd2gc WGS84:
    // lon=3.1, geodetic lat=-0.5, h=2500.0m =>
    // (-5599000.5577049947, 233011.67223479203,
    //  -3040909.4706983363) meters.
    //
    // Independent reference #2, ERFA t_c2tcio:
    // rc2t [rows GCRS->ITRS] from supplied rc2i/rpom and ERA:
    // [-.1810332128307110439, +.9834769806938470149, +.6555535638685466874e-4]
    // [-.9834768134135996657, -.1810332203649448367, +.5749801116141106528e-3]
    // [+.5773474014081407076e-3, +.3961832391772658944e-4, +.9999998325501691969].
    //
    // Expected below = published rc2t transpose * published gd2gc
    // Cartesian vector, kilometers; this is an independent *numerical
    // compound fixture*, not a live ephemeris/GCRS realization check.
    let result = wgs84_observer_to_gcrs(site(), orientation()).unwrap();
    near_xyz(
        result.position_itrs_km,
        [-5_599.000_557_704_994, 233.011_672_234_792_03, -3_040.909_470_698_336],
        2.0e-10,
    );
    near(result.era_rad, 1.752_833_255_303_07, 1.0e-12);
    near_xyz(
        result.position_gcrs_km,
        [782.687_821_523_805_6, -5_548.791_492_538_574, -3_041.142_028_898_221_3],
        2.0e-8,
    );
}

#[test]
fn composed_pipeline_agrees_with_all_three_independent_intermediate_steps() {
    let params = orientation();
    let combined = wgs84_observer_to_gcrs(site(), params).unwrap();
    let tirs = itrs_to_tirs_position(
        combined.position_itrs_km,
        params.polar_xp_rad,
        params.polar_yp_rad,
        params.tio_locator_sp_rad,
    )
    .unwrap();
    let cirs = tirs_to_cirs_earth_spin(tirs, combined.era_rad, TAU * 1.002_737_811_911_354_6)
        .unwrap()
        .position_km;
    let independently_staged = cirs_to_gcrs_position(
        cirs,
        params.cip_x,
        params.cip_y,
        params.cio_locator_s_rad,
    )
    .unwrap();
    near_xyz(combined.position_gcrs_km, independently_staged, 5.0e-12);
}

#[test]
fn all_zero_eop_and_cip_yield_only_explicit_earth_rotation() {
    // Zero inputs here are explicit *synthetic* test values, not a
    // physically valid or silently supplied default EOP/CIP model.
    let mut params = orientation();
    params.polar_xp_rad = 0.0;
    params.polar_yp_rad = 0.0;
    params.tio_locator_sp_rad = 0.0;
    params.cip_x = 0.0;
    params.cip_y = 0.0;
    params.cio_locator_s_rad = 0.0;
    let point = wgs84_observer_to_gcrs(site(), params).unwrap();
    let (sin, cos) = point.era_rad.sin_cos();
    let [x, y, z] = point.position_itrs_km;
    near_xyz(
        point.position_gcrs_km,
        [cos * x - sin * y, sin * x + cos * y, z],
        5.0e-12,
    );
}

#[test]
fn rejects_non_ut1_time_scale_instead_of_silent_utc_or_tt_approximation() {
    let mut params = orientation();
    params.ut1_epoch = JulianDate::new(2_453_737, 0.0, TimeScale::Tt).unwrap();
    assert_eq!(
        wgs84_observer_to_gcrs(site(), params),
        Err(ObserverGcrsError::EarthRotation(
            EarthRotationError::RequiresUt1(TimeScale::Tt)
        ))
    );
}

#[test]
fn rejects_invalid_geodetic_inputs_and_cio_pole_data() {
    let mut observer = site();
    observer.latitude_rad_north = 100.0;
    assert_eq!(
        wgs84_observer_to_gcrs(observer, orientation()),
        Err(ObserverGcrsError::Geodesy(Wgs84Error::InvalidLatitude))
    );

    let mut params = orientation();
    params.cip_x = 1.2;
    assert!(matches!(
        wgs84_observer_to_gcrs(site(), params),
        Err(ObserverGcrsError::CioOrientation(_))
    ));
    params = orientation();
    params.polar_xp_rad = f64::NAN;
    assert!(matches!(
        wgs84_observer_to_gcrs(site(), params),
        Err(ObserverGcrsError::PolarMotion(_))
    ));
}

#[test]
fn output_is_position_only_and_has_declared_kilometer_units() {
    let result = wgs84_observer_to_gcrs(site(), orientation()).unwrap();
    let norm = |xyz: [f64; 3]| xyz.into_iter().map(|x| x * x).sum::<f64>().sqrt();
    near(norm(result.position_gcrs_km), norm(result.position_itrs_km), 5.0e-12);
    assert!(norm(result.position_gcrs_km) > 6_000.0);
    assert!(norm(result.position_gcrs_km) < 7_000.0);
}
