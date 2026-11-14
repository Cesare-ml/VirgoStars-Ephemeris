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
*/

//! Synthetic regression for TDB Julian-day granule boundary resolution.
//! This checks binary64 parser behavior, not JPL scientific accuracy.

use std::io::Cursor;
use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};

const RECORD_BYTES: usize = 3000;

fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn fixture(start: f64, step: f64, granules: i32) -> Vec<u8> {
    // Two classic header records plus one data record; Mercury has one
    // coefficient per axis per granule. Libration pads to 3000 bytes.
    let mut bytes = vec![0u8; RECORD_BYTES * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    put_f64(&mut bytes, 2652, start);
    put_f64(&mut bytes, 2660, start + step);
    put_f64(&mut bytes, 2668, step);
    put_f64(&mut bytes, 2688, 81.0);
    put_i32(&mut bytes, 2840, 441);

    put_i32(&mut bytes, 2696, 3);
    put_i32(&mut bytes, 2700, 1);
    put_i32(&mut bytes, 2704, granules);
    put_i32(&mut bytes, 2844, 373);
    put_i32(&mut bytes, 2848, 1);
    put_i32(&mut bytes, 2852, 1);

    put_f64(&mut bytes, RECORD_BYTES * 2, start);
    put_f64(&mut bytes, RECORD_BYTES * 2 + 8, start + step);
    bytes
}

fn assert_unresolvable(bytes: Vec<u8>) {
    assert!(matches!(
        De441::new(Cursor::new(bytes)),
        Err(De441Error::InvalidFile(
            "coefficient granule dates exceed f64 JD resolution"
        ))
    ));
}

#[test]
fn rejects_underflowed_subrecord_width_even_when_header_coverage_is_finite() {
    let smallest_subnormal_day = f64::from_bits(1);
    assert_unresolvable(fixture(0.0, smallest_subnormal_day, 2));
}

#[test]
fn rejects_two_adjacent_granules_collapsing_to_one_julian_date() {
    // At this large JD, the two-ULP principal record can hold only three
    // distinct binary64 dates, insufficient for three distinct granules.
    let start: f64 = 2_451_545.0;
    let two_ulps = f64::from_bits(start.to_bits() + 2) - start;
    assert_unresolvable(fixture(start, two_ulps, 3));
}

#[test]
fn accepts_distinguishable_granules_at_both_small_and_classic_scales() {
    let start: f64 = 2_451_545.0;
    let two_ulps = f64::from_bits(start.to_bits() + 2) - start;
    for (jd, step, granules) in [(start, two_ulps, 2), (1000.0, 32.0, 2)] {
        let mut eph = De441::new(Cursor::new(fixture(jd, step, granules)))
            .expect("every granule boundary has a distinct binary64 Julian date");
        for epoch in [jd, jd + step] {
            let state = eph
                .state(
                    Body::Mercury,
                    Body::SolarSystemBarycenter,
                    JulianDateTdb(epoch),
                )
                .expect("synthetic zero coefficients yield a finite state");
            assert_eq!(state.position_km, [0.0; 3]);
            assert_eq!(state.velocity_km_per_day, [0.0; 3]);
        }
    }
}

#[test]
fn accepts_single_granule_at_one_ulp_time_step() {
    let start: f64 = 2_451_545.0;
    let one_ulp = f64::from_bits(start.to_bits() + 1) - start;
    let eph = De441::new(Cursor::new(fixture(start, one_ulp, 1)))
        .expect("single granule has distinguishable start/end");
    assert_eq!(
        eph.coverage(),
        (JulianDateTdb(start), JulianDateTdb(start + one_ulp))
    );
}

fn two_record_fixture(start: f64, step: f64, granules: i32) -> Vec<u8> {
    let mut bytes = fixture(start, step, granules);
    // Duplicate the synthetic first coefficient record, update the
    // second record's time tags and the header's final coverage date.
    let record = bytes[RECORD_BYTES * 2..RECORD_BYTES * 3].to_vec();
    bytes.extend_from_slice(&record);
    put_f64(&mut bytes, 2660, start + 2.0 * step);
    put_f64(&mut bytes, RECORD_BYTES * 3, start + step);
    put_f64(&mut bytes, RECORD_BYTES * 3 + 8, start + 2.0 * step);
    bytes
}

#[test]
fn rejects_granule_collapse_only_in_last_principal_record() {
    // Binary64 spacing doubles at 2^20 JD. The first record has four
    // representable half-ULPs, enough for three granules. The second
    // has only two full ULPs, so its internal boundaries collide.
    let pivot = 1_048_576.0_f64;
    let higher_ulp = f64::from_bits(pivot.to_bits() + 1) - pivot;
    let step = 2.0 * higher_ulp;
    let start = pivot - step;
    assert!(
        De441::new(Cursor::new(fixture(start, step, 3))).is_ok(),
        "the first record alone must have distinct granule boundaries"
    );
    assert_unresolvable(two_record_fixture(start, step, 3));
}

#[test]
fn valid_granule_boundary_selects_next_series_and_final_endpoint_is_inclusive() {
    let mut bytes = fixture(1000.0, 32.0, 2);
    // Each Mercury T0 coefficient stores a unique (granule, axis)
    // marker. Checking the actual public state API proves that a
    // boundary selects the following granule and the last endpoint
    // selects the preceding one, not a zero-duration phantom record.
    for granule in 0..2 {
        for axis in 0..3 {
            let offset = RECORD_BYTES * 2 + (2 + granule * 3 + axis) * 8;
            put_f64(&mut bytes, offset, (granule * 100 + axis) as f64);
        }
    }
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    let immediately_before = f64::from_bits(1016.0_f64.to_bits() - 1);
    for (epoch, selected_granule) in [
        (1000.0, 0),
        (immediately_before, 0),
        (1016.0, 1),
        (1032.0, 1),
    ] {
        let state = eph
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(epoch),
            )
            .expect("synthetic constant coefficients must interpolate");
        for axis in 0..3 {
            assert_eq!(
                state.position_km[axis],
                (selected_granule * 100 + axis) as f64
            );
            assert_eq!(state.velocity_km_per_day[axis], 0.0);
        }
    }
}

#[test]
fn rounded_internal_boundaries_select_following_granules_across_tdb_scales() {
    // Modification 2026-11-14: floor((jd - record_start) / width) is
    // sometimes one index too small at a representable boundary. For
    // instance, jd = 1 + 1/3 rounded to f64 yields a quotient below 1.
    // The parser must instead use the exact same rounded boundaries
    // that it checked in De441::new.
    for (start, step, granules) in [
        (1.0, 1.0, 3),
        (2_451_545.0, 1.0, 3),
        (-2_451_545.0, 1.0, 3),
        (0.0, 1.0, 9),
        (1000.0, 32.0, 97),
    ] {
        let mut bytes = fixture(start, step, granules);
        let width = step / f64::from(granules);
        for granule in 0..granules as usize {
            for axis in 0..3 {
                let at = RECORD_BYTES * 2 + (2 + 3 * granule + axis) * 8;
                put_f64(&mut bytes, at, (100 * granule + axis + 1) as f64);
            }
        }
        let mut eph = De441::new(Cursor::new(bytes))
            .expect("each synthetic granule boundary must be representable");
        for granule in 0..granules as usize {
            let boundary = start + granule as f64 * width;
            let state = eph
                .state(
                    Body::Mercury,
                    Body::SolarSystemBarycenter,
                    JulianDateTdb(boundary),
                )
                .expect("boundary state must select its following granule");
            for axis in 0..3 {
                assert_eq!(
                    state.position_km[axis],
                    (100 * granule + axis + 1) as f64,
                    "JD {boundary}, granule {granule}, axis {axis}"
                );
                assert_eq!(state.velocity_km_per_day[axis], 0.0);
            }
            if granule > 0 {
                let before = if boundary > 0.0 {
                    f64::from_bits(boundary.to_bits() - 1)
                } else {
                    f64::from_bits(boundary.to_bits() + 1)
                };
                assert!(before < boundary && before >= start);
                let previous = eph
                    .state(
                        Body::Mercury,
                        Body::SolarSystemBarycenter,
                        JulianDateTdb(before),
                    )
                    .expect("immediately preceding epoch must select previous granule");
                for (axis, position) in previous.position_km.iter().enumerate() {
                    assert_eq!(
                        *position,
                        (100 * (granule - 1) + axis + 1) as f64,
                        "JD {before}, preceding granule {}, axis {axis}",
                        granule - 1
                    );
                }
            }
        }
        let final_epoch = start + step;
        let final_state = eph
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(final_epoch),
            )
            .expect("inclusive final endpoint must select last granule");
        assert_eq!(
            final_state.position_km[0],
            (100 * (granules as usize - 1) + 1) as f64
        );
    }
}

#[test]
fn wide_tdb_record_normalization_does_not_overflow_before_division() {
    // Modification 2026-11-14: for a one-granule record of width 1e308
    // days, the Chebyshev x at the inclusive end is mathematically +1.
    // Multiplying the day offset by two before dividing overflows f64;
    // the correct polynomial value and derivative are still finite.
    let step = 1.0e308;
    let mut bytes = fixture(0.0, step, 1);
    // Two coefficients per axis: constant + linear Chebyshev T1.
    put_i32(&mut bytes, 2700, 2);
    for axis in 0..3usize {
        let start = RECORD_BYTES * 2 + (2 + axis * 2) * 8;
        put_f64(&mut bytes, start, 10.0 + axis as f64);
        put_f64(&mut bytes, start + 8, axis as f64 + 1.0);
    }
    let mut eph = De441::new(Cursor::new(bytes)).unwrap();
    for (epoch, normalized_x) in [(0.0, -1.0), (step / 2.0, 0.0), (step, 1.0)] {
        let state = eph
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(epoch),
            )
            .expect("mathematically finite Chebyshev coordinate and velocity");
        for (axis, position) in state.position_km.iter().enumerate() {
            let expected = 10.0 + axis as f64 + (axis as f64 + 1.0) * normalized_x;
            assert_eq!(*position, expected, "epoch {epoch}, axis {axis}");
            let expected_velocity = (axis as f64 + 1.0) * 2.0 / step;
            // The equally valid overflow-safe scaling in PR #178 can
            // differ by one least-subnormal ULP from multiply-then-divide.
            // Require the analytic velocity within that binary64 bound.
            let error = (state.velocity_km_per_day[axis] - expected_velocity).abs();
            assert!(
                error <= f64::from_bits(1),
                "epoch {epoch}, axis {axis}: velocity error {error}"
            );
        }
    }
}
