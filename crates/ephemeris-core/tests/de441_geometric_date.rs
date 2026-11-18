/*
EXHIBIT A. Common Public Attribution License Version 1.0.

The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0

Modification (2026-11-18): synthetic geometric ICRF-to-date-frame tests. These
tests do not constitute real NASA/JPL numerical validation.
*/

use virgostars_ephemeris_core::de441::{Body, JulianDateTdb, StateVector};
use virgostars_ephemeris_core::de441_geometric_date::{
    geocentric_geometric_true_ecliptic_of_date, project_geocentric_geometric_state,
    GeometricDateError, ORIENTATION_MODEL,
};
use virgostars_ephemeris_core::de441_pack_manager::{De441PackError, De441PackManager};
use virgostars_ephemeris_core::earth_orientation::OrientationError;
use virgostars_ephemeris_core::six_field_units::IAU_AU_KM;

const J2000_TDB: JulianDateTdb = JulianDateTdb(2_451_545.0);

fn geometric(position: [f64; 3]) -> StateVector {
    StateVector {
        target: Body::Sun,
        center: Body::Earth,
        epoch: J2000_TDB,
        position_km: position,
        velocity_km_per_day: [0.0; 3],
    }
}

#[test]
fn rotation_only_preserves_distance_and_explicit_tdb_tt_epochs() {
    let input = geometric([0.0, IAU_AU_KM, 0.0]);
    let output = project_geocentric_geometric_state(&input, 2_451_545.0).unwrap();
    assert_eq!(output.target, Body::Sun);
    assert_eq!(output.center, Body::Earth);
    assert_eq!(output.epoch_tdb, J2000_TDB);
    assert_eq!(output.orientation_epoch_tt_jd, 2_451_545.0);
    assert_eq!(output.orientation_model, ORIENTATION_MODEL);
    assert!((output.distance_au - 1.0).abs() < 1e-12);
    assert!((output.distance_km - IAU_AU_KM).abs() < 0.001);
    // Equatorial +Y lies south of the ecliptic by Earth's obliquity.
    assert!((output.longitude_deg - 90.0).abs() < 0.1);
    assert!((output.latitude_deg + 23.44).abs() < 0.2);
}

#[test]
fn orientation_of_date_changes_geometric_longitude_without_implying_apparent() {
    let state = geometric([IAU_AU_KM, 0.0, 0.0]);
    let a = project_geocentric_geometric_state(&state, 2_451_545.0).unwrap();
    let b = project_geocentric_geometric_state(&state, 2_460_000.0).unwrap();
    assert!(a.longitude_deg >= 0.0 && a.longitude_deg < 360.0);
    assert!(b.longitude_deg >= 0.0 && b.longitude_deg < 360.0);
    assert!((a.longitude_deg - b.longitude_deg).abs() > 1e-5);
    assert!((a.distance_au - b.distance_au).abs() < 1e-12);
}

#[test]
fn synthetic_geometric_direction_matches_independent_erfa_orientation() {
    // Reference generated with pyerfa 2.0.1.5: pmat06, nut00b, obl06,
    // numat, rx(eps+deps). ICRF axes to true ecliptic/equinox of TT date.
    // Fixed before comparison: 1e-8 degree per angular component. This checks
    // orientation only, not DE441 interpolation or physical model accuracy.
    let input = geometric([
        0.3 * IAU_AU_KM,
        -0.4 * IAU_AU_KM,
        0.8660254037844386 * IAU_AU_KM,
    ]);
    for (tt, longitude, latitude) in [
        (2_415_020.0, 354.359_005_744_462_8, 72.491_601_976_577_58),
        (2_451_545.0, 355.705_486_259_885_7, 72.491_742_162_241_76),
        (2_488_069.0, 357.066_080_400_851_1, 72.491_982_501_326_55),
    ] {
        let actual = project_geocentric_geometric_state(&input, tt).unwrap();
        let lon_error = (actual.longitude_deg - longitude).abs();
        let lat_error = (actual.latitude_deg - latitude).abs();
        println!(
            "TT {tt}: ERFA residual longitude={lon_error:.3e} deg latitude={lat_error:.3e} deg"
        );
        assert!(lon_error < 1e-8 && lat_error < 1e-8);
        assert!((actual.distance_au - 1.0).abs() < 1e-12);
    }
}

#[test]
fn reject_non_geocentric_states_zero_ranges_and_invalid_floats() {
    let mut state = geometric([1.0, 1.0, 1.0]);
    state.center = Body::SolarSystemBarycenter;
    assert!(matches!(
        project_geocentric_geometric_state(&state, 2_451_545.0),
        Err(GeometricDateError::InvalidGeocentricState)
    ));

    state.center = Body::Earth;
    state.target = Body::Earth;
    assert!(matches!(
        project_geocentric_geometric_state(&state, 2_451_545.0),
        Err(GeometricDateError::InvalidGeocentricState)
    ));

    let zero = geometric([0.0, 0.0, 0.0]);
    assert!(matches!(
        project_geocentric_geometric_state(&zero, 2_451_545.0),
        Err(GeometricDateError::UndefinedLongitude)
    ));

    let nonfinite = geometric([f64::NAN, 1.0, 0.0]);
    assert!(matches!(
        project_geocentric_geometric_state(&nonfinite, 2_451_545.0),
        Err(GeometricDateError::NonFinite)
    ));

    // Each input remains finite, but the vector norm necessarily overflows.
    let overflow = geometric([f64::MAX, f64::MAX, f64::MAX]);
    assert!(matches!(
        project_geocentric_geometric_state(&overflow, 2_451_545.0),
        Err(GeometricDateError::NonFinite)
    ));
}

#[test]
fn tt_coverage_is_a_hard_gate_even_when_tdb_is_valid() {
    let state = geometric([IAU_AU_KM, 0.0, 0.0]);
    assert!(matches!(
        project_geocentric_geometric_state(&state, 2_400_000.0),
        Err(GeometricDateError::Orientation(
            OrientationError::UnsupportedEpoch { .. }
        ))
    ));
    assert!(matches!(
        project_geocentric_geometric_state(&state, f64::NAN),
        Err(GeometricDateError::Orientation(
            OrientationError::NonFinite { .. }
        ))
    ));
}

#[test]
fn manager_adapter_requires_a_real_pack_and_never_falls_back() {
    let missing_directory = std::env::temp_dir()
        .join("virgostars-geometric-date-test-no-packs")
        .join(format!("pid-{}", std::process::id()));
    let mut manager = De441PackManager::from_mobile_directory(missing_directory).unwrap();
    let err =
        geocentric_geometric_true_ecliptic_of_date(&mut manager, Body::Sun, J2000_TDB, 2_451_545.0)
            .unwrap_err();
    assert!(matches!(
        err,
        GeometricDateError::Ephemeris(De441PackError::MissingPack { .. })
    ));
}

#[test]
fn invalid_orientation_and_earth_target_stop_before_pack_io() {
    let mut manager = De441PackManager::from_mobile_directory("unread-test-directory").unwrap();
    assert!(matches!(
        geocentric_geometric_true_ecliptic_of_date(
            &mut manager,
            Body::Earth,
            J2000_TDB,
            2_451_545.0,
        ),
        Err(GeometricDateError::InvalidGeocentricState)
    ));
    assert!(matches!(
        geocentric_geometric_true_ecliptic_of_date(&mut manager, Body::Sun, J2000_TDB, 2_400_000.0,),
        Err(GeometricDateError::Orientation(
            OrientationError::UnsupportedEpoch { .. }
        ))
    ));
}

// Modification 2026-11-18: offline, caller-supplied data integration test.
// This tests slice identity with the same source, not independent accuracy.
#[test]
#[ignore = "requires DE441_PATH and DE441_PACKS_DIR with locally held DE441 data"]
fn real_packs_match_source_on_both_sides_of_logical_boundaries() {
    use virgostars_ephemeris_core::de441::De441;
    let source_path = std::env::var_os("DE441_PATH").expect("DE441_PATH");
    let packs_dir = std::env::var_os("DE441_PACKS_DIR").expect("DE441_PACKS_DIR");
    let mut source = De441::open(source_path).unwrap();
    let mut manager = De441PackManager::from_mobile_directory(packs_dir).unwrap();
    let inventory = manager.inventory();
    assert_eq!(inventory.len(), 10);
    assert!(inventory.iter().all(|pack| pack.installed));
    for pack in &inventory {
        let epoch = pack.spec.start_tdb;
        assert_eq!(manager.select_pack(epoch).unwrap().id, pack.spec.id);
        let actual = manager.state(Body::Sun, Body::Earth, epoch).unwrap();
        let expected = source.state(Body::Sun, Body::Earth, epoch).unwrap();
        assert_eq!(actual, expected);
        let before_end = JulianDateTdb(f64::from_bits(pack.spec.end_tdb.0.to_bits() - 1));
        assert_eq!(manager.select_pack(before_end).unwrap().id, pack.spec.id);
        assert_eq!(
            manager.state(Body::Sun, Body::Earth, before_end).unwrap(),
            source.state(Body::Sun, Body::Earth, before_end).unwrap()
        );
    }
    let result =
        geocentric_geometric_true_ecliptic_of_date(&mut manager, Body::Sun, J2000_TDB, 2_451_545.0)
            .unwrap();
    println!(
        "J2000 Sun/Earth geometric true ecliptic: longitude={} deg latitude={} deg distance={} km",
        result.longitude_deg, result.latitude_deg, result.distance_km
    );
}
