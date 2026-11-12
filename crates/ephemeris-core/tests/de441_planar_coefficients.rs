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

//! Synthetic classic DE441 byte fixture exercising the real reader path.
//! No NASA/JPL dataset, official numerical fixture or third-party file used.
//! Tests authored 2026-11-12 by VirgoStars contributors, CPAL-1.0.

use std::io::Cursor;

use virgostars_ephemeris_core::chebyshev::evaluate_chebyshev;
use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};
use virgostars_ephemeris_core::de441_granule_plan::{
    plan_de441_relative_granules, ActiveSeriesGranule,
};

const WORDS_PER_RECORD: usize = 441;
const RECORD_BYTES: usize = WORDS_PER_RECORD * 8;
const RECORD_OFFSET: usize = RECORD_BYTES * 2;

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn pointer(bytes: &mut [u8], index: usize, first_word: i32, degree_count: i32, granules: i32) {
    let offset = 2696 + index * 12;
    put_i32(bytes, offset, first_word);
    put_i32(bytes, offset + 4, degree_count);
    put_i32(bytes, offset + 8, granules);
}
fn series_coefficients(
    bytes: &mut [u8], first_word: usize, coefficient_count: usize,
    granule: usize, axis: usize, coefficients: &[f64],
) {
    assert_eq!(coefficients.len(), coefficient_count);
    let word = first_word - 1 + (granule * 3 + axis) * coefficient_count;
    for (k, &c) in coefficients.iter().enumerate() {
        put_f64(bytes, RECORD_OFFSET + (word + k) * 8, c);
    }
}

/// A valid classic binary *format*, with intentionally artificial positions.
/// The one principal record runs JD 1000..1002; Mars and Moon each have two
/// subrecords, while the Earth–Moon barycenter has one.
fn fixture() -> Vec<u8> {
    let mut bytes = vec![0_u8; RECORD_BYTES * 3];
    let name = b"JPL Planetary Ephemeris DE441";
    bytes[..name.len()].copy_from_slice(name);
    put_f64(&mut bytes, 2652, 1000.0);
    put_f64(&mut bytes, 2660, 1002.0);
    put_f64(&mut bytes, 2668, 2.0);
    put_f64(&mut bytes, 2688, 81.30056);
    put_i32(&mut bytes, 2840, 441);
    pointer(&mut bytes, 2, 380, 1, 1); // Earth–Moon barycenter
    pointer(&mut bytes, 3, 400, 2, 2); // Mars barycenter
    pointer(&mut bytes, 9, 430, 2, 2); // geocentric Moon
    put_f64(&mut bytes, RECORD_OFFSET, 1000.0);
    put_f64(&mut bytes, RECORD_OFFSET + 8, 1002.0);
    for axis in 0..3 {
        series_coefficients(&mut bytes, 380, 1, 0, axis,
            &[if axis == 0 { 90.0 } else { 0.0 }]);
    }
    for granule in 0..2 {
        let (x, y) = if granule == 0 {
            ([100.0, 2.0], [0.0, 1.0])
        } else {
            ([110.0, 3.0], [2.0, -1.0])
        };
        series_coefficients(&mut bytes, 400, 2, granule, 0, &x);
        series_coefficients(&mut bytes, 400, 2, granule, 1, &y);
        series_coefficients(&mut bytes, 400, 2, granule, 2, &[5.0, 0.0]);
        series_coefficients(&mut bytes, 430, 2, granule, 0, &[0.5, 0.0]);
        series_coefficients(&mut bytes, 430, 2, granule, 1, &[0.1, 0.0]);
        series_coefficients(&mut bytes, 430, 2, granule, 2, &[0.0, 0.0]);
    }
    bytes
}

fn selection(series_index: usize, subrecord_index: usize) -> ActiveSeriesGranule {
    ActiveSeriesGranule {
        series_index,
        principal_record_index: 0,
        subrecord_index,
    }
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1.0e-11,
        "expected {expected}, got {actual}");
}

#[test]
fn synthetic_header_is_accepted_and_yields_real_planned_subrecords() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    let plan = plan_de441_relative_granules(
        &reader.granule_layout(), Body::MarsBarycenter, Body::Earth,
        1000.0, 1002.0,
    ).unwrap();
    assert_eq!(plan.required_series_indices, vec![2, 3, 9]);
    assert_eq!(plan.slices.len(), 2);
    assert_eq!(plan.slices[0].start_jd_tdb, 1000.0);
    assert_eq!(plan.slices[0].end_jd_tdb, 1001.0);

    let mars = plan.slices[0].active_granules.iter()
        .copied().find(|s| s.series_index == 3).unwrap();
    let data = reader.planar_coefficients_for_granule(
        mars, JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ).unwrap();
    assert_eq!(data.selection, mars);
    assert_eq!(data.x_coefficients_km, vec![100.0, 2.0]);
    assert_eq!(data.y_coefficients_km, vec![0.0, 1.0]);
    assert_eq!(data.start_jd_tdb, JulianDateTdb(1000.0));
    assert_eq!(data.end_jd_tdb, JulianDateTdb(1001.0));
    near(data.midpoint_jd_tdb.0, 1000.5);
    near(data.radius_days, 0.5);
}

#[test]
fn extracted_polynomial_matches_existing_geometric_state_evaluator() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    let sample = 1000.25;
    let data = reader.planar_coefficients_for_granule(
        selection(3, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ).unwrap();
    let normalized = (sample - data.midpoint_jd_tdb.0) / data.radius_days;
    let x = evaluate_chebyshev(&data.x_coefficients_km, normalized).unwrap();
    let y = evaluate_chebyshev(&data.y_coefficients_km, normalized).unwrap();
    let state = reader.state(
        Body::MarsBarycenter, Body::SolarSystemBarycenter,
        JulianDateTdb(sample),
    ).unwrap();
    near(x.value, state.position_km[0]);
    near(y.value, state.position_km[1]);
    near(x.derivative_dx / data.radius_days, state.velocity_km_per_day[0]);
    near(y.derivative_dx / data.radius_days, state.velocity_km_per_day[1]);
}

#[test]
fn extracted_emb_and_moon_granules_reconstruct_existing_mars_earth_state() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    let epoch = 1000.25;
    let mut planar = Vec::new();
    for index in [2, 3, 9] {
        let data = reader.planar_coefficients_for_granule(
            selection(index, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
        ).unwrap();
        let normalized = (epoch - data.midpoint_jd_tdb.0) / data.radius_days;
        let x = evaluate_chebyshev(&data.x_coefficients_km, normalized).unwrap().value;
        let y = evaluate_chebyshev(&data.y_coefficients_km, normalized).unwrap().value;
        planar.push((x, y));
    }
    let ratio = 81.30056_f64;
    let expected_x = planar[1].0 - planar[0].0 + planar[2].0 / (1.0 + ratio);
    let expected_y = planar[1].1 - planar[0].1 + planar[2].1 / (1.0 + ratio);
    let result = reader.state(
        Body::MarsBarycenter, Body::Earth, JulianDateTdb(epoch),
    ).unwrap();
    near(result.position_km[0], expected_x);
    near(result.position_km[1], expected_y);
}

#[test]
fn consecutive_granules_read_distinct_coefficient_ranges_with_shared_cache() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    let one = reader.planar_coefficients_for_granule(
        selection(3, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ).unwrap();
    let two = reader.planar_coefficients_for_granule(
        selection(3, 1), JulianDateTdb(1001.0), JulianDateTdb(1002.0),
    ).unwrap();
    assert_eq!(one.x_coefficients_km, vec![100.0, 2.0]);
    assert_eq!(two.x_coefficients_km, vec![110.0, 3.0]);
    assert_eq!(two.y_coefficients_km, vec![2.0, -1.0]);
    let again = reader.planar_coefficients_for_granule(
        selection(3, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ).unwrap();
    assert_eq!(again, one);
}

#[test]
fn earth_moon_barycenter_and_geocentric_moon_are_extracted_separately() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    let emb = reader.planar_coefficients_for_granule(
        selection(2, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ).unwrap();
    let moon_geo = reader.planar_coefficients_for_granule(
        selection(9, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ).unwrap();
    assert_eq!(emb.x_coefficients_km, vec![90.0]);
    assert_eq!(moon_geo.x_coefficients_km, vec![0.5, 0.0]);
    // This API intentionally does NOT assert a composite Earth ephemeris.
    assert_ne!(emb.selection.series_index, moon_geo.selection.series_index);
}

#[test]
fn missing_series_and_nonpositional_series_are_rejected() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    assert!(matches!(reader.planar_coefficients_for_granule(
        selection(0, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ), Err(De441Error::MissingSeries(_))));
    for idx in [11, 12, usize::MAX] {
        assert!(matches!(reader.planar_coefficients_for_granule(
            selection(idx, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
        ), Err(De441Error::InvalidFile(_))));
    }
}

#[test]
fn invalid_selection_and_cross_granule_requests_fail_closed() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    let invalid = [
        selection(3, 2),
        ActiveSeriesGranule {
            series_index: 3, principal_record_index: 1, subrecord_index: 0,
        },
    ];
    for bad in invalid {
        assert!(matches!(reader.planar_coefficients_for_granule(
            bad, JulianDateTdb(1000.0), JulianDateTdb(1001.0),
        ), Err(De441Error::InvalidFile(_))));
    }
    for (start, end) in [
        (999.99, 1000.5),
        (1000.0, 1001.5),
        (1000.5, 1000.5),
        (f64::NAN, 1001.0),
        (1000.0, f64::INFINITY),
    ] {
        assert!(matches!(reader.planar_coefficients_for_granule(
            selection(3, 0), JulianDateTdb(start), JulianDateTdb(end),
        ), Err(De441Error::InvalidFile(_))));
    }
}

#[test]
fn corrupt_coefficient_and_corrupt_record_dates_are_not_cached_as_success() {
    let mut bad = fixture();
    // Index 3, granule 0, x coefficient 0: first word 400.
    put_f64(&mut bad, RECORD_OFFSET + (400 - 1) * 8, f64::NAN);
    let mut reader = De441::new(Cursor::new(bad)).unwrap();
    for _ in 0..2 {
        assert!(matches!(reader.planar_coefficients_for_granule(
            selection(3, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
        ), Err(De441Error::InvalidFile(_))));
    }

    let mut bad_record = fixture();
    put_f64(&mut bad_record, RECORD_OFFSET + 8, 1001.75);
    let mut reader = De441::new(Cursor::new(bad_record)).unwrap();
    assert!(matches!(reader.planar_coefficients_for_granule(
        selection(3, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ), Err(De441Error::InvalidFile(_))));
    assert!(matches!(reader.state(
        Body::MarsBarycenter, Body::SolarSystemBarycenter,
        JulianDateTdb(1000.25),
    ), Err(De441Error::InvalidFile(_))));
}


#[test]
fn cartesian_xyz_coefficients_match_all_geometric_position_and_velocity_axes() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    for (granule, jd, expected_x, expected_y) in [
        (0, 1000.25_f64, vec![100.0, 2.0], vec![0.0, 1.0]),
        (1, 1001.25_f64, vec![110.0, 3.0], vec![2.0, -1.0]),
    ] {
        let selected = selection(3, granule);
        let start = 1000.0 + granule as f64;
        let record = reader.cartesian_coefficients_for_granule(
            selected, JulianDateTdb(start), JulianDateTdb(start + 1.0),
        ).unwrap();
        assert_eq!(record.selection, selected);
        assert_eq!(record.xyz_coefficients_km[0], expected_x);
        assert_eq!(record.xyz_coefficients_km[1], expected_y);
        assert_eq!(record.xyz_coefficients_km[2], vec![5.0, 0.0]);
        near(record.midpoint_jd_tdb.0, start + 0.5);
        near(record.radius_days, 0.5);
        let tau = (jd - record.midpoint_jd_tdb.0) / record.radius_days;
        let state = reader.state(
            Body::MarsBarycenter,
            Body::SolarSystemBarycenter,
            JulianDateTdb(jd),
        ).unwrap();
        for axis in 0..3 {
            let s = evaluate_chebyshev(&record.xyz_coefficients_km[axis], tau).unwrap();
            near(s.value, state.position_km[axis]);
            near(s.derivative_dx / record.radius_days, state.velocity_km_per_day[axis]);
        }
    }
}

#[test]
fn all_three_axes_reconstruct_mars_relative_to_earth_using_real_source_series() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    for jd in [1000.25_f64, 1001.75_f64] {
        let granule = if jd < 1001.0 { 0 } else { 1 };
        let left = 1000.0 + granule as f64;
        let mut samples = [[(0.0_f64, 0.0_f64); 3]; 3];
        for (body_index, series_index) in [2, 3, 9].iter().enumerate() {
            let data = reader.cartesian_coefficients_for_granule(
                selection(*series_index, if *series_index == 2 { 0 } else { granule }),
                JulianDateTdb(left), JulianDateTdb(left + 1.0),
            ).unwrap();
            let tau = (jd - data.midpoint_jd_tdb.0) / data.radius_days;
            for (axis, coefficients) in data.xyz_coefficients_km.iter().enumerate() {
                let value = evaluate_chebyshev(coefficients, tau).unwrap();
                samples[body_index][axis] =
                    (value.value, value.derivative_dx / data.radius_days);
            }
        }

        let state = reader.state(
            Body::MarsBarycenter, Body::Earth, JulianDateTdb(jd),
        ).unwrap();
        let earth_moon_mass_ratio = 81.30056_f64;
        for axis in 0..3 {
            let predicted_position = samples[1][axis].0 - samples[0][axis].0
                + samples[2][axis].0 / (1.0 + earth_moon_mass_ratio);
            let predicted_velocity = samples[1][axis].1 - samples[0][axis].1
                + samples[2][axis].1 / (1.0 + earth_moon_mass_ratio);
            near(predicted_position, state.position_km[axis]);
            near(predicted_velocity, state.velocity_km_per_day[axis]);
        }
    }
}

#[test]
fn xyz_and_legacy_xy_extraction_use_same_granule_and_cache() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    let xyz = reader.cartesian_coefficients_for_granule(
        selection(3, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ).unwrap();
    let xy = reader.planar_coefficients_for_granule(
        selection(3, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ).unwrap();
    assert_eq!(xy.selection, xyz.selection);
    assert_eq!(xy.start_jd_tdb, xyz.start_jd_tdb);
    assert_eq!(xy.end_jd_tdb, xyz.end_jd_tdb);
    assert_eq!(xy.midpoint_jd_tdb, xyz.midpoint_jd_tdb);
    assert_eq!(xy.radius_days, xyz.radius_days);
    assert_eq!(xy.x_coefficients_km, xyz.xyz_coefficients_km[0]);
    assert_eq!(xy.y_coefficients_km, xyz.xyz_coefficients_km[1]);
}

#[test]
fn malformed_z_is_rejected_by_both_xyz_and_legacy_planar_extraction() {
    let mut bytes = fixture();
    // Mars series 3 starts at 1-based word 400 with two coefficients.
    // Granule 0 Z starts after the first four X/Y words.
    let first_z_word = 400 - 1 + 2 * 2;
    put_f64(&mut bytes, RECORD_OFFSET + first_z_word * 8, f64::NAN);
    let mut reader = De441::new(Cursor::new(bytes)).unwrap();

    assert!(matches!(reader.cartesian_coefficients_for_granule(
        selection(3, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ), Err(De441Error::InvalidFile(_))));
    assert!(matches!(reader.planar_coefficients_for_granule(
        selection(3, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ), Err(De441Error::InvalidFile(_))));
    // A different noncorrupt source series in the same principal record
    // remains readable and is unaffected by a failed axis extraction.
    let emb = reader.cartesian_coefficients_for_granule(
        selection(2, 0), JulianDateTdb(1000.0), JulianDateTdb(1001.0),
    ).unwrap();
    assert_eq!(emb.xyz_coefficients_km[0], vec![90.0]);
}

#[test]
fn xyz_extractor_rejects_uncovered_granules_and_unsupported_series() {
    let mut reader = De441::new(Cursor::new(fixture())).unwrap();
    let invalid_windows = [
        (selection(3, 0), 1000.0, 1001.5),
        (selection(3, 1), 1000.0, 1001.0),
        (selection(3, 0), f64::NAN, 1001.0),
        (selection(3, 0), 1000.0, 1000.0),
        (selection(3, 0), 1000.0, f64::INFINITY),
        (selection(3, 10), 1000.0, 1001.0),
        (selection(11, 0), 1000.0, 1001.0),
    ];
    for (selection, start, end) in invalid_windows {
        assert!(reader.cartesian_coefficients_for_granule(
            selection, JulianDateTdb(start), JulianDateTdb(end),
        ).is_err());
    }
}
