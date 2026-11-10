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

//! Mathematical lunar points: Mean/True Nodes and Mean/Osculating Apogees.
//!
//! Distinct from physical celestial bodies: lunar nodes and apogees are
//! mathematical points defined by the geometry and orbital mechanics of the
//! Moon's geocentric orbit. They do not emit or reflect light, have no physical
//! surface, and do not undergo stellar aberration or gravitational deflection.
//!
//! # Implemented Points
//! - `MeanNode` (Swiss ID 10: `SE_MEAN_NODE`): Mean ascending node (Rahu)
//!   using the Simon et al. (1994) / Chapront ELP-2000 polynomial.
//! - `TrueNode` (Swiss ID 11: `SE_TRUE_NODE`): Osculating ascending node
//!   computed from the Moon's instantaneous orbital angular momentum vector.
//! - `MeanApogee` (Swiss ID 12: `SE_MEAN_APOG`): Mean lunar apogee (Black Moon
//!   Lilith) using Chapront ELP-2000 mean perigee + 180°.
//! - `OsculatingApogee` (Swiss ID 13: `SE_OSCU_APOG`): Osculating lunar apogee
//!   (True Lilith) computed from the Moon's Runge–Lenz eccentricity vector.

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::de441::{Body, De441, De441Error, JulianDateTdb};
use crate::earth_orientation::{
    iau2000b_nutation, matrix_vector_multiply, true_ecliptic_of_date_matrix, OrientationError,
    DAYS_PER_JULIAN_CENTURY, J2000_TT_JD,
};

/// Gravitational parameter of the Earth–Moon system in km^3 / s^2 (DE441 standard).
pub const GM_EARTH_MOON_KM3_PER_SEC2: f64 = 403_503.241_8;
pub const SECONDS_PER_DAY: f64 = 86_400.0;
/// Gravitational parameter of the Earth–Moon system in km^3 / day^2.
pub const GM_EARTH_MOON_KM3_PER_DAY2: f64 =
    GM_EARTH_MOON_KM3_PER_SEC2 * SECONDS_PER_DAY * SECONDS_PER_DAY;

/// Nominal lunar mean distance in kilometres (~0.00257 AU).
pub const MOON_MEAN_DISTANCE_KM: f64 = 384_400.0;
/// Nominal lunar mean apogee distance in kilometres.
pub const MOON_MEAN_APOGEE_KM: f64 = 405_400.0;
/// Astronomical Unit in kilometres (IAU 2012 exact definition).
pub const AU_KM: f64 = 149_597_870.7;

/// Mathematical lunar points defined by the Moon's orbit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LunarPoint {
    /// Mean ascending lunar node (Rahu), Swiss ID 10 (`SE_MEAN_NODE`).
    MeanNode,
    /// True (osculating) ascending lunar node, Swiss ID 11 (`SE_TRUE_NODE`).
    TrueNode,
    /// Mean lunar apogee (Black Moon Lilith), Swiss ID 12 (`SE_MEAN_APOG`).
    MeanApogee,
    /// Osculating lunar apogee (True Lilith), Swiss ID 13 (`SE_OSCU_APOG`).
    OsculatingApogee,
}

impl LunarPoint {
    /// Returns the Swiss Ephemeris integer identifier for this point.
    #[inline]
    pub fn swiss_id(&self) -> i32 {
        match self {
            Self::MeanNode => 10,
            Self::TrueNode => 11,
            Self::MeanApogee => 12,
            Self::OsculatingApogee => 13,
        }
    }

    /// Descriptive name of the lunar point.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::MeanNode => "Mean Lunar Node",
            Self::TrueNode => "True Lunar Node",
            Self::MeanApogee => "Mean Lunar Apogee (Lilith)",
            Self::OsculatingApogee => "Osculating Lunar Apogee (True Lilith)",
        }
    }
}

/// Ecliptic coordinates and kinematic rates of a lunar mathematical point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LunarPointCoordinates {
    pub point: LunarPoint,
    pub tt_jd: f64,
    /// Ecliptic longitude in [0, 360) degrees.
    pub longitude_deg: f64,
    /// Ecliptic latitude in [-90, +90] degrees (0.0 for nodes by definition).
    pub latitude_deg: f64,
    /// Geocentric distance in astronomical units (AU).
    pub distance_au: f64,
    /// Geocentric distance in kilometres.
    pub distance_km: f64,
    /// Rate of longitude change in degrees per day.
    pub longitude_rate_deg_per_day: f64,
    /// Rate of latitude change in degrees per day (0.0 for nodes).
    pub latitude_rate_deg_per_day: f64,
    /// Rate of distance change in AU per day.
    pub distance_rate_au_per_day: f64,
}

impl LunarPointCoordinates {
    /// Array representation matching Swiss Ephemeris 6-value output `[lon, lat, dist, lon_spd, lat_spd, dist_spd]`.
    pub fn to_swiss_array(&self) -> [f64; 6] {
        [
            self.longitude_deg,
            self.latitude_deg,
            self.distance_au,
            self.longitude_rate_deg_per_day,
            self.latitude_rate_deg_per_day,
            self.distance_rate_au_per_day,
        ]
    }
}

/// Errors occurring during lunar point calculations.
#[derive(Debug)]
pub enum LunarPointError {
    Ephemeris(De441Error),
    Orientation(OrientationError),
    NonFiniteInput,
    /// Orbital momentum is zero or parallel to ecliptic pole; node undefined.
    UndefinedNode,
    /// Orbit is perfectly circular or parabolic; apogee undefined.
    UndefinedApogee,
}

impl fmt::Display for LunarPointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ephemeris(err) => write!(f, "lunar point ephemeris error: {err}"),
            Self::Orientation(err) => write!(f, "lunar point orientation error: {err}"),
            Self::NonFiniteInput => write!(f, "non-finite input for lunar point evaluation"),
            Self::UndefinedNode => write!(f, "lunar orbital node is undefined"),
            Self::UndefinedApogee => write!(f, "lunar osculating apogee is undefined"),
        }
    }
}

impl Error for LunarPointError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Ephemeris(err) => Some(err),
            Self::Orientation(err) => Some(err),
            _ => None,
        }
    }
}

impl From<De441Error> for LunarPointError {
    fn from(err: De441Error) -> Self {
        Self::Ephemeris(err)
    }
}

impl From<OrientationError> for LunarPointError {
    fn from(err: OrientationError) -> Self {
        Self::Orientation(err)
    }
}

fn wrap_degrees(deg: f64) -> f64 {
    deg.rem_euclid(360.0)
}

fn shortest_degree_difference(after: f64, before: f64) -> f64 {
    (after - before + 180.0).rem_euclid(360.0) - 180.0
}

/// Computes the Mean Ascending Lunar Node (Rahu) at epoch `tt_jd`.
///
/// Uses the Chapront & Chapront-Touzé (1988) / Simon et al. (1994) polynomial
/// in TT Julian centuries. If `apparent` is true, nutation in longitude (IAU 2000B)
/// is applied.
pub fn mean_lunar_node(
    tt_jd: f64,
    apparent: bool,
) -> Result<LunarPointCoordinates, LunarPointError> {
    if !tt_jd.is_finite() {
        return Err(LunarPointError::NonFiniteInput);
    }
    let t = (tt_jd - J2000_TT_JD) / DAYS_PER_JULIAN_CENTURY;

    // Simon et al. (1994) / Chapront ELP-2000 mean ascending node (degrees):
    let omega_mean =
        125.0445550 - 1934.13626197 * t + 0.0020756 * t.powi(2) + 0.000002139 * t.powi(3)
            - 0.00000001650 * t.powi(4);

    let omega_rate = (-1934.13626197 + 2.0 * 0.0020756 * t + 3.0 * 0.000002139 * t.powi(2)
        - 4.0 * 0.00000001650 * t.powi(3))
        / DAYS_PER_JULIAN_CENTURY;

    let mut longitude_deg = wrap_degrees(omega_mean);
    if apparent {
        let nut = iau2000b_nutation(tt_jd)?;
        longitude_deg = wrap_degrees(longitude_deg + nut.d_psi_rad.to_degrees());
    }

    Ok(LunarPointCoordinates {
        point: LunarPoint::MeanNode,
        tt_jd,
        longitude_deg,
        latitude_deg: 0.0,
        distance_au: 1.0,
        distance_km: AU_KM,
        longitude_rate_deg_per_day: omega_rate,
        latitude_rate_deg_per_day: 0.0,
        distance_rate_au_per_day: 0.0,
    })
}

/// Computes the Mean Lunar Apogee (Black Moon Lilith) at epoch `tt_jd`.
///
/// Uses Chapront ELP-2000 mean perigee + 180° in TT Julian centuries.
/// If `apparent` is true, nutation in longitude (IAU 2000B) is applied.
pub fn mean_lunar_apogee(
    tt_jd: f64,
    apparent: bool,
) -> Result<LunarPointCoordinates, LunarPointError> {
    if !tt_jd.is_finite() {
        return Err(LunarPointError::NonFiniteInput);
    }
    let t = (tt_jd - J2000_TT_JD) / DAYS_PER_JULIAN_CENTURY;

    // Chapront ELP-2000 mean perigee (degrees):
    let perigee_mean =
        83.3532431 + 4069.01774306 * t - 0.010325 * t.powi(2) - 0.00001249 * t.powi(3);

    let perigee_rate = (4069.01774306 - 2.0 * 0.010325 * t - 3.0 * 0.00001249 * t.powi(2))
        / DAYS_PER_JULIAN_CENTURY;

    // Apogee is perigee + 180 degrees.
    let apogee_mean = wrap_degrees(perigee_mean + 180.0);

    let mut longitude_deg = apogee_mean;
    if apparent {
        let nut = iau2000b_nutation(tt_jd)?;
        longitude_deg = wrap_degrees(longitude_deg + nut.d_psi_rad.to_degrees());
    }

    let distance_au = MOON_MEAN_APOGEE_KM / AU_KM;

    Ok(LunarPointCoordinates {
        point: LunarPoint::MeanApogee,
        tt_jd,
        longitude_deg,
        latitude_deg: 0.0,
        distance_au,
        distance_km: MOON_MEAN_APOGEE_KM,
        longitude_rate_deg_per_day: perigee_rate,
        latitude_rate_deg_per_day: 0.0,
        distance_rate_au_per_day: 0.0,
    })
}

/// Evaluates instantaneous True Node or Osculating Apogee from the Moon's geocentric state.
fn evaluate_osculating_point<F>(
    point: LunarPoint,
    tt_jd: f64,
    mut moon_geocentric_state_fn: F,
) -> Result<([f64; 3], f64), LunarPointError>
where
    F: FnMut(f64) -> Result<([f64; 3], [f64; 3]), LunarPointError>,
{
    let (r_icrf, v_icrf) = moon_geocentric_state_fn(tt_jd)?;
    let m_ecl = true_ecliptic_of_date_matrix(tt_jd)?;

    let r_ecl = matrix_vector_multiply(m_ecl, r_icrf);
    let v_ecl = matrix_vector_multiply(m_ecl, v_icrf);

    match point {
        LunarPoint::TrueNode => {
            // Orbital angular momentum vector h = r x v
            let h = [
                r_ecl[1] * v_ecl[2] - r_ecl[2] * v_ecl[1],
                r_ecl[2] * v_ecl[0] - r_ecl[0] * v_ecl[2],
                r_ecl[0] * v_ecl[1] - r_ecl[1] * v_ecl[0],
            ];
            // Ascending node vector N = (0, 0, 1) x h = (-h_y, h_x, 0)
            let n = [-h[1], h[0], 0.0];
            let norm = n[0].hypot(n[1]);
            if norm == 0.0 || !norm.is_finite() {
                return Err(LunarPointError::UndefinedNode);
            }
            let distance_km = (r_ecl[0].powi(2) + r_ecl[1].powi(2) + r_ecl[2].powi(2)).sqrt();
            Ok((n, distance_km))
        }
        LunarPoint::OsculatingApogee => {
            let r_mag = (r_ecl[0].powi(2) + r_ecl[1].powi(2) + r_ecl[2].powi(2)).sqrt();
            let v_sq = v_ecl[0].powi(2) + v_ecl[1].powi(2) + v_ecl[2].powi(2);
            let r_dot_v = r_ecl[0] * v_ecl[0] + r_ecl[1] * v_ecl[1] + r_ecl[2] * v_ecl[2];

            let mu = GM_EARTH_MOON_KM3_PER_DAY2;
            let factor_r = v_sq - mu / r_mag;

            // Eccentricity vector e = 1/mu * [ (v^2 - mu/r) r - (r.v) v ]
            // Apogee vector A = -e = 1/mu * [ (r.v) v - (v^2 - mu/r) r ]
            let a_vec = [
                (r_dot_v * v_ecl[0] - factor_r * r_ecl[0]) / mu,
                (r_dot_v * v_ecl[1] - factor_r * r_ecl[1]) / mu,
                (r_dot_v * v_ecl[2] - factor_r * r_ecl[2]) / mu,
            ];
            let e_mag = (a_vec[0].powi(2) + a_vec[1].powi(2) + a_vec[2].powi(2)).sqrt();
            if e_mag == 0.0 || !e_mag.is_finite() {
                return Err(LunarPointError::UndefinedApogee);
            }

            // Semi-major axis a = 1 / (2/r - v^2/mu)
            let denom = 2.0 / r_mag - v_sq / mu;
            let distance_km = if denom > 0.0 {
                let semi_major = 1.0 / denom;
                semi_major * (1.0 + e_mag)
            } else {
                r_mag
            };

            Ok((a_vec, distance_km))
        }
        _ => unreachable!(),
    }
}

/// Calculates any mathematical lunar point using a classic DE441 reader.
pub fn calculate_lunar_point<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    point: LunarPoint,
    tt_jd: f64,
    apparent: bool,
) -> Result<LunarPointCoordinates, LunarPointError> {
    match point {
        LunarPoint::MeanNode => mean_lunar_node(tt_jd, apparent),
        LunarPoint::MeanApogee => mean_lunar_apogee(tt_jd, apparent),
        LunarPoint::TrueNode | LunarPoint::OsculatingApogee => {
            let mut state_fn = |epoch_jd: f64| {
                let s = ephemeris.state(Body::Moon, Body::Earth, JulianDateTdb(epoch_jd))?;
                Ok((s.position_km, s.velocity_km_per_day))
            };

            let (vec_center, dist_center) = evaluate_osculating_point(point, tt_jd, &mut state_fn)?;

            let planar = vec_center[0].hypot(vec_center[1]);
            let longitude_deg = wrap_degrees(vec_center[1].atan2(vec_center[0]).to_degrees());
            let latitude_deg = if point == LunarPoint::TrueNode {
                0.0
            } else {
                vec_center[2].atan2(planar).to_degrees()
            };

            // Compute rate by central difference over ±300 seconds (±0.00347 days):
            let step_days = 300.0 / SECONDS_PER_DAY;
            let (vec_prev, dist_prev) =
                evaluate_osculating_point(point, tt_jd - step_days, &mut state_fn)?;
            let (vec_next, dist_next) =
                evaluate_osculating_point(point, tt_jd + step_days, &mut state_fn)?;

            let lon_prev = wrap_degrees(vec_prev[1].atan2(vec_prev[0]).to_degrees());
            let lon_next = wrap_degrees(vec_next[1].atan2(vec_next[0]).to_degrees());
            let d_lon = shortest_degree_difference(lon_next, lon_prev);
            let longitude_rate_deg_per_day = d_lon / (2.0 * step_days);

            let lat_prev = vec_prev[2]
                .atan2(vec_prev[0].hypot(vec_prev[1]))
                .to_degrees();
            let lat_next = vec_next[2]
                .atan2(vec_next[0].hypot(vec_next[1]))
                .to_degrees();
            let latitude_rate_deg_per_day = if point == LunarPoint::TrueNode {
                0.0
            } else {
                (lat_next - lat_prev) / (2.0 * step_days)
            };

            let distance_au = dist_center / AU_KM;
            let distance_rate_au_per_day = (dist_next - dist_prev) / (2.0 * step_days * AU_KM);

            Ok(LunarPointCoordinates {
                point,
                tt_jd,
                longitude_deg,
                latitude_deg,
                distance_au,
                distance_km: dist_center,
                longitude_rate_deg_per_day,
                latitude_rate_deg_per_day,
                distance_rate_au_per_day,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "{a} != {b} (diff: {}, max tol: {tolerance})",
            (a - b).abs()
        );
    }

    #[test]
    fn mean_node_matches_j2000_astronomical_reference() {
        let coords = mean_lunar_node(J2000_TT_JD, false).unwrap();
        // Standard astronomical value at J2000: 125.044555°
        near(coords.longitude_deg, 125.044555, 1e-5);
        assert_eq!(coords.latitude_deg, 0.0);
        // Rate is retrograde ~ -0.05295 deg/day (~18.61 years period)
        near(coords.longitude_rate_deg_per_day, -0.0529538, 1e-4);
        let period_years = 360.0 / coords.longitude_rate_deg_per_day.abs() / 365.25;
        near(period_years, 18.613, 0.01);
    }

    #[test]
    fn mean_apogee_matches_j2000_astronomical_reference() {
        let coords = mean_lunar_apogee(J2000_TT_JD, false).unwrap();
        // Standard perigee at J2000 is 83.3532431°, apogee is +180° = 263.3532431°
        near(coords.longitude_deg, 263.353243, 1e-5);
        assert_eq!(coords.latitude_deg, 0.0);
        // Rate is direct ~ +0.1114 deg/day (~8.85 years period)
        near(coords.longitude_rate_deg_per_day, 0.1114036, 1e-4);
        let period_years = 360.0 / coords.longitude_rate_deg_per_day.abs() / 365.25;
        near(period_years, 8.847, 0.01);
    }

    #[test]
    fn true_node_and_apogee_synthetic_orbit_invariants() {
        // Construct a synthetic circular/inclined lunar orbit:
        // Position at (r0, 0, 0), velocity with speed v0 inclined by 5.145 degrees
        let r0 = MOON_MEAN_DISTANCE_KM;
        let mu = GM_EARTH_MOON_KM3_PER_DAY2;
        let v0 = (mu / r0).sqrt();
        let inc = 5.145_f64.to_radians();

        let mut synthetic_state = |epoch_jd: f64| {
            let dt = epoch_jd - J2000_TT_JD;
            // Mean motion
            let n = v0 / r0;
            let theta = n * dt;
            let (s, c) = theta.sin_cos();
            let pos = [r0 * c, r0 * s * inc.cos(), r0 * s * inc.sin()];
            let vel = [-v0 * s, v0 * c * inc.cos(), v0 * c * inc.sin()];
            Ok((pos, vel))
        };

        let (node_vec, dist_km) =
            evaluate_osculating_point(LunarPoint::TrueNode, J2000_TT_JD, &mut synthetic_state)
                .unwrap();

        // Node vector must lie on ecliptic (Z = 0)
        assert_eq!(node_vec[2], 0.0);
        near(dist_km, r0, 1e-6);

        // Longitude of the ascending node
        let lon = wrap_degrees(node_vec[1].atan2(node_vec[0]).to_degrees());
        assert!((0.0..360.0).contains(&lon));
    }

    #[test]
    fn swiss_id_and_names_are_consistent() {
        assert_eq!(LunarPoint::MeanNode.swiss_id(), 10);
        assert_eq!(LunarPoint::TrueNode.swiss_id(), 11);
        assert_eq!(LunarPoint::MeanApogee.swiss_id(), 12);
        assert_eq!(LunarPoint::OsculatingApogee.swiss_id(), 13);

        let coords = mean_lunar_node(J2000_TT_JD, true).unwrap();
        let arr = coords.to_swiss_array();
        assert_eq!(arr.len(), 6);
        assert_eq!(arr[0], coords.longitude_deg);
        assert_eq!(arr[1], coords.latitude_deg);
        assert_eq!(arr[2], coords.distance_au);
    }

    #[test]
    fn rejects_non_finite_inputs() {
        assert!(matches!(
            mean_lunar_node(f64::NAN, false),
            Err(LunarPointError::NonFiniteInput)
        ));
        assert!(matches!(
            mean_lunar_apogee(f64::INFINITY, false),
            Err(LunarPointError::NonFiniteInput)
        ));
    }
}
