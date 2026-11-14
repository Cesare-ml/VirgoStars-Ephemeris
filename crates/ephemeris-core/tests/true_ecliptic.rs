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

//! Moving-frame derivative regressions, including an optional real DE441 file.
//! Added 2026-11-14. Not a Swiss numerical parity certificate.
use virgostars_ephemeris_core::aberration_state::{
    AberratedGeocentricState, ObserverAccelerationSource,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::true_ecliptic::{
    geocentric_true_ecliptic, project_true_ecliptic, TrueEclipticError,
};

fn delta(a: f64, b: f64) -> f64 {
    (a - b + 180.0).rem_euclid(360.0) - 180.0
}

fn sample(t: f64) -> AberratedGeocentricState {
    let dt = t - 2_451_545.0;
    AberratedGeocentricState {
        target: Body::MarsBarycenter,
        observation_epoch: JulianDateTdb(t),
        emission_epoch: JulianDateTdb(t - 0.005),
        light_time_seconds: 432.0,
        light_time_rate: 0.0,
        observer_acceleration_source: ObserverAccelerationSource::CallerProvided,
        position_km: [
            100_000_000.0 + 500_000.0 * dt,
            75_000_000.0 - 300_000.0 * dt,
            20_000_000.0 + 100_000.0 * dt,
        ],
        velocity_km_per_tdb_day: [500_000.0, -300_000.0, 100_000.0],
    }
}

#[test]
fn moving_frame_rates_agree_with_position_differences() {
    let t = 2_451_545.0;
    let h = 0.02;
    let state = project_true_ecliptic(&sample(t), t).unwrap();
    let before = project_true_ecliptic(&sample(t - h), t - h).unwrap();
    let after = project_true_ecliptic(&sample(t + h), t + h).unwrap();
    let interval = (t + h) - (t - h);
    assert!(
        (state.longitude_speed_deg_per_tdb_day
            - delta(after.longitude_deg, before.longitude_deg) / interval)
            .abs()
            < 2e-7
    );
    assert!(
        (state.latitude_speed_deg_per_tdb_day
            - (after.latitude_deg - before.latitude_deg) / interval)
            .abs()
            < 2e-7
    );
    assert!(
        (state.distance_speed_au_per_tdb_day - (after.distance_au - before.distance_au) / interval)
            .abs()
            < 2e-9
    );
}

#[test]
fn incoherent_scales_and_missing_frame_history_are_rejected() {
    let state = sample(2_451_545.0);
    assert!(matches!(
        project_true_ecliptic(&state, 2_451_545.1),
        Err(TrueEclipticError::IncoherentEpoch)
    ));
    assert!(matches!(
        project_true_ecliptic(&state, f64::NAN),
        Err(TrueEclipticError::IncoherentEpoch)
    ));
    let t = 2_415_020.0;
    assert!(matches!(
        project_true_ecliptic(&sample(t), t),
        Err(TrueEclipticError::Orientation(_))
    ));
}

#[test]
#[ignore = "requires DE441_PATH pointing to an external classic JPL DE441 binary"]
fn real_de441_true_ecliptic_rates_follow_positions() {
    let mut eph = De441::open(std::env::var_os("DE441_PATH").expect("set DE441_PATH")).unwrap();
    let t = 2_461_354.5;
    let h = 0.01;
    for body in [
        Body::Sun,
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::MarsBarycenter,
        Body::JupiterBarycenter,
    ] {
        let state = geocentric_true_ecliptic(&mut eph, body, JulianDateTdb(t), t).unwrap();
        let before = geocentric_true_ecliptic(&mut eph, body, JulianDateTdb(t - h), t - h).unwrap();
        let after = geocentric_true_ecliptic(&mut eph, body, JulianDateTdb(t + h), t + h).unwrap();
        let interval = (t + h) - (t - h);
        assert!(
            (state.longitude_speed_deg_per_tdb_day
                - delta(after.longitude_deg, before.longitude_deg) / interval)
                .abs()
                < 1e-4,
            "{body:?}"
        );
        assert!(
            (state.latitude_speed_deg_per_tdb_day
                - (after.latitude_deg - before.latitude_deg) / interval)
                .abs()
                < 1e-4,
            "{body:?}"
        );
        assert!(
            (state.distance_speed_au_per_tdb_day
                - (after.distance_au - before.distance_au) / interval)
                .abs()
                < 1e-7,
            "{body:?}"
        );
    }
}
