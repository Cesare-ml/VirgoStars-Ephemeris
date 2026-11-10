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

//! Independent IERS/ERFA planetary-nutation argument regressions.
//! No periodic nutation series or actual planetary positions are evaluated.

use std::f64::consts::{PI, TAU};

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::planetary_arguments::{
    planetary_fundamental_arguments, PlanetaryArgumentConvention, PlanetaryArgumentError,
    EARTH_MEAN_LONGITUDE, GENERAL_PRECESSION_IN_LONGITUDE, JUPITER_MEAN_LONGITUDE,
    MARS_MEAN_LONGITUDE, MERCURY_MEAN_LONGITUDE, NEPTUNE_MEAN_LONGITUDE, SATURN_MEAN_LONGITUDE,
    URANUS_MEAN_LONGITUDE, VENUS_MEAN_LONGITUDE,
};

fn tdb_jd(value: f64) -> JulianDate {
    JulianDate::from_f64(value, TimeScale::Tdb).unwrap()
}

fn iers(
    epoch: JulianDate,
) -> Result<
    virgostars_ephemeris_core::planetary_arguments::PlanetaryArguments,
    PlanetaryArgumentError,
> {
    planetary_fundamental_arguments(epoch, PlanetaryArgumentConvention::Iers2010)
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}",
    );
}

fn shortest_angle_delta(actual: f64, expected: f64) -> f64 {
    (actual - expected + PI).rem_euclid(TAU) - PI
}

#[test]
fn all_nine_fundamental_arguments_match_independent_erfa_fixtures() {
    // Published independent numeric examples in the ERFA C regression suite:
    // eraFame03, Fave03, Fae03, Fama03, Faju03, Fasa03, Faur03,
    // Fane03 and Fapa03 at T = 0.8 Julian centuries from J2000.
    // https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c
    // These test the *standalone IERS* functions, not the historically
    // different Neptune argument used by the MHB2000 periodic series.
    let actual = iers(tdb_jd(2_451_545.0 + 0.8 * 36_525.0)).unwrap();
    let expected = [
        5.417_338_184_297_29,
        3.424_900_460_533_758,
        1.744_713_738_913_082,
        3.275_506_840_277_781_5,
        5.275_711_665_202_481,
        5.371_574_539_440_827,
        5.180_636_450_180_414,
        2.079_343_830_860_413_5,
        0.019_508_847_622_4,
    ];
    for (index, reference) in expected.into_iter().enumerate() {
        near(
            shortest_angle_delta(actual.angles_rad[index], reference),
            0.0,
            1e-12,
        );
    }
}

#[test]
fn j2000_iers_coefficients_and_precession_rate_are_exactly_grounded() {
    // IERS 2010 Eq 5.44: constant and linear terms (radians, rad/century),
    // independent of the Rust constants under test.
    let published = [
        (4.402_608_842, 2_608.790_314_157_4),
        (3.176_146_697, 1_021.328_554_621_1),
        (1.753_470_314, 628.307_584_999_1),
        (6.203_480_913, 334.061_242_670_0),
        (0.599_546_497, 52.969_096_264_1),
        (0.874_016_757, 21.329_910_496_0),
        (5.481_293_872, 7.478_159_856_7),
        (5.311_886_287, 3.813_303_563_8),
    ];
    let result = iers(tdb_jd(2_451_545.0)).unwrap();
    for (index, (initial, slope)) in published.into_iter().enumerate() {
        near(result.angles_rad[index], initial, 1e-15);
        near(result.rates_rad_per_tdb_day[index], slope / 36_525.0, 1e-15);
    }
    near(
        result.angles_rad[GENERAL_PRECESSION_IN_LONGITUDE],
        0.0,
        1e-16,
    );
    near(
        result.rates_rad_per_tdb_day[GENERAL_PRECESSION_IN_LONGITUDE],
        0.024_381_75 / 36_525.0,
        1e-16,
    );
}

#[test]
fn mhb2000_neptune_is_explicitly_distinct_from_iers_fane03() {
    // IERS text after Eq 5.44: historical planetary terms use
    // 5.321159000+3.812777400*T rather than
    // 5.311886287+3.8133035638*T for Neptune.
    let centuries = 0.8;
    let date = tdb_jd(2_451_545.0 + centuries * 36_525.0);
    let iers_angles = iers(date).unwrap();
    let mhb = planetary_fundamental_arguments(date, PlanetaryArgumentConvention::Mhb2000).unwrap();
    let expected_mhb = (5.321_159 + 3.812_777_4 * centuries).rem_euclid(TAU);
    near(mhb.angles_rad[NEPTUNE_MEAN_LONGITUDE], expected_mhb, 1e-14);
    near(
        mhb.rates_rad_per_tdb_day[NEPTUNE_MEAN_LONGITUDE],
        3.812_777_4 / 36_525.0,
        1e-16,
    );
    assert!(
        shortest_angle_delta(
            mhb.angles_rad[NEPTUNE_MEAN_LONGITUDE],
            iers_angles.angles_rad[NEPTUNE_MEAN_LONGITUDE],
        )
        .abs()
            > 0.008,
    );
    for index in 0..9 {
        if index != NEPTUNE_MEAN_LONGITUDE {
            assert_eq!(mhb.angles_rad[index], iers_angles.angles_rad[index]);
            assert_eq!(
                mhb.rates_rad_per_tdb_day[index],
                iers_angles.rates_rad_per_tdb_day[index],
            );
        }
    }
}

#[test]
fn analytic_day_rates_match_independently_evaluated_angles_at_five_dates() {
    for centuries in [-1.6, -0.7, 0.0, 0.8, 1.6] {
        let epoch = 2_451_545.0 + centuries * 36_525.0;
        let current = iers(tdb_jd(epoch)).unwrap();
        let before = iers(tdb_jd(epoch - 0.5)).unwrap();
        let after = iers(tdb_jd(epoch + 0.5)).unwrap();
        for i in 0..9 {
            let measured = shortest_angle_delta(after.angles_rad[i], before.angles_rad[i]);
            near(current.rates_rad_per_tdb_day[i], measured, 2e-11);
        }
    }
}

#[test]
fn signed_phase_rate_matches_independent_nine_term_expression() {
    let a = iers(tdb_jd(2_451_545.0 + 37_000.0)).unwrap();
    let coeff: [i8; 9] = [2, -1, 3, 0, 1, -2, 0, 1, -1];
    let (phase, rate) = a.phase(coeff);
    let p = (2.0 * a.angles_rad[0] - a.angles_rad[1] + 3.0 * a.angles_rad[2] + a.angles_rad[4]
        - 2.0 * a.angles_rad[5]
        + a.angles_rad[7]
        - a.angles_rad[8])
        .rem_euclid(TAU);
    let v = 2.0 * a.rates_rad_per_tdb_day[0] - a.rates_rad_per_tdb_day[1]
        + 3.0 * a.rates_rad_per_tdb_day[2]
        + a.rates_rad_per_tdb_day[4]
        - 2.0 * a.rates_rad_per_tdb_day[5]
        + a.rates_rad_per_tdb_day[7]
        - a.rates_rad_per_tdb_day[8];
    near(phase, p, 1e-14);
    near(rate, v, 1e-15);
    assert_eq!(a.phase([0; 9]), (0.0, 0.0));
}

#[test]
fn indices_and_angle_ranges_are_stable_with_signed_precession_rate() {
    let expected = [
        MERCURY_MEAN_LONGITUDE,
        VENUS_MEAN_LONGITUDE,
        EARTH_MEAN_LONGITUDE,
        MARS_MEAN_LONGITUDE,
        JUPITER_MEAN_LONGITUDE,
        SATURN_MEAN_LONGITUDE,
        URANUS_MEAN_LONGITUDE,
        NEPTUNE_MEAN_LONGITUDE,
        GENERAL_PRECESSION_IN_LONGITUDE,
    ];
    assert_eq!(expected, [0, 1, 2, 3, 4, 5, 6, 7, 8]);
    for centuries in [-2.0, -0.75, 0.0, 0.8, 2.0] {
        let result = iers(tdb_jd(2_451_545.0 + centuries * 36_525.0)).unwrap();
        for rad in result.angles_rad {
            assert!((0.0..TAU).contains(&rad));
        }
        for rate in result.rates_rad_per_tdb_day {
            assert!(rate.is_finite() && rate > 0.0);
        }
    }
}

#[test]
fn split_julian_date_detects_fractional_second_motion() {
    let start = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap();
    let later = JulianDate::new(2_451_545, 0.5 + 0.25 / 86_400.0, TimeScale::Tdb).unwrap();
    let a = iers(start).unwrap();
    let b = iers(later).unwrap();
    let elapsed = 0.25 / 86_400.0;
    near(
        shortest_angle_delta(
            b.angles_rad[MERCURY_MEAN_LONGITUDE],
            a.angles_rad[MERCURY_MEAN_LONGITUDE],
        ),
        a.rates_rad_per_tdb_day[MERCURY_MEAN_LONGITUDE] * elapsed,
        1e-14,
    );
}

#[test]
fn rejects_wrong_time_scales_unrepresentable_epochs_and_extrapolation() {
    for scale in [TimeScale::Tt, TimeScale::Tai, TimeScale::Ut1] {
        let epoch = JulianDate::new(2_451_545, 0.5, scale).unwrap();
        assert_eq!(iers(epoch), Err(PlanetaryArgumentError::WrongTimeScale),);
    }
    let huge = JulianDate::new(1_i64 << 52, 0.5, TimeScale::Tdb).unwrap();
    assert_eq!(
        iers(huge),
        Err(PlanetaryArgumentError::UnrepresentableEpoch),
    );
    for sign in [-1.0, 1.0] {
        let boundary = 2_451_545.0 + sign * 73_050.0;
        assert!(iers(tdb_jd(boundary)).is_ok());
        assert_eq!(
            iers(tdb_jd(boundary + sign)),
            Err(PlanetaryArgumentError::OutsideTestedInterval),
        );
    }
}
