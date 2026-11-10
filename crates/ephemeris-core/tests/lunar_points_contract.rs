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

use virgostars_ephemeris_core::earth_orientation::{iau2000b_nutation, J2000_TT_JD};
use virgostars_ephemeris_core::lunar_points::{
    mean_lunar_apogee, mean_lunar_node, LunarPoint, LunarPointError,
};

fn near(a: f64, b: f64, tolerance: f64) {
    assert!(
        (a - b).abs() <= tolerance,
        "{a} != {b} (diff: {}, max tol: {tolerance})",
        (a - b).abs()
    );
}

#[test]
fn mean_node_j2000_analytical_parity() {
    let node = mean_lunar_node(J2000_TT_JD, false).expect("valid node");

    assert_eq!(node.point, LunarPoint::MeanNode);
    // Simon et al. (1994) / Chapront ELP-2000 reference: 125.044555°
    near(node.longitude_deg, 125.044555, 1e-5);
    assert_eq!(node.latitude_deg, 0.0);
    assert_eq!(node.distance_au, 1.0);

    // Retrograde motion rate ~ -0.05295 deg/day (~18.61 years nodal period)
    near(node.longitude_rate_deg_per_day, -0.0529538, 1e-4);
    assert!(node.longitude_rate_deg_per_day < 0.0);
    assert_eq!(node.latitude_rate_deg_per_day, 0.0);
    assert_eq!(node.distance_rate_au_per_day, 0.0);

    let period_years = 360.0 / node.longitude_rate_deg_per_day.abs() / 365.25;
    near(period_years, 18.613, 0.01);
}

#[test]
fn mean_apogee_j2000_analytical_parity() {
    let apogee = mean_lunar_apogee(J2000_TT_JD, false).expect("valid apogee");

    assert_eq!(apogee.point, LunarPoint::MeanApogee);
    // Chapront ELP-2000 mean perigee at J2000: 83.3532431° -> Apogee = +180° = 263.3532431°
    near(apogee.longitude_deg, 263.353243, 1e-5);
    assert_eq!(apogee.latitude_deg, 0.0);

    // Direct motion rate ~ +0.11140 deg/day (~8.85 years apsidal period)
    near(apogee.longitude_rate_deg_per_day, 0.1114036, 1e-4);
    assert!(apogee.longitude_rate_deg_per_day > 0.0);
    assert_eq!(apogee.latitude_rate_deg_per_day, 0.0);

    let period_years = 360.0 / apogee.longitude_rate_deg_per_day.abs() / 365.25;
    near(period_years, 8.847, 0.01);
}

#[test]
fn nutation_apparent_correction_applied() {
    let jd = 2_453_736.5; // 2006-01-15 00:00:00 TT
    let geom_node = mean_lunar_node(jd, false).unwrap();
    let app_node = mean_lunar_node(jd, true).unwrap();

    let nut = iau2000b_nutation(jd).unwrap();
    let expected_diff_deg = nut.d_psi_rad.to_degrees();

    near(
        app_node.longitude_deg - geom_node.longitude_deg,
        expected_diff_deg,
        1e-10,
    );
}

#[test]
fn swiss_contract_array_representation() {
    let coords = mean_lunar_node(J2000_TT_JD, true).unwrap();
    let arr = coords.to_swiss_array();

    assert_eq!(arr.len(), 6);
    near(arr[0], coords.longitude_deg, 1e-12);
    near(arr[1], coords.latitude_deg, 1e-12);
    near(arr[2], coords.distance_au, 1e-12);
    near(arr[3], coords.longitude_rate_deg_per_day, 1e-12);
    near(arr[4], coords.latitude_rate_deg_per_day, 1e-12);
    near(arr[5], coords.distance_rate_au_per_day, 1e-12);

    assert_eq!(LunarPoint::MeanNode.swiss_id(), 10);
    assert_eq!(LunarPoint::TrueNode.swiss_id(), 11);
    assert_eq!(LunarPoint::MeanApogee.swiss_id(), 12);
    assert_eq!(LunarPoint::OsculatingApogee.swiss_id(), 13);
}

#[test]
fn non_finite_inputs_rejected() {
    assert!(matches!(
        mean_lunar_node(f64::NAN, false),
        Err(LunarPointError::NonFiniteInput)
    ));
    assert!(matches!(
        mean_lunar_apogee(f64::INFINITY, false),
        Err(LunarPointError::NonFiniteInput)
    ));
}
