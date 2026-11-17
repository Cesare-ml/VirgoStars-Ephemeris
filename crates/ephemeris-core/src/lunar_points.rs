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

//! Mathematical lunar points: mean and true (osculating) node, mean and
//! osculating apogee. These are points defined by the Moon's orbit, not bodies:
//! they emit no light, so there is no light-time, aberration or deflection.
//!
//! # Definitions
//!
//! All results are in the **true ecliptic and equinox of date** (or, on request,
//! the true equator of date), i.e. mean-ecliptic-of-date longitude plus the IAU
//! 2000B nutation in longitude.
//!
//! - **Mean node**: Simon et al. (1994) polynomial for the mean ascending node of
//!   the lunar orbit on the ecliptic of date; latitude 0; distance = the nominal
//!   lunar distance 384 400 km.
//! - **Mean apogee** (mean Lilith): the point at orbital longitude `ϖ + 180°` on the
//!   *mean lunar orbit* (node = the mean node, inclination 5.145396°), projected on
//!   the ecliptic, so it has a latitude; `ϖ` is the Simon et al. (1994) mean
//!   longitude of the lunar perigee. Distance `a(1+e)` with `a = 384 400 km`,
//!   `e = 0.054900489`.
//! - **True node**: the ascending node of the Moon's *instantaneous* geocentric
//!   orbital plane. The plane's normal is `r × v` taken from the inertial (ICRF)
//!   geocentric state and rotated to the ecliptic of date; latitude 0. Distance =
//!   radius of the osculating Kepler ellipse at the node, `p / (1 + e cos ω)`.
//! - **Osculating apogee** (true Lilith): the apoapsis direction of the osculating
//!   Kepler ellipse of the Moon about the Earth, `-e` (Laplace–Runge–Lenz vector),
//!   with `μ = GM_earth + GM_moon`; distance `a(1+e)`.
//!
//! Rates are centered differences (±0.01 TDB day) of the final coordinates, so they
//! also include the motion of the output frame. Not provided: the "interpolated"
//! apogee/perigee of other implementations (no published definition).
//!
//! References: Simon, Bretagnon, Chapront-Touzé, Francou, Laskar 1994, A&A 282, 663
//! (mean elements); Chapront-Touzé & Chapront 1991; Meeus, *Astronomical
//! Algorithms* ch. 47 (mean eccentricity, inclination).

use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

use crate::apparent_ecliptic_of_date::{tdb_to_tt_jd, ApparentFrame};
use crate::de441::{Body, De441, De441Error, JulianDateTdb};
use crate::earth_orientation::{
    ecliptic_nutation, iau2000b_nutation, matrix_vector_multiply, true_ecliptic_of_date_matrix,
    OrientationError,
};

const AU_KM: f64 = 149_597_870.7;
const J2000_JD: f64 = 2_451_545.0;
const SPEED_OF_LIGHT_KM_S: f64 = 299_792.458;
/// Nominal lunar semi-major axis, km.
pub const MOON_SEMI_MAJOR_AXIS_KM: f64 = 384_400.0;
/// Mean lunar orbital eccentricity (Meeus).
pub const MOON_MEAN_ECCENTRICITY: f64 = 0.054_900_489;
/// Mean inclination of the lunar orbit to the ecliptic, degrees (Meeus).
pub const MOON_MEAN_INCLINATION_DEG: f64 = 5.145_396;
/// GM of Earth + Moon, km³/s² (DE440/DE441 values 398600.435507 + 4902.800118).
pub const GM_EARTH_PLUS_MOON_KM3_S2: f64 = 403_503.235_625;
/// Half-step of the finite-difference rates, TDB days.
pub const RATE_HALF_STEP_DAYS: f64 = 0.01;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LunarPoint {
    MeanNode,
    TrueNode,
    MeanApogee,
    OsculatingApogee,
}

impl LunarPoint {
    /// Swiss-style body number (10, 11, 12, 13) used by the Node facade.
    pub fn swiss_number(self) -> i32 {
        match self {
            Self::MeanNode => 10,
            Self::TrueNode => 11,
            Self::MeanApogee => 12,
            Self::OsculatingApogee => 13,
        }
    }

    pub fn from_swiss_number(n: i32) -> Option<Self> {
        match n {
            10 => Some(Self::MeanNode),
            11 => Some(Self::TrueNode),
            12 => Some(Self::MeanApogee),
            13 => Some(Self::OsculatingApogee),
            _ => None,
        }
    }

    fn needs_ephemeris(self) -> bool {
        matches!(self, Self::TrueNode | Self::OsculatingApogee)
    }
}

/// Six fields: angles in degrees, distance in AU, rates per TDB day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LunarPointState {
    pub point: LunarPoint,
    pub frame: ApparentFrame,
    pub tt_jd: f64,
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub distance_au: f64,
    pub longitude_rate_deg_per_day: f64,
    pub latitude_rate_deg_per_day: f64,
    pub distance_rate_au_per_day: f64,
}

#[derive(Debug)]
pub enum LunarPointError {
    Ephemeris(De441Error),
    Orientation(OrientationError),
    NonFinite,
    /// The orbital plane is parallel to the ecliptic or the state is degenerate.
    UndefinedNode,
    /// The osculating orbit is circular, parabolic or hyperbolic.
    UndefinedApogee,
    /// This point needs an ephemeris and none was supplied.
    EphemerisRequired,
}

impl fmt::Display for LunarPointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ephemeris(e) => write!(f, "lunar point ephemeris: {e}"),
            Self::Orientation(e) => write!(f, "lunar point orientation: {e}"),
            Self::NonFinite => write!(f, "non-finite epoch"),
            Self::UndefinedNode => write!(f, "the lunar orbital node is undefined"),
            Self::UndefinedApogee => write!(f, "the osculating lunar apogee is undefined"),
            Self::EphemerisRequired => write!(f, "this lunar point needs an ephemeris"),
        }
    }
}

impl Error for LunarPointError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Ephemeris(e) => Some(e),
            Self::Orientation(e) => Some(e),
            _ => None,
        }
    }
}

impl From<De441Error> for LunarPointError {
    fn from(e: De441Error) -> Self {
        Self::Ephemeris(e)
    }
}
impl From<OrientationError> for LunarPointError {
    fn from(e: OrientationError) -> Self {
        Self::Orientation(e)
    }
}

fn wrap_360(d: f64) -> f64 {
    let w = d.rem_euclid(360.0);
    if w >= 360.0 {
        0.0
    } else {
        w
    }
}

fn centuries(tt_jd: f64) -> f64 {
    (tt_jd - J2000_JD) / 36_525.0
}

/// Simon et al. (1994) mean longitude of the ascending node, degrees.
fn mean_node_deg(tt_jd: f64) -> f64 {
    let t = centuries(tt_jd);
    125.044_555_01 - 1_934.136_261_97 * t + 0.002_075_61 * t * t + 2.139e-6 * t.powi(3)
        - 1.65e-8 * t.powi(4)
}

/// Simon et al. (1994) mean longitude of the lunar perigee, degrees.
fn mean_perigee_deg(tt_jd: f64) -> f64 {
    let t = centuries(tt_jd);
    83.353_243_12 + 4_069.013_635_25 * t - 0.010_32 * t * t - t.powi(3) / 80_053.0
        + t.powi(4) / 18_999_000.0
}

type Cartesian = [f64; 3];

fn from_spherical(lon_deg: f64, lat_deg: f64, distance_au: f64) -> Cartesian {
    let (lon, lat) = (lon_deg.to_radians(), lat_deg.to_radians());
    [
        distance_au * lat.cos() * lon.cos(),
        distance_au * lat.cos() * lon.sin(),
        distance_au * lat.sin(),
    ]
}

/// Position of a point in the true ecliptic of date at `tdb`, as ecliptic
/// longitude/latitude (degrees) and distance (AU).
fn ecliptic_position<R: Read + Seek>(
    ephemeris: Option<&mut De441<R>>,
    point: LunarPoint,
    tdb: f64,
) -> Result<(f64, f64, f64), LunarPointError> {
    let tt = tdb_to_tt_jd(tdb).map_err(|_| LunarPointError::NonFinite)?;
    match point {
        LunarPoint::MeanNode => {
            let dpsi = iau2000b_nutation(tt)?.d_psi_rad.to_degrees();
            Ok((
                wrap_360(mean_node_deg(tt) + dpsi),
                0.0,
                MOON_SEMI_MAJOR_AXIS_KM / AU_KM,
            ))
        }
        LunarPoint::MeanApogee => {
            let dpsi = iau2000b_nutation(tt)?.d_psi_rad.to_degrees();
            let node = mean_node_deg(tt).to_radians();
            let u = (mean_perigee_deg(tt) + 180.0).to_radians() - node;
            let i = MOON_MEAN_INCLINATION_DEG.to_radians();
            let (x, y, z) = (u.cos(), u.sin() * i.cos(), u.sin() * i.sin());
            let lon = wrap_360((node + y.atan2(x)).to_degrees() + dpsi);
            let lat = z.asin().to_degrees();
            let distance_km = MOON_SEMI_MAJOR_AXIS_KM * (1.0 + MOON_MEAN_ECCENTRICITY);
            Ok((lon, lat, distance_km / AU_KM))
        }
        LunarPoint::TrueNode | LunarPoint::OsculatingApogee => {
            let ephemeris = ephemeris.ok_or(LunarPointError::EphemerisRequired)?;
            // The Moon's state is taken at the light-time retarded epoch, as for
            // the apparent Moon itself (the reference does the same: it improves
            // the agreement from 0.17" to 0.09" rms for the osculating apogee).
            let mut emission = tdb;
            for _ in 0..3 {
                let r = ephemeris
                    .state(Body::Moon, Body::Earth, JulianDateTdb(emission))?
                    .position_km;
                let d = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
                emission = tdb - d / SPEED_OF_LIGHT_KM_S / 86_400.0;
            }
            let moon = ephemeris.state(Body::Moon, Body::Earth, JulianDateTdb(emission))?;
            let matrix = true_ecliptic_of_date_matrix(tt)?;
            // The inertial geocentric state, expressed in the axes of the ecliptic of date.
            let r = matrix_vector_multiply(matrix, moon.position_km);
            let v = matrix_vector_multiply(matrix, moon.velocity_km_per_day);
            let mu = GM_EARTH_PLUS_MOON_KM3_S2 * 86_400.0 * 86_400.0;
            let r_len = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
            let v2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
            let rv = r[0] * v[0] + r[1] * v[1] + r[2] * v[2];
            let e_vec = [
                ((v2 - mu / r_len) * r[0] - rv * v[0]) / mu,
                ((v2 - mu / r_len) * r[1] - rv * v[1]) / mu,
                ((v2 - mu / r_len) * r[2] - rv * v[2]) / mu,
            ];
            let e = (e_vec[0] * e_vec[0] + e_vec[1] * e_vec[1] + e_vec[2] * e_vec[2]).sqrt();
            let inv_a = 2.0 / r_len - v2 / mu;
            if !e.is_finite() || e <= 0.0 || e >= 1.0 || inv_a <= 0.0 {
                return Err(LunarPointError::UndefinedApogee);
            }
            let a = 1.0 / inv_a;
            if point == LunarPoint::OsculatingApogee {
                let lon = wrap_360(e_vec[1].atan2(e_vec[0]).to_degrees() + 180.0);
                let lat = (-e_vec[2] / e).asin().to_degrees();
                return Ok((lon, lat, a * (1.0 + e) / AU_KM));
            }
            let h = [
                r[1] * v[2] - r[2] * v[1],
                r[2] * v[0] - r[0] * v[2],
                r[0] * v[1] - r[1] * v[0],
            ];
            // Ascending node direction N = z × h = (-h_y, h_x, 0).
            let n = [-h[1], h[0], 0.0];
            let n_len = n[0].hypot(n[1]);
            if !n_len.is_finite() || n_len == 0.0 {
                return Err(LunarPointError::UndefinedNode);
            }
            let h2 = h[0] * h[0] + h[1] * h[1] + h[2] * h[2];
            let p = h2 / mu;
            let cos_omega = (n[0] * e_vec[0] + n[1] * e_vec[1]) / (n_len * e);
            let r_node = p / (1.0 + e * cos_omega.clamp(-1.0, 1.0));
            Ok((wrap_360(n[1].atan2(n[0]).to_degrees()), 0.0, r_node / AU_KM))
        }
    }
}

fn to_frame(
    frame: ApparentFrame,
    tt_jd: f64,
    lon: f64,
    lat: f64,
    distance_au: f64,
) -> Result<(f64, f64, f64), LunarPointError> {
    match frame {
        ApparentFrame::TrueEclipticOfDate => Ok((lon, lat, distance_au)),
        ApparentFrame::TrueEquatorOfDate => {
            let eps = ecliptic_nutation(tt_jd)?.true_obliquity_rad;
            let [x, y, z] = from_spherical(lon, lat, 1.0);
            let (xe, ye, ze) = (
                x,
                y * eps.cos() - z * eps.sin(),
                y * eps.sin() + z * eps.cos(),
            );
            Ok((
                wrap_360(ye.atan2(xe).to_degrees()),
                ze.atan2(xe.hypot(ye)).to_degrees(),
                distance_au,
            ))
        }
    }
}

fn sample<R: Read + Seek>(
    ephemeris: &mut Option<&mut De441<R>>,
    point: LunarPoint,
    frame: ApparentFrame,
    tdb: f64,
) -> Result<(f64, f64, f64, f64), LunarPointError> {
    let tt = tdb_to_tt_jd(tdb).map_err(|_| LunarPointError::NonFinite)?;
    let (lon, lat, dist) = ecliptic_position(ephemeris.as_deref_mut(), point, tdb)?;
    let (a, b, c) = to_frame(frame, tt, lon, lat, dist)?;
    Ok((a, b, c, tt))
}

/// Compute a lunar point at a TDB epoch. `ephemeris` may be `None` for the two
/// mean points; the node and apogee of the osculating orbit require it.
pub fn lunar_point<R: Read + Seek>(
    mut ephemeris: Option<&mut De441<R>>,
    point: LunarPoint,
    frame: ApparentFrame,
    tdb: f64,
) -> Result<LunarPointState, LunarPointError> {
    if !tdb.is_finite() {
        return Err(LunarPointError::NonFinite);
    }
    if point.needs_ephemeris() && ephemeris.is_none() {
        return Err(LunarPointError::EphemerisRequired);
    }
    let h = RATE_HALF_STEP_DAYS;
    let centre = sample(&mut ephemeris, point, frame, tdb)?;
    let before = sample(&mut ephemeris, point, frame, tdb - h)?;
    let after = sample(&mut ephemeris, point, frame, tdb + h)?;
    let wrap = |d: f64| (d + 540.0).rem_euclid(360.0) - 180.0;
    Ok(LunarPointState {
        point,
        frame,
        tt_jd: centre.3,
        longitude_deg: centre.0,
        latitude_deg: centre.1,
        distance_au: centre.2,
        longitude_rate_deg_per_day: wrap(after.0 - before.0) / (2.0 * h),
        latitude_rate_deg_per_day: (after.1 - before.1) / (2.0 * h),
        distance_rate_au_per_day: (after.2 - before.2) / (2.0 * h),
    })
}
