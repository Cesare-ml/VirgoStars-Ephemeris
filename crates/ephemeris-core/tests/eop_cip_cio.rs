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

//! EOP IAU celestial pole offset + modeled CIO s+XY/2 composition.
//! CelesTrak 2000-01-01 dX/dY and ERFA t_xys06a are independent
//! numerical fixtures. Model and EOP epochs in other tests are
//! *synthetic*: no real-date physical realization is claimed.

use std::f64::consts::PI;

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::eop_cip_cio::{
    wgs84_gcrs_with_eop_corrected_iau_cip, EopCipCioError, ModeledCipCioAtTt,
};
use virgostars_ephemeris_core::eop_observer_gcrs::{
    wgs84_gcrs_from_eop_utc_and_cip_cio, ExternalCipCioAtTt,
};
use virgostars_ephemeris_core::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopCsvError, EopPolarQuality, EopPredictionPolicy,
};
use virgostars_ephemeris_core::eop_ut1_era::eop_ut1_era_from_utc_si_seconds;
use virgostars_ephemeris_core::observer_gcrs::Wgs84ObserverSite;

const HEADER: &str = "DATE,MJD,X,Y,UT1-UTC,LOD,DPSI,DEPS,DX,DY,DAT,DATA_TYPE";
const PUBLISHED_2000: &str =
    "2000-01-01,51544,0.043282,0.377909,0.3553880,0.0009536,-0.050263,-0.002471,-0.000005,-0.000050,32,O";
const NEXT_OBSERVED: &str =
    "2000-01-02,51545,0.143282,0.477909,0.3551880,0.0009000,-0.050263,-0.002471,0.000095,0.000150,32,O";
const NEXT_PREDICTED: &str =
    "2000-01-03,51546,0.143282,0.477909,0.3551880,0.0009000,-0.050263,-0.002471,0.000195,-0.000050,32,P";
const ARCSEC_RAD: f64 = PI / (180.0 * 3600.0);

// Published ERFA t_xys06a independently computed IAU 2006/2000A
// CIP/CIO numerical oracle at TT MJD 53736.0. No nutation series
// is computed in this feature; these values are model INPUTS.
const ERFA_X: f64 = 0.579_130_848_283_529_3e-3;
const ERFA_Y: f64 = 0.402_058_009_945_402e-4;
const ERFA_S: f64 = -0.122_003_229_416_457_99e-7;

fn catalog(rows: &[&str]) -> CelestrakEopPolarCatalog {
    CelestrakEopPolarCatalog::parse_csv(
        "external-example:CelesTrak-CSV",
        &format!("{HEADER}\n{}\n", rows.join("\n")),
    )
    .unwrap()
}

fn epoch(
    catalog: &CelestrakEopPolarCatalog,
    day: i64,
    seconds: f64,
    policy: EopPredictionPolicy,
) -> JulianDate {
    eop_ut1_era_from_utc_si_seconds(catalog, day, seconds, policy)
        .unwrap()
        .tt_epoch
}

fn modeled<'a>(tt_epoch: JulianDate, source_label: &'a str) -> ModeledCipCioAtTt<'a> {
    ModeledCipCioAtTt {
        tt_epoch,
        model_x: ERFA_X,
        model_y: ERFA_Y,
        cio_s_plus_xy_half_rad: ERFA_S + ERFA_X * ERFA_Y / 2.0,
        source_label,
    }
}

fn site() -> Wgs84ObserverSite {
    Wgs84ObserverSite {
        longitude_rad_east: 3.1,
        latitude_rad_north: -0.5,
        ellipsoidal_height_m: 2500.0,
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}; actual {actual}; tolerance {tolerance}"
    );
}

#[test]
fn published_celestrak_dx_dy_are_added_in_radians_and_cio_is_recomputed() {
    let cat = catalog(&[PUBLISHED_2000, NEXT_OBSERVED]);
    let model = modeled(epoch(&cat, 51544, 0.0, EopPredictionPolicy::ObservedOnly), "synthetic model at EOP TT");
    let result = wgs84_gcrs_with_eop_corrected_iau_cip(
        &cat, 51544, 0.0, EopPredictionPolicy::ObservedOnly, site(), model,
    )
    .unwrap();
    let c = result.corrected_cip_cio;
    near(c.eop_dx_rad, -0.000_005 * ARCSEC_RAD, 1.0e-25);
    near(c.eop_dy_rad, -0.000_050 * ARCSEC_RAD, 1.0e-25);
    near(c.corrected_x, ERFA_X - 0.000_005 * ARCSEC_RAD, 1.0e-17);
    near(c.corrected_y, ERFA_Y - 0.000_050 * ARCSEC_RAD, 1.0e-17);
    near(
        c.corrected_cio_s_rad,
        ERFA_S + ERFA_X * ERFA_Y / 2.0
            - (ERFA_X - 0.000_005 * ARCSEC_RAD) * (ERFA_Y - 0.000_050 * ARCSEC_RAD) / 2.0,
        1.0e-20,
    );
    assert_eq!(c.quality, EopPolarQuality::Observed);
    assert_eq!(result.observer.cip_cio_source_label, model.source_label);
}

#[test]
fn independent_erfa_xys06a_fixture_survives_zero_celestial_eop_offsets() {
    // Numeric values from ERFA t_xys06a at TT MJD 53736.
    // EOP rows deliberately set dX=dY=0. The synthetic EOP TT
    // does NOT coincide with that publication's evaluation TT;
    // this tests only algebraic identity, not physical epoch parity.
    let first = "2006-01-01,53736,0.1,0.2,0.3341,0,0,0,0,0,33,O";
    let second = "2006-01-02,53737,0.1,0.2,0.3341,0,0,0,0,0,33,O";
    let cat = catalog(&[first, second]);
    let e = epoch(&cat, 53736, 0.0, EopPredictionPolicy::ObservedOnly);
    let result = wgs84_gcrs_with_eop_corrected_iau_cip(
        &cat, 53736, 0.0, EopPredictionPolicy::ObservedOnly, site(),
        modeled(e, "ERFA t_xys06a numerical fixture only"),
    )
    .unwrap();
    let c = result.corrected_cip_cio;
    assert_eq!(c.corrected_x, ERFA_X);
    assert_eq!(c.corrected_y, ERFA_Y);
    near(c.corrected_cio_s_rad, ERFA_S, 1.0e-21);
    assert_eq!(c.eop_dx_rad, 0.0);
    assert_eq!(c.eop_dy_rad, 0.0);
}

#[test]
fn observed_half_day_interpolation_matches_direct_explicit_cip_cio_pipeline() {
    let cat = catalog(&[PUBLISHED_2000, NEXT_OBSERVED]);
    let e = epoch(&cat, 51544, 43200.0, EopPredictionPolicy::ObservedOnly);
    let model = modeled(e, "synthetic-model-for-pole-offset-test");
    let out = wgs84_gcrs_with_eop_corrected_iau_cip(
        &cat, 51544, 43200.0, EopPredictionPolicy::ObservedOnly, site(), model,
    )
    .unwrap();
    let c = out.corrected_cip_cio;
    near(c.eop_dx_rad, 0.000_045 * ARCSEC_RAD, 1.0e-24);
    near(c.eop_dy_rad, 0.000_050 * ARCSEC_RAD, 1.0e-24);
    let direct = wgs84_gcrs_from_eop_utc_and_cip_cio(
        &cat, 51544, 43200.0, EopPredictionPolicy::ObservedOnly, site(),
        ExternalCipCioAtTt {
            tt_epoch: e,
            cip_x: c.corrected_x,
            cip_y: c.corrected_y,
            cio_locator_s_rad: c.corrected_cio_s_rad,
            source_label: model.source_label,
        },
    )
    .unwrap();
    assert_eq!(out.observer, direct);
}

#[test]
fn predictions_require_explicit_authorization_and_remain_marked() {
    let cat = catalog(&[PUBLISHED_2000, NEXT_OBSERVED, NEXT_PREDICTED]);
    let e = epoch(&cat, 51545, 43200.0, EopPredictionPolicy::AllowPredicted);
    let model = modeled(e, "forecast-synthetic");
    assert_eq!(
        wgs84_gcrs_with_eop_corrected_iau_cip(
            &cat, 51545, 43200.0, EopPredictionPolicy::ObservedOnly, site(), model,
        ),
        Err(EopCipCioError::Eop(
            virgostars_ephemeris_core::eop_ut1_era::EopUt1EraError::Eop(
                EopCsvError::PredictionNotAuthorized,
            )
        ))
    );
    let out = wgs84_gcrs_with_eop_corrected_iau_cip(
        &cat, 51545, 43200.0, EopPredictionPolicy::AllowPredicted, site(), model,
    )
    .unwrap();
    assert_eq!(out.corrected_cip_cio.quality, EopPolarQuality::IncludesPrediction);
    assert_eq!(out.observer.eop.quality, EopPolarQuality::IncludesPrediction);
}

#[test]
fn wrong_scale_or_slightly_wrong_tt_epoch_is_rejected() {
    let cat = catalog(&[PUBLISHED_2000, NEXT_OBSERVED]);
    let e = epoch(&cat, 51544, 0.0, EopPredictionPolicy::ObservedOnly);
    let mut other = modeled(e, "valid-model-label");
    other.tt_epoch = JulianDate::new(e.day_number(), e.midnight_fraction(), TimeScale::Ut1).unwrap();
    assert_eq!(
        wgs84_gcrs_with_eop_corrected_iau_cip(
            &cat, 51544, 0.0, EopPredictionPolicy::ObservedOnly, site(), other,
        ),
        Err(EopCipCioError::ModelEpochRequiresTt(TimeScale::Ut1))
    );
    let mut shifted = modeled(e, "valid-model-label");
    shifted.tt_epoch = JulianDate::new(
        e.day_number(), e.midnight_fraction() + 1.0e-5 / 86400.0, TimeScale::Tt,
    )
    .unwrap();
    assert_eq!(
        wgs84_gcrs_with_eop_corrected_iau_cip(
            &cat, 51544, 0.0, EopPredictionPolicy::ObservedOnly, site(), shifted,
        ),
        Err(EopCipCioError::ModelEpochMismatch)
    );
}

#[test]
fn invalid_model_coordinates_and_blank_provenance_fail_closed() {
    let cat = catalog(&[PUBLISHED_2000, NEXT_OBSERVED]);
    let e = epoch(&cat, 51544, 0.0, EopPredictionPolicy::ObservedOnly);
    for label in ["", "  ", "\t"] {
        assert_eq!(
            wgs84_gcrs_with_eop_corrected_iau_cip(
                &cat, 51544, 0.0, EopPredictionPolicy::ObservedOnly,
                site(), modeled(e, label),
            ),
            Err(EopCipCioError::MissingModelSource)
        );
    }
    for bad in [f64::NAN, f64::INFINITY, 1.2] {
        let mut model = modeled(e, "synthetic-invalid-model");
        model.model_x = bad;
        assert_eq!(
            wgs84_gcrs_with_eop_corrected_iau_cip(
                &cat, 51544, 0.0, EopPredictionPolicy::ObservedOnly,
                site(), model,
            ),
            Err(EopCipCioError::InvalidModeledCoordinates)
        );
    }
    let mut bad_s = modeled(e, "synthetic-invalid-s");
    bad_s.cio_s_plus_xy_half_rad = f64::NEG_INFINITY;
    assert_eq!(
        wgs84_gcrs_with_eop_corrected_iau_cip(
            &cat, 51544, 0.0, EopPredictionPolicy::ObservedOnly, site(), bad_s,
        ),
        Err(EopCipCioError::InvalidModeledCoordinates)
    );
}

#[test]
fn invalid_eop_celestial_offset_is_rejected_during_initial_parse() {
    let altered = PUBLISHED_2000.replace("-0.000005,-0.000050", "5.0,-0.000050");
    assert_eq!(
        CelestrakEopPolarCatalog::parse_csv(
            "invalid celestial offset", &format!("{HEADER}\n{altered}\n"),
        ),
        Err(EopCsvError::InvalidCipOffset { line: 2 })
    );
}

#[test]
fn missing_day_and_bad_utc_elapsed_second_propagate_without_model_fallback() {
    let cat = catalog(&[PUBLISHED_2000, NEXT_OBSERVED]);
    let e = epoch(&cat, 51544, 0.0, EopPredictionPolicy::ObservedOnly);
    let model = modeled(e, "explicit-model");
    assert_eq!(
        wgs84_gcrs_with_eop_corrected_iau_cip(
            &cat, 51545, 1.0, EopPredictionPolicy::ObservedOnly, site(), model,
        ),
        Err(EopCipCioError::Eop(
            virgostars_ephemeris_core::eop_ut1_era::EopUt1EraError::Eop(
                EopCsvError::MissingFollowingDay,
            )
        ))
    );
    assert_eq!(
        wgs84_gcrs_with_eop_corrected_iau_cip(
            &cat, 51544, f64::NAN, EopPredictionPolicy::ObservedOnly, site(), model,
        ),
        Err(EopCipCioError::Eop(
            virgostars_ephemeris_core::eop_ut1_era::EopUt1EraError::InvalidElapsedUtcSecond
        ))
    );
}
