/* EXHIBIT A. Common Public Attribution License Version 1.0.
The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

use std::io::Cursor;
use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};

fn put_f64(data: &mut [u8], offset: usize, value: f64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(data: &mut [u8], offset: usize, value: i32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// Valid classic-shaped file with no planetary series. A harmless pointer to
/// series 13 establishes the minimum 3000-byte record size.
fn missing_series_fixture() -> Vec<u8> {
    let size = 3000;
    let mut data = vec![0u8; size * 3];
    let title = b"JPL Planetary Ephemeris DE441";
    data[..title.len()].copy_from_slice(title);

    put_f64(&mut data, 2652, 1000.0);
    put_f64(&mut data, 2660, 1032.0);
    put_f64(&mut data, 2668, 32.0);
    put_f64(&mut data, 2688, 81.0);
    put_i32(&mut data, 2840, 441);
    put_i32(&mut data, 2844, 373);
    put_i32(&mut data, 2848, 1);
    put_i32(&mut data, 2852, 1);

    put_f64(&mut data, size * 2, 1000.0);
    put_f64(&mut data, size * 2 + 8, 1032.0);
    data
}

#[test]
fn missing_planetary_series_identifies_exact_body_or_system_barycenter() {
    let mut eph = De441::new(Cursor::new(missing_series_fixture())).unwrap();
    let epoch = JulianDateTdb(1016.0);
    for (target, name) in [
        (Body::Mercury, "Mercury"),
        (Body::Venus, "Venus"),
        (Body::EarthMoonBarycenter, "Earth-Moon barycenter"),
        (Body::MarsBarycenter, "Mars system barycenter"),
        (Body::JupiterBarycenter, "Jupiter system barycenter"),
        (Body::SaturnBarycenter, "Saturn system barycenter"),
        (Body::UranusBarycenter, "Uranus system barycenter"),
        (Body::NeptuneBarycenter, "Neptune system barycenter"),
        (Body::PlutoBarycenter, "Pluto system barycenter"),
        (Body::Sun, "Sun"),
    ] {
        match eph.state(target, Body::SolarSystemBarycenter, epoch) {
            Err(De441Error::MissingSeries(found)) => assert_eq!(found, name),
            other => panic!("expected missing {name} series, got {other:?}"),
        }
    }
}

#[test]
fn earth_and_moon_report_their_actual_missing_input_series() {
    let mut eph = De441::new(Cursor::new(missing_series_fixture())).unwrap();
    let epoch = JulianDateTdb(1016.0);
    // An Earth barycentric state depends on the EMB series.
    assert!(matches!(
        eph.state(Body::Earth, Body::SolarSystemBarycenter, epoch),
        Err(De441Error::MissingSeries("Earth-Moon barycenter"))
    ));
    // A directly geocentric Moon state depends on the lunar series.
    assert!(matches!(
        eph.state(Body::Moon, Body::Earth, epoch),
        Err(De441Error::MissingSeries("geocentric Moon"))
    ));
}

#[test]
fn unrepresentable_and_outside_coverage_epochs_are_rejected_even_for_identity() {
    let mut eph = De441::new(Cursor::new(missing_series_fixture())).unwrap();
    // Identity vectors must not bypass the epoch validity check.
    for jd in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        -f64::MAX,
        999.0,
        1033.0,
    ] {
        assert!(matches!(
            eph.state(Body::Mercury, Body::Mercury, JulianDateTdb(jd)),
            Err(De441Error::OutsideCoverage { .. })
        ));
    }

    for jd in [1000.0, 1032.0] {
        let zero = eph
            .state(Body::Mercury, Body::Mercury, JulianDateTdb(jd))
            .unwrap();
        assert_eq!(zero.position_km, [0.0; 3]);
        assert_eq!(zero.velocity_km_per_day, [0.0; 3]);
    }
}
