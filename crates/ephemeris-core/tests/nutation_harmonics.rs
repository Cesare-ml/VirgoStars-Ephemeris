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

//! Synthetic mathematical regressions for the periodic harmonic evaluator.
//!
//! IMPORTANT: no real IAU 2000A coefficient table is bundled in this test.
//! Passing these tests is NOT a full Δψ/Δε numerical comparison with ERFA.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::nutation_harmonics::{
    evaluate_nutation_harmonics, LuniSolarHarmonicTerm, NutationHarmonicError,
    NutationHarmonicTerm, PlanetaryHarmonicTerm,
};

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}"
    );
}

fn tdb(days_since_j2000: f64) -> JulianDate {
    JulianDate::from_f64(2_451_545.0 + days_since_j2000, TimeScale::Tdb).unwrap()
}

fn empty_term<const N: usize>(multipliers: [i8; N]) -> NutationHarmonicTerm<N> {
    NutationHarmonicTerm {
        multipliers,
        longitude_sin_rad: 0.0,
        longitude_sin_slope_rad_per_century: 0.0,
        longitude_cos_rad: 0.0,
        obliquity_cos_rad: 0.0,
        obliquity_cos_slope_rad_per_century: 0.0,
        obliquity_sin_rad: 0.0,
    }
}

#[test]
fn single_lunisolar_term_has_exact_analytical_value_and_rate() {
    let term = LuniSolarHarmonicTerm {
        longitude_sin_rad: 2.0,
        longitude_sin_slope_rad_per_century: 4.0,
        longitude_cos_rad: 3.0,
        obliquity_cos_rad: 5.0,
        obliquity_cos_slope_rad_per_century: 6.0,
        obliquity_sin_rad: 7.0,
        ..empty_term([1, 0, 0, 0, 0])
    };
    let result = evaluate_nutation_harmonics(
        tdb(0.0),
        [FRAC_PI_2, 0.0, 0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0, 0.0, 0.0],
        &[term],
    )
    .unwrap();
    near(result.longitude_rad, 2.0, 1e-14);
    near(result.obliquity_rad, 7.0, 1e-14);
    near(
        result.longitude_rate_rad_per_tdb_day,
        4.0 / 36_525.0 - 0.3,
        1e-14,
    );
    near(result.obliquity_rate_rad_per_tdb_day, -0.5, 1e-14);
    assert_eq!(result.terms_evaluated, 1);
}

#[test]
fn both_derivatives_match_independent_centered_perturbations() {
    let base = [0.3, 1.1, 0.5, 0.7, 0.9];
    let rates = [0.008, -0.001, 0.004, 0.003, -0.002];
    let terms = [
        NutationHarmonicTerm {
            longitude_sin_rad: 0.07,
            longitude_sin_slope_rad_per_century: -0.2,
            longitude_cos_rad: 0.04,
            obliquity_cos_rad: -0.03,
            obliquity_cos_slope_rad_per_century: 0.13,
            obliquity_sin_rad: 0.09,
            ..empty_term([2, -1, 1, -2, 1])
        },
        NutationHarmonicTerm {
            longitude_sin_rad: -0.13,
            longitude_sin_slope_rad_per_century: 0.01,
            longitude_cos_rad: 0.27,
            obliquity_cos_rad: 0.19,
            obliquity_cos_slope_rad_per_century: 0.08,
            obliquity_sin_rad: -0.03,
            ..empty_term([-1, 0, 3, 0, -2])
        },
    ];
    let at = |time_days: f64| {
        let arguments = std::array::from_fn(|i| (base[i] + rates[i] * time_days).rem_euclid(TAU));
        evaluate_nutation_harmonics(tdb(80.0 + time_days), arguments, rates, &terms).unwrap()
    };
    let h = 0.01;
    let before = at(-h);
    let now = at(0.0);
    let after = at(h);
    near(
        now.longitude_rate_rad_per_tdb_day,
        (after.longitude_rad - before.longitude_rad) / (2.0 * h),
        1e-10,
    );
    near(
        now.obliquity_rate_rad_per_tdb_day,
        (after.obliquity_rad - before.obliquity_rad) / (2.0 * h),
        1e-10,
    );
    assert_eq!(now.terms_evaluated, 2);
}

#[test]
fn fourteen_argument_planetary_phase_is_not_lunar_only() {
    let mut arguments = [0.0; 14];
    let mut speeds = [0.0; 14];
    arguments[13] = PI / 3.0;
    speeds[13] = 0.2;
    let mut multipliers = [0_i8; 14];
    multipliers[13] = 1;
    let term = PlanetaryHarmonicTerm {
        longitude_sin_rad: 0.25,
        obliquity_cos_rad: 0.5,
        ..empty_term(multipliers)
    };
    let result = evaluate_nutation_harmonics(tdb(0.0), arguments, speeds, &[term]).unwrap();
    near(result.longitude_rad, 0.25 * (PI / 3.0).sin(), 1e-15);
    near(result.obliquity_rad, 0.25, 1e-15);
    near(
        result.longitude_rate_rad_per_tdb_day,
        0.25 * (PI / 3.0).cos() * 0.2,
        1e-15,
    );
    near(
        result.obliquity_rate_rad_per_tdb_day,
        -0.5 * (PI / 3.0).sin() * 0.2,
        1e-15,
    );
}

#[test]
fn all_zero_multipliers_keep_only_secular_obliquity_changes() {
    let term = LuniSolarHarmonicTerm {
        longitude_sin_rad: 1.0,
        longitude_sin_slope_rad_per_century: 4.0,
        longitude_cos_rad: 2.0,
        obliquity_cos_rad: 3.0,
        obliquity_cos_slope_rad_per_century: 10.0,
        obliquity_sin_rad: 5.0,
        ..empty_term([0; 5])
    };
    let result = evaluate_nutation_harmonics(tdb(36_525.0), [0.0; 5], [0.0; 5], &[term]).unwrap();
    near(result.longitude_rad, 2.0, 1e-15);
    near(result.obliquity_rad, 13.0, 1e-15);
    near(result.longitude_rate_rad_per_tdb_day, 0.0, 1e-15);
    near(
        result.obliquity_rate_rad_per_tdb_day,
        10.0 / 36_525.0,
        1e-15,
    );
}

#[test]
fn compensated_accumulator_preserves_small_terms_between_large_cancellations() {
    // A naive loop gives (1e16+1)-1e16 == 0 in binary64.
    // Neumaier compensation preserves the +1 residual for both series.
    let terms = [1e16, 1.0, -1e16].map(|amplitude| LuniSolarHarmonicTerm {
        longitude_sin_rad: amplitude,
        obliquity_sin_rad: amplitude,
        ..empty_term([1, 0, 0, 0, 0])
    });
    let sum =
        evaluate_nutation_harmonics(tdb(0.0), [FRAC_PI_2, 0.0, 0.0, 0.0, 0.0], [0.0; 5], &terms)
            .unwrap();
    near(sum.longitude_rad, 1.0, 1e-15);
    near(sum.obliquity_rad, 1.0, 1e-15);
    assert_eq!(sum.terms_evaluated, 3);
}

#[test]
fn wrapping_a_fundamental_angle_does_not_spike_its_derivative() {
    let term = LuniSolarHarmonicTerm {
        longitude_sin_rad: 0.3,
        obliquity_cos_rad: 0.4,
        ..empty_term([1, 0, 0, 0, 0])
    };
    let baseline = evaluate_nutation_harmonics(tdb(0.0), [0.0; 5], [0.01; 5], &[term]).unwrap();
    let wrapped = evaluate_nutation_harmonics(
        tdb(0.0),
        [TAU - 1e-12, 0.0, 0.0, 0.0, 0.0],
        [0.01; 5],
        &[term],
    )
    .unwrap();
    near(
        baseline.longitude_rate_rad_per_tdb_day,
        wrapped.longitude_rate_rad_per_tdb_day,
        1e-13,
    );
    near(
        baseline.obliquity_rate_rad_per_tdb_day,
        wrapped.obliquity_rate_rad_per_tdb_day,
        1e-13,
    );
}

#[test]
fn accepts_full_length_synthetic_tables_without_claiming_iau_parity() {
    let zero_lunar = empty_term([0; 5]);
    let zero_planet = empty_term([0; 14]);
    let lunar = vec![zero_lunar; 678];
    let planetary = vec![zero_planet; 687];
    let a = evaluate_nutation_harmonics(tdb(0.0), [0.0; 5], [0.0; 5], &lunar).unwrap();
    let b = evaluate_nutation_harmonics(tdb(0.0), [0.0; 14], [0.0; 14], &planetary).unwrap();
    assert_eq!(a.terms_evaluated, 678);
    assert_eq!(b.terms_evaluated, 687);
    assert_eq!(a.longitude_rad, 0.0);
    assert_eq!(b.obliquity_rad, 0.0);
}

#[test]
fn rejects_missing_oversized_or_wrong_width_tables() {
    let lunar = empty_term([0; 5]);
    assert_eq!(
        evaluate_nutation_harmonics(tdb(0.0), [0.0; 5], [0.0; 5], &[]),
        Err(NutationHarmonicError::EmptyTerms),
    );
    assert_eq!(
        evaluate_nutation_harmonics(tdb(0.0), [0.0; 5], [0.0; 5], &vec![lunar; 679]),
        Err(NutationHarmonicError::TooManyTerms),
    );
    assert_eq!(
        evaluate_nutation_harmonics(
            tdb(0.0),
            [0.0; 14],
            [0.0; 14],
            &vec![empty_term([0; 14]); 688]
        ),
        Err(NutationHarmonicError::TooManyTerms),
    );
    assert_eq!(
        evaluate_nutation_harmonics(tdb(0.0), [0.0; 3], [0.0; 3], &[empty_term([0; 3])]),
        Err(NutationHarmonicError::UnsupportedArgumentCount),
    );
}

#[test]
fn rejects_wrong_scale_unsafe_dates_and_unvalidated_extrapolation() {
    let term = empty_term([0; 5]);
    for scale in [TimeScale::Tt, TimeScale::Tai, TimeScale::Ut1] {
        let date = JulianDate::new(2_451_545, 0.5, scale).unwrap();
        assert_eq!(
            evaluate_nutation_harmonics(date, [0.0; 5], [0.0; 5], &[term]),
            Err(NutationHarmonicError::WrongTimeScale),
        );
    }
    let huge = JulianDate::new(1_i64 << 52, 0.5, TimeScale::Tdb).unwrap();
    assert_eq!(
        evaluate_nutation_harmonics(huge, [0.0; 5], [0.0; 5], &[term]),
        Err(NutationHarmonicError::UnrepresentableEpoch),
    );
    for sign in [-1.0, 1.0] {
        let edge = tdb(sign * 73_050.0);
        assert!(evaluate_nutation_harmonics(edge, [0.0; 5], [0.0; 5], &[term]).is_ok());
        let outside = tdb(sign * 73_051.0);
        assert_eq!(
            evaluate_nutation_harmonics(outside, [0.0; 5], [0.0; 5], &[term]),
            Err(NutationHarmonicError::OutsideTestedInterval),
        );
    }
}

#[test]
fn rejects_nonfinite_angles_rates_coefficients_and_unnormalized_angles() {
    let term = empty_term([0; 5]);
    let mut invalid = [0.0; 5];
    invalid[3] = f64::NAN;
    assert_eq!(
        evaluate_nutation_harmonics(tdb(0.0), invalid, [0.0; 5], &[term]),
        Err(NutationHarmonicError::NonFiniteArgument),
    );
    invalid = [0.0; 5];
    invalid[1] = -0.1;
    assert_eq!(
        evaluate_nutation_harmonics(tdb(0.0), invalid, [0.0; 5], &[term]),
        Err(NutationHarmonicError::UnnormalizedArgument),
    );
    invalid[1] = TAU;
    assert_eq!(
        evaluate_nutation_harmonics(tdb(0.0), invalid, [0.0; 5], &[term]),
        Err(NutationHarmonicError::UnnormalizedArgument),
    );
    let mut rates = [0.0; 5];
    rates[0] = f64::INFINITY;
    assert_eq!(
        evaluate_nutation_harmonics(tdb(0.0), [0.0; 5], rates, &[term]),
        Err(NutationHarmonicError::NonFiniteArgument),
    );
    let bad_term = LuniSolarHarmonicTerm {
        obliquity_cos_rad: f64::NAN,
        ..term
    };
    assert_eq!(
        evaluate_nutation_harmonics(tdb(0.0), [0.0; 5], [0.0; 5], &[bad_term]),
        Err(NutationHarmonicError::NonFiniteCoefficient),
    );
}

#[test]
fn finite_coefficients_that_overflow_are_rejected_not_silently_nan() {
    let term = LuniSolarHarmonicTerm {
        longitude_sin_rad: f64::MAX,
        ..empty_term([1, 0, 0, 0, 0])
    };
    let result = evaluate_nutation_harmonics(
        tdb(0.0),
        [FRAC_PI_2, 0.0, 0.0, 0.0, 0.0],
        [0.0; 5],
        &[term, term],
    );
    assert_eq!(result, Err(NutationHarmonicError::ArithmeticOverflow));
}
