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

//! Geometric intersection tests; 2026-11-14.
use virgostars_ephemeris_core::house_angles::{from_armc, from_epochs, HouseAnglesError};

#[test]
fn known_armc_and_obliquity_give_rome_ascendant_and_midheaven() {
    // Local Swiss 0.5.17 at UT JD 2461354.5, Rome 41.9028 N, 12.4964 E;
    // use its reported ARMC and true obliquity as direct geometric inputs.
    // This checks intersections only, not time conversion or general parity.
    let angles = from_armc(61.66688244516181, 41.9028, 23.437859777570328).unwrap();
    assert!((angles.ascendant_deg - 157.82641479007887).abs() < 1e-10);
    assert!((angles.midheaven_deg - 63.6782674613323).abs() < 1e-10);
}

#[test]
fn invalid_location_and_horizon_singularity_are_explicit() {
    assert!(matches!(
        from_armc(10.0, 90.0, 23.4),
        Err(HouseAnglesError::InvalidLocation)
    ));
    assert!(matches!(
        from_epochs(2451545.0, 2451545.0, 1.0, 181.0),
        Err(HouseAnglesError::InvalidLocation)
    ));
    assert!(matches!(
        from_armc(90.0, -66.6, 23.4),
        Err(HouseAnglesError::InvalidGeometry)
    ));
    assert!(from_epochs(2451545.0, 2451545.0, 41.9, 12.5).is_ok());
    assert!(matches!(
        from_epochs(2451545.0, 2451546.0, 41.9, 12.5),
        Err(HouseAnglesError::IncoherentEpoch)
    ));
}
