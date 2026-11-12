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

//! Offline integration tests of caller-explicit GCRS→DE441-ICRF frame and
//! UT1/TDB epoch contracts. All binary data/EOP are synthetic, with no
//! claim of physical frame verification or independent JPL parity.

use std::f64::consts::TAU;
use std::io::Cursor;

use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::observer_frame_bridge::{
    wgs84_de441_geometric_sightline_with_alignment, ExplicitGcrsIcrfAlignment,
    ObserverFrameBridgeError,
};
use virgostars_ephemeris_core::observer_gcrs::{
    wgs84_observer_to_gcrs, ObserverGcrsError, ObserverOrientationInputs, Wgs84ObserverSite,
};

const RECORD_BYTES: usize = 3_000;
const EPOCH_TDB: JulianDateTdb = JulianDateTdb(1_016.0);
const SYNTHETIC_TDB_MINUS_UT1_SECONDS: f64 = 69.0;

fn f64_at(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn i32_at(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// A single synthetic classic DE441 coefficient record. Moon's Earth-relative
// position is constant [100000, -50000, 25000] km with zero velocity.
// It is not a NASA/JPL truth record and not a physical Moon epoch.
fn fixture() -> Vec<u8> {
    let mut bytes = vec![0u8; RECORD_BYTES * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    f64_at(&mut bytes, 2652, 1_000.0);
    f64_at(&mut bytes, 2660, 1_032.0);
    f64_at(&mut bytes, 2668, 32.0);
    f64_at(&mut bytes, 2688, 81.0);
    i32_at(&mut bytes, 2840, 441);
    for (series, first) in [(9, 3), (12, 373)] {
        let ptr = if series == 12 { 2844 } else { 2696 + series * 12 };
        i32_at(&mut bytes, ptr, first);
        i32_at(&mut bytes, ptr + 4, 1);
        i32_at(&mut bytes, ptr + 8, 1);
    }
    let offset = 2 * RECORD_BYTES;
    f64_at(&mut bytes, offset, 1_000.0);
    f64_at(&mut bytes, offset + 8, 1_032.0);
    for (i, value) in [100_000.0, -50_000.0, 25_000.0].into_iter().enumerate() {
        f64_at(&mut bytes, offset + (2 + i) * 8, value);
    }
    bytes
}

fn site() -> Wgs84ObserverSite {
    Wgs84ObserverSite {
        longitude_rad_east: 0.0,
        latitude_rad_north: 0.0,
        ellipsoidal_height_m: 0.0,
    }
}

fn orientation() -> ObserverOrientationInputs {
    let ut1_epoch = JulianDate::from_f64(
        EPOCH_TDB.0 - SYNTHETIC_TDB_MINUS_UT1_SECONDS / 86_400.0,
        TimeScale::Ut1,
    )
    .unwrap();
    ObserverOrientationInputs {
        ut1_epoch,
        polar_xp_rad: 0.0,
        polar_yp_rad: 0.0,
        tio_locator_sp_rad: 0.0,
        cip_x: 0.0,
        cip_y: 0.0,
        cio_locator_s_rad: 0.0,
    }
}

fn rotation(angle_rad: f64) -> [[f64; 3]; 3] {
    let (s, c) = angle_rad.sin_cos();
    [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
}

fn alignment<'a>(label: &'a str) -> ExplicitGcrsIcrfAlignment<'a> {
    ExplicitGcrsIcrfAlignment {
        rotation_gcrs_to_icrf: rotation(0.31),
        tdb_minus_ut1_seconds: SYNTHETIC_TDB_MINUS_UT1_SECONDS,
        source_label: label,
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}; tolerance {tolerance}"
    );
}

#[test]
fn explicit_non_identity_frame_rotation_composes_real_reader_and_site_pipeline() {
    let mut ephemeris = De441::new(Cursor::new(fixture())).unwrap();
    let orientation = orientation();
    let result = wgs84_de441_geometric_sightline_with_alignment(
        &mut ephemeris,
        Body::Moon,
        site(),
        orientation,
        EPOCH_TDB,
        alignment("synthetic-analytic-z-rotation-0.31"),
    )
    .unwrap();

    let site_gcrs = wgs84_observer_to_gcrs(site(), orientation).unwrap();
    assert_eq!(result.observer_gcrs, site_gcrs);
    let [x, y, z] = site_gcrs.position_gcrs_km;
    let (s, c) = 0.31_f64.sin_cos();
    let expected_observer = [c * x - s * y, s * x + c * y, z];
    for (actual, expected) in result.observer_icrf_km.into_iter().zip(expected_observer) {
        near(actual, expected, 2.0e-12);
    }

    assert_eq!(result.sightline_icrf.target, Body::Moon);
    assert_eq!(result.sightline_icrf.epoch_tdb, EPOCH_TDB);
    assert_eq!(
        result.sightline_icrf.target_geocentric_icrf_km,
        [100_000.0, -50_000.0, 25_000.0]
    );
    let offset: [f64; 3] = std::array::from_fn(|i| {
        [100_000.0, -50_000.0, 25_000.0][i] - expected_observer[i]
    });
    for (actual, expected) in result.sightline_icrf.displacement_icrf_km.into_iter().zip(offset) {
        near(actual, expected, 2.0e-11);
    }
    let expected_distance = offset[0].hypot(offset[1]).hypot(offset[2]);
    near(result.sightline_icrf.range_km, expected_distance, 1.0e-10);
    near(
        result.sightline_icrf.right_ascension_icrf_rad.unwrap(),
        offset[1].atan2(offset[0]).rem_euclid(TAU),
        1.0e-14,
    );
}

#[test]
fn tdb_ut1_offset_guard_rejects_mismatched_epochs() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    let mut inputs = alignment("synthetic-input");
    inputs.tdb_minus_ut1_seconds += 0.01;
    assert!(matches!(
        wgs84_de441_geometric_sightline_with_alignment(
            &mut eph, Body::Moon, site(), orientation(), EPOCH_TDB, inputs,
        ),
        Err(ObserverFrameBridgeError::EpochMismatch)
    ));
    assert!(matches!(
        wgs84_de441_geometric_sightline_with_alignment(
            &mut eph, Body::Moon, site(), orientation(), JulianDateTdb(1_017.0),
            alignment("synthetic-input"),
        ),
        Err(ObserverFrameBridgeError::EpochMismatch)
    ));
}

#[test]
fn rejects_unjustified_missing_alignment_label() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    for label in ["", " ", "\n\t"] {
        assert!(matches!(
            wgs84_de441_geometric_sightline_with_alignment(
                &mut eph, Body::Moon, site(), orientation(), EPOCH_TDB, alignment(label),
            ),
            Err(ObserverFrameBridgeError::MissingAlignmentSource)
        ));
    }
}

#[test]
fn rejects_reflections_shears_nonfinite_and_nonorthogonal_frame_matrices() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    let reflection = [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let shear = [[1.0, 0.1, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let nonfinite = [[f64::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for invalid in [reflection, shear, nonfinite] {
        let mut frame = alignment("synthetic-invalid-matrix");
        frame.rotation_gcrs_to_icrf = invalid;
        assert!(matches!(
            wgs84_de441_geometric_sightline_with_alignment(
                &mut eph, Body::Moon, site(), orientation(), EPOCH_TDB, frame,
            ),
            Err(ObserverFrameBridgeError::InvalidAlignmentRotation)
        ));
    }
}

#[test]
fn rejects_nonfinite_tdb_offset_and_time_coordinate() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut frame = alignment("synthetic-invalid-time");
        frame.tdb_minus_ut1_seconds = invalid;
        assert!(matches!(
            wgs84_de441_geometric_sightline_with_alignment(
                &mut eph, Body::Moon, site(), orientation(), EPOCH_TDB, frame,
            ),
            Err(ObserverFrameBridgeError::InvalidTdbMinusUt1Offset)
        ));
        assert!(matches!(
            wgs84_de441_geometric_sightline_with_alignment(
                &mut eph, Body::Moon, site(), orientation(), JulianDateTdb(invalid),
                alignment("synthetic-input"),
            ),
            Err(ObserverFrameBridgeError::InvalidTdbEpoch)
        ));
    }
}

#[test]
fn rejects_tt_labeled_ut1_instead_of_silently_converting() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    let mut inputs = orientation();
    inputs.ut1_epoch = JulianDate::from_f64(
        EPOCH_TDB.0 - SYNTHETIC_TDB_MINUS_UT1_SECONDS / 86_400.0,
        TimeScale::Tt,
    )
    .unwrap();
    assert!(matches!(
        wgs84_de441_geometric_sightline_with_alignment(
            &mut eph, Body::Moon, site(), inputs, EPOCH_TDB, alignment("synthetic-tt-error"),
        ),
        Err(ObserverFrameBridgeError::Observer(ObserverGcrsError::EarthRotation(_)))
    ));
}

#[test]
fn de441_target_errors_propagate_without_substituting_a_barycenter() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    assert!(matches!(
        wgs84_de441_geometric_sightline_with_alignment(
            &mut eph, Body::MarsBarycenter, site(), orientation(), EPOCH_TDB,
            alignment("synthetic-missing-planet"),
        ),
        Err(ObserverFrameBridgeError::De441(_))
    ));
}

#[test]
fn caller_can_explicitly_request_synthetic_identity_without_implicit_default() {
    let mut eph = De441::new(Cursor::new(fixture())).unwrap();
    let mut frame = alignment("explicit-identity-only-for-synthetic-fixture");
    frame.rotation_gcrs_to_icrf =
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let result = wgs84_de441_geometric_sightline_with_alignment(
        &mut eph, Body::Moon, site(), orientation(), EPOCH_TDB, frame,
    )
    .unwrap();
    assert_eq!(result.observer_icrf_km, result.observer_gcrs.position_gcrs_km);
}
