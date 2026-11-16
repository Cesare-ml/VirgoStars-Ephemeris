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

//! Tests for the astronomy supplied to house systems.

use virgostars_ephemeris_core::house_angles::{angles_from_armc, house_angles, HouseAnglesError};

const EPS: f64 = 23.439_291_1;

fn near(a: f64, b: f64, tol: f64) {
    let d = ((a - b + 540.0).rem_euclid(360.0)) - 180.0;
    assert!(d.abs() <= tol, "{a} vs {b}: {d}");
}

#[test]
fn cardinal_armc_values_on_the_equator() {
    // ARMC 0: MC at 0 Aries, ASC at 90 (Cancer).
    near(angles_from_armc(0.0, EPS, 0.0).1, 0.0, 1e-12);
    near(angles_from_armc(0.0, EPS, 0.0).0, 90.0, 1e-12);
    // ARMC 90: MC at 90, ASC at 180.
    near(angles_from_armc(90.0, EPS, 0.0).1, 90.0, 1e-12);
    near(angles_from_armc(90.0, EPS, 0.0).0, 180.0, 1e-12);
    // ARMC 180: ASC at 270. ARMC 270: MC at 270, ASC at 0.
    near(angles_from_armc(180.0, EPS, 0.0).0, 270.0, 1e-12);
    near(angles_from_armc(270.0, EPS, 0.0).1, 270.0, 1e-12);
    near(angles_from_armc(270.0, EPS, 0.0).0, 0.0, 1e-12);
}

#[test]
fn ascendant_follows_the_midheaven_by_less_than_a_half_circle_in_the_north() {
    for armc in (0..360).step_by(7) {
        for lat in [10.0, 41.9, 55.0, 64.0] {
            let (asc, mc) = angles_from_armc(f64::from(armc), 23.44, lat);
            let ahead = (asc - mc).rem_euclid(360.0);
            assert!(
                ahead > 0.0 && ahead < 180.0,
                "armc {armc} lat {lat}: {ahead}"
            );
        }
    }
}

#[test]
fn the_southern_hemisphere_mirrors_the_northern_one() {
    // Rotating the sky by half a turn and flipping the latitude maps the
    // Ascendant to the opposite point.
    for armc in (0..360).step_by(11) {
        for lat in [20.0, 41.9, 60.0] {
            let south = angles_from_armc(f64::from(armc), 23.44, -lat).0;
            let north = angles_from_armc(f64::from(armc) + 180.0, 23.44, lat).0;
            near(south, north + 180.0, 1e-9);
        }
    }
}

#[test]
fn full_pipeline_has_sane_values_and_fails_closed() {
    // 2000-01-01 12:00 TT, with UT1 equal to TT only as a test shortcut.
    let a = house_angles(2_451_545.0, 2_451_545.0, 0.0, 0.0).unwrap();
    assert!(
        (a.true_obliquity_deg - 23.4372).abs() < 0.001,
        "{}",
        a.true_obliquity_deg
    );
    // Greenwich apparent sidereal time at J2000.0 is about 18.697 h = 280.46 deg.
    near(a.armc_deg, 280.46, 0.05);
    let east = house_angles(2_451_545.0, 2_451_545.0, 41.9, 12.5).unwrap();
    near(east.armc_deg, a.armc_deg + 12.5, 1e-9);
    for (lat, lon) in [
        (90.0, 0.0),
        (-90.0, 0.0),
        (f64::NAN, 0.0),
        (10.0, f64::INFINITY),
        (10.0, 400.0),
    ] {
        assert_eq!(
            house_angles(2_451_545.0, 2_451_545.0, lat, lon),
            Err(HouseAnglesError::InvalidSite),
            "{lat} {lon}"
        );
    }
    // The IAU 2000B range is enforced by the orientation module.
    assert!(matches!(
        house_angles(2_300_000.0, 2_300_000.0, 10.0, 10.0),
        Err(HouseAnglesError::Orientation(_))
    ));
}
