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

//! Independent ERFA UT1/ERA numerical references, UTC positive/negative
//! leap SI-second continuity and guarded EOP interpolation regression.
//! All EOP rows here are synthetic unless explicitly marked otherwise.
//! SOFA/ERFA numeric oracles: t_utcut1 and t_era00, not imported code.
//! Leap-step date 2016-12-31 is independently listed by NIST.

use std::f64::consts::TAU;

use virgostars_ephemeris_core::calendar::TimeScale;
use virgostars_ephemeris_core::earth_rotation::earth_rotation_angle;
use virgostars_ephemeris_core::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopCsvError, EopPolarQuality, EopPredictionPolicy,
};
use virgostars_ephemeris_core::eop_polar_orientation::approximate_itrs_to_tirs_from_eop_utc;
use virgostars_ephemeris_core::eop_ut1_era::{
    eop_ut1_era_from_utc_si_seconds, EopUt1EraError, EopUt1EraOrientation,
};

const HEADER: &str = "DATE,MJD,X,Y,UT1-UTC,LOD,DPSI,DEPS,DX,DY,DAT,DATA_TYPE";
const DAY: f64 = 86_400.0;

// Artificial EOP for 2006-01-15, chosen to match ERFA t_utcut1.
// Official ERFA oracle uses UTC JD=2453750.5+0.892100694 and DUT1=0.3341.
// DAT 33 at this epoch. These are *synthetic* EOP rows for this test.
const ERFA_UTC: &str = "2006-01-15,53750,0.01,0.02,0.3341,0,0,0,0,0,33,O";
const ERFA_UTC_NEXT: &str = "2006-01-16,53751,0.02,0.03,0.3341,0,0,0,0,0,33,O";

// Synthetic EOP chosen to put UT1 at published ERFA t_era00 epoch,
// UT1 JD = 2400000.5 + 54388.0.
const ERFA_ERA: &str = "2007-10-15,54388,0.01,0.02,0.0,0,0,0,0,0,33,O";
const ERFA_ERA_NEXT: &str = "2007-10-16,54389,0.01,0.02,0.0,0,0,0,0,0,33,O";

// DUT1 values here are synthetic but continuous *UT1−TAI* across
// historically genuine 2016-12-31 positive leap DAT 36→37.
const LEAP_PRE: &str = "2016-12-31,57753,0.10,0.20,-0.4,0,0,0,0,0,36,O";
const LEAP_POST: &str = "2017-01-01,57754,0.30,0.40,0.6,0,0,0,0,0,37,O";
const LEAP_POST_NEXT: &str = "2017-01-02,57755,0.30,0.40,0.6,0,0,0,0,0,37,O";

fn catalog(rows: &[&str]) -> CelestrakEopPolarCatalog {
    CelestrakEopPolarCatalog::parse_csv(
        "externally selected EOP fixture; no online source",
        &format!("{HEADER}\n{}\n", rows.join("\n")),
    )
    .unwrap()
}

fn sample(
    cat: &CelestrakEopPolarCatalog,
    day: i64,
    second: f64,
    policy: EopPredictionPolicy,
) -> Result<EopUt1EraOrientation, EopUt1EraError> {
    eop_ut1_era_from_utc_si_seconds(cat, day, second, policy)
}

fn observed(cat: &CelestrakEopPolarCatalog, day: i64, second: f64) -> EopUt1EraOrientation {
    sample(cat, day, second, EopPredictionPolicy::ObservedOnly).unwrap()
}

fn near(actual: f64, expected: f64, tol: f64) {
    assert!(
        (actual - expected).abs() <= tol,
        "expected {expected}, got {actual}, tolerance {tol}"
    );
}

fn elapsed_seconds_between(a: &EopUt1EraOrientation, b: &EopUt1EraOrientation) -> (f64, f64) {
    let diff = |a_day: i64, a_frac: f64, b_day: i64, b_frac: f64| {
        ((b_day - a_day) as f64 + (b_frac - a_frac)) * DAY
    };
    (
        diff(
            a.ut1_epoch.day_number(),
            a.ut1_epoch.midnight_fraction(),
            b.ut1_epoch.day_number(),
            b.ut1_epoch.midnight_fraction(),
        ),
        diff(
            a.tt_epoch.day_number(),
            a.tt_epoch.midnight_fraction(),
            b.tt_epoch.day_number(),
            b.tt_epoch.midnight_fraction(),
        ),
    )
}

#[test]
fn independent_erfa_utcut1_fixture_matches_split_jd_and_dut1() {
    // SOFA/ERFA t_utcut1:
    // eraUtcut1(2453750.5, 0.892100694, 0.3341)
    // produces UT1 second part = 0.8921045608981481481.
    let cat = catalog(&[ERFA_UTC, ERFA_UTC_NEXT]);
    let utc_second = 0.892_100_694 * DAY;
    let out = observed(&cat, 53_750, utc_second);
    assert_eq!(out.quality, EopPolarQuality::Observed);
    assert_eq!(out.utc_day_length_seconds, DAY);
    assert_eq!(out.tt_epoch.scale(), TimeScale::Tt);
    assert_eq!(out.ut1_epoch.scale(), TimeScale::Ut1);
    assert_eq!(out.ut1_epoch.day_number(), 2_453_751);
    near(
        out.ut1_epoch.midnight_fraction(),
        0.892_104_560_898_148_1,
        1.0e-12,
    );
    near(out.ut1_minus_tai_seconds, -32.6659, 1.0e-12);
    near(out.ut1_minus_utc_seconds, 0.3341, 1.0e-12);
    near(
        out.tt_epoch.midnight_fraction(),
        0.892_100_694 + 65.184 / DAY,
        1.0e-14,
    );
}

#[test]
fn independent_erfa_era00_fixture_survives_eop_to_ut1_chain() {
    // ERFA t_era00(2400000.5, 54388.0) expected
    // ERA = 0.4022837240028158102 radians.
    let cat = catalog(&[ERFA_ERA, ERFA_ERA_NEXT]);
    let out = observed(&cat, 54_388, 0.0);
    assert_eq!(out.ut1_epoch.day_number(), 2_454_389);
    assert_eq!(out.ut1_epoch.midnight_fraction(), 0.0);
    near(out.earth_rotation.angle_rad, 0.402_283_724_002_815_8, 1.0e-12);
    assert_eq!(out.earth_rotation, earth_rotation_angle(out.ut1_epoch).unwrap());
    assert!(out.earth_rotation.angle_rad >= 0.0 && out.earth_rotation.angle_rad < TAU);
}

#[test]
fn ordinary_day_interpolation_composes_parent_tt_and_polar_modules() {
    let cat = catalog(&[ERFA_UTC, ERFA_UTC_NEXT]);
    let half = observed(&cat, 53_750, 43_200.0);
    let old = approximate_itrs_to_tirs_from_eop_utc(
        &cat, 53_750, 0.5, EopPredictionPolicy::ObservedOnly,
    )
    .unwrap();
    assert_eq!(half.quality, old.quality);
    near(half.xp_rad, old.xp_rad, 0.0);
    near(half.yp_rad, old.yp_rad, 0.0);
    near(
        half.tt_epoch.midnight_fraction(),
        old.tt_epoch.midnight_fraction(),
        2.0e-16,
    );
    for (row_new, row_old) in half
        .approximate_polar
        .itrs_to_tirs
        .iter()
        .zip(old.approximate_polar.itrs_to_tirs)
    {
        for (value_new, value_old) in row_new.iter().zip(row_old) {
            near(*value_new, *value_old, 2.0e-16);
        }
    }
    near(half.ut1_minus_tai_seconds, -32.6659, 1.0e-12);
}

#[test]
fn positive_2016_leap_second_is_a_real_extra_si_second_without_ut1_jump() {
    let cat = catalog(&[LEAP_PRE, LEAP_POST, LEAP_POST_NEXT]);
    let before = observed(&cat, 57_753, 86_399.0);
    let leap = observed(&cat, 57_753, 86_400.0); // 23:59:60 UTC
    let following = observed(&cat, 57_754, 0.0); // 00:00:00 UTC
    assert_eq!(leap.utc_day_length_seconds, 86_401.0);
    assert_eq!(leap.tai_minus_utc_seconds_at_0h, 36);
    assert_eq!(following.tai_minus_utc_seconds_at_0h, 37);
    near(leap.ut1_minus_tai_seconds, -36.4, 1.0e-12);
    near(following.ut1_minus_tai_seconds, -36.4, 1.0e-12);
    for (a, b) in [(&before, &leap), (&leap, &following)] {
        let (ut1, tt) = elapsed_seconds_between(a, b);
        near(ut1, 1.0, 3.0e-11);
        near(tt, 1.0, 3.0e-11);
    }
    // No unrealistic 1-second-per-day drift of UT1−TAI.
    let midday = observed(&cat, 57_753, 43_200.0);
    near(midday.ut1_minus_tai_seconds, -36.4, 1.0e-12);
    near(midday.ut1_minus_utc_seconds, -0.4, 1.0e-12);
}

#[test]
fn positive_leap_day_end_and_negative_leap_day_bounds_are_explicit() {
    let cat = catalog(&[LEAP_PRE, LEAP_POST]);
    assert!(sample(&cat, 57_753, 86_400.999, EopPredictionPolicy::ObservedOnly).is_ok());
    assert_eq!(
        sample(&cat, 57_753, 86_401.0, EopPredictionPolicy::ObservedOnly),
        Err(EopUt1EraError::InvalidElapsedUtcSecond)
    );
    // A mathematically valid **synthetic negative leap**:
    // the closing 23:59:59 UTC minute never appears.
    let negative = catalog(&[
        "2016-12-31,57753,0.1,0.2,0.6,0,0,0,0,0,37,O",
        "2017-01-01,57754,0.1,0.2,-0.4,0,0,0,0,0,36,O",
    ]);
    let result = observed(&negative, 57_753, 86_398.5);
    assert_eq!(result.utc_day_length_seconds, 86_399.0);
    assert_eq!(
        sample(&negative, 57_753, 86_399.0, EopPredictionPolicy::ObservedOnly),
        Err(EopUt1EraError::InvalidElapsedUtcSecond)
    );
    near(result.ut1_minus_tai_seconds, -36.4, 1.0e-12);
}

#[test]
fn predictions_are_opt_in_and_the_quality_flag_is_not_hidden() {
    let predicted = ERFA_UTC_NEXT.replace(",33,O", ",33,P");
    let cat = catalog(&[ERFA_UTC, &predicted]);
    assert_eq!(
        sample(&cat, 53_750, 100.0, EopPredictionPolicy::ObservedOnly),
        Err(EopUt1EraError::Eop(EopCsvError::PredictionNotAuthorized))
    );
    let out = sample(&cat, 53_750, 100.0, EopPredictionPolicy::AllowPredicted).unwrap();
    assert_eq!(out.quality, EopPolarQuality::IncludesPrediction);
    assert_eq!(out.source_label, "externally selected EOP fixture; no online source");
}

#[test]
fn corrupted_eop_dat_and_dut1_provenance_fail_closed() {
    let big_jump = ERFA_UTC_NEXT.replace(",33,O", ",35,O");
    let cat = catalog(&[ERFA_UTC, &big_jump]);
    assert_eq!(
        sample(&cat, 53_750, 5.0, EopPredictionPolicy::ObservedOnly),
        Err(EopUt1EraError::InvalidLeapStep)
    );
    let bad_dat = ERFA_UTC_NEXT.replace(",33,O", ",0,O");
    let cat = catalog(&[ERFA_UTC, &bad_dat]);
    assert_eq!(
        sample(&cat, 53_750, 5.0, EopPredictionPolicy::ObservedOnly),
        Err(EopUt1EraError::UnsupportedTaiUtcOffset)
    );
    let invalid_dut1 = ERFA_UTC_NEXT.replace(",0.3341,", ",1.2,");
    let cat = catalog(&[ERFA_UTC, &invalid_dut1]);
    assert_eq!(
        sample(&cat, 53_750, 5.0, EopPredictionPolicy::ObservedOnly),
        Err(EopUt1EraError::InvalidDut1)
    );
    let excessive_drift = ERFA_UTC_NEXT.replace(",0.3341,", ",0.6000,");
    let cat = catalog(&[ERFA_UTC, &excessive_drift]);
    assert_eq!(
        sample(&cat, 53_750, 5.0, EopPredictionPolicy::ObservedOnly),
        Err(EopUt1EraError::ExcessiveUt1TaiChange)
    );
}

#[test]
fn rejects_missing_daily_eop_samples_and_unrepresentable_time() {
    let one_row = catalog(&[ERFA_UTC]);
    assert_eq!(
        sample(&one_row, 53_750, 0.0, EopPredictionPolicy::ObservedOnly),
        Err(EopUt1EraError::Eop(EopCsvError::MissingFollowingDay))
    );
    let separated = catalog(&[
        ERFA_UTC,
        "2006-01-17,53752,0.02,0.03,0.3341,0,0,0,0,0,33,O",
    ]);
    assert_eq!(
        sample(&separated, 53_750, 20.0, EopPredictionPolicy::ObservedOnly),
        Err(EopUt1EraError::Eop(EopCsvError::MissingFollowingDay))
    );
    let cat = catalog(&[ERFA_UTC, ERFA_UTC_NEXT]);
    for sec in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.001, 86_400.0] {
        assert_eq!(
            sample(&cat, 53_750, sec, EopPredictionPolicy::ObservedOnly),
            Err(EopUt1EraError::InvalidElapsedUtcSecond)
        );
    }
    assert_eq!(
        sample(&cat, 53_752, 0.0, EopPredictionPolicy::ObservedOnly),
        Err(EopUt1EraError::Eop(EopCsvError::DayOutsideCatalog))
    );
}

#[test]
fn pre_1972_utc_with_fractional_tai_offsets_is_not_invented() {
    let cat = catalog(&[
        "1971-12-31,41316,0.1,0.2,0.1,0,0,0,0,0,10,O",
        "1972-01-01,41317,0.1,0.2,0.1,0,0,0,0,0,10,O",
    ]);
    assert_eq!(
        sample(&cat, 41_316, 0.0, EopPredictionPolicy::ObservedOnly),
        Err(EopUt1EraError::BeforeModernUtc)
    );
}

#[test]
fn continuous_ut1_is_increasing_and_era_matches_independent_rotation_rate() {
    let cat = catalog(&[ERFA_UTC, ERFA_UTC_NEXT]);
    let before = observed(&cat, 53_750, 10.0);
    let after = observed(&cat, 53_750, 11.0);
    let (ut1, tt) = elapsed_seconds_between(&before, &after);
    near(ut1, 1.0, 2.0e-11);
    near(tt, 1.0, 2.0e-11);
    let delta = (after.earth_rotation.angle_rad - before.earth_rotation.angle_rad + TAU / 2.0)
        .rem_euclid(TAU)
        - TAU / 2.0;
    let predicted_delta = before.earth_rotation.rate_rad_per_ut1_day / DAY;
    near(delta, predicted_delta, 2.0e-14);
}
