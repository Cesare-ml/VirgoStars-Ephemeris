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

//! Independent Horizons observer-table diagnostic, added 2026-11-15.
//! Model differences and sample limitations are documented separately.

use std::fs::File;
use virgostars_ephemeris_core::de441::{De441, JulianDateTdb};
use virgostars_ephemeris_core::planetary_solar_deflection::{
    calculate_deflected_planet, PlanetFrame, PlanetTarget,
};
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;

#[test]
#[ignore = "requires DE441_PATH; independent Horizons fixture is documented"]
fn horizons_mercury_venus_multiple_epochs_positions_and_rates() {
    let mut eph = De441::open(std::env::var_os("DE441_PATH").expect("DE441_PATH")).unwrap();
    // JD TT, target, longitude/latitude at center, at TT -0.01d and +0.01d.
    // Horizons quantity 31 uses IAU76/80 while this implementation uses
    // IAU2006/2000B, and prints angles to 1e-7 deg. TDB=TT is an explicit
    // sub-two-millisecond fixture approximation, not an actual converter.
    let cases = [
        (
            2_451_545.0,
            PlanetTarget::Mercury,
            [271.888_113_8, -0.994_746_6],
            [271.872_551_6, -0.993_771_4],
            [271.903_676_6, -0.995_721_4],
        ),
        (
            2_451_545.0,
            PlanetTarget::Venus,
            [241.564_881_2, 2.066_375_7],
            [241.552_790_8, 2.066_656_4],
            [241.576_971_7, 2.066_094_8],
        ),
        (
            2_455_197.5,
            PlanetTarget::Mercury,
            [288.996_652_5, 1.635_008_0],
            [289.006_802_0, 1.631_813_6],
            [288.986_487_3, 1.638_201_4],
        ),
        (
            2_455_197.5,
            PlanetTarget::Venus,
            [277.848_991_5, -0.436_545_8],
            [277.836_408_7, -0.436_165_4],
            [277.861_574_2, -0.436_926_1],
        ),
        (
            2_469_807.5,
            PlanetTarget::Mercury,
            [270.060_255_3, 3.135_708_7],
            [270.069_835_7, 3.135_028_1],
            [270.050_690_4, 3.136_385_4],
        ),
        (
            2_469_807.5,
            PlanetTarget::Venus,
            [281.247_030_7, -0.606_033_6],
            [281.234_447_1, -0.605_670_9],
            [281.259_614_3, -0.606_396_2],
        ),
    ];
    for (tt, target, center, before, after) in cases {
        let s = calculate_deflected_planet(
            &mut eph,
            None::<&mut SpkKernelSet<File>>,
            target,
            JulianDateTdb(tt),
            tt,
            PlanetFrame::TrueEcliptic,
        )
        .unwrap();
        let longitude_delta = (s.first_angle_deg - center[0] + 180.0).rem_euclid(360.0) - 180.0;
        let reference_rate = (after[0] - before[0] + 180.0_f64).rem_euclid(360.0) - 180.0;
        assert!(
            longitude_delta.abs() < 0.00003,
            "{tt} {target:?} longitude {longitude_delta}"
        );
        assert!(
            (s.second_angle_deg - center[1]).abs() < 0.00003,
            "{tt} {target:?} latitude"
        );
        assert!(
            (s.first_angle_speed_deg_per_tdb_day - reference_rate / 0.02).abs() < 0.00003,
            "{tt} {target:?} longitude rate"
        );
        assert!(
            (s.second_angle_speed_deg_per_tdb_day - (after[1] - before[1]) / 0.02).abs() < 0.00003,
            "{tt} {target:?} latitude rate"
        );
    }
}
