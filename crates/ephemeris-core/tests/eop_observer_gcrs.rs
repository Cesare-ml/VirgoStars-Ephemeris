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

//! Offline end-to-end EOP/UT1/TT/ERA/SP00/polar/CIP/CIO/WGS84 to GCRS.
//!
//! ERFA t_era00 and t_gd2gc are independent numerical sub-fixtures.
//! Other CIP/CIO and EOP rows are explicitly synthetic and do NOT
//! establish physical TT/CIO realization or real-date IERS fidelity.

use std::f64::consts::TAU;

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::eop_observer_gcrs::{
    wgs84_gcrs_from_eop_utc_and_cip_cio, EopWgs84GcrsError, ExternalCipCioAtTt,
};
use virgostars_ephemeris_core::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopCsvError, EopPolarQuality, EopPredictionPolicy,
};
use virgostars_ephemeris_core::eop_ut1_era::{eop_ut1_era_from_utc_si_seconds, EopUt1EraError};
use virgostars_ephemeris_core::observer_gcrs::{
    wgs84_observer_to_gcrs, ObserverGcrsError, ObserverOrientationInputs, Wgs84ObserverSite,
};
use virgostars_ephemeris_core::wgs84_ecef::Wgs84Error;

const HEADER: &str = "DATE,MJD,X,Y,UT1-UTC,LOD,DPSI,DEPS,DX,DY,DAT,DATA_TYPE";
const ORACLE_ERA_FIRST: &str = "2007-10-15,54388,0.10,0.20,0.0,0,0,0,0,0,33,O";
const ORACLE_ERA_SECOND: &str = "2007-10-16,54389,0.30,0.40,0.0,0,0,0,0,0,33,O";
const LEAP_FIRST: &str = "2016-12-31,57753,0.10,0.20,-0.4,0,0,0,0,0,36,O";
const LEAP_SECOND: &str = "2017-01-01,57754,0.30,0.40,0.6,0,0,0,0,0,37,O";
const LEAP_THIRD: &str = "2017-01-02,57755,0.30,0.40,0.6,0,0,0,0,0,37,O";

fn catalog(rows: &[&str]) -> CelestrakEopPolarCatalog {
    CelestrakEopPolarCatalog::parse_csv(
        "synthetic:EOP-CIP-integration",
        &format!("{HEADER}\n{}\n", rows.join("\n")),
    )
    .unwrap()
}

fn sample_site() -> Wgs84ObserverSite {
    // WGS84 numerical oracle: ERFA t_gd2gc published geodetic
    // lon=3.1, lat=-0.5 rad, height=2500 m.
    Wgs84ObserverSite {
        longitude_rad_east: 3.1,
        latitude_rad_north: -0.5,
        ellipsoidal_height_m: 2500.0,
    }
}

fn cip<'a>(epoch: JulianDate, label: &'a str) -> ExternalCipCioAtTt<'a> {
    // Numerical CIP/CIO vectors in this fixture are *synthetic*.
    // They are not an external X/Y/s model at the reported TT epoch.
    ExternalCipCioAtTt {
        tt_epoch: epoch,
        cip_x: 0.000_579_130_847_216_815_3,
        cip_y: 0.000_040_205_956_615_939_9,
        cio_locator_s_rad: -1.220_040_848_124_797_4e-8,
        source_label: label,
    }
}

fn eop_tt(cat: &CelestrakEopPolarCatalog, mjd: i64, sec: f64) -> JulianDate {
    eop_ut1_era_from_utc_si_seconds(cat, mjd, sec, EopPredictionPolicy::ObservedOnly)
        .unwrap()
        .tt_epoch
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}; got {actual}, tolerance {tolerance}"
    );
}

fn near_xyz(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (a, b) in actual.into_iter().zip(expected) {
        near(a, b, tolerance);
    }
}

#[test]
fn independent_erfa_era00_and_gd2gc_numerical_oracles_survive_full_composition() {
    // Independent ERFA t_era00:
    // UT1 JD=2400000.5+54388.0 => ERA=0.4022837240028158102 rad.
    // Separate ERFA t_gd2gc WGS84 geocentric Cartesian result
    // (-5599000.5577049947, 233011.67223479203, -3040909.4706983363) m.
    let cat = catalog(&[ORACLE_ERA_FIRST, ORACLE_ERA_SECOND]);
    let params = cip(eop_tt(&cat, 54_388, 0.0), "synthetic CIP/CIO at derived TT");
    let out = wgs84_gcrs_from_eop_utc_and_cip_cio(
        &cat,
        54_388,
        0.0,
        EopPredictionPolicy::ObservedOnly,
        sample_site(),
        params,
    )
    .unwrap();

    near(
        out.eop.earth_rotation.angle_rad,
        0.402_283_724_002_815_8,
        1.0e-12,
    );
    near(out.observer_gcrs.era_rad, 0.402_283_724_002_815_8, 1.0e-12);
    near_xyz(
        out.observer_gcrs.position_itrs_km,
        [
            -5_599.000_557_704_994,
            233.011_672_234_792_03,
            -3_040.909_470_698_336,
        ],
        2.0e-10,
    );
    assert_eq!(out.eop.quality, EopPolarQuality::Observed);
    assert_eq!(out.cip_cio_source_label, "synthetic CIP/CIO at derived TT");
    assert_eq!(out.eop.source_label, cat.source_label);
    assert_eq!(out.eop.ut1_epoch.scale(), TimeScale::Ut1);
    assert_eq!(out.eop.tt_epoch.scale(), TimeScale::Tt);
    let norm = |v: [f64; 3]| v.into_iter().map(|x| x * x).sum::<f64>().sqrt();
    near(
        norm(out.observer_gcrs.position_itrs_km),
        norm(out.observer_gcrs.position_gcrs_km),
        2.0e-11,
    );
    assert!(out.observer_gcrs.era_rad < TAU);
}

#[test]
fn complete_eop_wrapper_agrees_with_original_independently_implemented_site_pipeline() {
    let cat = catalog(&[ORACLE_ERA_FIRST, ORACLE_ERA_SECOND]);
    let external = cip(
        eop_tt(&cat, 54_388, 24_000.5),
        "synthetic-CIP-direct-comparison",
    );
    let out = wgs84_gcrs_from_eop_utc_and_cip_cio(
        &cat,
        54_388,
        24_000.5,
        EopPredictionPolicy::ObservedOnly,
        sample_site(),
        external,
    )
    .unwrap();
    let old = wgs84_observer_to_gcrs(
        sample_site(),
        ObserverOrientationInputs {
            ut1_epoch: out.eop.ut1_epoch,
            polar_xp_rad: out.eop.xp_rad,
            polar_yp_rad: out.eop.yp_rad,
            tio_locator_sp_rad: out.eop.approximate_polar.locator.sp_rad,
            cip_x: external.cip_x,
            cip_y: external.cip_y,
            cio_locator_s_rad: external.cio_locator_s_rad,
        },
    )
    .unwrap();
    near_xyz(
        out.observer_gcrs.position_itrs_km,
        old.position_itrs_km,
        0.0,
    );
    near_xyz(
        out.observer_gcrs.position_gcrs_km,
        old.position_gcrs_km,
        0.0,
    );
    assert_eq!(out.observer_gcrs.era_rad, old.era_rad);
}

#[test]
fn cip_cio_epoch_requires_tai_independent_exact_tt_provenance() {
    let cat = catalog(&[ORACLE_ERA_FIRST, ORACLE_ERA_SECOND]);
    let epoch = eop_tt(&cat, 54_388, 0.0);
    let mut wrong = cip(epoch, "synthetic");
    wrong.tt_epoch = JulianDate::new(
        epoch.day_number(),
        epoch.midnight_fraction(),
        TimeScale::Ut1,
    )
    .unwrap();
    assert!(matches!(
        wgs84_gcrs_from_eop_utc_and_cip_cio(
            &cat,
            54_388,
            0.0,
            EopPredictionPolicy::ObservedOnly,
            sample_site(),
            wrong,
        ),
        Err(EopWgs84GcrsError::CipCioEpochRequiresTt(TimeScale::Ut1))
    ));
    let mut shifted = cip(epoch, "synthetic");
    shifted.tt_epoch = JulianDate::new(
        epoch.day_number(),
        epoch.midnight_fraction() + 1.0e-5 / 86_400.0,
        TimeScale::Tt,
    )
    .unwrap();
    assert_eq!(
        wgs84_gcrs_from_eop_utc_and_cip_cio(
            &cat,
            54_388,
            0.0,
            EopPredictionPolicy::ObservedOnly,
            sample_site(),
            shifted,
        ),
        Err(EopWgs84GcrsError::CipCioEpochMismatch)
    );
    // Same epoch label but wrong actual source would still be physically
    // suspect: this strict API cannot authenticate a model's claims.
}

#[test]
fn blank_cip_cio_provenance_is_rejected_not_defaulted() {
    let cat = catalog(&[ORACLE_ERA_FIRST, ORACLE_ERA_SECOND]);
    for label in ["", "  ", "\n\t"] {
        assert_eq!(
            wgs84_gcrs_from_eop_utc_and_cip_cio(
                &cat,
                54_388,
                0.0,
                EopPredictionPolicy::ObservedOnly,
                sample_site(),
                cip(eop_tt(&cat, 54_388, 0.0), label),
            ),
            Err(EopWgs84GcrsError::MissingCipCioSource)
        );
    }
}

#[test]
fn malformed_cip_cio_geometry_is_rejected_by_original_checked_module() {
    let cat = catalog(&[ORACLE_ERA_FIRST, ORACLE_ERA_SECOND]);
    let mut c = cip(eop_tt(&cat, 54_388, 0.0), "synthetic bad CIP");
    c.cip_x = 1.2;
    assert!(matches!(
        wgs84_gcrs_from_eop_utc_and_cip_cio(
            &cat,
            54_388,
            0.0,
            EopPredictionPolicy::ObservedOnly,
            sample_site(),
            c,
        ),
        Err(EopWgs84GcrsError::Observer(
            ObserverGcrsError::CioOrientation(_)
        ))
    ));
    let mut c = cip(eop_tt(&cat, 54_388, 0.0), "synthetic bad CIO");
    c.cio_locator_s_rad = f64::NAN;
    assert!(matches!(
        wgs84_gcrs_from_eop_utc_and_cip_cio(
            &cat,
            54_388,
            0.0,
            EopPredictionPolicy::ObservedOnly,
            sample_site(),
            c,
        ),
        Err(EopWgs84GcrsError::Observer(
            ObserverGcrsError::CioOrientation(_)
        ))
    ));
    let mut bad_site = sample_site();
    bad_site.latitude_rad_north = 2.0;
    assert_eq!(
        wgs84_gcrs_from_eop_utc_and_cip_cio(
            &cat,
            54_388,
            0.0,
            EopPredictionPolicy::ObservedOnly,
            bad_site,
            cip(eop_tt(&cat, 54_388, 0.0), "synthetic"),
        ),
        Err(EopWgs84GcrsError::Observer(ObserverGcrsError::Geodesy(
            Wgs84Error::InvalidLatitude,
        )))
    );
}

#[test]
fn utc_leap_second_remains_continuous_in_gcrs_and_epoch_consistent() {
    let cat = catalog(&[LEAP_FIRST, LEAP_SECOND, LEAP_THIRD]);
    let sec1 = 86_400.0;
    let tt1 = eop_tt(&cat, 57_753, sec1);
    let result = wgs84_gcrs_from_eop_utc_and_cip_cio(
        &cat,
        57_753,
        sec1,
        EopPredictionPolicy::ObservedOnly,
        sample_site(),
        cip(tt1, "synthetic CIP at leap SI second"),
    )
    .unwrap();
    assert_eq!(result.eop.utc_day_length_seconds, 86_401.0);
    assert_eq!(result.eop.ut1_minus_tai_seconds, -36.4);
    let tt2 = eop_tt(&cat, 57_754, 0.0);
    let next = wgs84_gcrs_from_eop_utc_and_cip_cio(
        &cat,
        57_754,
        0.0,
        EopPredictionPolicy::ObservedOnly,
        sample_site(),
        cip(tt2, "synthetic CIP at next UTC midnight"),
    )
    .unwrap();
    let diff = (next.eop.ut1_epoch.day_number() - result.eop.ut1_epoch.day_number()) as f64
        + next.eop.ut1_epoch.midnight_fraction()
        - result.eop.ut1_epoch.midnight_fraction();
    near(diff * 86_400.0, 1.0, 4.0e-11);
    let a = result.observer_gcrs.position_gcrs_km;
    let b = next.observer_gcrs.position_gcrs_km;
    // Physical site displacement due to a ~1-second Earth spin is
    // a few hundred meters, not a one-second *jump* in UT1/ERA.
    let distance = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    assert!(distance > 0.0 && distance < 1.0);
}

#[test]
fn prediction_quality_and_provenance_survive_output() {
    let predicted = ORACLE_ERA_SECOND.replace(",33,O", ",33,P");
    let cat = catalog(&[ORACLE_ERA_FIRST, &predicted]);
    let tt =
        eop_ut1_era_from_utc_si_seconds(&cat, 54_388, 60.0, EopPredictionPolicy::AllowPredicted)
            .unwrap()
            .tt_epoch;
    let external = cip(tt, "provider:CIP/CIO-predicted-test");
    assert_eq!(
        wgs84_gcrs_from_eop_utc_and_cip_cio(
            &cat,
            54_388,
            60.0,
            EopPredictionPolicy::ObservedOnly,
            sample_site(),
            external,
        ),
        Err(EopWgs84GcrsError::Eop(EopUt1EraError::Eop(
            EopCsvError::PredictionNotAuthorized
        )))
    );
    let result = wgs84_gcrs_from_eop_utc_and_cip_cio(
        &cat,
        54_388,
        60.0,
        EopPredictionPolicy::AllowPredicted,
        sample_site(),
        external,
    )
    .unwrap();
    assert_eq!(result.eop.quality, EopPolarQuality::IncludesPrediction);
    assert_eq!(result.cip_cio_source_label, external.source_label);
    assert_eq!(result.eop.source_label, cat.source_label);
}

#[test]
fn missing_eop_data_and_invalid_elapsed_utc_seconds_propagate() {
    let cat = catalog(&[ORACLE_ERA_FIRST, ORACLE_ERA_SECOND]);
    let epoch = eop_tt(&cat, 54_388, 0.0);
    assert_eq!(
        wgs84_gcrs_from_eop_utc_and_cip_cio(
            &cat,
            54_389,
            0.0,
            EopPredictionPolicy::ObservedOnly,
            sample_site(),
            cip(epoch, "synthetic"),
        ),
        Err(EopWgs84GcrsError::Eop(EopUt1EraError::Eop(
            EopCsvError::MissingFollowingDay
        )))
    );
    assert_eq!(
        wgs84_gcrs_from_eop_utc_and_cip_cio(
            &cat,
            54_388,
            f64::NAN,
            EopPredictionPolicy::ObservedOnly,
            sample_site(),
            cip(epoch, "synthetic"),
        ),
        Err(EopWgs84GcrsError::Eop(
            EopUt1EraError::InvalidElapsedUtcSecond
        ))
    );
}
