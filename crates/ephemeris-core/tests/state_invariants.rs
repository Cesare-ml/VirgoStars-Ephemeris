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

//! Synthetic geometric-state invariants for the classic DE441 reader.
//! These tests are not independent JPL/Horizons accuracy evidence.

use std::io::Cursor;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

fn f64_le(bytes: &mut [u8], at: usize, value: f64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn i32_le(bytes: &mut [u8], at: usize, value: i32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

// Three series: Mercury, Earth-Moon barycenter, geocentric Moon.
// Constant and linear Chebyshev coefficients produce nonzero velocities.
fn synthetic_kernel() -> Vec<u8> {
    let record_size = 3000;
    let mut bytes = vec![0u8; record_size * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    f64_le(&mut bytes, 2652, 1000.0);
    f64_le(&mut bytes, 2660, 1032.0);
    f64_le(&mut bytes, 2668, 32.0);
    f64_le(&mut bytes, 2688, 81.0);
    i32_le(&mut bytes, 2840, 441);

    // 2 + 3*2 + 3*2 + 3*2 = 20 words, plus a harmless
    // distant pointer to set the classic record length to 3000 bytes.
    for (index, first) in [(0, 3), (2, 9), (9, 15)] {
        let at = 2696 + index * 12;
        i32_le(&mut bytes, at, first);
        i32_le(&mut bytes, at + 4, 2);
        i32_le(&mut bytes, at + 8, 1);
    }
    i32_le(&mut bytes, 2844, 373);
    i32_le(&mut bytes, 2848, 1);
    i32_le(&mut bytes, 2852, 1);

    let record = record_size * 2;
    f64_le(&mut bytes, record, 1000.0);
    f64_le(&mut bytes, record + 8, 1032.0);
    for (first, base, slope) in [(3usize, 100.0, 2.0), (9, 200.0, 4.0), (15, 40.0, 1.0)] {
        for axis in 0..3 {
            let at = record + (first - 1 + axis * 2) * 8;
            f64_le(&mut bytes, at, base + axis as f64 * 10.0);
            f64_le(&mut bytes, at + 8, slope + axis as f64);
        }
    }
    bytes
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-10, "{actual} != {expected}");
}

#[test]
fn relative_states_are_antisymmetric_and_transitive_in_position_and_velocity() {
    let mut eph = De441::new(Cursor::new(synthetic_kernel())).unwrap();
    for jd in [1000.0, 1008.0, 1016.0, 1024.0, 1032.0] {
        let epoch = JulianDateTdb(jd);
        let ab = eph.state(Body::Moon, Body::Earth, epoch).unwrap();
        let ba = eph.state(Body::Earth, Body::Moon, epoch).unwrap();
        let ac = eph.state(Body::Moon, Body::Mercury, epoch).unwrap();
        let bc = eph.state(Body::Earth, Body::Mercury, epoch).unwrap();

        for axis in 0..3 {
            near(ab.position_km[axis], -ba.position_km[axis]);
            near(ab.velocity_km_per_day[axis], -ba.velocity_km_per_day[axis]);
            near(
                ac.position_km[axis],
                ab.position_km[axis] + bc.position_km[axis],
            );
            near(
                ac.velocity_km_per_day[axis],
                ab.velocity_km_per_day[axis] + bc.velocity_km_per_day[axis],
            );
        }
        assert_eq!(ab.target, Body::Moon);
        assert_eq!(ab.center, Body::Earth);
        assert_eq!(ab.epoch, epoch);
    }
}

#[test]
fn barycentric_earth_moon_reconstruction_preserves_mass_weighted_center() {
    let mut eph = De441::new(Cursor::new(synthetic_kernel())).unwrap();
    for jd in [1000.0, 1016.0, 1032.0] {
        let epoch = JulianDateTdb(jd);
        let emb = eph
            .state(
                Body::EarthMoonBarycenter,
                Body::SolarSystemBarycenter,
                epoch,
            )
            .unwrap();
        let earth = eph
            .state(Body::Earth, Body::SolarSystemBarycenter, epoch)
            .unwrap();
        let moon = eph
            .state(Body::Moon, Body::SolarSystemBarycenter, epoch)
            .unwrap();
        let moon_geo = eph.state(Body::Moon, Body::Earth, epoch).unwrap();
        for axis in 0..3 {
            near(
                emb.position_km[axis],
                (81.0 * earth.position_km[axis] + moon.position_km[axis]) / 82.0,
            );
            near(
                emb.velocity_km_per_day[axis],
                (81.0 * earth.velocity_km_per_day[axis] + moon.velocity_km_per_day[axis]) / 82.0,
            );
            near(
                moon_geo.position_km[axis],
                moon.position_km[axis] - earth.position_km[axis],
            );
            near(
                moon_geo.velocity_km_per_day[axis],
                moon.velocity_km_per_day[axis] - earth.velocity_km_per_day[axis],
            );
        }
    }
}
