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

//! Offline, original UTC→TT/EOP→SP00 polar matrix integration tests.
//! The first CSV record is the published CelesTrak format sample;
//! following rows are synthetic. Matrix parity uses separately tested
//! IERS polar-motion and SP00 primitives, not live EOP/JPL data.

use std::f64::consts::PI;

use virgostars_ephemeris_core::calendar::TimeScale;
use virgostars_ephemeris_core::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopCsvError, EopPolarQuality, EopPredictionPolicy,
};
use virgostars_ephemeris_core::eop_polar_orientation::{
    approximate_itrs_to_tirs_from_eop_utc, EopSp00PolarError,
};
use virgostars_ephemeris_core::polar_motion::itrs_to_tirs_polar_motion_matrix;
use virgostars_ephemeris_core::tio_locator::approximate_tio_locator_sp00;

const HEADER: &str = "DATE,MJD,X,Y,UT1-UTC,LOD,DPSI,DEPS,DX,DY,DAT,DATA_TYPE";
const HISTORICAL: &str =
    "2000-01-01,51544,0.043282,0.377909,0.3553880,0.0009536,-0.050263,-0.002471,-0.000005,-0.000050,32,O";
const NEXT_OBSERVED: &str =
    "2000-01-02,51545,0.143282,0.477909,0.3551880,0.0009000,-0.050263,-0.002471,-0.000005,-0.000050,32,O";
const NEXT_PREDICTED: &str =
    "2000-01-03,51546,0.243282,0.577909,0.3550000,0.0009000,-0.050263,-0.002471,-0.000005,-0.000050,32,P";

const RAD_PER_ARCSEC: f64 = PI / (180.0 * 3_600.0);

fn catalog(rows: &[&str]) -> CelestrakEopPolarCatalog {
    let text = format!("{HEADER}\n{}\n", rows.join("\n"));
    CelestrakEopPolarCatalog::parse_csv("audited-external-EOP-example", &text).unwrap()
}

fn near(actual: f64, expected: f64, tol: f64) {
    assert!(
        (actual - expected).abs() <= tol,
        "expected {expected}, got {actual}; tolerance {tol}"
    );
}

#[test]
fn independent_celestrak_example_yields_correct_tt_dat_and_eop_radians() {
    // CelesTrak's published 2000-01-01 observed sample is a FORMAT
    // oracle, not evidence about a current Earth orientation model.
    let catalog = catalog(&[HISTORICAL, NEXT_OBSERVED]);
    let result = approximate_itrs_to_tirs_from_eop_utc(
        &catalog,
        51_544,
        0.0,
        EopPredictionPolicy::ObservedOnly,
    )
    .unwrap();
    assert_eq!(result.source_label, "audited-external-EOP-example");
    assert_eq!(result.quality, EopPolarQuality::Observed);
    assert_eq!(result.tai_minus_utc_seconds, 32);
    assert_eq!(result.tt_epoch.scale(), TimeScale::Tt);
    assert_eq!(result.tt_epoch.day_number(), 2_451_545);
    near(
        result.tt_epoch.midnight_fraction(),
        64.184 / 86_400.0,
        1.0e-15,
    );
    near(result.xp_rad, 0.043_282 * RAD_PER_ARCSEC, 1.0e-21);
    near(result.yp_rad, 0.377_909 * RAD_PER_ARCSEC, 1.0e-21);
    // This SP00 reference comes from the *published IERS linear
    // formula*, with TT obtained from 0h UTC + known DAT.
    let centuries_since_j2000 = (-0.5 + 64.184 / 86_400.0) / 36_525.0;
    let expected_sp = -47.0e-6 * RAD_PER_ARCSEC * centuries_since_j2000;
    near(
        result.approximate_polar.locator.sp_rad,
        expected_sp,
        1.0e-24,
    );
    let direct = itrs_to_tirs_polar_motion_matrix(
        0.043_282 * RAD_PER_ARCSEC,
        0.377_909 * RAD_PER_ARCSEC,
        expected_sp,
    )
    .unwrap();
    for (row, expected_row) in result.approximate_polar.itrs_to_tirs.iter().zip(direct) {
        for (value, expected) in row.iter().zip(expected_row) {
            near(*value, expected, 1.0e-16);
        }
    }
}

#[test]
fn half_day_sample_matches_split_tt_and_composes_existing_modules() {
    let input = catalog(&[HISTORICAL, NEXT_OBSERVED]);
    let output = approximate_itrs_to_tirs_from_eop_utc(
        &input,
        51_544,
        0.5,
        EopPredictionPolicy::ObservedOnly,
    )
    .unwrap();
    near(
        output.tt_epoch.midnight_fraction(),
        0.5 + 64.184 / 86_400.0,
        1.0e-15,
    );
    near(output.xp_rad, 0.093_282 * RAD_PER_ARCSEC, 1.0e-21);
    near(output.yp_rad, 0.427_909 * RAD_PER_ARCSEC, 1.0e-21);
    assert_eq!(output.quality, EopPolarQuality::Observed);
    let sp = approximate_tio_locator_sp00(output.tt_epoch).unwrap();
    assert_eq!(output.approximate_polar.locator, sp);
    assert_eq!(
        output.approximate_polar.itrs_to_tirs,
        itrs_to_tirs_polar_motion_matrix(output.xp_rad, output.yp_rad, sp.sp_rad).unwrap()
    );
}

#[test]
fn normal_utc_day_tt_midnight_carry_is_explicit_and_split_precise() {
    let input = catalog(&[HISTORICAL, NEXT_OBSERVED]);
    let fraction = 0.999_9;
    let output = approximate_itrs_to_tirs_from_eop_utc(
        &input,
        51_544,
        fraction,
        EopPredictionPolicy::ObservedOnly,
    )
    .unwrap();
    assert_eq!(output.tt_epoch.day_number(), 2_451_546);
    near(
        output.tt_epoch.midnight_fraction(),
        fraction + 64.184 / 86_400.0 - 1.0,
        1.0e-15,
    );
    assert!(output.tt_epoch.midnight_fraction() >= 0.0);
    assert!(output.tt_epoch.midnight_fraction() < 1.0);
}

#[test]
fn prediction_requires_opt_in_and_provenance_is_preserved() {
    let input = catalog(&[HISTORICAL, NEXT_OBSERVED, NEXT_PREDICTED]);
    assert_eq!(
        approximate_itrs_to_tirs_from_eop_utc(
            &input,
            51_545,
            0.5,
            EopPredictionPolicy::ObservedOnly,
        ),
        Err(EopSp00PolarError::Eop(EopCsvError::PredictionNotAuthorized))
    );
    let result = approximate_itrs_to_tirs_from_eop_utc(
        &input,
        51_545,
        0.5,
        EopPredictionPolicy::AllowPredicted,
    )
    .unwrap();
    assert_eq!(result.quality, EopPolarQuality::IncludesPrediction);
    near(result.xp_rad, 0.193_282 * RAD_PER_ARCSEC, 1.0e-21);
    assert_eq!(result.tai_minus_utc_seconds, 32);
}

#[test]
fn detected_leap_second_day_refuses_silent_normal_day_assumption() {
    // Synthetic DUT1 values; DAT 36 at 2016-12-31 midnight then
    // DAT 37 at 2017-01-01 midnight, the actual historical jump.
    let input = catalog(&[
        "2016-12-31,57753,0.10,0.20,-0.20,0,0,0,0,0,36,O",
        "2017-01-01,57754,0.30,0.40,0.80,0,0,0,0,0,37,O",
    ]);
    assert_eq!(
        approximate_itrs_to_tirs_from_eop_utc(
            &input,
            57_753,
            0.5,
            EopPredictionPolicy::ObservedOnly,
        ),
        Err(EopSp00PolarError::LeapSecondBoundary)
    );
    // At each exact 0h UTC, the DAT metadata applies unambiguously;
    // the rest of leap-second day remains deliberately unsupported.
    let start = approximate_itrs_to_tirs_from_eop_utc(
        &input,
        57_753,
        0.0,
        EopPredictionPolicy::ObservedOnly,
    )
    .unwrap();
    let end = approximate_itrs_to_tirs_from_eop_utc(
        &input,
        57_754,
        0.0,
        EopPredictionPolicy::ObservedOnly,
    )
    .unwrap();
    assert_eq!(start.tai_minus_utc_seconds, 36);
    assert_eq!(end.tai_minus_utc_seconds, 37);
    near(
        start.tt_epoch.midnight_fraction(),
        68.184 / 86_400.0,
        1.0e-15,
    );
    near(end.tt_epoch.midnight_fraction(), 69.184 / 86_400.0, 1.0e-15);
}

#[test]
fn rejects_pre_1972_utc_even_when_integer_dat_is_supplied() {
    let input = catalog(&[
        "1971-12-31,41316,0.1,0.2,0,0,0,0,0,0,10,O",
        "1972-01-01,41317,0.1,0.2,0,0,0,0,0,0,10,O",
    ]);
    assert_eq!(
        approximate_itrs_to_tirs_from_eop_utc(
            &input,
            41_316,
            0.0,
            EopPredictionPolicy::ObservedOnly,
        ),
        Err(EopSp00PolarError::BeforeModernUtc)
    );
    assert!(approximate_itrs_to_tirs_from_eop_utc(
        &input,
        41_317,
        0.0,
        EopPredictionPolicy::ObservedOnly,
    )
    .is_ok());
}

#[test]
fn rejects_unsupported_tai_utc_offsets_in_unauthenticated_eop_files() {
    let input = catalog(&["2000-01-01,51544,0.1,0.2,0,0,0,0,0,0,0,O"]);
    assert_eq!(
        approximate_itrs_to_tirs_from_eop_utc(
            &input,
            51_544,
            0.0,
            EopPredictionPolicy::ObservedOnly,
        ),
        Err(EopSp00PolarError::UnsupportedTaiUtcOffset)
    );
}

#[test]
fn propagates_catalog_range_gaps_and_invalid_utc_fractions() {
    let input = catalog(&[HISTORICAL, NEXT_PREDICTED]);
    assert_eq!(
        approximate_itrs_to_tirs_from_eop_utc(
            &input,
            51_544,
            0.5,
            EopPredictionPolicy::AllowPredicted,
        ),
        Err(EopSp00PolarError::Eop(EopCsvError::MissingFollowingDay))
    );
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1, 1.0] {
        assert_eq!(
            approximate_itrs_to_tirs_from_eop_utc(
                &input,
                51_544,
                bad,
                EopPredictionPolicy::ObservedOnly,
            ),
            Err(EopSp00PolarError::Eop(EopCsvError::InvalidUtcFraction))
        );
    }
}
