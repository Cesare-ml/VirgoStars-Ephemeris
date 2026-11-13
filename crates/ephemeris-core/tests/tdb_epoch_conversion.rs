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

//! Public API checks for TDB epoch representation conversion.
//! These are binary64 arithmetic tests, not NASA/JPL astronomical validation.

use virgostars_ephemeris_core::{
    JulianDayTdb, TdbSecondsPastJ2000, J2000_JD_TDB, SECONDS_PER_DAY,
};

#[test]
fn exact_day_and_half_day_offsets_round_trip() {
    for seconds in [-SECONDS_PER_DAY, -43_200.0, 0.0, 43_200.0, SECONDS_PER_DAY] {
        let et = TdbSecondsPastJ2000::new(seconds).unwrap();
        let julian_day = et.to_julian_day_tdb().unwrap();
        assert_eq!(julian_day.value(), J2000_JD_TDB + seconds / SECONDS_PER_DAY);
        assert_eq!(
            julian_day.to_tdb_seconds_past_j2000().unwrap().value(),
            seconds
        );
    }
}

#[test]
fn fractional_day_conversion_is_explicitly_lossy_at_modern_epochs() {
    for seconds in [-1.0, 1.0] {
        let et = TdbSecondsPastJ2000::new(seconds).unwrap();
        let recovered = et
            .to_julian_day_tdb()
            .unwrap()
            .to_tdb_seconds_past_j2000()
            .unwrap()
            .value();
        // A single f64 JD near J2000 has ~40 microsecond spacing.
        assert!((recovered - seconds).abs() < 25.0e-6);
    }

    // A microsecond at J2000 is not representable in a single binary64 JD.
    let tiny_offset = TdbSecondsPastJ2000::new(1.0e-6).unwrap();
    assert_eq!(
        tiny_offset.to_julian_day_tdb().unwrap().value(),
        J2000_JD_TDB
    );
}

#[test]
fn epoch_constructors_reject_nonfinite_inputs() {
    assert!(TdbSecondsPastJ2000::new(f64::NAN).is_err());
    assert!(TdbSecondsPastJ2000::new(f64::INFINITY).is_err());
    assert!(JulianDayTdb::new(f64::NEG_INFINITY).is_err());
}
