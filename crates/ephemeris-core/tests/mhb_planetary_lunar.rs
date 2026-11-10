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

//! Synthetic and independently calculated decimal scientific fixtures for
//! the original four historical MHB2000 planetary lunar arguments.
//! No full nutation coefficient table or astronomical oracle is bundled.

use std::f64::consts::{PI, TAU};

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::mhb_planetary_lunar::{
    mhb2000_planetary_lunar_arguments, MhbPlanetaryLunarError,
    MOON_ARGUMENT_OF_LATITUDE, MOON_ASCENDING_NODE, MOON_MEAN_ANOMALY, MOON_SUN_ELONGATION,
};

fn at(day_offset: f64) -> JulianDate {
    // Keep fractional-day precision when finite-differencing speeds:
    // collapsing JD 2451545 plus a small offset first would quantize time.
    let whole_days = day_offset.floor();
    let fractional_day = day_offset - whole_days;
    if fractional_day < 0.5 {
        JulianDate::new(
            2_451_545 + whole_days as i64,
            0.5 + fractional_day,
            TimeScale::Tdb,
        )
        .unwrap()
    } else {
        JulianDate::new(
            2_451_546 + whole_days as i64,
            fractional_day - 0.5,
            TimeScale::Tdb,
        )
        .unwrap()
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}"
    );
}

fn angle_difference(a: f64, b: f64) -> f64 {
    (a - b + PI).rem_euclid(TAU) - PI
}

#[test]
fn published_mhb2000_j2000_constants_and_signed_slopes_are_preserved() {
    // Historical MHB2000 planetary-series scientific constants (rad and
    // rad/TDB century). The test anchors the four-term convention:
    // [l,F,D,Omega], without an l-prime slot.
    let expected = [
        (2.355_555_98, 8_328.691_426_955_4),
        (1.627_905_234, 8_433.466_158_131),
        (5.198_466_741, 7_771.377_146_812_1),
        (2.182_439_20, -33.757_045),
    ];
    let actual = mhb2000_planetary_lunar_arguments(at(0.0)).unwrap();
    for (i, (initial, slope)) in expected.into_iter().enumerate() {
        near(actual.angles_rad[i], initial, 1e-15);
        near(actual.rates_rad_per_tdb_day[i], slope / 36_525.0, 1e-15);
    }
    assert_eq!(
        [
            MOON_MEAN_ANOMALY,
            MOON_ARGUMENT_OF_LATITUDE,
            MOON_SUN_ELONGATION,
            MOON_ASCENDING_NODE
        ],
        [0, 1, 2, 3]
    );
}

#[test]
fn independent_high_precision_decimal_reference_at_t_point_eight() {
    // Numeric references calculated separately at T=0.8 using 70-digit
    // decimal precision and 2*pi reduction, not from this Rust evaluator.
    // These are independent *mathematical* checks of the four published
    // linear polynomials, not an external full IAU Nut00a test vector.
    let expected = [
        5.132_271_933_958_334,
        0.259_811_827_924_124,
        1.946_730_082_889_388,
        0.309_544_428_718_346,
    ];
    let actual = mhb2000_planetary_lunar_arguments(at(0.8 * 36_525.0)).unwrap();
    for (index, reference) in expected.into_iter().enumerate() {
        near(angle_difference(actual.angles_rad[index], reference), 0.0, 2e-12);
    }
}

#[test]
fn simplified_lunar_anomaly_is_not_the_full_lunisolar_iau_polynomial() {
    // The standalone IAU full Delaunay l reference at numeric T=0.8,
    // recorded by independent ERFA Fal03 tests, is 5.132369751108684.
    let simplified = mhb2000_planetary_lunar_arguments(at(0.8 * 36_525.0)).unwrap();
    let full_iau_l = 5.132_369_751_108_684;
    assert!(angle_difference(simplified.angles_rad[0], full_iau_l).abs() > 5e-5);
}

#[test]
fn exact_rates_match_finite_differences_across_multiple_dates() {
    let h = 0.1;
    for centuries in [-1.6, -0.7, 0.0, 0.8, 1.6] {
        let day = centuries * 36_525.0;
        let now = mhb2000_planetary_lunar_arguments(at(day)).unwrap();
        let earlier = mhb2000_planetary_lunar_arguments(at(day - h)).unwrap();
        let later = mhb2000_planetary_lunar_arguments(at(day + h)).unwrap();
        for i in 0..4 {
            let numerical = angle_difference(later.angles_rad[i], earlier.angles_rad[i]) / (2.0 * h);
            near(now.rates_rad_per_tdb_day[i], numerical, 4e-11);
        }
    }
}

#[test]
fn phase_combination_and_signs_are_explicit() {
    let args = mhb2000_planetary_lunar_arguments(at(500.0)).unwrap();
    let (angle, rate) = args.phase([2, -1, 0, 3]);
    let expected_angle = (2.0 * args.angles_rad[0] - args.angles_rad[1]
        + 3.0 * args.angles_rad[3]).rem_euclid(TAU);
    let expected_rate = 2.0 * args.rates_rad_per_tdb_day[0] - args.rates_rad_per_tdb_day[1]
        + 3.0 * args.rates_rad_per_tdb_day[3];
    near(angle, expected_angle, 1e-14);
    near(rate, expected_rate, 1e-15);
    assert_eq!(args.phase([0; 4]), (0.0, 0.0));
    assert!(args.rates_rad_per_tdb_day[MOON_ASCENDING_NODE] < 0.0);
}

#[test]
fn can_combine_four_lunar_and_nine_planetary_terms_with_zero_solar_coefficient() {
    // This independent test checks only numeric ordering: later code
    // supplies F6-F14 from the separate planetary-argument draft PR.
    let args = mhb2000_planetary_lunar_arguments(at(0.0)).unwrap();
    let mut multipliers = [0_i8; 14];
    multipliers[0] = 2;
    multipliers[2] = -1;
    multipliers[4] = 3;
    multipliers[5] = 1;
    multipliers[13] = -2;
    let mut planetary = [0.0; 9];
    planetary[0] = PI / 5.0;
    planetary[8] = PI / 7.0;
    let mut planetary_rates = [0.0; 9];
    planetary_rates[0] = 0.04;
    planetary_rates[8] = -0.002;
    let (phase, speed) =
        args.phase_with_planets(multipliers, planetary, planetary_rates).unwrap();
    near(
        phase,
        (2.0 * args.angles_rad[0] - args.angles_rad[1]
            + 3.0 * args.angles_rad[3] + planetary[0]
            - 2.0 * planetary[8]).rem_euclid(TAU),
        1e-14,
    );
    near(
        speed,
        2.0 * args.rates_rad_per_tdb_day[0] - args.rates_rad_per_tdb_day[1]
            + 3.0 * args.rates_rad_per_tdb_day[3]
            + 0.04 - 2.0 * (-0.002),
        1e-14,
    );
}

#[test]
fn any_solar_anomaly_multiplier_is_rejected_rather_than_silently_ignored() {
    let args = mhb2000_planetary_lunar_arguments(at(0.0)).unwrap();
    let mut multipliers = [0_i8; 14];
    multipliers[1] = 1;
    assert_eq!(
        args.phase_with_planets(multipliers, [0.0; 9], [0.0; 9]),
        Err(MhbPlanetaryLunarError::UnsupportedSolarAnomalyMultiplier),
    );
    multipliers[1] = -1;
    assert_eq!(
        args.phase_with_planets(multipliers, [0.0; 9], [0.0; 9]),
        Err(MhbPlanetaryLunarError::UnsupportedSolarAnomalyMultiplier),
    );
}

#[test]
fn invalid_external_planetary_phase_or_rate_is_not_accepted() {
    let args = mhb2000_planetary_lunar_arguments(at(0.0)).unwrap();
    let mut planetary = [0.0; 9];
    planetary[1] = f64::NAN;
    assert_eq!(
        args.phase_with_planets([0; 14], planetary, [0.0; 9]),
        Err(MhbPlanetaryLunarError::NonFinitePlanetaryArgument),
    );
    planetary[1] = -0.1;
    assert_eq!(
        args.phase_with_planets([0; 14], planetary, [0.0; 9]),
        Err(MhbPlanetaryLunarError::UnnormalizedPlanetaryArgument),
    );
    planetary[1] = TAU;
    assert_eq!(
        args.phase_with_planets([0; 14], planetary, [0.0; 9]),
        Err(MhbPlanetaryLunarError::UnnormalizedPlanetaryArgument),
    );
    let mut rates = [0.0; 9];
    planetary[1] = 0.0;
    rates[2] = f64::INFINITY;
    assert_eq!(
        args.phase_with_planets([0; 14], planetary, rates),
        Err(MhbPlanetaryLunarError::NonFinitePlanetaryArgument),
    );
}

#[test]
fn normalized_angles_have_continuous_signed_rates_across_wrap() {
    for centuries in [-2.0, -0.1, 0.0, 0.8, 2.0] {
        let args = mhb2000_planetary_lunar_arguments(at(centuries * 36_525.0)).unwrap();
        for angle in args.angles_rad {
            assert!((0.0..TAU).contains(&angle));
        }
        for rate in args.rates_rad_per_tdb_day {
            assert!(rate.is_finite());
        }
    }
    // The l argument crosses the 2pi boundary near this T; its true
    // derivative must not exhibit an artificial jump at wrap.
    let cross_days = (TAU - 2.355_555_98) / 8_328.691_426_955_4 * 36_525.0;
    let earlier = mhb2000_planetary_lunar_arguments(at(cross_days - 0.0001)).unwrap();
    let later = mhb2000_planetary_lunar_arguments(at(cross_days + 0.0001)).unwrap();
    let numerical = angle_difference(later.angles_rad[0], earlier.angles_rad[0]) / 0.0002;
    near(earlier.rates_rad_per_tdb_day[0], numerical, 2e-9);
}

#[test]
fn strict_tdb_epoch_and_conservative_range_are_enforced() {
    for scale in [TimeScale::Tt, TimeScale::Tai, TimeScale::Ut1] {
        assert_eq!(
            mhb2000_planetary_lunar_arguments(JulianDate::new(2_451_545, 0.5, scale).unwrap()),
            Err(MhbPlanetaryLunarError::WrongTimeScale),
        );
    }
    assert_eq!(
        mhb2000_planetary_lunar_arguments(
            JulianDate::new(1_i64 << 52, 0.5, TimeScale::Tdb).unwrap(),
        ),
        Err(MhbPlanetaryLunarError::UnrepresentableEpoch),
    );
    for direction in [-1.0, 1.0] {
        assert!(mhb2000_planetary_lunar_arguments(at(direction * 73_050.0)).is_ok());
        assert_eq!(
            mhb2000_planetary_lunar_arguments(at(direction * 73_051.0)),
            Err(MhbPlanetaryLunarError::OutsideTestedInterval),
        );
    }
}
