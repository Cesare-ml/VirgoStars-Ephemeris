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

//! Deterministic arithmetic contract for split JD(TDB) from ET seconds.
//! These do not validate a NASA/JPL kernel or astronomical corrections.

use virgostars_ephemeris_core::calendar::{CalendarError, JulianDate, TimeScale};
use virgostars_ephemeris_core::time_split::split_julian_day_tdb_from_et;
use virgostars_ephemeris_core::{TdbSecondsPastJ2000, J2000_JD_TDB, SECONDS_PER_DAY};

fn convert(seconds: f64) -> Result<JulianDate, CalendarError> {
    split_julian_day_tdb_from_et(TdbSecondsPastJ2000::new(seconds).unwrap())
}

fn recovered_seconds(split: JulianDate) -> f64 {
    (split.day_number() - 2_451_545) as f64 * SECONDS_PER_DAY
        + (split.midnight_fraction() - 0.5) * SECONDS_PER_DAY
}

#[test]
fn noon_midnight_and_whole_day_boundaries_are_exact() {
    for (seconds, day, fraction) in [
        (0.0, 2_451_545, 0.5),
        (43_200.0, 2_451_546, 0.0),
        (-43_200.0, 2_451_545, 0.0),
        (86_400.0, 2_451_546, 0.5),
        (-86_400.0, 2_451_544, 0.5),
    ] {
        let jd = convert(seconds).unwrap();
        assert_eq!(jd.day_number(), day);
        assert_eq!(jd.midnight_fraction(), fraction);
        assert_eq!(jd.scale(), TimeScale::Tdb);
        assert_eq!(recovered_seconds(jd), seconds);
    }
}

#[test]
fn microseconds_survive_when_single_f64_jd_would_round_them_away() {
    for seconds in [1.0e-6, -1.0e-6] {
        let jd = convert(seconds).unwrap();
        assert!((recovered_seconds(jd) - seconds).abs() < 1.0e-9);
        assert_eq!(jd.day_number(), 2_451_545);
        // The midpoint-like single-f64 JD representation loses this offset.
        let (whole, fraction) = jd.parts().unwrap();
        assert_eq!(whole + fraction, J2000_JD_TDB);
        assert_ne!(jd.midnight_fraction(), 0.5);
    }
}

#[test]
fn microseconds_on_both_sides_of_midnight_choose_correct_days() {
    for (seconds, expected_day) in [
        (43_200.0 - 1.0e-6, 2_451_545),
        (43_200.0 + 1.0e-6, 2_451_546),
        (-43_200.0 - 1.0e-6, 2_451_544),
        (-43_200.0 + 1.0e-6, 2_451_545),
    ] {
        let jd = convert(seconds).unwrap();
        assert_eq!(jd.day_number(), expected_day);
        assert!((recovered_seconds(jd) - seconds).abs() < 1.0e-9);
    }
}

#[test]
fn positive_and_negative_multi_year_offsets_retain_second_precision() {
    for seconds in [
        345_678_912.125,
        -345_678_912.125,
        100_000_000_000.0,
        -100_000_000_000.0,
    ] {
        let jd = convert(seconds).unwrap();
        assert!((recovered_seconds(jd) - seconds).abs() < 1.0e-5);
    }
}

#[test]
fn refuses_epochs_beyond_supported_exact_integer_split_range() {
    for seconds in [f64::MAX, -f64::MAX] {
        assert_eq!(convert(seconds), Err(CalendarError::YearOutOfRange));
    }
    assert!(TdbSecondsPastJ2000::new(f64::NAN).is_err());
    assert!(TdbSecondsPastJ2000::new(f64::INFINITY).is_err());
}
