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

//! Tests for the astronomy supplied to house systems.

use virgostars_ephemeris_core::house_angles::{angles_from_armc, house_angles, HouseAnglesError};

const EPS: f64 = 23.439_291_1;

fn near(a: f64, b: f64, tol: f64) {
    let d = ((a - b + 540.0).rem_euclid(360.0)) - 180.0;
    assert!(d.abs() <= tol, "{a} vs {b}: {d}");
}

#[test]
fn cardinal_armc_values_on_the_equator() {
    // ARMC 0: MC at 0 Aries, ASC at 90 (Cancer).
    near(angles_from_armc(0.0, EPS, 0.0).1, 0.0, 1e-12);
    near(angles_from_armc(0.0, EPS, 0.0).0, 90.0, 1e-12);
    // ARMC 90: MC at 90, ASC at 180.
    near(angles_from_armc(90.0, EPS, 0.0).1, 90.0, 1e-12);
    near(angles_from_armc(90.0, EPS, 0.0).0, 180.0, 1e-12);
    // ARMC 180: ASC at 270. ARMC 270: MC at 270, ASC at 0.
    near(angles_from_armc(180.0, EPS, 0.0).0, 270.0, 1e-12);
    near(angles_from_armc(270.0, EPS, 0.0).1, 270.0, 1e-12);
    near(angles_from_armc(270.0, EPS, 0.0).0, 0.0, 1e-12);
}

#[test]
fn ascendant_follows_the_midheaven_by_less_than_a_half_circle_in_the_north() {
    for armc in (0..360).step_by(7) {
        for lat in [10.0, 41.9, 55.0, 64.0] {
            let (asc, mc) = angles_from_armc(f64::from(armc), 23.44, lat);
            let ahead = (asc - mc).rem_euclid(360.0);
            assert!(
                ahead > 0.0 && ahead < 180.0,
                "armc {armc} lat {lat}: {ahead}"
            );
        }
    }
}

#[test]
fn the_southern_hemisphere_mirrors_the_northern_one() {
    // Rotating the sky by half a turn and flipping the latitude maps the
    // Ascendant to the opposite point.
    for armc in (0..360).step_by(11) {
        for lat in [20.0, 41.9, 60.0] {
            let south = angles_from_armc(f64::from(armc), 23.44, -lat).0;
            let north = angles_from_armc(f64::from(armc) + 180.0, 23.44, lat).0;
            near(south, north + 180.0, 1e-9);
        }
    }
}

/// In equatorial Cartesian coordinates the ecliptic point has direction
/// (cos λ, sin λ cos ε, sin λ sin ε). Projecting it onto the local zenith gives
/// its horizon altitude, and differentiating in sidereal angle gives the
/// rising/setting test. This is independent of the implementation's atan2
/// branch choice and does not compute astronomical ephemerides.
fn horizon_altitude_sine_and_rise_rate(
    armc_deg: f64,
    obliquity_deg: f64,
    latitude_deg: f64,
    ecliptic_longitude_deg: f64,
) -> (f64, f64) {
    let (sin_a, cos_a) = armc_deg.to_radians().sin_cos();
    let (sin_e, cos_e) = obliquity_deg.to_radians().sin_cos();
    let (sin_phi, cos_phi) = latitude_deg.to_radians().sin_cos();
    let (sin_l, cos_l) = ecliptic_longitude_deg.to_radians().sin_cos();
    let (x, y, z) = (cos_l, sin_l * cos_e, sin_l * sin_e);
    let altitude_sine = sin_phi * z + cos_phi * (x * cos_a + y * sin_a);
    let rise_rate = cos_phi * (-x * sin_a + y * cos_a);
    (altitude_sine, rise_rate)
}

#[test]
fn polar_ascendant_is_on_eastern_rising_horizon_both_hemispheres() {
    for obliquity in [23.44, 23.437_676_823_739_082] {
        for latitude in [
            -89.999, -85.0, -80.0, -67.0, -66.55, -45.0, 0.0, 45.0, 66.55, 67.0, 80.0, 85.0, 89.999,
        ] {
            for armc in (0..360).step_by(5) {
                let (ascendant, _mc) = angles_from_armc(f64::from(armc), obliquity, latitude);
                let (altitude_sine, rise_rate) = horizon_altitude_sine_and_rise_rate(
                    f64::from(armc),
                    obliquity,
                    latitude,
                    ascendant,
                );
                assert!(
                    altitude_sine.abs() < 1e-10,
                    "not on horizon: armc={armc} lat={latitude} asc={ascendant} alt_sine={altitude_sine}"
                );
                assert!(
                    rise_rate >= -1e-12,
                    "setting rather than rising: armc={armc} lat={latitude} asc={ascendant} rate={rise_rate}"
                );
            }
        }
    }
}

#[test]
fn owner_j2000_north_polar_fixture_uses_the_rising_antipode() {
    // The owner-provided real-addon latitude receipt (UT1 JD 2451545, TT JD
    // 2451545.0008, 80°N, 12.4964°E) was setting at 164.5489205114233°.
    // The opposite crossing is the geometrically rising Ascendant.
    let armc = 292.953_471_863_758_47;
    let epsilon = 23.437_676_823_739_082;
    let (north_asc, mc) = angles_from_armc(armc, epsilon, 80.0);
    near(north_asc, 344.548_920_511_423_3, 1e-9);
    near(mc, 291.234_820_988_22, 1e-9);
    let (_, north_rate) = horizon_altitude_sine_and_rise_rate(armc, epsilon, 80.0, north_asc);
    assert!(north_rate > 0.13);

    let (south_asc, _) = angles_from_armc(armc, epsilon, -80.0);
    near(south_asc, 7.168_816_302_952_17, 1e-9);
    let (_, south_rate) = horizon_altitude_sine_and_rise_rate(armc, epsilon, -80.0, south_asc);
    assert!(south_rate > 0.16);
}

#[test]
fn polar_fix_preserves_reference_fitted_extra_angle_branches() {
    use virgostars_ephemeris_core::house_angles::extra_angles_from_armc;

    let armc = 292.953_471_863_758_47;
    let epsilon = 23.437_676_823_739_082;
    // At 10°N the Munkasey auxiliary latitude is 80°N. At 80°S, the
    // Koch co-Ascendant's auxiliary latitude is 80°N. They historically
    // select 164.5489°; changing them implicitly would break a separate API.
    let mc = angles_from_armc(armc, epsilon, 10.0).1;
    let tropical = extra_angles_from_armc(armc, epsilon, 10.0, mc);
    near(
        tropical.munkasey_co_ascendant_deg,
        164.548_920_511_423_3,
        1e-9,
    );
    let polar = extra_angles_from_armc(armc, epsilon, -80.0, mc);
    near(polar.koch_co_ascendant_deg, 164.548_920_511_423_3, 1e-9);
    near(
        polar.munkasey_polar_ascendant_deg,
        344.548_920_511_423_3,
        1e-9,
    );
    // The East Point remains purely equatorial and independent of latitude.
    near(
        tropical.equatorial_ascendant_deg,
        polar.equatorial_ascendant_deg,
        1e-12,
    );
}

#[test]
fn full_native_angle_pipeline_keeps_north_and_south_polar_ascendants_rising() {
    let ut1 = 2_451_545.0;
    let tt = 2_451_545.0008;
    for latitude in [-80.0, 80.0] {
        let angles = house_angles(ut1, tt, latitude, 12.4964).unwrap();
        let (altitude_sine, rise_rate) = horizon_altitude_sine_and_rise_rate(
            angles.armc_deg,
            angles.true_obliquity_deg,
            latitude,
            angles.ascendant_deg,
        );
        assert!(altitude_sine.abs() < 1e-10);
        assert!(rise_rate > 0.0);
    }
}

#[test]
fn full_pipeline_has_sane_values_and_fails_closed() {
    // 2000-01-01 12:00 TT, with UT1 equal to TT only as a test shortcut.
    let a = house_angles(2_451_545.0, 2_451_545.0, 0.0, 0.0).unwrap();
    assert!(
        (a.true_obliquity_deg - 23.4372).abs() < 0.001,
        "{}",
        a.true_obliquity_deg
    );
    // Greenwich apparent sidereal time at J2000.0 is about 18.697 h = 280.46 deg.
    near(a.armc_deg, 280.46, 0.05);
    let east = house_angles(2_451_545.0, 2_451_545.0, 41.9, 12.5).unwrap();
    near(east.armc_deg, a.armc_deg + 12.5, 1e-9);
    for (lat, lon) in [
        (90.0, 0.0),
        (-90.0, 0.0),
        (f64::NAN, 0.0),
        (10.0, f64::INFINITY),
        (10.0, 400.0),
    ] {
        assert_eq!(
            house_angles(2_451_545.0, 2_451_545.0, lat, lon),
            Err(HouseAnglesError::InvalidSite),
            "{lat} {lon}"
        );
    }
    // The IAU 2000B range is enforced by the orientation module.
    assert!(matches!(
        house_angles(2_300_000.0, 2_300_000.0, 10.0, 10.0),
        Err(HouseAnglesError::Orientation(_))
    ));
}

// ---- the five extra points of the reference's house result ----

use virgostars_ephemeris_core::horizon::{ecliptic_to_equatorial, equatorial_to_horizontal};

#[test]
fn extra_points_satisfy_their_definitions() {
    let tt = 2_455_000.5;
    for &(lat, lon) in &[
        (41.9, 12.5),
        (-33.9, 151.2),
        (64.1, -21.9),
        (10.0, 0.0),
        (-10.0, 100.0),
        (-0.2, -78.5),
        (60.0, 20.0),
    ] {
        let a = house_angles(tt, tt, lat, lon).unwrap();
        let x = a.extra;
        // East Point: the point of the ecliptic whose right ascension is ARMC + 90 degrees.
        let (ra, _) = ecliptic_to_equatorial(tt, x.equatorial_ascendant_deg, 0.0).unwrap();
        near(ra, a.armc_deg + 90.0, 1e-9);
        // Vertex: on the prime vertical (azimuth east or west), in the west.
        let (vra, vdec) = ecliptic_to_equatorial(tt, x.vertex_deg, 0.0).unwrap();
        let h = equatorial_to_horizontal(vra, vdec, a.armc_deg, lat).unwrap();
        let az = h.azimuth_north_east_deg;
        assert!(
            (az - 270.0).abs() < 1e-6 || (az - 90.0).abs() < 1e-6,
            "{lat}: azimuth {az}"
        );
        if lat.abs() > 23.5 && lat > 0.0 {
            assert!(
                (az - 270.0).abs() < 1e-6,
                "north of the tropics the vertex is west: {az}"
            );
        }
        // The polar ascendant is the opposite of the Koch co-ascendant.
        near(
            x.munkasey_polar_ascendant_deg,
            x.koch_co_ascendant_deg + 180.0,
            1e-12,
        );
    }
    let eq = house_angles(tt, tt, 0.0, 33.0).unwrap();
    // On the equator the Koch co-ascendant and the East Point are the Ascendant.
    near(eq.extra.koch_co_ascendant_deg, eq.ascendant_deg, 1e-9);
    near(eq.extra.equatorial_ascendant_deg, eq.ascendant_deg, 1e-9);
}
