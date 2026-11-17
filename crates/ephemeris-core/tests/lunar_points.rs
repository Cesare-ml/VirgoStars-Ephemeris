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

//! Lunar points. The `#[ignore]` tests need a classic DE441 file in `DE441_PATH`.

use virgostars_ephemeris_core::apparent_ecliptic_of_date::{tdb_to_tt_jd, ApparentFrame};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::earth_orientation::{
    ecliptic_nutation, iau2000b_nutation, matrix_vector_multiply, true_ecliptic_of_date_matrix,
};
use virgostars_ephemeris_core::lunar_points::{
    lunar_point, LunarPoint, LunarPointError, MOON_MEAN_ECCENTRICITY, MOON_MEAN_INCLINATION_DEG,
    MOON_SEMI_MAJOR_AXIS_KM,
};

const J2000: f64 = 2_451_545.0;
const AU_KM: f64 = 149_597_870.7;
type NoKernel = Option<&'static mut De441<std::fs::File>>;

fn wrap(d: f64) -> f64 {
    (d + 540.0).rem_euclid(360.0) - 180.0
}

fn mean(
    point: LunarPoint,
    frame: ApparentFrame,
    tdb: f64,
) -> virgostars_ephemeris_core::lunar_points::LunarPointState {
    lunar_point(NoKernel::None, point, frame, tdb).unwrap()
}

#[test]
fn mean_node_is_the_published_polynomial_plus_nutation_with_a_retrograde_rate() {
    let s = mean(
        LunarPoint::MeanNode,
        ApparentFrame::TrueEclipticOfDate,
        J2000,
    );
    let dpsi = iau2000b_nutation(tdb_to_tt_jd(J2000).unwrap())
        .unwrap()
        .d_psi_rad
        .to_degrees();
    assert!(
        (s.longitude_deg - (125.044_555_01 + dpsi)).abs() < 1e-9,
        "{}",
        s.longitude_deg
    );
    assert_eq!(s.latitude_deg, 0.0);
    assert!((s.distance_au * AU_KM - MOON_SEMI_MAJOR_AXIS_KM).abs() < 1e-6);
    // The polynomial's -0.0529538 deg/day plus the nutation rate (~ +2e-6 deg/day at J2000).
    assert!(
        (s.longitude_rate_deg_per_day + 0.052_951_8).abs() < 2e-6,
        "{}",
        s.longitude_rate_deg_per_day
    );
    assert_eq!(s.distance_rate_au_per_day, 0.0);
}

#[test]
fn mean_apogee_lies_on_the_inclined_mean_orbit_with_the_documented_distance() {
    let mut max_lat: f64 = 0.0;
    for k in 0..400 {
        let tdb = 2_415_100.5 + f64::from(k) * 180.31;
        let s = mean(
            LunarPoint::MeanApogee,
            ApparentFrame::TrueEclipticOfDate,
            tdb,
        );
        assert!(s.latitude_deg.abs() <= MOON_MEAN_INCLINATION_DEG + 1e-9);
        max_lat = max_lat.max(s.latitude_deg.abs());
        let expected = MOON_SEMI_MAJOR_AXIS_KM * (1.0 + MOON_MEAN_ECCENTRICITY);
        assert!((s.distance_au * AU_KM - expected).abs() < 1e-6);
        assert!((0.0..360.0).contains(&s.longitude_deg));
        // The apogee advances (prograde, about 0.111 deg/day on average).
        assert!(s.longitude_rate_deg_per_day > 0.05 && s.longitude_rate_deg_per_day < 0.2);
    }
    assert!(
        max_lat > 5.0,
        "the latitude reaches the orbital inclination over the grid: {max_lat}"
    );
    // At the node-opposite extremes (argument 90 degrees from the node) the latitude equals the inclination.
}

#[test]
fn equatorial_output_is_the_ecliptic_one_rotated_by_the_true_obliquity() {
    let tdb = J2000 + 9_000.37;
    let ecl = mean(
        LunarPoint::MeanApogee,
        ApparentFrame::TrueEclipticOfDate,
        tdb,
    );
    let equ = mean(
        LunarPoint::MeanApogee,
        ApparentFrame::TrueEquatorOfDate,
        tdb,
    );
    let eps = ecliptic_nutation(tdb_to_tt_jd(tdb).unwrap())
        .unwrap()
        .true_obliquity_rad;
    let (lon, lat) = (
        ecl.longitude_deg.to_radians(),
        ecl.latitude_deg.to_radians(),
    );
    let (x, y, z) = (lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin());
    let ye = y * eps.cos() - z * eps.sin();
    let ze = y * eps.sin() + z * eps.cos();
    assert!(wrap(equ.longitude_deg - ye.atan2(x).to_degrees()).abs() < 1e-9);
    assert!((equ.latitude_deg - ze.atan2(x.hypot(ye)).to_degrees()).abs() < 1e-9);
    assert_eq!(ecl.distance_au, equ.distance_au);
}

#[test]
fn osculating_points_require_an_ephemeris_and_a_finite_epoch() {
    for point in [LunarPoint::TrueNode, LunarPoint::OsculatingApogee] {
        assert!(matches!(
            lunar_point(
                NoKernel::None,
                point,
                ApparentFrame::TrueEclipticOfDate,
                J2000
            ),
            Err(LunarPointError::EphemerisRequired)
        ));
    }
    assert!(matches!(
        lunar_point(
            NoKernel::None,
            LunarPoint::MeanNode,
            ApparentFrame::TrueEclipticOfDate,
            f64::NAN
        ),
        Err(LunarPointError::NonFinite)
    ));
    assert!(matches!(
        lunar_point(
            NoKernel::None,
            LunarPoint::MeanNode,
            ApparentFrame::TrueEclipticOfDate,
            2_300_000.0
        ),
        Err(LunarPointError::Orientation(_))
    ));
    assert_eq!(
        LunarPoint::from_swiss_number(11),
        Some(LunarPoint::TrueNode)
    );
    assert_eq!(LunarPoint::from_swiss_number(14), None);
    assert_eq!(LunarPoint::OsculatingApogee.swiss_number(), 13);
}

fn kernel() -> De441<std::fs::File> {
    De441::open(std::env::var_os("DE441_PATH").expect("DE441_PATH is not set")).unwrap()
}

#[test]
#[ignore = "requires a legally obtained classic DE441 file in DE441_PATH"]
fn real_de441_true_node_is_the_node_of_the_inertial_orbital_plane() {
    let mut eph = kernel();
    for tdb in [
        2_415_100.5,
        2_440_000.25,
        J2000,
        2_459_000.125,
        2_480_000.75,
    ] {
        let s = lunar_point(
            Some(&mut eph),
            LunarPoint::TrueNode,
            ApparentFrame::TrueEclipticOfDate,
            tdb,
        )
        .unwrap();
        assert_eq!(s.latitude_deg, 0.0);
        // Independent derivation at the retarded epoch from the raw geocentric Moon state.
        let mut te = tdb;
        for _ in 0..3 {
            let r = eph
                .state(Body::Moon, Body::Earth, JulianDateTdb(te))
                .unwrap()
                .position_km;
            te = tdb - (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt() / 299_792.458 / 86_400.0;
        }
        let st = eph
            .state(Body::Moon, Body::Earth, JulianDateTdb(te))
            .unwrap();
        let m = true_ecliptic_of_date_matrix(tdb_to_tt_jd(tdb).unwrap()).unwrap();
        let r = matrix_vector_multiply(m, st.position_km);
        let v = matrix_vector_multiply(m, st.velocity_km_per_day);
        let h = [
            r[1] * v[2] - r[2] * v[1],
            r[2] * v[0] - r[0] * v[2],
            r[0] * v[1] - r[1] * v[0],
        ];
        // atan2(N_y, N_x) with N = (-h_y, h_x, 0).
        let expected = h[0].atan2(-h[1]).to_degrees().rem_euclid(360.0);
        assert!(
            wrap(s.longitude_deg - expected).abs() < 1e-9,
            "{tdb}: {} vs {expected}",
            s.longitude_deg
        );
        // The node is within about 1.7 degrees of the mean node (the well-known oscillation).
        let mean_node =
            mean(LunarPoint::MeanNode, ApparentFrame::TrueEclipticOfDate, tdb).longitude_deg;
        assert!(wrap(s.longitude_deg - mean_node).abs() < 2.0, "{tdb}");
        // Its distance is a real point of the osculating ellipse: between perigee and apogee.
        let km = s.distance_au * AU_KM;
        assert!(km > 355_000.0 && km < 407_000.0, "{km}");
    }
}

#[test]
#[ignore = "requires a legally obtained classic DE441 file in DE441_PATH"]
fn real_de441_osculating_apogee_is_a_valid_apoapsis_and_all_rates_match_centered_differences() {
    let mut eph = kernel();
    let h = 1.0_f64 / 256.0;
    for point in [
        LunarPoint::MeanNode,
        LunarPoint::TrueNode,
        LunarPoint::MeanApogee,
        LunarPoint::OsculatingApogee,
    ] {
        for frame in [
            ApparentFrame::TrueEclipticOfDate,
            ApparentFrame::TrueEquatorOfDate,
        ] {
            let mut jd = 2_415_100.5;
            while jd < 2_487_000.0 {
                let c = lunar_point(Some(&mut eph), point, frame, jd).unwrap();
                let p = lunar_point(Some(&mut eph), point, frame, jd + h).unwrap();
                let m = lunar_point(Some(&mut eph), point, frame, jd - h).unwrap();
                let lon = wrap(p.longitude_deg - m.longitude_deg) / (2.0 * h);
                let lat = (p.latitude_deg - m.latitude_deg) / (2.0 * h);
                // The osculating apogee swings by degrees per day: allow its centered-difference truncation.
                let tol = if point == LunarPoint::OsculatingApogee {
                    0.05
                } else {
                    1e-3
                };
                assert!(
                    (c.longitude_rate_deg_per_day - lon).abs() < tol,
                    "{point:?} {frame:?} {jd}: {} vs {lon}",
                    c.longitude_rate_deg_per_day
                );
                assert!(
                    (c.latitude_rate_deg_per_day - lat).abs() < tol,
                    "{point:?} {frame:?} {jd}"
                );
                if point == LunarPoint::OsculatingApogee {
                    let km = c.distance_au * AU_KM;
                    assert!(
                        km > 370_000.0 && km < 425_000.0,
                        "apogee distance {km} at {jd}"
                    );
                }
                jd += 4_567.8;
            }
        }
    }
}
