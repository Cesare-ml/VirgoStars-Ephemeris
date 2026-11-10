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

use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::light_time::geocentric_light_time;

/// Horizons API 2026-11-10; DE441 target and Earth, geocentric, ICRF,
/// TDB JD 2461354.5, VECTORS, KM-S, FRAME, VEC_CORR=LT, VEC_TABLE=3.
/// Fixed tolerances allow single-f64 emission JD quantization; see the doc.
#[test]
#[ignore = "requires DE441_PATH pointing to the external classic JPL DE441 binary"]
fn light_time_states_match_horizons_for_solar_system_targets() {
    let path = std::env::var("DE441_PATH").expect("set DE441_PATH");
    let mut ephemeris = De441::open(path).unwrap();
    let cases = [
        (
            Body::Moon,
            [
                -2.271092990545779e5,
                -2.863237983404845e5,
                -1.620441867628545e5,
            ],
            [
                7.936871780642107e-1,
                -5.390344826676774e-1,
                -2.373305408385367e-1,
            ],
            1.333498431623768,
        ),
        (
            Body::Venus,
            [
                -4.238188819403984e7,
                -1.552509477906777e7,
                -9.024076156753547e6,
            ],
            [-7.240717529836648, -2.057405793676001, 1.251391274829897],
            153.5368740945382,
        ),
        (
            Body::MarsBarycenter,
            [
                -1.614080644210615e8,
                1.105419500423874e8,
                5.485717291371012e7,
            ],
            [
                -1.608898179679485e-1,
                -2.239514836155294e1,
                -9.199660864495826,
            ],
            677.7296430371571,
        ),
        (
            Body::JupiterBarycenter,
            [
                -6.564_834_834_981_32e8,
                4.186401692284813e8,
                1.924872858802854e8,
            ],
            [
                1.285064693536208e1,
                -2.638_394_601_650_39e1,
                -1.116713643880395e1,
            ],
            2675.345227915447,
        ),
    ];
    let mut failures = Vec::new();
    for (body, expected_position, expected_velocity, expected_delay) in cases {
        let actual =
            geocentric_light_time(&mut ephemeris, body, JulianDateTdb(2_461_354.5)).unwrap();
        let target = ephemeris
            .state(body, Body::SolarSystemBarycenter, actual.emission_epoch)
            .unwrap();
        let observer = ephemeris
            .state(
                Body::Earth,
                Body::SolarSystemBarycenter,
                actual.observation_epoch,
            )
            .unwrap();
        let before_jd = JulianDateTdb(actual.observation_epoch.0 - 600.0 / 86_400.0);
        let after_jd = JulianDateTdb(actual.observation_epoch.0 + 600.0 / 86_400.0);
        let before = geocentric_light_time(&mut ephemeris, body, before_jd).unwrap();
        let after = geocentric_light_time(&mut ephemeris, body, after_jd).unwrap();
        let seconds = (after_jd.0 - before_jd.0) * 86_400.0;
        let mut max_position: f64 = 0.0;
        let mut max_reference_velocity: f64 = 0.0;
        let mut max_derivative_error: f64 = 0.0;
        for i in 0..3 {
            max_position = max_position.max((actual.position_km[i] - expected_position[i]).abs());
            // Horizons VEC_CORR=LT returns the difference of instantaneous
            // velocities at emission/reception in this query, not d(position)/dt.
            let instantaneous =
                (target.velocity_km_per_day[i] - observer.velocity_km_per_day[i]) / 86_400.0;
            max_reference_velocity =
                max_reference_velocity.max((instantaneous - expected_velocity[i]).abs());
            let numerical_derivative = (after.position_km[i] - before.position_km[i]) / seconds;
            max_derivative_error = max_derivative_error
                .max((actual.velocity_km_per_day[i] / 86_400.0 - numerical_derivative).abs());
        }
        let delay_error = (actual.light_time_seconds - expected_delay).abs();
        println!("{body:?}: position {max_position:.9} km, Horizons velocity {max_reference_velocity:.12} km/s, finite-difference derivative {max_derivative_error:.12} km/s, delay {delay_error:.12} s");
        if max_position >= 0.01
            || max_reference_velocity >= 1.0e-6
            || max_derivative_error >= 3.0e-6
            || delay_error >= 1.0e-6
        {
            failures.push(body);
        }
    }
    assert!(failures.is_empty(), "reference failures: {failures:?}");
}

#[test]
#[ignore = "requires DE441_PATH pointing to the external classic JPL DE441 binary"]
fn real_kernel_covers_common_bodies_and_rejects_missing_emission_history() {
    use virgostars_ephemeris_core::de441::De441Error;
    use virgostars_ephemeris_core::light_time::LightTimeError;

    let path = std::env::var("DE441_PATH").expect("set DE441_PATH");
    let mut ephemeris = De441::open(path).unwrap();
    let bodies = [
        Body::Sun,
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::MarsBarycenter,
        Body::JupiterBarycenter,
        Body::SaturnBarycenter,
        Body::UranusBarycenter,
        Body::NeptuneBarycenter,
        Body::PlutoBarycenter,
    ];
    for jd in [2_441_317.5, 2_451_545.0, 2_461_354.5, 2_488_069.5] {
        for body in bodies {
            let state = geocentric_light_time(&mut ephemeris, body, JulianDateTdb(jd)).unwrap();
            assert!(state.light_time_seconds > 0.0);
            assert!(state.emission_epoch.0 < jd);
        }
    }
    let (start, _) = ephemeris.coverage();
    assert!(matches!(
        geocentric_light_time(&mut ephemeris, Body::Mercury, start),
        Err(LightTimeError::Ephemeris(
            De441Error::OutsideCoverage { .. }
        ))
    ));
}
