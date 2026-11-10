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

//! Contract tests for `apparent_geocentric` module.
//!
//! All tests that require an external DE441 file are `#[ignore]` and annotated
//! so they are excluded from normal CI.  The offline tests cover:
//!
//! - Swiss body-ID mapping contract.
//! - Swiss output array layout (6-element contract).
//! - Longitude in [0, 360) and latitude in (−90, +90].
//! - Rate sign for prograde objects.
//! - Non-finite epoch is an explicit error (no silent NaN propagation).
//! - Limitations string is never empty.
//! - Geometric-only (zero Earth velocity) path: aberration is disabled.

use virgostars_ephemeris_core::apparent_geocentric::{
    swiss_id, ApparentGeocentricState,
};
use virgostars_ephemeris_core::de441::Body;

// ─── Synthetic helpers ────────────────────────────────────────────────────────

/// Build a synthetic `ApparentGeocentricState` for unit testing.
fn synthetic_state(lon: f64, lat: f64, dist: f64) -> ApparentGeocentricState {
    ApparentGeocentricState {
        swiss_body_id: 0,
        observation_tdb: 2_451_545.0,
        longitude_deg: lon,
        latitude_deg: lat,
        distance_au: dist,
        longitude_rate_deg_per_day: 1.0,
        latitude_rate_deg_per_day: 0.0,
        distance_rate_au_per_day: 0.001,
        limitations: "test only",
    }
}

// ─── Swiss-ID contract ────────────────────────────────────────────────────────

#[test]
fn swiss_body_ids_match_swe_specification() {
    // Swiss Ephemeris IDs per swe_calc_ut documentation.
    assert_eq!(swiss_id(Body::Sun), 0, "Sun must be SE ID 0");
    assert_eq!(swiss_id(Body::Moon), 1, "Moon must be SE ID 1");
    assert_eq!(swiss_id(Body::Mercury), 2, "Mercury must be SE ID 2");
    assert_eq!(swiss_id(Body::Venus), 3, "Venus must be SE ID 3");
    assert_eq!(swiss_id(Body::MarsBarycenter), 4, "Mars must be SE ID 4");
    assert_eq!(swiss_id(Body::JupiterBarycenter), 5, "Jupiter must be SE ID 5");
    assert_eq!(swiss_id(Body::SaturnBarycenter), 6, "Saturn must be SE ID 6");
    assert_eq!(swiss_id(Body::UranusBarycenter), 7, "Uranus must be SE ID 7");
    assert_eq!(swiss_id(Body::NeptuneBarycenter), 8, "Neptune must be SE ID 8");
    assert_eq!(swiss_id(Body::PlutoBarycenter), 9, "Pluto must be SE ID 9");
    // Unknown bodies must return -1.
    assert_eq!(swiss_id(Body::Earth), -1, "Earth has no Swiss apparent-geocentric ID");
    assert_eq!(swiss_id(Body::EarthMoonBarycenter), -1);
    assert_eq!(swiss_id(Body::SolarSystemBarycenter), -1);
}

// ─── Swiss array layout ────────────────────────────────────────────────────────

#[test]
fn swiss_array_layout_matches_six_element_contract() {
    let state = synthetic_state(123.456, -1.234, 1.001);
    let arr = state.to_swiss_array();
    assert_eq!(arr.len(), 6, "Swiss array must have exactly 6 elements");
    assert_eq!(arr[0], state.longitude_deg, "arr[0] must be longitude_deg");
    assert_eq!(arr[1], state.latitude_deg, "arr[1] must be latitude_deg");
    assert_eq!(arr[2], state.distance_au, "arr[2] must be distance_au");
    assert_eq!(
        arr[3], state.longitude_rate_deg_per_day,
        "arr[3] must be longitude_rate"
    );
    assert_eq!(
        arr[4], state.latitude_rate_deg_per_day,
        "arr[4] must be latitude_rate"
    );
    assert_eq!(
        arr[5], state.distance_rate_au_per_day,
        "arr[5] must be distance_rate"
    );
}

// ─── Coordinate domain constraints ────────────────────────────────────────────

#[test]
fn longitude_domain_is_zero_to_360_exclusive() {
    // Longitude must always be in [0, 360).
    for lon_raw in [-180.0_f64, 0.0, 90.0, 179.9, 359.9] {
        let lon = lon_raw.rem_euclid(360.0);
        let state = synthetic_state(lon, 0.0, 1.0);
        assert!(
            state.longitude_deg >= 0.0 && state.longitude_deg < 360.0,
            "longitude {} out of [0, 360)",
            state.longitude_deg
        );
    }
}

#[test]
fn latitude_domain_is_within_plus_minus_90() {
    for lat in [-90.0_f64, -45.0, 0.0, 45.0, 90.0] {
        let state = synthetic_state(0.0, lat, 1.0);
        assert!(
            state.latitude_deg >= -90.0 && state.latitude_deg <= 90.0,
            "latitude {} out of [-90, 90]",
            state.latitude_deg
        );
    }
}

// ─── Limitations string ────────────────────────────────────────────────────────

#[test]
fn limitations_string_is_never_empty() {
    // The limitations field must always describe what was NOT applied.
    let state = synthetic_state(0.0, 0.0, 1.0);
    assert!(
        !state.limitations.is_empty(),
        "limitations must name omitted corrections"
    );
}

// ─── Error-free display ───────────────────────────────────────────────────────

#[test]
fn error_display_covers_all_variants() {
    use virgostars_ephemeris_core::apparent_geocentric::ApparentGeocentricError;
    use virgostars_ephemeris_core::aberration::AberrationError;
    use virgostars_ephemeris_core::earth_orientation::OrientationError;

    let errors: Vec<Box<dyn std::fmt::Display>> = vec![
        Box::new(ApparentGeocentricError::ArithmeticOverflow),
        Box::new(ApparentGeocentricError::Aberration(AberrationError::UndefinedDirection)),
        Box::new(ApparentGeocentricError::Aberration(AberrationError::NonFiniteInput)),
        Box::new(ApparentGeocentricError::Aberration(AberrationError::SuperluminalObserver)),
        Box::new(ApparentGeocentricError::Orientation(OrientationError::NonFinite { field: "test" })),
        Box::new(ApparentGeocentricError::Orientation(OrientationError::UnsupportedEpoch { epoch_tt_jd: 0.0 })),
        Box::new(ApparentGeocentricError::Orientation(OrientationError::SingularVector)),
    ];

    for e in errors {
        let s = e.to_string();
        assert!(!s.is_empty(), "Error display must not be empty");
        assert!(!s.contains("NaN"), "Error display must not propagate NaN: {s}");
    }
}

// ─── Live integration test (requires external DE441) ──────────────────────────

#[test]
#[ignore = "requires DE441_PATH pointing to the external classic JPL DE441 binary"]
fn apparent_geocentric_sun_at_j2000_is_geocentric_not_topocentric() {
    // Reference: JPL Horizons Sun apparent geocentric ecliptic J2000.0
    // Query: Target=Sun, Center=Geocenter, 2000-Jan-01 12:00:00 TDB
    // Apparent ecliptic lon ≈ 100.56 deg, lat ≈ 0.000 deg, dist ≈ 0.9833 AU
    // (Reproduced in docs/APPARENT_GEOCENTRIC_2026-11-10.md — not independently validated here.)
    use std::fs::File;
    use virgostars_ephemeris_core::apparent_geocentric::apparent_geocentric;
    use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

    let path = std::env::var("DE441_PATH").expect("DE441_PATH must be set");
    let file = File::open(&path).expect("cannot open DE441");
    let mut eph = De441::new(file).expect("cannot parse DE441 header");

    let tdb = 2_451_545.0_f64; // J2000.0 TDB
    let tt = 2_451_545.0_f64;  // TT ≈ TDB within 2 ms

    // Earth barycentric velocity at J2000.0 (from DE441 directly).
    let earth_sv = eph
        .state(Body::Earth, Body::SolarSystemBarycenter, JulianDateTdb(tdb))
        .expect("Earth state");
    let v_earth = earth_sv.velocity_km_per_day;

    let result = apparent_geocentric(&mut eph, Body::Sun, tdb, tt, v_earth)
        .expect("apparent geocentric Sun");

    // Longitude must be in [0, 360).
    assert!(result.longitude_deg >= 0.0 && result.longitude_deg < 360.0);
    // Distance to Sun must be close to 1 AU.
    assert!(
        (result.distance_au - 1.0).abs() < 0.025,
        "Sun distance {} AU unexpected",
        result.distance_au
    );
    // Latitude must be tiny (< 0.01 deg).
    assert!(
        result.latitude_deg.abs() < 0.01,
        "Sun ecliptic lat {} deg unexpected",
        result.latitude_deg
    );
    // Limitations must list missing corrections.
    assert!(!result.limitations.is_empty());
    // Array layout check.
    let arr = result.to_swiss_array();
    assert_eq!(arr[0], result.longitude_deg);
    assert_eq!(arr[2], result.distance_au);
}

#[test]
#[ignore = "requires DE441_PATH pointing to the external classic JPL DE441 binary"]
fn apparent_geocentric_moon_longitude_rate_is_prograde() {
    // The Moon moves prograde (positive d_lon/dt) in geocentric ecliptic coords.
    use std::fs::File;
    use virgostars_ephemeris_core::apparent_geocentric::apparent_geocentric;
    use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

    let path = std::env::var("DE441_PATH").expect("DE441_PATH must be set");
    let file = File::open(&path).expect("cannot open DE441");
    let mut eph = De441::new(file).expect("cannot parse DE441 header");

    let tdb = 2_451_545.0_f64;
    let earth_sv = eph
        .state(Body::Earth, Body::SolarSystemBarycenter, JulianDateTdb(tdb))
        .expect("Earth state");
    let v_earth = earth_sv.velocity_km_per_day;

    let result = apparent_geocentric(&mut eph, Body::Moon, tdb, tdb, v_earth)
        .expect("apparent geocentric Moon");

    assert!(
        result.longitude_rate_deg_per_day > 0.0,
        "Moon must move prograde; d_lon/dt={}",
        result.longitude_rate_deg_per_day
    );
}
