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
    geocentric_true_ecliptic, geocentric_true_equatorial, project_true_ecliptic,
    project_true_equatorial, TrueEclipticError,
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
fn moving_equatorial_rates_agree_with_position_differences() {
    let t = 2_451_545.0;
    let h = 0.02;
    let center = project_true_equatorial(&sample(t), t).unwrap();
    let a = project_true_equatorial(&sample(t - h), t - h).unwrap();
    let b = project_true_equatorial(&sample(t + h), t + h).unwrap();
    let span = (t + h) - (t - h);
    assert!(
        (center.right_ascension_speed_deg_per_tdb_day
            - delta(b.right_ascension_deg, a.right_ascension_deg) / span)
            .abs()
            < 2e-7
    );
    assert!(
        (center.declination_speed_deg_per_tdb_day - (b.declination_deg - a.declination_deg) / span)
            .abs()
            < 2e-7
    );
    assert!(
        (center.distance_speed_au_per_tdb_day - (b.distance_au - a.distance_au) / span).abs()
            < 2e-9
    );
    assert!(matches!(
        project_true_equatorial(&sample(t), t + 1.0),
        Err(TrueEclipticError::IncoherentEpoch)
    ));
}

#[test]
#[ignore = "requires DE441_PATH pointing to an external classic JPL DE441 binary"]
fn true_equatorial_matches_recorded_swiss_sample_at_same_tt() {
    let mut eph = De441::open(std::env::var_os("DE441_PATH").expect("set DE441_PATH")).unwrap();
    let tt = 2_461_354.500_800_741;
    let tdb = JulianDateTdb(2_461_354.500_800_725);
    // Swiss 0.5.17, same local DE441, swe_calc(TT, JPLEPH|SPEED|EQUATORIAL).
    // Loose comparison limits do not establish independent scientific accuracy.
    for (body, ra, dec, dist) in [
        (
            Body::Sun,
            225.13669688642472,
            -17.081149005284875,
            0.9904801297010831,
        ),
        (
            Body::Moon,
            231.98126372218556,
            -24.00822518494715,
            0.0026723304358203352,
        ),
        (
            Body::Mercury,
            214.44960154471735,
            -12.372684577507627,
            0.7332818069086801,
        ),
        (
            Body::Venus,
            200.46980504375202,
            -11.44562216419675,
            0.30768946726419966,
        ),
        (
            Body::MarsBarycenter,
            145.96438622338124,
            15.540680571668778,
            1.358155892721995,
        ),
        (
            Body::JupiterBarycenter,
            147.84002808091148,
            13.76041974600778,
            5.36134941128455,
        ),
    ] {
        let state = geocentric_true_equatorial(&mut eph, body, tdb, tt).unwrap();
        assert!(
            delta(state.right_ascension_deg, ra).abs() < 0.0001,
            "{body:?}"
        );
        assert!((state.declination_deg - dec).abs() < 0.0001, "{body:?}");
        assert!((state.distance_au - dist).abs() < 1e-7, "{body:?}");
    }
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
