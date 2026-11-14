// EXHIBIT A. Common Public Attribution License Version 1.0.
//
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

//! Exhaustive supported-Body geometric state-pair tests for classic DE441.
//!
//! Created 2026-11-14: analytic independent linear XYZ oracles across all
//! 13 public bodies, both coefficient records, and complete pair algebra.
//! Synthetic mathematics only; these are not NASA/JPL/Horizons state vectors.

use std::io::Cursor;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

const START_JD: f64 = 2_451_545.0;
const RECORD_DAYS: f64 = 32.0;
const RECORD_SIZE: usize = 3000;
const EARTH_MOON_RATIO: f64 = 81.0;

const BODIES: [Body; 13] = [
    Body::SolarSystemBarycenter,
    Body::Mercury,
    Body::Venus,
    Body::EarthMoonBarycenter,
    Body::Earth,
    Body::Moon,
    Body::MarsBarycenter,
    Body::JupiterBarycenter,
    Body::SaturnBarycenter,
    Body::UranusBarycenter,
    Body::NeptuneBarycenter,
    Body::PlutoBarycenter,
    Body::Sun,
];

#[derive(Clone, Copy)]
struct ExpectedState {
    position: [f64; 3],
    velocity: [f64; 3],
}

impl ExpectedState {
    const ZERO: Self = Self {
        position: [0.0; 3],
        velocity: [0.0; 3],
    };

    fn relative_to(self, center: Self) -> Self {
        let mut difference = Self::ZERO;
        for axis in 0..3 {
            difference.position[axis] = self.position[axis] - center.position[axis];
            difference.velocity[axis] = self.velocity[axis] - center.velocity[axis];
        }
        difference
    }

    fn add_scaled(self, other: Self, scale: f64) -> Self {
        let mut sum = self;
        for axis in 0..3 {
            sum.position[axis] += other.position[axis] * scale;
            sum.velocity[axis] += other.velocity[axis] * scale;
        }
        sum
    }
}

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn polynomial_coefficients(series: usize, record: usize, axis: usize) -> (f64, f64) {
    // Different values/slopes for each public series and each coefficient
    // record reveal swapped planet indexes, axes and stale record selection.
    let constant = 1_000.0 * (series + 1) as f64
        + 100.0 * axis as f64
        + 10_000.0 * record as f64;
    let linear = 4.0 * (series + 1) as f64 + 2.0 * axis as f64 + 2.0 * record as f64;
    (constant, linear)
}

/// Two synthetic classic-format data records; 11 active 3-axis T0/T1 series.
/// Series 9 holds *geocentric Moon* and series 2 the Earth-Moon barycenter.
/// Pointer 12 is a non-overlapping 3000-byte record-size sentinel.
fn classic_fixture() -> Vec<u8> {
    let mut bytes = vec![0u8; RECORD_SIZE * 4];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    put_f64(&mut bytes, 2652, START_JD);
    put_f64(&mut bytes, 2660, START_JD + 2.0 * RECORD_DAYS);
    put_f64(&mut bytes, 2668, RECORD_DAYS);
    put_f64(&mut bytes, 2688, EARTH_MOON_RATIO);
    put_i32(&mut bytes, 2840, 441);

    for series in 0..=10usize {
        let first = 3 + series * 6;
        let header = 2696 + series * 12;
        put_i32(&mut bytes, header, first as i32);
        put_i32(&mut bytes, header + 4, 2);
        put_i32(&mut bytes, header + 8, 1);
    }
    put_i32(&mut bytes, 2844, 373);
    put_i32(&mut bytes, 2848, 1);
    put_i32(&mut bytes, 2852, 1);

    for record in 0..2usize {
        let base = (record + 2) * RECORD_SIZE;
        let start = START_JD + record as f64 * RECORD_DAYS;
        put_f64(&mut bytes, base, start);
        put_f64(&mut bytes, base + 8, start + RECORD_DAYS);
        for series in 0..=10usize {
            for axis in 0..3usize {
                let first = 3 + series * 6;
                let offset = base + (first - 1 + axis * 2) * 8;
                let (constant, linear) = polynomial_coefficients(series, record, axis);
                put_f64(&mut bytes, offset, constant);
                put_f64(&mut bytes, offset + 8, linear);
            }
        }
    }
    bytes
}

fn series_oracle(series: usize, jd: f64) -> ExpectedState {
    let record = if jd >= START_JD + RECORD_DAYS { 1 } else { 0 };
    let record_start = START_JD + record as f64 * RECORD_DAYS;
    let normalized = 2.0 * (jd - record_start) / RECORD_DAYS - 1.0;
    let mut result = ExpectedState::ZERO;
    for axis in 0..3 {
        let (constant, linear) = polynomial_coefficients(series, record, axis);
        result.position[axis] = constant + linear * normalized;
        result.velocity[axis] = linear / (RECORD_DAYS / 2.0);
    }
    result
}

/// An independent physical oracle: B = (81*Earth + Moon)/82 and
/// lunar series = Moon - Earth, so Earth = B - lunar/82 and
/// Moon = B + 81*lunar/82.
fn body_oracle(body: Body, jd: f64) -> ExpectedState {
    match body {
        Body::SolarSystemBarycenter => ExpectedState::ZERO,
        Body::Mercury => series_oracle(0, jd),
        Body::Venus => series_oracle(1, jd),
        Body::EarthMoonBarycenter => series_oracle(2, jd),
        Body::Earth => series_oracle(2, jd)
            .add_scaled(series_oracle(9, jd), -1.0 / (EARTH_MOON_RATIO + 1.0)),
        Body::Moon => series_oracle(2, jd)
            .add_scaled(series_oracle(9, jd), EARTH_MOON_RATIO / (EARTH_MOON_RATIO + 1.0)),
        Body::MarsBarycenter => series_oracle(3, jd),
        Body::JupiterBarycenter => series_oracle(4, jd),
        Body::SaturnBarycenter => series_oracle(5, jd),
        Body::UranusBarycenter => series_oracle(6, jd),
        Body::NeptuneBarycenter => series_oracle(7, jd),
        Body::PlutoBarycenter => series_oracle(8, jd),
        Body::Sun => series_oracle(10, jd),
    }
}

fn near(actual: f64, expected: f64, tolerance: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{context}: expected {expected:.16e}, got {actual:.16e}, tolerance {tolerance:.2e}"
    );
}

#[test]
fn all_169_supported_target_center_pairs_match_independent_xyz_and_rate_oracles() {
    let mut ephemeris = De441::new(Cursor::new(classic_fixture())).unwrap();
    // Adjacent records are deliberately discontinuous: these samples reveal
    // off-by-one selection at the *exact* record boundary and final endpoint.
    for jd in [
        START_JD,
        START_JD + 8.0,
        START_JD + 16.0,
        START_JD + 31.0,
        START_JD + 32.0,
        START_JD + 48.0,
        START_JD + 63.0,
        START_JD + 64.0,
    ] {
        for target in BODIES {
            for center in BODIES {
                let actual = ephemeris.state(target, center, JulianDateTdb(jd)).unwrap();
                let expected = body_oracle(target, jd).relative_to(body_oracle(center, jd));
                let label = format!("{target:?} relative to {center:?} at TDB JD {jd}");
                assert_eq!(actual.target, target);
                assert_eq!(actual.center, center);
                assert_eq!(actual.epoch, JulianDateTdb(jd));
                for axis in 0..3 {
                    near(actual.position_km[axis], expected.position[axis], 1.0e-9, &label);
                    near(
                        actual.velocity_km_per_day[axis],
                        expected.velocity[axis],
                        1.0e-12,
                        &label,
                    );
                }
            }
        }
    }
}

#[test]
fn full_pair_matrix_is_antisymmetric_reflexive_and_transitive() {
    let mut ephemeris = De441::new(Cursor::new(classic_fixture())).unwrap();
    for jd in [START_JD, START_JD + 31.0, START_JD + 32.0, START_JD + 64.0] {
        let epochs = JulianDateTdb(jd);
        let matrix: Vec<Vec<_>> = BODIES
            .iter()
            .map(|&target| {
                BODIES
                    .iter()
                    .map(|&center| ephemeris.state(target, center, epochs).unwrap())
                    .collect()
            })
            .collect();

        for a in 0..BODIES.len() {
            for b in 0..BODIES.len() {
                for axis in 0..3 {
                    near(matrix[a][a].position_km[axis], 0.0, 0.0, "self position");
                    near(matrix[a][a].velocity_km_per_day[axis], 0.0, 0.0, "self velocity");
                    near(
                        matrix[a][b].position_km[axis],
                        -matrix[b][a].position_km[axis],
                        1.0e-9,
                        "pair position antisymmetry",
                    );
                    near(
                        matrix[a][b].velocity_km_per_day[axis],
                        -matrix[b][a].velocity_km_per_day[axis],
                        1.0e-12,
                        "pair velocity antisymmetry",
                    );
                    for c in 0..BODIES.len() {
                        near(
                            matrix[a][c].position_km[axis],
                            matrix[a][b].position_km[axis] + matrix[b][c].position_km[axis],
                            1.0e-9,
                            "pair position transitivity",
                        );
                        near(
                            matrix[a][c].velocity_km_per_day[axis],
                            matrix[a][b].velocity_km_per_day[axis]
                                + matrix[b][c].velocity_km_per_day[axis],
                            1.0e-12,
                            "pair velocity transitivity",
                        );
                    }
                }
            }
        }
    }
}
