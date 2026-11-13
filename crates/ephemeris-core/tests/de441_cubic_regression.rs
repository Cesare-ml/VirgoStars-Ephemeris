/* EXHIBIT A. Common Public Attribution License Version 1.0.
The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0.
The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party materials licensed separately.
The Original Developer and Initial Developer is Cesare-ml (VirgoStars project). Copyright (c) 2026 Cesare-ml. All Rights Reserved.
Contributors retain applicable rights in their own contributions under the License.
SPDX-License-Identifier: CPAL-1.0
*/

use std::io::Cursor;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

const RECORD_BYTES: usize = 3000;

fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn cubic_fixture() -> Vec<u8> {
    let mut bytes = vec![0u8; RECORD_BYTES * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    write_f64(&mut bytes, 2652, 1000.0);
    write_f64(&mut bytes, 2660, 1032.0);
    write_f64(&mut bytes, 2668, 32.0);
    write_f64(&mut bytes, 2688, 81.0);
    write_i32(&mut bytes, 2840, 441);

    // Mercury: four Chebyshev coefficients per axis, two 16-day granules.
    write_i32(&mut bytes, 2696, 3);
    write_i32(&mut bytes, 2700, 4);
    write_i32(&mut bytes, 2704, 2);
    // Unused libration pointer fixes the synthetic record length at 3000 bytes.
    write_i32(&mut bytes, 2844, 373);
    write_i32(&mut bytes, 2848, 1);
    write_i32(&mut bytes, 2852, 1);

    write_f64(&mut bytes, RECORD_BYTES * 2, 1000.0);
    write_f64(&mut bytes, RECORD_BYTES * 2 + 8, 1032.0);

    for (granule, coefficients) in [[10.0, 2.0, 3.0, 4.0], [20.0, -1.0, 0.0, 2.0]]
        .into_iter()
        .enumerate()
    {
        let offset = RECORD_BYTES * 2 + (2 + granule * 12) * 8;
        for (index, coefficient) in coefficients.into_iter().enumerate() {
            write_f64(&mut bytes, offset + index * 8, coefficient);
        }
    }
    bytes
}

#[test]
fn cubic_chebyshev_position_and_analytic_velocity_match_independent_polynomials() {
    // T0=1, T1=x, T2=2x²-1, T3=4x³-3x.
    // dT1/dx=1, dT2/dx=4x, dT3/dx=12x²-3.
    // Each granule spans 16 TDB days, so dx/dJD=2/16.
    let mut ephemeris = De441::new(Cursor::new(cubic_fixture())).unwrap();
    for (jd, expected_position, expected_velocity) in [
        (1008.0, 7.0, -1.25),
        (1012.0, 5.5, 1.0),
        (1024.0, 20.0, -0.875),
        (1028.0, 17.5, -0.125),
        (1032.0, 21.0, 2.125),
    ] {
        let state = ephemeris
            .state(
                Body::Mercury,
                Body::SolarSystemBarycenter,
                JulianDateTdb(jd),
            )
            .unwrap();
        assert_eq!(state.position_km, [expected_position, 0.0, 0.0]);
        assert_eq!(
            state.velocity_km_per_day,
            [expected_velocity, 0.0, 0.0],
            "unexpected TDB-day velocity at JD {jd}"
        );
    }
}
