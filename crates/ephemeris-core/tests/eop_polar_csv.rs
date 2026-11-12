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

//! Offline CelesTrak EOP CSV parser and polar interpolation contracts.
//! The first sample is an independent, published CelesTrak data-format
//! numerical reference. Other rows are explicitly synthetic fixtures;
//! no real IERS source authenticity or current-epoch accuracy is claimed.

use std::f64::consts::PI;

use virgostars_ephemeris_core::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopCsvError, EopPolarQuality, EopPredictionPolicy, EopRecordType,
};

const HEADER: &str = "DATE,MJD,X,Y,UT1-UTC,LOD,DPSI,DEPS,DX,DY,DAT,DATA_TYPE";
const PUBLISHED_SAMPLE: &str =
    "2000-01-01,51544,0.043282,0.377909,0.3553880,0.0009536,-0.050263,-0.002471,-0.000005,-0.000050,32,O";
const NEXT_OBSERVED: &str =
    "2000-01-02,51545,0.143282,0.477909,0.3551880,0.0009000,-0.050263,-0.002471,-0.000005,-0.000050,32,O";
const NEXT_PREDICTED: &str =
    "2000-01-03,51546,0.243282,0.577909,0.3550000,0.0009000,-0.050263,-0.002471,-0.000005,-0.000050,32,P";
const RAD_PER_ARCSEC: f64 = PI / (180.0 * 3_600.0);

fn parse(rows: &[&str]) -> Result<CelestrakEopPolarCatalog, EopCsvError> {
    let contents = format!("{HEADER}\n{}\n", rows.join("\n"));
    CelestrakEopPolarCatalog::parse_csv("CelesTrak CSV archived example, manually provided", &contents)
}

fn near(value: f64, expected: f64, tolerance: f64) {
    assert!(
        (value - expected).abs() <= tolerance,
        "actual {value} vs expected {expected}, tolerance {tolerance}"
    );
}

#[test]
fn published_celestrak_utc_day_reference_has_correct_units_and_metadata() {
    // Independent published reference:
    // https://celestrak.org/SpaceData/EOP-format.php
    let catalog = parse(&[PUBLISHED_SAMPLE]).unwrap();
    assert_eq!(catalog.samples().len(), 1);
    let row = catalog.samples()[0];
    assert_eq!(row.mjd_utc, 51544);
    assert_eq!(row.record_type, EopRecordType::Observed);
    near(row.xp_rad, 0.043_282 * RAD_PER_ARCSEC, 1.0e-21);
    near(row.yp_rad, 0.377_909 * RAD_PER_ARCSEC, 1.0e-21);
    assert_eq!(row.ut1_minus_utc_seconds_at_0h, 0.355_388);
    assert_eq!(row.tai_minus_utc_seconds_at_0h, 32);
    let exact = catalog.interpolate_observed(51544, 0.0).unwrap();
    assert_eq!(exact.quality, EopPolarQuality::Observed);
    assert_eq!(exact.xp_rad, row.xp_rad);
    assert_eq!(exact.yp_rad, row.yp_rad);
}

#[test]
fn linear_half_day_interpolation_converts_arcseconds_before_application() {
    // Second daily row is SYNTHETIC, to check the interpolation
    // without pretending to validate physical polar motion forecasts.
    let catalog = parse(&[PUBLISHED_SAMPLE, NEXT_OBSERVED]).unwrap();
    let midpoint = catalog.interpolate_observed(51544, 0.5).unwrap();
    near(midpoint.xp_rad, 0.093_282 * RAD_PER_ARCSEC, 2.0e-21);
    near(midpoint.yp_rad, 0.427_909 * RAD_PER_ARCSEC, 2.0e-21);
    assert_eq!(midpoint.quality, EopPolarQuality::Observed);
    assert_eq!(midpoint.mjd_utc, 51544);
    assert_eq!(midpoint.fraction_of_utc_day, 0.5);
    let end = catalog.interpolate_observed(51545, 0.0).unwrap();
    near(end.xp_rad, 0.143_282 * RAD_PER_ARCSEC, 1.0e-21);
}

#[test]
fn predicted_rows_are_never_mistaken_for_observed_without_opt_in() {
    let catalog = parse(&[PUBLISHED_SAMPLE, NEXT_OBSERVED, NEXT_PREDICTED]).unwrap();
    assert_eq!(catalog.samples()[2].record_type, EopRecordType::Predicted);
    assert_eq!(
        catalog.interpolate_observed(51545, 0.5),
        Err(EopCsvError::PredictionNotAuthorized)
    );
    assert_eq!(
        catalog.interpolate_observed(51546, 0.0),
        Err(EopCsvError::PredictionNotAuthorized)
    );
    let allowed = catalog
        .interpolate(51545, 0.5, EopPredictionPolicy::AllowPredicted)
        .unwrap();
    assert_eq!(allowed.quality, EopPolarQuality::IncludesPrediction);
    near(allowed.xp_rad, 0.193_282 * RAD_PER_ARCSEC, 2.0e-21);
    let exact = catalog
        .interpolate(51546, 0.0, EopPredictionPolicy::AllowPredicted)
        .unwrap();
    assert_eq!(exact.quality, EopPolarQuality::IncludesPrediction);
}

#[test]
fn missing_adjacent_day_is_not_silently_extrapolated_or_bridged() {
    let catalog = parse(&[PUBLISHED_SAMPLE, NEXT_PREDICTED]).unwrap();
    assert_eq!(
        catalog.interpolate_observed(51544, 0.5),
        Err(EopCsvError::MissingFollowingDay)
    );
    assert_eq!(
        catalog.interpolate_observed(51545, 0.0),
        Err(EopCsvError::DayOutsideCatalog)
    );
    let exact_last = catalog
        .interpolate(51546, 0.0, EopPredictionPolicy::AllowPredicted)
        .unwrap();
    assert_eq!(exact_last.mjd_utc, 51546);
    assert_eq!(
        catalog.interpolate(51546, 0.25, EopPredictionPolicy::AllowPredicted),
        Err(EopCsvError::MissingFollowingDay)
    );
}

#[test]
fn fractional_day_must_be_finite_and_in_half_open_unit_interval() {
    let catalog = parse(&[PUBLISHED_SAMPLE, NEXT_OBSERVED]).unwrap();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.5, 1.0, 1.5] {
        assert_eq!(
            catalog.interpolate_observed(51544, bad),
            Err(EopCsvError::InvalidUtcFraction)
        );
    }
    assert!(catalog.interpolate_observed(51544, 0.999).is_ok());
}

#[test]
fn source_header_and_sorted_order_are_explicitly_validated() {
    assert_eq!(
        CelestrakEopPolarCatalog::parse_csv("   ", &format!("{HEADER}\n{PUBLISHED_SAMPLE}")),
        Err(EopCsvError::MissingSourceLabel)
    );
    assert_eq!(
        CelestrakEopPolarCatalog::parse_csv("known sample", "X,Y,MJD\n0.1,0.2,51544"),
        Err(EopCsvError::InvalidHeader)
    );
    assert_eq!(
        CelestrakEopPolarCatalog::parse_csv("known sample", HEADER),
        Err(EopCsvError::EmptyCatalog)
    );
    assert_eq!(
        parse(&[PUBLISHED_SAMPLE, PUBLISHED_SAMPLE]),
        Err(EopCsvError::UnsortedOrDuplicateDays { line: 3 })
    );
    assert_eq!(
        parse(&[NEXT_OBSERVED, PUBLISHED_SAMPLE]),
        Err(EopCsvError::UnsortedOrDuplicateDays { line: 3 })
    );
    let with_bom = format!("\u{feff}\r\n{HEADER}\r\n{PUBLISHED_SAMPLE}\r\n\r\n");
    // A BOM is recognized only when it precedes the first character,
    // as defined by the parser contract (not after a blank line).
    let no_leading_blank = with_bom.replacen("\r\n", "", 1);
    assert_eq!(
        CelestrakEopPolarCatalog::parse_csv("known sample", &no_leading_blank)
            .unwrap()
            .samples()
            .len(),
        1
    );
}

#[test]
fn malformation_date_mjd_and_units_fail_closed() {
    let wrong_mjd = PUBLISHED_SAMPLE.replace("51544", "51545");
    assert_eq!(
        parse(&[&wrong_mjd]),
        Err(EopCsvError::DateMjdMismatch { line: 2 })
    );
    let wrong_date = PUBLISHED_SAMPLE.replace("2000-01-01", "2000-02-30");
    assert_eq!(
        parse(&[&wrong_date]),
        Err(EopCsvError::MalformedRow { line: 2 })
    );
    let invalid_pole = PUBLISHED_SAMPLE.replace("0.043282", "3600.0");
    assert_eq!(
        parse(&[&invalid_pole]),
        Err(EopCsvError::InvalidPolarCoordinate { line: 2 })
    );
    let nonfinite_pole = PUBLISHED_SAMPLE.replace("0.043282", "NaN");
    assert_eq!(
        parse(&[&nonfinite_pole]),
        Err(EopCsvError::MalformedRow { line: 2 })
    );
    let invalid_other_field = PUBLISHED_SAMPLE.replace("0.0009536", "inf");
    assert_eq!(
        parse(&[&invalid_other_field]),
        Err(EopCsvError::MalformedRow { line: 2 })
    );
    let invalid_kind = PUBLISHED_SAMPLE.replace(",32,O", ",32,UNKNOWN");
    assert_eq!(
        parse(&[&invalid_kind]),
        Err(EopCsvError::MalformedRow { line: 2 })
    );
}

#[test]
fn leap_second_metadata_are_not_automatically_interpolated_as_ut1() {
    // These two rows are SYNTHETIC. Their 0h UTC DAT fields straddle
    // a real leap-second date, but the numeric values are illustrative,
    // not a physical UT1-UTC reference.
    let before =
        "2016-12-31,57753,0.10,0.20,-0.20,0.0,0.0,0.0,0.0,0.0,36,O";
    let after =
        "2017-01-01,57754,0.30,0.40,0.80,0.0,0.0,0.0,0.0,0.0,37,O";
    let catalog = parse(&[before, after]).unwrap();
    let a = catalog.samples()[0];
    let b = catalog.samples()[1];
    assert_eq!(a.tai_minus_utc_seconds_at_0h, 36);
    assert_eq!(b.tai_minus_utc_seconds_at_0h, 37);
    let interpolated = catalog.interpolate_observed(57753, 0.5).unwrap();
    near(interpolated.xp_rad, 0.20 * RAD_PER_ARCSEC, 2.0e-21);
    near(interpolated.yp_rad, 0.30 * RAD_PER_ARCSEC, 2.0e-21);
    // The interpolation API returns ONLY xp,yp, never a fictitious
    // interpolated UT1-UTC value across the leap discontinuity.
    assert_eq!(a.ut1_minus_utc_seconds_at_0h, -0.20);
    assert_eq!(b.ut1_minus_utc_seconds_at_0h, 0.80);
}
