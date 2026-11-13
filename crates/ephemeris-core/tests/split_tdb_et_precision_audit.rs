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

//! Opt-in ET-to-split-JD roundtrip error audit, not Horizons validation.

use virgostars_ephemeris_core::calendar::{CalendarError, TimeScale};
use virgostars_ephemeris_core::time_split::{
    audited_split_julian_day_tdb_from_et, split_julian_day_tdb_from_et, SplitEpochAuditError,
};
use virgostars_ephemeris_core::TdbSecondsPastJ2000;

fn audited(seconds: f64, max_error: f64) -> Result<f64, SplitEpochAuditError> {
    let et = TdbSecondsPastJ2000::new(seconds).unwrap();
    audited_split_julian_day_tdb_from_et(et, max_error).map(|value| {
        assert_eq!(value.julian_date.scale(), TimeScale::Tdb);
        assert_eq!(
            value.julian_date,
            split_julian_day_tdb_from_et(et).unwrap()
        );
        value.roundtrip_error_seconds
    })
}

#[test]
fn exactly_representable_epoch_offsets_pass_zero_budget() {
    for seconds in [0.0, 43_200.0, -43_200.0, 86_400.0, -86_400.0] {
        assert_eq!(audited(seconds, 0.0), Ok(0.0));
    }
}

#[test]
fn picosecond_offset_is_detected_when_split_fraction_rounds_to_noon() {
    let epoch = TdbSecondsPastJ2000::new(1.0e-12).unwrap();
    let rounded = split_julian_day_tdb_from_et(epoch).unwrap();
    assert_eq!(rounded.midnight_fraction(), 0.5);
    assert!(matches!(
        audited(1.0e-12, 1.0e-13),
        Err(SplitEpochAuditError::PrecisionLoss {
            error_seconds,
            max_error_seconds
        }) if error_seconds == 1.0e-12 && max_error_seconds == 1.0e-13
    ));
    assert_eq!(audited(1.0e-12, 1.0e-11), Ok(1.0e-12));
}

#[test]
fn microsecond_offsets_are_within_nanosecond_roundtrip_budget() {
    for seconds in [1.0e-6, -1.0e-6, 43_200.0 + 1.0e-6, -43_200.0 - 1.0e-6] {
        let error = audited(seconds, 1.0e-9).unwrap();
        assert!(error < 1.0e-9);
    }
}

#[test]
fn rejects_invalid_precision_budgets_without_falling_back() {
    for budget in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
        assert_eq!(
            audited(0.0, budget),
            Err(SplitEpochAuditError::InvalidErrorBudget)
        );
    }
}

#[test]
fn propagates_the_split_converter_range_restriction() {
    assert_eq!(
        audited(f64::MAX, 0.0),
        Err(SplitEpochAuditError::InvalidJulianDate(
            CalendarError::YearOutOfRange
        ))
    );
}

#[test]
fn rejects_negative_subpicosecond_boundary_when_euclidean_remainder_is_unrepresentable() {
    assert_eq!(
        audited(-1.0e-12, 1.0),
        Err(SplitEpochAuditError::InvalidJulianDate(
            CalendarError::UnrepresentableTime
        ))
    );
}
