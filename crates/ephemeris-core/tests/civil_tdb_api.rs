/* EXHIBIT A. Common Public Attribution License Version 1.0.
The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

//! Civil UTC-offset to approximate TDB Julian-date adapter contracts.
//! This is a bounded time-scale regression, not scientific DE441 parity.

use virgostars_ephemeris_core::first_sky::{CivilTimestamp, SkyError};

fn tdb(timestamp: &str) -> f64 {
    CivilTimestamp::parse(timestamp)
        .unwrap()
        .to_approximate_tdb_julian_date()
        .unwrap()
        .0
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.12}, expected={expected:.12}, tolerance={tolerance:.3e}"
    );
}

#[test]
fn offsets_describing_the_same_utc_instant_yield_the_same_tdb_epoch() {
    let utc = tdb("2026-11-10T12:30:00Z");
    near(utc, tdb("2026-11-10T13:30:00+01:00"), 1.0e-10);
    near(utc, tdb("2026-11-10T07:00:00-05:30"), 1.0e-10);
    near(utc, tdb("2026-11-11T02:30:00+14:00"), 1.0e-10);
}

#[test]
fn modern_reference_uses_utc_to_tai_to_tt_plus_small_tdb_minus_tt() {
    // 2017-01-01 00:00:00 UTC has JD 2457754.5.
    // TAI-UTC=37 seconds and TT-TAI=32.184 seconds.
    // The truncated TDB-TT correction is below 3 milliseconds.
    // This is a time-scale anchor, not an external TDB precision oracle.
    let jd_tdb = tdb("2017-01-01T00:00:00Z");
    let expected_tt = 2_457_754.5 + (37.0 + 32.184) / 86_400.0;
    near(jd_tdb, expected_tt, 0.003 / 86_400.0);
    assert!(jd_tdb > 2_457_754.5);
}

#[test]
fn utc_leap_offset_transition_is_applied_on_utc_not_local_date() {
    // The nominal civil-clock interval is 3600 s; TAI/TT/TDB interval
    // spans 3601 s because the IERS TAI-UTC step occurs at midnight.
    let before = tdb("2016-12-31T23:30:00Z");
    let after = tdb("2017-01-01T00:30:00Z");
    near((after - before) * 86_400.0, 3_601.0, 0.1);

    near(before, tdb("2017-01-01T00:30:00+01:00"), 1.0e-10);
    near(after, tdb("2016-12-31T23:30:00-01:00"), 1.0e-10);
}

#[test]
fn utc_bounds_use_normalized_instant_not_local_calendar_label() {
    assert!(tdb("1972-01-01T00:00:00Z").is_finite());
    assert!(tdb("1971-12-31T23:00:00-01:00").is_finite());
    assert!(tdb("2027-06-27T23:59:59Z").is_finite());
    assert!(tdb("2027-06-28T13:59:59+14:00").is_finite());

    for invalid in [
        "1971-12-31T23:59:59Z",
        "1972-01-01T00:00:00+01:00",
        "2027-06-28T00:00:00Z",
        "2027-06-27T10:00:00-14:00",
    ] {
        let error = CivilTimestamp::parse(invalid)
            .unwrap()
            .to_approximate_tdb_julian_date()
            .unwrap_err();
        assert!(
            matches!(error, SkyError::UnsupportedUtc),
            "unexpected error for {invalid}: {error}"
        );
    }
}

#[test]
fn leap_second_labels_are_not_silently_interpreted_as_ordinary_seconds() {
    assert!(CivilTimestamp::parse("2016-12-31T23:59:60Z").is_err());
}
