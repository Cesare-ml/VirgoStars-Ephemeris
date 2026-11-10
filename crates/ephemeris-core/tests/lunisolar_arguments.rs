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

//! IAU 2000A lunar-solar argument regressions, from independent IERS and
//! ERFA numerical fixtures. The full 2000A nutation series is not included.

use std::f64::consts::{PI, TAU};

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::lunisolar_arguments::{
    lunisolar_fundamental_arguments, LuniSolarArguments, LuniSolarConvention, LuniSolarError, MOON_ASCENDING_NODE,
    MOON_ARGUMENT_OF_LATITUDE, MOON_MEAN_ANOMALY, MOON_SUN_ELONGATION, SUN_MEAN_ANOMALY,
};

fn tdb_jd(value: f64) -> JulianDate {
    JulianDate::from_f64(value, TimeScale::Tdb).unwrap()
}

fn iers_arguments(epoch: JulianDate) -> Result<LuniSolarArguments, LuniSolarError> {
    lunisolar_fundamental_arguments(epoch, LuniSolarConvention::Iers2010)
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.2e}"
    );
}

fn shortest_angle_difference(actual: f64, expected: f64) -> f64 {
    (actual - expected + PI).rem_euclid(TAU) - PI
}

#[test]
fn matches_all_five_independent_erfa_fundamental_argument_fixtures() {
    // Five independent numerical examples from ERFA t_erfa_c.c:
    // eraFal03/Falp03/Faf03/Fad03/Faom03 evaluated at T=0.8 Julian centuries.
    // The ERFA numeric tests supply the original MHB2000 fixed terms for
    // l' and D. IERS §5.7.2 explicitly notes their rounded c0 convention.
    // This test compares the same numeric T, NOT a TT-to-TDB conversion.
    let epoch = tdb_jd(2_451_545.0 + 0.8 * 36_525.0);
    let actual = iers_arguments(epoch).unwrap();
    let reference_radians = [
        5.132_369_751_108_684,
        6.226_797_973_505_507,
        0.259_771_136_674_550,
        1.946_709_205_396_926,
        -5.973_618_440_951_302,
    ];
    for (index, expected) in reference_radians.into_iter().enumerate() {
        let delta = shortest_angle_difference(actual.angles_rad[index], expected);
        near(delta, 0.0, 2e-12);
    }
    assert!(actual.angles_rad[MOON_ASCENDING_NODE] > 0.0);
}

#[test]
fn mhb2000_rounding_is_explicit_and_does_not_change_rates() {
    // IERS 2010 section 5.7.2: original MHB2000 IAU 2000A series uses
    // fixed l' and D offsets of +2 and -2 microarcseconds respectively.
    // ERFA standalone Falp03/Fad03 instead implement unrounded Eq. 5.43.
    let epoch = tdb_jd(2_451_545.0 + 0.8 * 36_525.0);
    let iers = iers_arguments(epoch).unwrap();
    let mhb = lunisolar_fundamental_arguments(epoch, LuniSolarConvention::Mhb2000).unwrap();
    let microarcsecond_radians = PI / (180.0 * 3_600.0) * 1e-6;
    for index in 0..5 {
        let expected_microarcseconds = if index == SUN_MEAN_ANOMALY {
            2.0
        } else if index == MOON_SUN_ELONGATION {
            -2.0
        } else {
            0.0
        };
        near(
            shortest_angle_difference(mhb.angles_rad[index], iers.angles_rad[index]),
            expected_microarcseconds * microarcsecond_radians,
            2e-15,
        );
        near(
            mhb.rates_rad_per_tdb_day[index],
            iers.rates_rad_per_tdb_day[index],
            1e-15,
        );
    }
}

#[test]
fn j2000_angles_and_rates_match_published_arcsecond_coefficients() {
    // The five c0/c1 pairs are the IERS/MHB2000 constants, not generated
    // by this repository. The J2000 epoch is noon in the TDB time scale.
    let reference = [
        (485_868.249_036, 1_717_915_923.217_8),
        (1_287_104.793_048, 129_596_581.048_1),
        (335_779.526_232, 1_739_527_262.847_8),
        (1_072_260.703_692, 1_602_961_601.209_0),
        (450_160.398_036, -6_962_890.543_1),
    ];
    let actual = iers_arguments(
        JulianDate::new(2_451_545, 0.5, TimeScale::Tdb).unwrap(),
    )
    .unwrap();
    let radians_per_arcsec = PI / (180.0 * 3_600.0);
    for (i, (angle, slope)) in reference.into_iter().enumerate() {
        near(actual.angles_rad[i], angle * radians_per_arcsec, 1e-14);
        near(
            actual.rates_rad_per_tdb_day[i],
            slope * radians_per_arcsec / 36_525.0,
            1e-14,
        );
    }
}

#[test]
fn analytic_rates_match_wrapped_centered_position_differences() {
    // The position reference is recomputed at displaced TDB dates and
    // compared via the shortest signed angle difference, not a derivative
    // obtained from the same Horner recurrence.
    let half_step_days = 0.5;
    for centuries in [-1.5, -0.7, 0.0, 0.8, 1.6] {
        let day = 2_451_545.0 + centuries * 36_525.0;
        let current = iers_arguments(tdb_jd(day)).unwrap();
        let before =
            iers_arguments(tdb_jd(day - half_step_days)).unwrap();
        let after =
            iers_arguments(tdb_jd(day + half_step_days)).unwrap();
        for i in 0..5 {
            let difference = shortest_angle_difference(
                after.angles_rad[i],
                before.angles_rad[i],
            );
            near(
                current.rates_rad_per_tdb_day[i],
                difference / (2.0 * half_step_days),
                2e-10,
            );
        }
    }
}

#[test]
fn argument_order_and_phase_multiplier_convention_are_stable() {
    let a = iers_arguments(tdb_jd(2_451_545.0)).unwrap();
    assert_eq!(MOON_MEAN_ANOMALY, 0);
    assert_eq!(SUN_MEAN_ANOMALY, 1);
    assert_eq!(MOON_ARGUMENT_OF_LATITUDE, 2);
    assert_eq!(MOON_SUN_ELONGATION, 3);
    assert_eq!(MOON_ASCENDING_NODE, 4);
    assert_eq!(a.phase([0; 5]), (0.0, 0.0));

    // A future lunar-solar series term has integer multipliers for
    // [l,l',F,D,Omega], and the phase speed is the matching linear sum.
    let coefficients = [2, -1, 1, -2, 1];
    let (phase, rate) = a.phase(coefficients);
    let expected_angle = (2.0 * a.angles_rad[0] - a.angles_rad[1]
        + a.angles_rad[2] - 2.0 * a.angles_rad[3]
        + a.angles_rad[4]).rem_euclid(TAU);
    let expected_rate = 2.0 * a.rates_rad_per_tdb_day[0]
        - a.rates_rad_per_tdb_day[1]
        + a.rates_rad_per_tdb_day[2]
        - 2.0 * a.rates_rad_per_tdb_day[3]
        + a.rates_rad_per_tdb_day[4];
    near(phase, expected_angle, 1e-15);
    near(rate, expected_rate, 1e-14);
}

#[test]
fn all_angles_are_normalized_but_rates_keep_their_physical_sign() {
    for centuries in [-2.0, -0.1, 0.0, 0.8, 2.0] {
        let a = iers_arguments(tdb_jd(
            2_451_545.0 + centuries * 36_525.0,
        ))
        .unwrap();
        for angle in a.angles_rad {
            assert!((0.0..TAU).contains(&angle), "unwrapped angle {angle}");
        }
        for rate in a.rates_rad_per_tdb_day {
            assert!(rate.is_finite());
        }
        assert!(a.rates_rad_per_tdb_day[MOON_ASCENDING_NODE] < 0.0);
        assert!(a.rates_rad_per_tdb_day[MOON_MEAN_ANOMALY] > 0.0);
    }
}

#[test]
fn rejects_tdb_mislabelling_and_unrepresentable_split_julian_dates() {
    for scale in [TimeScale::Tt, TimeScale::Tai, TimeScale::Ut1] {
        let epoch = JulianDate::new(2_451_545, 0.5, scale).unwrap();
        assert_eq!(
            iers_arguments(epoch),
            Err(LuniSolarError::WrongTimeScale),
        );
    }
    let unrepresentable =
        JulianDate::new(1_i64 << 52, 0.5, TimeScale::Tdb).unwrap();
    assert_eq!(
        iers_arguments(unrepresentable),
        Err(LuniSolarError::UnrepresentableEpoch),
    );
}

#[test]
fn bounded_epoch_policy_is_explicit_not_a_claim_of_iau_validity() {
    let year_centuries = 73_050.0;
    for direction in [-1.0, 1.0] {
        let edge = 2_451_545.0 + direction * year_centuries;
        assert!(iers_arguments(tdb_jd(edge)).is_ok());
        assert_eq!(
            iers_arguments(tdb_jd(edge + direction)),
            Err(LuniSolarError::OutsideTestedInterval),
        );
    }
}
