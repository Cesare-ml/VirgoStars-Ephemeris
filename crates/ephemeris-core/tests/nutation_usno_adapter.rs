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

//! Unit and structural regressions for converting **caller-supplied**
//! USNO printed-layout rows into the original VirgoStars harmonic format.
//! Contains ONE independent numeric reference row from USNO Circular 179,
//! Appendix E (2005), p. 88, not a copied complete coefficient table.

use std::f64::consts::{FRAC_PI_2, PI};

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::nutation_harmonics::evaluate_nutation_harmonics;
use virgostars_ephemeris_core::nutation_table_import::{
    parse_lunisolar_table, parse_planetary_table,
};
use virgostars_ephemeris_core::nutation_usno_adapter::{
    convert_usno_complete_series, convert_usno_lunisolar_batch, convert_usno_lunisolar_row,
    convert_usno_planetary_batch, convert_usno_planetary_row, UsnoBatchError, UsnoBatchMode,
    UsnoConversionError, UsnoFullSeriesError, UsnoPrintedHarmonicRow,
};

fn blank_row(index: usize) -> UsnoPrintedHarmonicRow {
    UsnoPrintedHarmonicRow {
        printed_term_number: index,
        multipliers: [0; 14],
        coefficients_arcseconds: [0.0; 6],
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() < tolerance,
        "actual={actual:.17e} expected={expected:.17e} tolerance={tolerance:.2e}"
    );
}

#[test]
fn independent_usno_first_nutation_term_converts_to_radians_in_correct_fields() {
    // G. H. Kaplan, USNO Circular 179 (2005), printed page 88, term 1.
    // Numeric *reference* values only. Planetary multipliers are zero;
    // printed j=14 (Omega) is 1. USNO coefficient units are arcseconds.
    let mut printed = blank_row(1);
    printed.multipliers[13] = 1;
    printed.coefficients_arcseconds = [
        -17.206_416_1,
        -0.017_466_6,
        0.003_338_6,
        9.205_233_1,
        0.000_908_6,
        0.001_537_7,
    ];

    let result = convert_usno_lunisolar_row(printed).unwrap();
    assert_eq!(result.multipliers, [0, 0, 0, 0, 1]);
    let expected = printed
        .coefficients_arcseconds
        .map(|arcseconds| (arcseconds / 3_600.0).to_radians());
    near(result.longitude_sin_rad, expected[0], 1e-19);
    near(
        result.longitude_sin_slope_rad_per_century,
        expected[1],
        1e-19,
    );
    near(result.longitude_cos_rad, expected[2], 1e-19);
    near(result.obliquity_cos_rad, expected[3], 1e-19);
    near(
        result.obliquity_cos_slope_rad_per_century,
        expected[4],
        1e-19,
    );
    near(result.obliquity_sin_rad, expected[5], 1e-19);
}

#[test]
fn lunar_conversion_maps_each_of_five_source_lunar_columns() {
    for index in 9..14 {
        let mut printed = blank_row(678);
        printed.multipliers[index] = -3;
        let converted = convert_usno_lunisolar_row(printed).unwrap();
        for internal in 0..5 {
            assert_eq!(
                converted.multipliers[internal],
                if internal == index - 9 { -3 } else { 0 }
            );
        }
    }
}

#[test]
fn planetary_conversion_maps_all_permitted_source_basis_vectors() {
    for printed_position in 0..14 {
        if printed_position == 10 {
            continue; // l-prime is not a planetary MHB2000 series argument.
        }
        let mut printed = blank_row(679);
        printed.multipliers[printed_position] = -2;
        let converted = convert_usno_planetary_row(printed).unwrap();
        let expected_internal = if printed_position < 9 {
            printed_position + 5
        } else {
            printed_position - 9
        };
        for internal in 0..14 {
            assert_eq!(
                converted.multipliers[internal],
                if internal == expected_internal { -2 } else { 0 },
                "USNO source position {printed_position}; canonical index {internal}"
            );
        }
    }
}

#[test]
fn rejects_global_index_outside_the_requested_source_family() {
    for index in [0, 679, 1365, usize::MAX] {
        assert_eq!(
            convert_usno_lunisolar_row(blank_row(index)),
            Err(UsnoConversionError::WrongPrintedTermRange)
        );
    }
    for index in [0, 1, 678, 1366, usize::MAX] {
        assert_eq!(
            convert_usno_planetary_row(blank_row(index)),
            Err(UsnoConversionError::WrongPrintedTermRange)
        );
    }
    assert!(convert_usno_lunisolar_row(blank_row(1)).is_ok());
    assert!(convert_usno_planetary_row(blank_row(1365)).is_ok());
}

#[test]
fn lunar_terms_reject_even_one_unexpected_planetary_multiplier() {
    for index in 0..9 {
        let mut printed = blank_row(5);
        printed.multipliers[index] = 1;
        assert_eq!(
            convert_usno_lunisolar_row(printed),
            Err(UsnoConversionError::LunisolarHasPlanetaryMultiplier)
        );
    }
}

#[test]
fn planetary_terms_reject_solar_anomaly_and_nonzero_secular_slopes() {
    let mut row = blank_row(679);
    row.multipliers[10] = -1;
    assert_eq!(
        convert_usno_planetary_row(row),
        Err(UsnoConversionError::PlanetaryHasSolarAnomalyMultiplier)
    );
    row.multipliers[10] = 0;
    for index in [1, 4] {
        row.coefficients_arcseconds[index] = 0.01;
        assert_eq!(
            convert_usno_planetary_row(row),
            Err(UsnoConversionError::PlanetaryHasAmplitudeSlope)
        );
        row.coefficients_arcseconds[index] = 0.0;
    }
    assert!(convert_usno_planetary_row(row).is_ok());
}

#[test]
fn rejects_nonfinite_or_impossibly_large_arcsecond_coefficients() {
    let mut row = blank_row(1);
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        row.coefficients_arcseconds[0] = invalid;
        assert_eq!(
            convert_usno_lunisolar_row(row),
            Err(UsnoConversionError::NonFiniteCoefficient)
        );
    }
    row.coefficients_arcseconds[0] = 360_000.0;
    assert_eq!(
        convert_usno_lunisolar_row(row),
        Err(UsnoConversionError::ImplausibleCoefficient)
    );
    row.coefficients_arcseconds[0] = -360_000.0;
    row.printed_term_number = 679;
    assert_eq!(
        convert_usno_planetary_row(row),
        Err(UsnoConversionError::ImplausibleCoefficient)
    );
}

#[test]
fn converted_lunar_term_round_trips_through_v1_parser_and_evaluator() {
    let mut printed = blank_row(1);
    printed.multipliers[13] = 1;
    printed.coefficients_arcseconds = [
        -17.206_416_1,
        -0.017_466_6,
        0.003_338_6,
        9.205_233_1,
        0.000_908_6,
        0.001_537_7,
    ];
    let converted = convert_usno_lunisolar_row(printed).unwrap();
    let multipliers = converted.multipliers.map(|v| v.to_string()).join(",");
    let text = format!(
        "VIRGOSTARS_NUTATION_TERMS_V1\nfamily=lunisolar\narguments=l,lprime,F,D,Omega\namplitudes=radians\nslopes=radians_per_tdb_century\nsource=external_unverified\nreference=https://aa.usno.navy.mil/downloads/Circular_179_bw.pdf\ncount=1\nDATA\n1,{multipliers},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}\n",
        converted.longitude_sin_rad,
        converted.longitude_sin_slope_rad_per_century,
        converted.longitude_cos_rad,
        converted.obliquity_cos_rad,
        converted.obliquity_cos_slope_rad_per_century,
        converted.obliquity_sin_rad,
    );
    let parsed = parse_lunisolar_table(&text).unwrap();
    assert_eq!(parsed.terms, vec![converted]);

    let epoch = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap();
    let result = evaluate_nutation_harmonics(
        epoch,
        [0.0, 0.0, 0.0, 0.0, FRAC_PI_2],
        [0.0; 5],
        &parsed.terms,
    )
    .unwrap();
    // This is the first term evaluated at a deliberately synthetic
    // argument, NOT an independently verified whole-series nutation value.
    near(result.longitude_rad, converted.longitude_sin_rad, 1e-19);
    near(result.obliquity_rad, converted.obliquity_sin_rad, 1e-19);
    assert_eq!(result.terms_evaluated, 1);
}

#[test]
fn converted_planetary_term_round_trips_with_exact_source_permutation() {
    let mut printed = blank_row(679);
    printed.multipliers[0] = 2; // Mercury -> internal index 5
    printed.multipliers[8] = 1; // pA -> internal index 13
    printed.multipliers[9] = -1; // l -> internal index 0
    printed.multipliers[13] = 3; // Omega -> internal index 4
    printed.coefficients_arcseconds = [0.000_2, 0.0, -0.000_3, 0.000_4, 0.0, 0.000_5];
    let term = convert_usno_planetary_row(printed).unwrap();
    assert_eq!(
        term.multipliers,
        [-1, 0, 0, 0, 3, 2, 0, 0, 0, 0, 0, 0, 0, 1]
    );
    let multipliers = term.multipliers.map(|v| v.to_string()).join(",");
    let text = format!(
        "VIRGOSTARS_NUTATION_TERMS_V1\nfamily=planetary\narguments=l,lprime,F,D,Omega,Mercury,Venus,Earth,Mars,Jupiter,Saturn,Uranus,Neptune,pA\namplitudes=radians\nslopes=radians_per_tdb_century\nsource=synthetic\nreference=synthetic:usno-planetary-permutation\ncount=1\nDATA\n1,{multipliers},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}\n",
        term.longitude_sin_rad,
        term.longitude_sin_slope_rad_per_century,
        term.longitude_cos_rad,
        term.obliquity_cos_rad,
        term.obliquity_cos_slope_rad_per_century,
        term.obliquity_sin_rad,
    );
    let parsed = parse_planetary_table(&text).unwrap();
    assert_eq!(parsed.terms, vec![term]);
    assert!(parsed.terms[0].longitude_sin_rad.abs() > 0.0);
    assert_eq!(term.longitude_sin_slope_rad_per_century, 0.0);
    assert_eq!(term.obliquity_cos_slope_rad_per_century, 0.0);
}

#[test]
fn source_units_are_arcseconds_not_microarcseconds_or_degrees() {
    let mut printed = blank_row(1);
    printed.coefficients_arcseconds[0] = 1.0;
    let converted = convert_usno_lunisolar_row(printed).unwrap();
    near(converted.longitude_sin_rad, PI / (180.0 * 3_600.0), 1e-20);
    // Rates use the *same* arcsecond-to-radian conversion; the numerical
    // time denominator is applied later by the harmonic evaluator.
    printed.coefficients_arcseconds[1] = 0.5;
    let term = convert_usno_lunisolar_row(printed).unwrap();
    near(
        term.longitude_sin_slope_rad_per_century,
        PI / (180.0 * 7_200.0),
        1e-20,
    );
}

fn synthetic_rows(start: usize, count: usize) -> Vec<UsnoPrintedHarmonicRow> {
    (start..start + count).map(blank_row).collect()
}

#[test]
fn batch_accepts_explicit_partial_contiguous_lunar_and_planetary_intervals() {
    let lunar = synthetic_rows(29, 3);
    let planetary = synthetic_rows(1_359, 7);
    let a = convert_usno_lunisolar_batch(&lunar, UsnoBatchMode::ContiguousSubset).unwrap();
    let b = convert_usno_planetary_batch(&planetary, UsnoBatchMode::ContiguousSubset).unwrap();
    assert_eq!(a.len(), 3);
    assert_eq!(b.len(), 7);
    assert!(a.iter().all(|term| term.multipliers == [0; 5]));
    assert!(b.iter().all(|term| term.multipliers == [0; 14]));
}

#[test]
fn batch_detects_gaps_duplicate_indices_and_reordering_precisely() {
    let base = synthetic_rows(100, 4);
    for (position, actual) in [(1, 100), (2, 104), (3, 102)] {
        let mut rows = base.clone();
        rows[position].printed_term_number = actual;
        assert_eq!(
            convert_usno_lunisolar_batch(&rows, UsnoBatchMode::ContiguousSubset),
            Err(UsnoBatchError::InvalidPrintedIndex {
                position,
                expected: 100 + position,
                actual,
            })
        );
    }
    let mut planetary = synthetic_rows(700, 3);
    planetary.swap(0, 1);
    assert_eq!(
        convert_usno_planetary_batch(&planetary, UsnoBatchMode::ContiguousSubset),
        Err(UsnoBatchError::InvalidPrintedIndex {
            position: 1,
            expected: 702,
            actual: 700,
        })
    );
}

#[test]
fn full_family_never_accepts_a_truncated_or_shifted_input() {
    let short_lunar = synthetic_rows(1, 677);
    assert_eq!(
        convert_usno_lunisolar_batch(&short_lunar, UsnoBatchMode::CompleteFamily),
        Err(UsnoBatchError::IncompleteFamily {
            expected: 678,
            actual: 677,
        })
    );
    let short_planetary = synthetic_rows(679, 686);
    assert_eq!(
        convert_usno_planetary_batch(&short_planetary, UsnoBatchMode::CompleteFamily),
        Err(UsnoBatchError::IncompleteFamily {
            expected: 687,
            actual: 686,
        })
    );
    let shifted_lunar = synthetic_rows(2, 678);
    assert_eq!(
        convert_usno_lunisolar_batch(&shifted_lunar, UsnoBatchMode::CompleteFamily),
        Err(UsnoBatchError::InvalidPrintedIndex {
            position: 0,
            expected: 1,
            actual: 2,
        })
    );
    let shifted_planetary = synthetic_rows(678, 687);
    assert_eq!(
        convert_usno_planetary_batch(&shifted_planetary, UsnoBatchMode::CompleteFamily),
        Err(UsnoBatchError::InvalidPrintedIndex {
            position: 0,
            expected: 679,
            actual: 678,
        })
    );
}

#[test]
fn both_complete_families_convert_all_1365_synthetic_indices_in_order() {
    let mut lunar = synthetic_rows(1, 678);
    let mut planetary = synthetic_rows(679, 687);
    lunar[0].multipliers[13] = 1;
    lunar[677].coefficients_arcseconds[0] = 0.05;
    planetary[0].multipliers[0] = 2;
    planetary[686].multipliers[8] = -3;
    let converted = convert_usno_complete_series(&lunar, &planetary).unwrap();
    assert_eq!(converted.lunisolar.len(), 678);
    assert_eq!(converted.planetary.len(), 687);
    assert_eq!(converted.lunisolar[0].multipliers, [0, 0, 0, 0, 1]);
    assert!(converted.lunisolar[677].longitude_sin_rad > 0.0);
    assert_eq!(converted.planetary[0].multipliers[5], 2);
    assert_eq!(converted.planetary[686].multipliers[13], -3);
}

#[test]
fn full_series_identifies_the_family_of_a_missing_middle_row() {
    let lunar = synthetic_rows(1, 678);
    let mut planetary = synthetic_rows(679, 687);
    planetary[408].printed_term_number = 1_090;
    assert_eq!(
        convert_usno_complete_series(&lunar, &planetary),
        Err(UsnoFullSeriesError::Planetary(
            UsnoBatchError::InvalidPrintedIndex {
                position: 408,
                expected: 1_087,
                actual: 1_090,
            }
        ))
    );
}

#[test]
fn a_corrupt_row_rejects_the_entire_batch_not_a_partial_success() {
    let mut lunar = synthetic_rows(1, 678);
    lunar[450].coefficients_arcseconds[0] = f64::NAN;
    assert_eq!(
        convert_usno_lunisolar_batch(&lunar, UsnoBatchMode::CompleteFamily),
        Err(UsnoBatchError::InvalidRow {
            position: 450,
            cause: UsnoConversionError::NonFiniteCoefficient,
        })
    );
    let mut planet = synthetic_rows(679, 687);
    planet[600].multipliers[10] = 1;
    assert_eq!(
        convert_usno_planetary_batch(&planet, UsnoBatchMode::CompleteFamily),
        Err(UsnoBatchError::InvalidRow {
            position: 600,
            cause: UsnoConversionError::PlanetaryHasSolarAnomalyMultiplier,
        })
    );
}

#[test]
fn partial_batches_cannot_cross_family_boundaries() {
    let crossing_lunar = synthetic_rows(677, 3);
    assert_eq!(
        convert_usno_lunisolar_batch(&crossing_lunar, UsnoBatchMode::ContiguousSubset),
        Err(UsnoBatchError::InvalidRow {
            position: 2,
            cause: UsnoConversionError::WrongPrintedTermRange,
        })
    );
    let crossing_planet = synthetic_rows(1_364, 3);
    assert_eq!(
        convert_usno_planetary_batch(&crossing_planet, UsnoBatchMode::ContiguousSubset),
        Err(UsnoBatchError::InvalidRow {
            position: 2,
            cause: UsnoConversionError::WrongPrintedTermRange,
        })
    );
}

#[test]
fn empty_or_oversized_batches_fail_even_when_partial_import_is_enabled() {
    assert_eq!(
        convert_usno_lunisolar_batch(&[], UsnoBatchMode::ContiguousSubset),
        Err(UsnoBatchError::EmptyInput)
    );
    assert_eq!(
        convert_usno_planetary_batch(&[], UsnoBatchMode::CompleteFamily),
        Err(UsnoBatchError::EmptyInput)
    );
    assert_eq!(
        convert_usno_lunisolar_batch(
            &synthetic_rows(1, 679),
            UsnoBatchMode::ContiguousSubset,
        ),
        Err(UsnoBatchError::TooManyRows)
    );
    assert_eq!(
        convert_usno_planetary_batch(
            &synthetic_rows(679, 688),
            UsnoBatchMode::ContiguousSubset,
        ),
        Err(UsnoBatchError::TooManyRows)
    );
}

#[test]
fn full_series_failure_is_familyspecific_even_when_lunars_incomplete() {
    let lunar = synthetic_rows(1, 677);
    let planetary = synthetic_rows(679, 687);
    assert_eq!(
        convert_usno_complete_series(&lunar, &planetary),
        Err(UsnoFullSeriesError::Lunisolar(
            UsnoBatchError::IncompleteFamily {
                expected: 678,
                actual: 677,
            }
        ))
    );
}
