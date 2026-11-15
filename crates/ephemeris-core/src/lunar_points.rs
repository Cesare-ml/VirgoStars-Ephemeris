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
    ecliptic_nutation, iau2000b_nutation, matrix_vector_multiply, true_ecliptic_of_date_matrix,
    OrientationError, DAYS_PER_JULIAN_CENTURY, J2000_TT_JD, JD_TT_MAX, JD_TT_MIN,
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
pub const MOON_MEAN_APOGEE_KM: f64 = 405_500.0;
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

/// Geocentric lunar mathematical point in true equator and equinox of date.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LunarPointEquatorial {
    pub point: LunarPoint,
    pub tt_jd: f64,
    pub right_ascension_deg: f64,
    pub declination_deg: f64,
    pub distance_au: f64,
    pub right_ascension_rate_deg_per_tt_day: f64,
    pub declination_rate_deg_per_tt_day: f64,
    pub distance_rate_au_per_tt_day: f64,
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
    /// Ecliptic longitude, latitude or distance is outside the valid domain.
    InvalidEclipticDirection,
    /// TT and TDB are not plausibly the same reception instant.
    IncoherentEpoch,
    /// The selected point does not support the requested frame.
    UnsupportedFrame,
    /// Orbital momentum is zero or parallel to ecliptic pole; node undefined.
    UndefinedNode,
    /// Orbit is perfectly circular or parabolic; apogee undefined.
    UndefinedApogee,
    /// Right ascension and its rate are undefined at an equatorial pole.
    SingularEquatorialDirection,
}

impl fmt::Display for LunarPointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ephemeris(err) => write!(f, "lunar point ephemeris error: {err}"),
            Self::Orientation(err) => write!(f, "lunar point orientation error: {err}"),
            Self::NonFiniteInput => write!(f, "non-finite input for lunar point evaluation"),
            Self::InvalidEclipticDirection => write!(f, "invalid ecliptic direction or distance"),
            Self::IncoherentEpoch => write!(f, "TT and TDB must describe the same instant"),
            Self::UnsupportedFrame => {
                write!(f, "osculating points require the true ecliptic of date")
            }
            Self::UndefinedNode => write!(f, "lunar orbital node is undefined"),
            Self::UndefinedApogee => write!(f, "lunar osculating apogee is undefined"),
            Self::SingularEquatorialDirection => {
                write!(f, "equatorial right ascension is undefined at the pole")
            }
        }
    }
}

/// Rotate a geocentric true-ecliptic-of-date point to the true equator of
/// date. Rates include the changing true obliquity (a centered ±0.01 TT-day
/// difference); input and output angular rates are per TT day.
pub fn lunar_point_to_equatorial(
    p: LunarPointCoordinates,
) -> Result<LunarPointEquatorial, LunarPointError> {
    if ![
        p.tt_jd,
        p.longitude_deg,
        p.latitude_deg,
        p.distance_au,
        p.longitude_rate_deg_per_day,
        p.latitude_rate_deg_per_day,
        p.distance_rate_au_per_day,
    ]
    .iter()
    .all(|x| x.is_finite())
    {
        return Err(LunarPointError::NonFiniteInput);
    }
    if !(0.0..360.0).contains(&p.longitude_deg)
        || p.latitude_deg.abs() > 90.0
        || p.distance_au <= 0.0
    {
        return Err(LunarPointError::InvalidEclipticDirection);
    }
    let eps = ecliptic_nutation(p.tt_jd)?.true_obliquity_rad;
    let before = p.tt_jd - 0.01;
    let after = p.tt_jd + 0.01;
    let eps_rate = (ecliptic_nutation(after)?.true_obliquity_rad
        - ecliptic_nutation(before)?.true_obliquity_rad)
        / (after - before);
    let (sl, cl) = p.longitude_deg.to_radians().sin_cos();
    let (sb, cb) = p.latitude_deg.to_radians().sin_cos();
    let (se, ce) = eps.sin_cos();
    let dl = p.longitude_rate_deg_per_day.to_radians();
    let db = p.latitude_rate_deg_per_day.to_radians();
    let x = cb * cl;
    let ecl_y = cb * sl;
    let ecl_z = sb;
    let dx = -sb * db * cl - cb * sl * dl;
    let de_y = -sb * db * sl + cb * cl * dl;
    let de_z = cb * db;
    let y = ce * ecl_y - se * ecl_z;
    let z = se * ecl_y + ce * ecl_z;
    let dy = ce * de_y - se * de_z - eps_rate * z;
    let dz = se * de_y + ce * de_z + eps_rate * y;
    let h = x.hypot(y);
    if h < 1e-12 {
        return Err(LunarPointError::SingularEquatorialDirection);
    }
    let dh = (x * dx + y * dy) / h;
    let ra_rate = (x * dy - y * dx) / (h * h);
    let dec_rate = (h * dz - z * dh) / (h * h + z * z);
    Ok(LunarPointEquatorial {
        point: p.point,
        tt_jd: p.tt_jd,
        right_ascension_deg: wrap_degrees(y.atan2(x).to_degrees()),
        declination_deg: z.atan2(h).to_degrees(),
        distance_au: p.distance_au,
        right_ascension_rate_deg_per_tt_day: ra_rate.to_degrees(),
        declination_rate_deg_per_tt_day: dec_rate.to_degrees(),
        distance_rate_au_per_tt_day: p.distance_rate_au_per_day,
    })
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

// Numerical derivative of the explicit IAU 2000B nutation correction.
fn nutation_rate_deg_per_day(tt_jd: f64) -> Result<f64, LunarPointError> {
    let before = tt_jd - 0.01;
    let after = tt_jd + 0.01;
    Ok(
        (iau2000b_nutation(after)?.d_psi_rad - iau2000b_nutation(before)?.d_psi_rad).to_degrees()
            / (after - before),
    )
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
    if !(JD_TT_MIN..JD_TT_MAX).contains(&tt_jd) {
        return Err(LunarPointError::Orientation(
            OrientationError::UnsupportedEpoch { epoch_tt_jd: tt_jd },
        ));
    }
    let t = (tt_jd - J2000_TT_JD) / DAYS_PER_JULIAN_CENTURY;

    // Simon et al. (1994) / Chapront ELP-2000 mean ascending node (degrees):
    let omega_mean =
        125.0445550 - 1934.13626197 * t + 0.0020756 * t.powi(2) + 0.000002139 * t.powi(3)
            - 0.00000001650 * t.powi(4);

    let mut omega_rate = (-1934.13626197 + 2.0 * 0.0020756 * t + 3.0 * 0.000002139 * t.powi(2)
        - 4.0 * 0.00000001650 * t.powi(3))
        / DAYS_PER_JULIAN_CENTURY;

    let mut longitude_deg = wrap_degrees(omega_mean);
    if apparent {
        let nut = iau2000b_nutation(tt_jd)?;
        longitude_deg = wrap_degrees(longitude_deg + nut.d_psi_rad.to_degrees());
        omega_rate += nutation_rate_deg_per_day(tt_jd)?;
    }

    Ok(LunarPointCoordinates {
        point: LunarPoint::MeanNode,
        tt_jd,
        longitude_deg,
        latitude_deg: 0.0,
        distance_au: MOON_MEAN_DISTANCE_KM / AU_KM,
        distance_km: MOON_MEAN_DISTANCE_KM,
        longitude_rate_deg_per_day: omega_rate,
        latitude_rate_deg_per_day: 0.0,
        distance_rate_au_per_day: 0.0,
    })
}

/// Computes the Mean Lunar Apogee (Black Moon Lilith) at epoch `tt_jd`.
///
/// Uses Chapront ELP-2000 mean perigee + 180° in TT Julian centuries.
/// Projects the mean apsis from a fixed 5.145° inclined lunar orbit onto the
/// ecliptic. If `apparent` is true, IAU 2000B longitude nutation is applied.
/// This is an approximate mean model, not established Swiss parity.
pub fn mean_lunar_apogee(
    tt_jd: f64,
    apparent: bool,
) -> Result<LunarPointCoordinates, LunarPointError> {
    if !tt_jd.is_finite() {
        return Err(LunarPointError::NonFiniteInput);
    }
    let evaluate = |epoch: f64| -> Result<(f64, f64), LunarPointError> {
        let t = (epoch - J2000_TT_JD) / DAYS_PER_JULIAN_CENTURY;
        let perigee =
            83.3532431 + 4069.01774306 * t - 0.010325 * t.powi(2) - 0.00001249 * t.powi(3);
        let node = mean_lunar_node(epoch, false)?.longitude_deg;
        let argument = (perigee + 180.0 - node).to_radians();
        let inclination = 5.145_f64.to_radians();
        let (s, c) = argument.sin_cos();
        let mut longitude = wrap_degrees(node + (s * inclination.cos()).atan2(c).to_degrees());
        if apparent {
            longitude = wrap_degrees(longitude + iau2000b_nutation(epoch)?.d_psi_rad.to_degrees());
        }
        let latitude = (s * inclination.sin()).asin().to_degrees();
        Ok((longitude, latitude))
    };
    let (longitude_deg, latitude_deg) = evaluate(tt_jd)?;
    let before_jd = tt_jd - 0.01;
    let after_jd = tt_jd + 0.01;
    let before = evaluate(before_jd)?;
    let after = evaluate(after_jd)?;
    let interval = after_jd - before_jd;
    let distance_au = MOON_MEAN_APOGEE_KM / AU_KM;

    Ok(LunarPointCoordinates {
        point: LunarPoint::MeanApogee,
        tt_jd,
        longitude_deg,
        latitude_deg,
        distance_au,
        distance_km: MOON_MEAN_APOGEE_KM,
        longitude_rate_deg_per_day: shortest_degree_difference(after.0, before.0) / interval,
        latitude_rate_deg_per_day: (after.1 - before.1) / interval,
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
            // Radius at the ascending-node direction of the osculating conic.
            let r_mag = r_ecl[0].hypot(r_ecl[1]).hypot(r_ecl[2]);
            let v_sq: f64 = v_ecl.iter().map(|v| v * v).sum();
            let rv: f64 = r_ecl.iter().zip(v_ecl).map(|(r, v)| r * v).sum();
            let h_sq: f64 = h.iter().map(|v| v * v).sum();
            if !r_mag.is_finite() || r_mag <= 0.0 {
                return Err(LunarPointError::UndefinedNode);
            }
            let mu = GM_EARTH_MOON_KM3_PER_DAY2;
            let e = std::array::from_fn::<_, 3, _>(|i| {
                ((v_sq - mu / r_mag) * r_ecl[i] - rv * v_ecl[i]) / mu
            });
            let denominator = 1.0 + (e[0] * n[0] + e[1] * n[1]) / norm;
            if !denominator.is_finite() || denominator <= 0.0 {
                return Err(LunarPointError::UndefinedNode);
            }
            let distance_km = h_sq / (mu * denominator);
            if !distance_km.is_finite() || distance_km <= 0.0 {
                return Err(LunarPointError::UndefinedNode);
            }
            Ok((n, distance_km))
        }
        LunarPoint::OsculatingApogee => {
            let r_mag = (r_ecl[0].powi(2) + r_ecl[1].powi(2) + r_ecl[2].powi(2)).sqrt();
            if !r_mag.is_finite() || r_mag <= 0.0 {
                return Err(LunarPointError::UndefinedApogee);
            }
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
            if !denom.is_finite() || denom <= 0.0 || e_mag >= 1.0 {
                return Err(LunarPointError::UndefinedApogee);
            }
            let distance_km = (1.0 + e_mag) / denom;
            if !distance_km.is_finite() {
                return Err(LunarPointError::UndefinedApogee);
            }

            Ok((a_vec, distance_km))
        }
        _ => unreachable!(),
    }
}

/// Calculates a mathematical lunar point using a classic DE441 reader.
/// TT and TDB must denote the same instant. Osculating points support only
/// the true ecliptic of date; samples assume dTT/dTDB = 1 over 300 seconds.
pub fn calculate_lunar_point<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    point: LunarPoint,
    tt_jd: f64,
    tdb: JulianDateTdb,
    apparent: bool,
) -> Result<LunarPointCoordinates, LunarPointError> {
    if !tt_jd.is_finite() || !tdb.0.is_finite() || (tt_jd - tdb.0).abs() > 1.0 / SECONDS_PER_DAY {
        return Err(LunarPointError::IncoherentEpoch);
    }
    match point {
        LunarPoint::MeanNode => mean_lunar_node(tt_jd, apparent),
        LunarPoint::MeanApogee => mean_lunar_apogee(tt_jd, apparent),
        LunarPoint::TrueNode | LunarPoint::OsculatingApogee => {
            if !apparent {
                return Err(LunarPointError::UnsupportedFrame);
            }
            let mut state_fn = |epoch_jd: f64| {
                let sample_tdb = JulianDateTdb(tdb.0 + (epoch_jd - tt_jd));
                let s = ephemeris.state(Body::Moon, Body::Earth, sample_tdb)?;
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
            let interval_days = (tt_jd + step_days) - (tt_jd - step_days);
            let longitude_rate_deg_per_day = d_lon / interval_days;

            let lat_prev = vec_prev[2]
                .atan2(vec_prev[0].hypot(vec_prev[1]))
                .to_degrees();
            let lat_next = vec_next[2]
                .atan2(vec_next[0].hypot(vec_next[1]))
                .to_degrees();
            let latitude_rate_deg_per_day = if point == LunarPoint::TrueNode {
                0.0
            } else {
                (lat_next - lat_prev) / interval_days
            };

            let distance_au = dist_center / AU_KM;
            let distance_rate_au_per_day = (dist_next - dist_prev) / (interval_days * AU_KM);

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

    #[test]
    fn equatorial_transform_matches_independent_erfa_position_and_rates() {
        // pyerfa 2.0.1.5: s2c, obl06 + nut00b, rx, c2s. Rates from
        // symmetric ±0.001 day samples with linearly changing input angles.
        let p = LunarPointCoordinates {
            point: LunarPoint::MeanApogee,
            tt_jd: 2_461_354.500_800_741,
            longitude_deg: 276.281_945_530_206_7,
            latitude_deg: -3.906_235_991_198_891_5,
            distance_au: 0.0027,
            distance_km: 0.0027 * AU_KM,
            longitude_rate_deg_per_day: 0.111_518_434_789_885_32,
            latitude_rate_deg_per_day: 0.02,
            distance_rate_au_per_day: 0.00001,
        };
        let eq = lunar_point_to_equatorial(p).unwrap();
        near(eq.right_ascension_deg, 277.049_654_679_266_5, 1e-10);
        near(eq.declination_deg, -27.190_505_893_487_636, 1e-10);
        near(
            eq.right_ascension_rate_deg_per_tt_day,
            0.123_830_728_597_340_57,
            1e-7,
        );
        near(
            eq.declination_rate_deg_per_tt_day,
            0.025_438_465_277_360_933,
            1e-7,
        );
        assert_eq!(eq.distance_au, p.distance_au);
        assert_eq!(eq.distance_rate_au_per_tt_day, p.distance_rate_au_per_day);
        let eps = ecliptic_nutation(p.tt_jd).unwrap().true_obliquity_deg();
        let pole = LunarPointCoordinates {
            longitude_deg: 90.0,
            latitude_deg: 90.0 - eps,
            ..p
        };
        assert!(matches!(
            lunar_point_to_equatorial(pole),
            Err(LunarPointError::SingularEquatorialDirection)
        ));
        assert!(matches!(
            lunar_point_to_equatorial(LunarPointCoordinates {
                longitude_deg: 360.0,
                ..p
            }),
            Err(LunarPointError::InvalidEclipticDirection)
        ));
        let near_wrap = lunar_point_to_equatorial(LunarPointCoordinates {
            longitude_deg: 359.99,
            latitude_deg: 0.0,
            ..p
        })
        .unwrap();
        assert!(near_wrap.right_ascension_deg > 359.0 && near_wrap.right_ascension_deg < 360.0);
    }

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
        // The orbit-plane apogee is projected onto the inclined ecliptic.
        near(coords.longitude_deg, 263.4681027428204, 1e-10);
        near(coords.latitude_deg, 3.4194600919590745, 1e-10);
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

    #[test]
    fn apparent_mean_rates_track_their_returned_positions() {
        let t = 2_461_354.5;
        let before = t - 0.02;
        let after = t + 0.02;
        for calculate in [mean_lunar_node, mean_lunar_apogee] {
            let center = calculate(t, true).unwrap();
            let a = calculate(before, true).unwrap();
            let b = calculate(after, true).unwrap();
            let span = after - before;
            near(
                center.longitude_rate_deg_per_day,
                shortest_degree_difference(b.longitude_deg, a.longitude_deg) / span,
                1e-7,
            );
            near(
                center.latitude_rate_deg_per_day,
                (b.latitude_deg - a.latitude_deg) / span,
                1e-7,
            );
        }
    }

    #[test]
    fn unbound_osculating_apogee_is_not_replaced_with_current_distance() {
        let r = MOON_MEAN_DISTANCE_KM;
        let speed = (3.0 * GM_EARTH_MOON_KM3_PER_DAY2 / r).sqrt();
        let state = |_| Ok(([r, 0.0, 0.0], [0.0, speed, speed * 0.1]));
        assert!(matches!(
            evaluate_osculating_point(LunarPoint::OsculatingApogee, J2000_TT_JD, state),
            Err(LunarPointError::UndefinedApogee)
        ));
    }
}
