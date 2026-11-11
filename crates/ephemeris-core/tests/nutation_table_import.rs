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

//! Synthetic fixtures only: controlled nutation term input and integration.
//! No official IERS/USNO/SOFA coefficient tables are embedded here.

use std::f64::consts::FRAC_PI_2;

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::nutation_harmonics::evaluate_nutation_harmonics;
use virgostars_ephemeris_core::nutation_table_import::{
    parse_lunisolar_table, parse_planetary_table, TableParseErrorKind, TableSourceKind,
};

const LUNAR_ORDER: &str = "arguments=l,lprime,F,D,Omega";
const PLANETARY_ORDER: &str =
    "arguments=l,lprime,F,D,Omega,Mercury,Venus,Earth,Mars,Jupiter,Saturn,Uranus,Neptune,pA";

fn fixture(family: &str, order: &str, source: &str, reference: &str, rows: &[String]) -> String {
    format!(
        "VIRGOSTARS_NUTATION_TERMS_V1\nfamily={family}\n{order}\namplitudes=radians\nslopes=radians_per_tdb_century\nsource={source}\nreference={reference}\ncount={}\nDATA\n{}\n",
        rows.len(),
        rows.join("\n")
    )
}

fn lunar(rows: &[String]) -> String {
    fixture(
        "lunisolar",
        LUNAR_ORDER,
        "synthetic",
        "synthetic:unit-test",
        rows,
    )
}

fn lunar_row(index: usize, multiplier: i8, s: &str) -> String {
    format!("{index},{multiplier},0,0,0,0,{s},0,0,0.0002,0,0")
}

fn planetary_row(index: usize) -> String {
    let multipliers = [1_i8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let cols = multipliers.map(|value| value.to_string()).join(",");
    format!("{index},{cols},0.0001,0,0,0.0002,0,0")
}

fn planetary(rows: &[String]) -> String {
    fixture(
        "planetary",
        PLANETARY_ORDER,
        "external_unverified",
        "https://example.org/unverified-source",
        rows,
    )
}

fn failure(text: &str, is_planetary: bool) -> TableParseErrorKind {
    if is_planetary {
        parse_planetary_table(text).unwrap_err().kind
    } else {
        parse_lunisolar_table(text).unwrap_err().kind
    }
}

#[test]
fn imports_two_synthetic_lunisolar_rows_with_exact_field_mapping() {
    let file = lunar(&[lunar_row(1, 1, "0.0001"), lunar_row(2, -1, "0.0003")]);
    let parsed = parse_lunisolar_table(&file).unwrap();
    assert_eq!(parsed.source_kind, TableSourceKind::Synthetic);
    assert_eq!(parsed.source_reference, "synthetic:unit-test");
    assert_eq!(parsed.terms.len(), 2);
    assert!(!parsed.has_full_family_length());
    assert_eq!(parsed.terms[0].multipliers, [1, 0, 0, 0, 0]);
    assert_eq!(parsed.terms[1].multipliers, [-1, 0, 0, 0, 0]);
    assert_eq!(parsed.terms[1].longitude_sin_rad, 0.0003);
    assert_eq!(parsed.terms[0].obliquity_cos_rad, 0.0002);
}

#[test]
fn imported_terms_evaluate_in_the_existing_harmonic_rust_engine() {
    let parsed = parse_lunisolar_table(&lunar(&[lunar_row(1, 1, "0.0001")])).unwrap();
    let tdb = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap();
    let evaluated = evaluate_nutation_harmonics(
        tdb,
        [FRAC_PI_2, 0.0, 0.0, 0.0, 0.0],
        [0.25, 0.0, 0.0, 0.0, 0.0],
        &parsed.terms,
    )
    .unwrap();
    assert!((evaluated.longitude_rad - 0.0001).abs() < 1e-15);
    assert!(evaluated.obliquity_rad.abs() < 1e-15);
    assert!((evaluated.obliquity_rate_rad_per_tdb_day + 0.00005).abs() < 1e-15);
    assert_eq!(evaluated.terms_evaluated, 1);
}

#[test]
fn planetary_table_requires_explicit_unverified_source_and_order() {
    let parsed = parse_planetary_table(&planetary(&[planetary_row(1)])).unwrap();
    assert_eq!(parsed.source_kind, TableSourceKind::ExternalUnverified);
    assert_eq!(parsed.terms.len(), 1);
    assert_eq!(parsed.terms[0].multipliers[0], 1);
    assert_eq!(parsed.terms[0].multipliers[1], 0);
    assert_eq!(parsed.terms[0].multipliers[13], 1);
    assert!(!parsed.has_full_family_length());
}

#[test]
fn rejects_noncanonical_heliocentric_first_multiplier_order() {
    let original = lunar(&[lunar_row(1, 1, "0.0001")]);
    assert_eq!(
        failure(&original.replace(LUNAR_ORDER, PLANETARY_ORDER), false),
        TableParseErrorKind::InvalidHeader,
    );
    let other = planetary(&[planetary_row(1)]);
    assert_eq!(
        failure(
            &other.replace(
                PLANETARY_ORDER,
                "arguments=Mercury,Venus,Earth,Mars,Jupiter,Saturn,Uranus,Neptune,pA,l,lprime,F,D,Omega",
            ),
            true,
        ),
        TableParseErrorKind::InvalidHeader,
    );
}

#[test]
fn rejects_raw_arcseconds_degrees_or_wrong_slope_units() {
    let sample = lunar(&[lunar_row(1, 1, "0.0001")]);
    for wrong in [
        sample.replace("amplitudes=radians", "amplitudes=arcseconds"),
        sample.replace("amplitudes=radians", "amplitudes=degrees"),
        sample.replace(
            "slopes=radians_per_tdb_century",
            "slopes=arcseconds_per_century",
        ),
        sample.replace(
            "VIRGOSTARS_NUTATION_TERMS_V1",
            "VIRGOSTARS_NUTATION_TERMS_V0",
        ),
    ] {
        assert_eq!(failure(&wrong, false), TableParseErrorKind::InvalidHeader);
    }
}

#[test]
fn rejects_unsupported_or_missing_provenance_without_license_assertion() {
    let sample = lunar(&[lunar_row(1, 1, "0.0001")]);
    for changed in [
        sample.replace("source=synthetic", "source=approved"),
        sample.replace("reference=synthetic:unit-test", "reference="),
        sample.replace(
            "reference=synthetic:unit-test",
            "reference=https://not-a-synthetic",
        ),
    ] {
        let reason = failure(&changed, false);
        assert!(
            reason == TableParseErrorKind::InvalidHeader
                || reason == TableParseErrorKind::InvalidSourceReference
        );
    }
    let external = planetary(&[planetary_row(1)]);
    assert_eq!(
        failure(
            &external.replace("https://example.org", "file://example.org"),
            true,
        ),
        TableParseErrorKind::InvalidSourceReference,
    );
}

#[test]
fn rejects_missing_duplicate_or_out_of_order_one_based_term_indices() {
    let valid = lunar(&[lunar_row(1, 1, "0.0001"), lunar_row(2, -1, "0.0003")]);
    assert_eq!(
        failure(&valid.replace("2,-1,", "1,-1,"), false),
        TableParseErrorKind::InvalidRowSequence,
    );
    assert_eq!(
        failure(&valid.replace("2,-1,", "3,-1,"), false),
        TableParseErrorKind::InvalidRowSequence,
    );
    assert_eq!(
        failure(&valid.replace("2,-1,", "02,-1,"), false),
        TableParseErrorKind::InvalidRowSequence,
    );
    let sample = lunar(&[lunar_row(1, 1, "0.0001")]);
    assert_eq!(
        failure(&sample.replace("1,1,", "0,1,"), false),
        TableParseErrorKind::InvalidRowSequence,
    );
}

#[test]
fn declared_count_must_match_nonempty_exactly_and_respect_family_limits() {
    let sample = lunar(&[lunar_row(1, 1, "0.0001")]);
    for (value, expected) in [
        ("0", TableParseErrorKind::InvalidCount),
        ("679", TableParseErrorKind::InvalidCount),
        ("01", TableParseErrorKind::InvalidCount),
        ("2", TableParseErrorKind::CountMismatch),
    ] {
        assert_eq!(
            failure(&sample.replace("count=1", &format!("count={value}")), false),
            expected,
        );
    }
    let extra = format!("{sample}{}", lunar_row(2, 1, "0.0002"));
    assert_eq!(failure(&extra, false), TableParseErrorKind::CountMismatch);
    assert_eq!(
        failure(
            &planetary(&[planetary_row(1)]).replace("count=1", "count=688"),
            true,
        ),
        TableParseErrorKind::InvalidCount,
    );
}

#[test]
fn rejects_missing_data_rows_invalid_multiplier_or_bad_width() {
    let sample = lunar(&[lunar_row(1, 1, "0.0001")]);
    assert_eq!(
        failure(&sample.replace("1,1,0,", "1,128,0,"), false),
        TableParseErrorKind::InvalidMultiplier,
    );
    assert_eq!(
        failure(&sample.replace("1,1,0,", "1,1,"), false),
        TableParseErrorKind::InvalidRowWidth,
    );
    assert_eq!(
        failure(&sample.replace("0.0002,0,0", "0.0002,0,0,0"), false),
        TableParseErrorKind::InvalidRowWidth,
    );
    let missing = "VIRGOSTARS_NUTATION_TERMS_V1\nfamily=lunisolar\n";
    assert_eq!(failure(missing, false), TableParseErrorKind::MissingField);
}

#[test]
fn finite_and_physical_guards_reject_unconverted_or_nan_coefficients() {
    let sample = lunar(&[lunar_row(1, 1, "0.0001")]);
    assert_eq!(
        failure(&sample.replace("0.0001", "NaN"), false),
        TableParseErrorKind::NonFiniteCoefficient,
    );
    assert_eq!(
        failure(&sample.replace("0.0001", "Infinity"), false),
        TableParseErrorKind::NonFiniteCoefficient,
    );
    assert_eq!(
        failure(&sample.replace("0.0001", "17.2064161"), false),
        TableParseErrorKind::ImplausibleCoefficient,
    );
    assert_eq!(
        failure(&sample.replace("0.0001", "no-number"), false),
        TableParseErrorKind::InvalidCoefficient,
    );
}

#[test]
fn planetary_historical_solar_and_secular_term_guards_are_enforced() {
    let file = planetary(&[planetary_row(1)]);
    assert_eq!(
        failure(&file.replacen("1,1,0,", "1,1,2,", 1), true),
        TableParseErrorKind::SolarAnomalyNotUsed,
    );
    // Check both historical planetary slope columns independently.
    let row = planetary_row(1);
    let mut fields: Vec<&str> = row.split(',').collect();
    fields[16] = "0.00001"; // N=14, coefficient 1 (lon slope)
    assert_eq!(
        failure(&planetary(&[fields.join(",")]), true),
        TableParseErrorKind::PlanetarySecularSlopeNotUsed,
    );
    fields[16] = "0";
    fields[19] = "-0.00001"; // N=14, coefficient 4 (obl slope)
    assert_eq!(
        failure(&planetary(&[fields.join(",")]), true),
        TableParseErrorKind::PlanetarySecularSlopeNotUsed,
    );
}

#[test]
fn complete_length_is_structural_only_not_a_certification() {
    let lunar_rows: Vec<String> = (1..=678).map(|n| lunar_row(n, 0, "0")).collect();
    let planet_rows: Vec<String> = (1..=687).map(planetary_row).collect();
    let lunar = parse_lunisolar_table(&lunar(&lunar_rows)).unwrap();
    let planet = parse_planetary_table(&planetary(&planet_rows)).unwrap();
    assert!(lunar.has_full_family_length());
    assert!(planet.has_full_family_length());
    assert_eq!(lunar.terms.len() + planet.terms.len(), 1365);
}

#[test]
fn oversized_input_is_rejected_before_row_allocation() {
    let sample = lunar(&[lunar_row(1, 1, "0.0001")]);
    let huge = format!("{sample}{}", "X".repeat(280_000));
    assert_eq!(failure(&huge, false), TableParseErrorKind::FileTooLarge);
}
