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
// Modification (2026-11-16): cross-check the integrated orientation stages.

use std::f64::consts::TAU;
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::earth_orientation::{era_iau2000, gmst_iau2006};
use virgostars_ephemeris_core::earth_rotation::earth_rotation_angle;
use virgostars_ephemeris_core::gmst_iau2006::iau2006_gmst_from_era;
use virgostars_ephemeris_core::local_sidereal::local_mean_sidereal_time;

#[test]
fn split_ut1_and_tt_compose_to_published_erfa_gmst06() {
    // ERFA t_gmst06: UT1 and TT JD 2400000.5 + 53736.0.
    let ut1 = JulianDate::new(2_453_737, 0.0, TimeScale::Ut1).unwrap();
    let tt = JulianDate::new(2_453_737, 0.0, TimeScale::Tt).unwrap();
    let era = earth_rotation_angle(ut1).unwrap().angle_rad;
    let gmst = iau2006_gmst_from_era(era, tt).unwrap();
    assert!((gmst - 1.754_174_971_870_091_2).abs() < 1e-12);
    assert!((gmst - gmst_iau2006(2_453_736.5, 2_453_736.5).unwrap()).abs() < 1e-12);

    let local = local_mean_sidereal_time(gmst, 15.0).unwrap();
    assert!((local - (gmst + TAU / 24.0).rem_euclid(TAU)).abs() < 1e-14);
}

#[test]
fn orientation_rejects_unrepresentable_ut1_and_normalizes_small_negative_angles() {
    assert!(era_iau2000(f64::MAX).is_err());
    assert!(era_iau2000(-f64::MAX).is_err());
    let local = local_mean_sidereal_time(0.0, -1e-20).unwrap();
    assert!((0.0..TAU).contains(&local));
}
