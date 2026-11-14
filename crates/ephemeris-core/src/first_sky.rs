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

//! First experimental time-and-place geometric planet positions.
//!
//! Uses an explicit UTC offset, classic DE441, IAU 2006 mean precession and
//! WGS84. Results are approximate topocentric geometric directions, not
//! apparent positions or a complete Swiss Ephemeris replacement.

use std::error::Error;
use std::f64::consts::{PI, TAU};
use std::fmt;
use std::io::{Read, Seek};

use crate::calendar::{Calendar, CalendarDateTime, CalendarError, JulianDate, TimeScale};
use crate::de441::{Body, De441, De441Error};
use crate::time::{tai_to_tt, AtomicTimeError};

const DAY_SECONDS: f64 = 86_400.0;
const J2000: f64 = 2_451_545.0;
const ARCSEC_TO_RAD: f64 = PI / (180.0 * 3600.0);
const WGS84_A_KM: f64 = 6378.137;
const WGS84_FLATTENING: f64 = 1.0 / 298.257_223_563;

// MJD at UTC midnight when the new TAI-UTC value becomes effective.
// IERS Bulletin C 72, July 2026; this table expires at 2027-06-28 UTC.
const LEAP_SECONDS: &[(i64, i32)] = &[
    (41317, 10),
    (41499, 11),
    (41683, 12),
    (42048, 13),
    (42413, 14),
    (42778, 15),
    (43144, 16),
    (43509, 17),
    (43874, 18),
    (44239, 19),
    (44786, 20),
    (45151, 21),
    (45516, 22),
    (46247, 23),
    (47161, 24),
    (47892, 25),
    (48257, 26),
    (48804, 27),
    (49169, 28),
    (49534, 29),
    (50083, 30),
    (50630, 31),
    (51179, 32),
    (53736, 33),
    (54832, 34),
    (56109, 35),
    (57204, 36),
    (57754, 37),
];

/// UTC-offset timestamp, parsed from `YYYY-MM-DDTHH:MM:SS[.fraction]Z`
/// or a fixed offset such as `+01:00`. Leap-second labels are unsupported.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CivilTimestamp {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: f64,
    /// Civil local time minus UTC, in minutes.
    pub utc_offset_minutes: i16,
}

/// WGS84 geodetic latitude and east-positive longitude in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observer {
    pub latitude_deg: f64,
    pub longitude_deg_east: f64,
    pub height_m: f64,
}

/// All directions are geometric and topocentric, in mean axes of date.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetPosition {
    pub body: Body,
    pub mean_ecliptic_longitude_deg: f64,
    pub mean_ecliptic_latitude_deg: f64,
    pub mean_right_ascension_deg: f64,
    pub mean_declination_deg: f64,
    pub altitude_deg: f64,
    /// Azimuth measured east from north; undefined at the zenith or nadir.
    pub azimuth_deg: Option<f64>,
    pub distance_km: f64,
}

/// Explicit limitations applying to every result from this experimental model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkyApproximation {
    Ut1EqualsUtc,
    TruncatedTdbMinusTt,
    NoNutation,
    NoLightTime,
    NoAberration,
    NoGravitationalDeflection,
    NoPolarMotion,
    NoAtmosphericRefraction,
    OuterPlanetSystemBarycenters,
}

pub const MODEL_APPROXIMATIONS: &[SkyApproximation] = &[
    SkyApproximation::Ut1EqualsUtc,
    SkyApproximation::TruncatedTdbMinusTt,
    SkyApproximation::NoNutation,
    SkyApproximation::NoLightTime,
    SkyApproximation::NoAberration,
    SkyApproximation::NoGravitationalDeflection,
    SkyApproximation::NoPolarMotion,
    SkyApproximation::NoAtmosphericRefraction,
    SkyApproximation::OuterPlanetSystemBarycenters,
];

#[derive(Clone, Debug, PartialEq)]
pub struct SkyPositions {
    pub utc_day_number: i64,
    pub utc_fraction_of_day: f64,
    pub tt_julian_day: f64,
    pub tdb_julian_day: f64,
    pub tai_minus_utc_seconds: i32,
    pub observer: Observer,
    pub approximations: &'static [SkyApproximation],
    pub positions: Vec<PlanetPosition>,
}

#[derive(Debug)]
pub enum SkyError {
    InvalidInput(&'static str),
    UnsupportedUtc,
    Calendar(CalendarError),
    AtomicTime(AtomicTimeError),
    Ephemeris(De441Error),
}
impl fmt::Display for SkyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(s) => write!(f, "invalid input: {s}"),
            Self::UnsupportedUtc => write!(
                f,
                "UTC is outside the validated IERS table (1972-01-01 through 2027-06-27)"
            ),
            Self::Calendar(e) => write!(f, "calendar: {e}"),
            Self::AtomicTime(e) => write!(f, "atomic time: {e}"),
            Self::Ephemeris(e) => write!(f, "ephemeris: {e}"),
        }
    }
}
impl Error for SkyError {}
impl From<CalendarError> for SkyError {
    fn from(e: CalendarError) -> Self {
        Self::Calendar(e)
    }
}
impl From<AtomicTimeError> for SkyError {
    fn from(e: AtomicTimeError) -> Self {
        Self::AtomicTime(e)
    }
}
impl From<De441Error> for SkyError {
    fn from(e: De441Error) -> Self {
        Self::Ephemeris(e)
    }
}

impl CivilTimestamp {
    /// Parse a timestamp with an explicit `Z` or numeric `±HH:MM` offset.
    pub fn parse(value: &str) -> Result<Self, SkyError> {
        if !value.is_ascii() || value.len() < 20 {
            return Err(SkyError::InvalidInput(
                "timestamp must include a numeric UTC offset or Z",
            ));
        }
        let (date, clock_with_offset) = value
            .split_once('T')
            .ok_or(SkyError::InvalidInput("expected YYYY-MM-DDTHH:MM:SS±HH:MM"))?;
        if date.len() != 10 || &date[4..5] != "-" || &date[7..8] != "-" {
            return Err(SkyError::InvalidInput("invalid date syntax"));
        }
        if !date[0..4].bytes().all(|b| b.is_ascii_digit())
            || !date[5..7].bytes().all(|b| b.is_ascii_digit())
            || !date[8..10].bytes().all(|b| b.is_ascii_digit())
        {
            return Err(SkyError::InvalidInput("invalid date digits"));
        }
        let year = date[0..4]
            .parse()
            .map_err(|_| SkyError::InvalidInput("invalid year"))?;
        let month = date[5..7]
            .parse()
            .map_err(|_| SkyError::InvalidInput("invalid month"))?;
        let day = date[8..10]
            .parse()
            .map_err(|_| SkyError::InvalidInput("invalid day"))?;
        let (clock, utc_offset_minutes) = if let Some(clock) = clock_with_offset.strip_suffix('Z') {
            (clock, 0)
        } else {
            if clock_with_offset.len() < 14 {
                return Err(SkyError::InvalidInput("timestamp requires ±HH:MM"));
            }
            let at = clock_with_offset.len() - 6;
            let offset = &clock_with_offset[at..];
            let sign = match &offset[0..1] {
                "+" => 1,
                "-" => -1,
                _ => return Err(SkyError::InvalidInput("timestamp requires ±HH:MM")),
            };
            if &offset[3..4] != ":" {
                return Err(SkyError::InvalidInput("invalid UTC offset"));
            }
            if !offset[1..3].bytes().all(|b| b.is_ascii_digit())
                || !offset[4..6].bytes().all(|b| b.is_ascii_digit())
            {
                return Err(SkyError::InvalidInput("invalid UTC offset digits"));
            }
            let h: i16 = offset[1..3]
                .parse()
                .map_err(|_| SkyError::InvalidInput("invalid UTC offset"))?;
            let m: i16 = offset[4..6]
                .parse()
                .map_err(|_| SkyError::InvalidInput("invalid UTC offset"))?;
            if h > 14 || m > 59 || (h == 14 && m != 0) {
                return Err(SkyError::InvalidInput("UTC offset must be within ±14:00"));
            }
            (&clock_with_offset[..at], sign * (h * 60 + m))
        };
        if clock.len() < 8 || &clock[2..3] != ":" || &clock[5..6] != ":" {
            return Err(SkyError::InvalidInput("invalid clock syntax"));
        }
        if !clock[0..2].bytes().all(|b| b.is_ascii_digit())
            || !clock[3..5].bytes().all(|b| b.is_ascii_digit())
            || !clock[6..8].bytes().all(|b| b.is_ascii_digit())
            || (clock.len() > 8
                && (&clock[8..9] != "."
                    || clock.len() == 9
                    || !clock[9..].bytes().all(|b| b.is_ascii_digit())))
        {
            return Err(SkyError::InvalidInput("invalid clock digits"));
        }
        let hour = clock[0..2]
            .parse()
            .map_err(|_| SkyError::InvalidInput("invalid hour"))?;
        let minute = clock[3..5]
            .parse()
            .map_err(|_| SkyError::InvalidInput("invalid minute"))?;
        let second = clock[6..]
            .parse()
            .map_err(|_| SkyError::InvalidInput("invalid second"))?;
        let timestamp = Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            utc_offset_minutes,
        };
        timestamp.validate()?;
        Ok(timestamp)
    }

    fn validate(self) -> Result<(), SkyError> {
        if !(-840..=840).contains(&self.utc_offset_minutes) {
            return Err(SkyError::InvalidInput("UTC offset must be within ±14:00"));
        }
        // Use the calendar module only to validate/encode civil clock fields.
        // This internal label is arithmetic only; no clock is asserted to be UT1.
        CalendarDateTime {
            year: self.year,
            month: self.month,
            day: self.day,
            hour: self.hour,
            minute: self.minute,
            second: self.second,
            scale: TimeScale::Ut1,
        }
        .to_julian_date(Calendar::ProlepticGregorian)?;
        Ok(())
    }
}

fn shift_day(
    day: i64,
    fraction: f64,
    seconds: f64,
    scale: TimeScale,
) -> Result<JulianDate, SkyError> {
    let shifted = fraction + seconds / DAY_SECONDS;
    let (day, fraction) = if shifted < 0.0 {
        (day - 1, shifted + 1.0)
    } else if shifted >= 1.0 {
        (day + 1, shifted - 1.0)
    } else {
        (day, shifted)
    };
    Ok(JulianDate::new(day, fraction, scale)?)
}

#[derive(Clone, Copy)]
struct Epochs {
    utc: JulianDate,
    tt_jd: f64,
    tdb_jd: f64,
    tai_utc: i32,
}

fn epochs(civil: CivilTimestamp) -> Result<Epochs, SkyError> {
    civil.validate()?;
    let local = CalendarDateTime {
        year: civil.year,
        month: civil.month,
        day: civil.day,
        hour: civil.hour,
        minute: civil.minute,
        second: civil.second,
        scale: TimeScale::Ut1,
    }
    .to_julian_date(Calendar::ProlepticGregorian)?;
    // UTC is encoded using nominal 86400-second civil days; leap-second labels
    // are rejected. UT1 will later be explicitly approximated as UTC.
    let utc = shift_day(
        local.day_number(),
        local.midnight_fraction(),
        -f64::from(civil.utc_offset_minutes) * 60.0,
        TimeScale::Ut1,
    )?;
    let mjd = utc.day_number() - 2_400_001;
    let expiry = CalendarDateTime {
        year: 2027,
        month: 6,
        day: 28,
        hour: 0,
        minute: 0,
        second: 0.0,
        scale: TimeScale::Ut1,
    }
    .to_julian_date(Calendar::ProlepticGregorian)?
    .day_number();
    if mjd < 41317 || utc.day_number() >= expiry {
        return Err(SkyError::UnsupportedUtc);
    }
    let tai_utc = LEAP_SECONDS
        .iter()
        .rev()
        .find(|(day, _)| *day <= mjd)
        .map(|(_, value)| *value)
        .ok_or(SkyError::UnsupportedUtc)?;
    let tai = shift_day(
        utc.day_number(),
        utc.midnight_fraction(),
        f64::from(tai_utc),
        TimeScale::Tai,
    )?;
    let tt = tai_to_tt(tai)?;
    let (tt_a, tt_b) = tt.parts()?;
    let tt_jd = tt_a + tt_b;
    // USNO Circular 179, section 2.3: truncated TDB−TT series, seconds.
    let t = ((tt_a - J2000) + tt_b) / 36_525.0;
    let tdb_tt = 0.001657 * (628.3076 * t + 6.2401).sin()
        + 0.000022 * (575.3385 * t + 4.2970).sin()
        + 0.000014 * (1256.6152 * t + 6.1969).sin()
        + 0.000005 * (606.9777 * t + 4.0212).sin()
        + 0.000005 * (52.9691 * t + 0.4444).sin()
        + 0.000002 * (21.3299 * t + 5.5431).sin()
        + 0.000010 * t * (628.3076 * t + 4.2490).sin();
    let tdb = shift_day(
        tt.day_number(),
        tt.midnight_fraction(),
        tdb_tt,
        TimeScale::Tdb,
    )?;
    let tdb_jd = tdb.to_de441_epoch()?.0;
    Ok(Epochs {
        utc,
        tt_jd,
        tdb_jd,
        tai_utc,
    })
}

type Matrix = [[f64; 3]; 3];
fn multiply(a: Matrix, b: Matrix) -> Matrix {
    let mut output = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            output[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    output
}
fn x_rotation(angle: f64) -> Matrix {
    let (s, c) = angle.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]]
}
fn z_rotation(angle: f64) -> Matrix {
    let (s, c) = angle.sin_cos();
    [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]]
}
fn poly(t: f64, coefficients: [f64; 6]) -> f64 {
    coefficients.iter().rev().fold(0.0, |sum, c| sum * t + c) * ARCSEC_TO_RAD
}

/// IAU 2006 Fukushima–Williams bias/precession, ICRF to mean equator of TT date.
fn mean_date_matrix(tt_jd: f64) -> (Matrix, f64) {
    let t = (tt_jd - J2000) / 36_525.0;
    let g = poly(
        t,
        [
            -0.052928,
            10.556378,
            0.4932044,
            -0.00031238,
            -0.000002788,
            0.0000000260,
        ],
    );
    let p = poly(
        t,
        [
            84381.412819,
            -46.811016,
            0.0511268,
            0.00053289,
            -0.000000440,
            -0.0000000176,
        ],
    );
    let q = poly(
        t,
        [
            -0.041775,
            5038.481484,
            1.5584175,
            -0.00018522,
            -0.000026452,
            -0.0000000148,
        ],
    );
    let e = poly(
        t,
        [
            84381.406,
            -46.836769,
            -0.0001831,
            0.00200340,
            -0.000000576,
            -0.0000000434,
        ],
    );
    (
        multiply(
            multiply(multiply(x_rotation(-e), z_rotation(-q)), x_rotation(p)),
            z_rotation(g),
        ),
        e,
    )
}
fn rotate(matrix: Matrix, v: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| (0..3).map(|j| matrix[i][j] * v[j]).sum())
}
fn gmst(utc: JulianDate, tt_jd: f64) -> f64 {
    // USNO approximate GMST; substitute UTC for UT1 explicitly.
    let d0 = utc.day_number() as f64 - 0.5 - J2000;
    let h = utc.midnight_fraction() * 24.0;
    let t = (tt_jd - J2000) / 36_525.0;
    let hours = 6.697375 + 0.065707485828 * d0 + 1.0027379 * h + 0.0854103 * t + 0.0000258 * t * t;
    (hours * PI / 12.0).rem_euclid(TAU)
}
fn spherical(v: [f64; 3]) -> Result<(f64, f64, f64), SkyError> {
    let planar = v[0].hypot(v[1]);
    let distance = planar.hypot(v[2]);
    if !distance.is_finite() || distance == 0.0 || planar == 0.0 {
        return Err(SkyError::InvalidInput("undefined geometric direction"));
    }
    Ok((
        normalized_degrees(v[1].atan2(v[0])),
        v[2].atan2(planar).to_degrees(),
        distance,
    ))
}
fn normalized_degrees(angle: f64) -> f64 {
    let value = angle.rem_euclid(TAU).to_degrees();
    if value == 0.0 || value >= 360.0 {
        0.0
    } else {
        value
    }
}
fn validate_observer(site: Observer) -> Result<(), SkyError> {
    if !site.latitude_deg.is_finite()
        || !site.longitude_deg_east.is_finite()
        || !site.height_m.is_finite()
        || !(-90.0..=90.0).contains(&site.latitude_deg)
        || !(-180.0..=180.0).contains(&site.longitude_deg_east)
        || !(-12_000.0..=100_000.0).contains(&site.height_m)
    {
        return Err(SkyError::InvalidInput(
            "expected latitude ±90°, longitude ±180° east, and height -12000..100000 m",
        ));
    }
    Ok(())
}

fn observer_vector(site: Observer, local_angle: f64) -> [f64; 3] {
    let (sin_lat, cos_lat) = site.latitude_deg.to_radians().sin_cos();
    let (sin_ang, cos_ang) = local_angle.sin_cos();
    let e2 = WGS84_FLATTENING * (2.0 - WGS84_FLATTENING);
    let n = WGS84_A_KM / (1.0 - e2 * sin_lat * sin_lat).sqrt();
    let height = site.height_m / 1000.0;
    [
        (n + height) * cos_lat * cos_ang,
        (n + height) * cos_lat * sin_ang,
        (n * (1.0 - e2) + height) * sin_lat,
    ]
}

fn horizontal_direction(
    v: [f64; 3],
    latitude: f64,
    angle: f64,
) -> Result<(f64, Option<f64>), SkyError> {
    let (sin_lat, cos_lat) = latitude.sin_cos();
    let (sin_ang, cos_ang) = angle.sin_cos();
    let east = -sin_ang * v[0] + cos_ang * v[1];
    let north = -sin_lat * cos_ang * v[0] - sin_lat * sin_ang * v[1] + cos_lat * v[2];
    let up = cos_lat * cos_ang * v[0] + cos_lat * sin_ang * v[1] + sin_lat * v[2];
    let horizontal = east.hypot(north);
    let distance = horizontal.hypot(up);
    if !distance.is_finite() || distance == 0.0 {
        return Err(SkyError::InvalidInput("non-finite topocentric direction"));
    }
    let altitude_deg = up.atan2(horizontal).to_degrees();
    let azimuth_deg = if horizontal <= 1e-14 * distance {
        None
    } else {
        Some(normalized_degrees(east.atan2(north)))
    };
    Ok((altitude_deg, azimuth_deg))
}

/// Calculate an experimental topocentric geometric sky from the supplied DE441 reader.
/// Outer planets are their system barycenters, not planetary centers.
pub fn calculate<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    civil: CivilTimestamp,
    site: Observer,
) -> Result<SkyPositions, SkyError> {
    validate_observer(site)?;
    let epoch = epochs(civil)?;
    let (bias_precession, obliquity) = mean_date_matrix(epoch.tt_jd);
    let angle = gmst(epoch.utc, epoch.tt_jd) + site.longitude_deg_east.to_radians();
    let obs = observer_vector(site, angle);
    let bodies = [
        Body::Sun,
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::MarsBarycenter,
        Body::JupiterBarycenter,
        Body::SaturnBarycenter,
        Body::UranusBarycenter,
        Body::NeptuneBarycenter,
        Body::PlutoBarycenter,
    ];
    let mut positions = Vec::with_capacity(bodies.len());
    for body in bodies {
        let state =
            ephemeris.state(body, Body::Earth, crate::de441::JulianDateTdb(epoch.tdb_jd))?;
        let geocentric = rotate(bias_precession, state.position_km);
        let topocentric = std::array::from_fn(|i| geocentric[i] - obs[i]);
        let (ra, dec, distance_km) = spherical(topocentric)?;
        let (longitude, latitude, _) = spherical(rotate(x_rotation(obliquity), topocentric))?;
        let (altitude_deg, azimuth_deg) =
            horizontal_direction(topocentric, site.latitude_deg.to_radians(), angle)?;
        positions.push(PlanetPosition {
            body,
            mean_ecliptic_longitude_deg: longitude,
            mean_ecliptic_latitude_deg: latitude,
            mean_right_ascension_deg: ra,
            mean_declination_deg: dec,
            altitude_deg,
            azimuth_deg,
            distance_km,
        });
    }
    Ok(SkyPositions {
        utc_day_number: epoch.utc.day_number(),
        utc_fraction_of_day: epoch.utc.midnight_fraction(),
        tt_julian_day: epoch.tt_jd,
        tdb_julian_day: epoch.tdb_jd,
        tai_minus_utc_seconds: epoch.tai_utc,
        observer: site,
        approximations: MODEL_APPROXIMATIONS,
        positions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64, tolerance: f64) {
        assert!((a - b).abs() <= tolerance, "{a} != {b}");
    }
    fn at(value: &str) -> Epochs {
        epochs(CivilTimestamp::parse(value).unwrap()).unwrap()
    }

    // Modification (2026-11-14): independently anchor the IAU 2006 mean
    // obliquity to ERFA's published eraObl06 conformance fixture (TT, radians).
    #[test]
    fn iau_2006_mean_obliquity_matches_erfa_obl06_reference() {
        // ERFA src/t_erfa_c.c, t_obl06: TT JD = 2400000.5 + 54388.0.
        let (_, obliquity_rad) = mean_date_matrix(2_400_000.5 + 54_388.0);
        near(obliquity_rad, 0.409_074_922_938_725_8, 1.0e-14);
    }

    // Modification (2026-11-14): guard orthonormality and handedness of the
    // fixed-epoch ICRF-to-mean-equator bias/precession rotation.
    #[test]
    fn mean_date_precession_matrix_preserves_orthonormal_right_handed_axes() {
        for jd_tt in [
            2_451_545.0,
            2_400_000.5 + 50_123.999_9,
            2_400_000.5 + 54_388.0,
        ] {
            let (matrix, _) = mean_date_matrix(jd_tt);
            for i in 0..3 {
                for j in 0..3 {
                    let dot: f64 = (0..3).map(|k| matrix[i][k] * matrix[j][k]).sum();
                    near(dot, if i == j { 1.0 } else { 0.0 }, 2.0e-14);
                }
            }
            let determinant = matrix[0][0]
                * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
                - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
                + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]);
            near(determinant, 1.0, 2.0e-14);
        }
    }

    #[test]
    fn civil_offset_and_leap_seconds_are_applied_after_utc_normalization() {
        let a = at("2026-11-10T13:30:00+01:00");
        let b = at("2026-11-10T12:30:00Z");
        near(a.tt_jd, b.tt_jd, 1e-12);
        near(a.tdb_jd, b.tdb_jd, 1e-12);
        assert_eq!(a.tai_utc, 37);
        assert_eq!(at("2017-01-01T00:30:00+01:00").tai_utc, 36);
        let before = at("2016-12-31T23:59:59Z");
        let after = at("2017-01-01T00:00:00Z");
        assert_eq!((before.tai_utc, after.tai_utc), (36, 37));
        near((after.tt_jd - before.tt_jd) * DAY_SECONDS, 2.0, 0.0001);
    }

    #[test]
    fn timestamp_validation_rejects_unsupported_and_ambiguous_inputs() {
        for value in [
            "2026-11-10T12:00:00",
            "2026-11-10T12:00:60Z",
            "2026-02-30T12:00:00Z",
            "2026-11-10T12:00:00+14:01",
            "2026-11-10T12:00:00--1:00",
            "2026-11-10T12:00:0e3Z",
            "2026-11-10T12:00:00.Z",
        ] {
            assert!(CivilTimestamp::parse(value).is_err(), "{value}");
        }
        for value in ["1971-12-31T23:59:59Z", "2027-06-28T00:00:00Z"] {
            assert!(matches!(
                epochs(CivilTimestamp::parse(value).unwrap()),
                Err(SkyError::UnsupportedUtc)
            ));
        }
        assert!(epochs(CivilTimestamp::parse("2027-06-27T23:59:59Z").unwrap()).is_ok());
    }

    #[test]
    fn precession_matches_independent_erfa_pmat06_reference() {
        // ERFA t_pmat06 numerical conformance data; see THIRD_PARTY_NOTICES.md.
        let expected = [
            [
                0.999_999_550_517_600_7,
                0.000_869_540_461_734_820_8,
                0.000_377_973_520_186_558_9,
            ],
            [
                -0.000_869_540_472_377_203_1,
                0.999_999_621_949_602_7,
                -0.000_000_136_175_249_708_027,
            ],
            [
                -0.000_377_973_495_703_408_95,
                -0.000_000_192_488_084_789_445_7,
                0.999_999_928_567_997_2,
            ],
        ];
        let (matrix, _) = mean_date_matrix(2_400_000.5 + 50_123.999_9);
        for i in 0..3 {
            for j in 0..3 {
                near(matrix[i][j], expected[i][j], 1e-14);
            }
        }
    }

    #[test]
    fn gmst_and_wgs84_geometries_match_reference_and_cardinal_directions() {
        let j2000 = JulianDate::new(2_451_545, 0.5, TimeScale::Ut1).unwrap();
        near(gmst(j2000, J2000).to_degrees() / 15.0, 18.697375, 0.00005);
        let equator = Observer {
            latitude_deg: 0.0,
            longitude_deg_east: 0.0,
            height_m: 0.0,
        };
        let p = observer_vector(equator, 0.0);
        near(p[0], 6378.137, 1e-9);
        near(p[1], 0.0, 1e-9);
        near(p[2], 0.0, 1e-9);
        let pole = observer_vector(
            Observer {
                latitude_deg: 90.0,
                ..equator
            },
            0.0,
        );
        near(pole[2], 6_356.752_314_245, 1e-8);
        let (alt, az) = horizontal_direction([0.0, 1.0, 0.0], 0.0, 0.0).unwrap();
        near(alt, 0.0, 1e-12);
        near(az.unwrap(), 90.0, 1e-12);
        let (alt, az) = horizontal_direction([0.0, 0.0, 1.0], 0.0, 0.0).unwrap();
        near(alt, 0.0, 1e-12);
        near(az.unwrap(), 0.0, 1e-12);
        let (alt, az) = horizontal_direction([1.0, 0.0, 0.0], 0.0, 0.0).unwrap();
        near(alt, 90.0, 1e-12);
        assert_eq!(az, None);
        assert_eq!(spherical([1.0, -1e-20, 0.0]).unwrap().0, 0.0);
    }

    #[test]
    #[ignore = "requires DE441_PATH pointing to the external JPL binary"]
    fn supplied_kernel_produces_time_and_place_dependent_planet_positions() {
        let path = std::env::var_os("DE441_PATH").expect("set DE441_PATH");
        let mut eph = De441::open(path).unwrap();
        let time = CivilTimestamp::parse("2026-11-10T13:30:00+01:00").unwrap();
        let milan = Observer {
            latitude_deg: 45.4642,
            longitude_deg_east: 9.19,
            height_m: 120.0,
        };
        let result = calculate(&mut eph, time, milan).unwrap();
        assert!(result
            .approximations
            .contains(&SkyApproximation::Ut1EqualsUtc));
        assert_eq!(result.positions.len(), 10);
        assert_eq!(result.positions[0].body, Body::Sun);
        assert_eq!(result.positions[1].body, Body::Moon);
        assert!(result
            .positions
            .iter()
            .all(|p| (0.0..360.0).contains(&p.mean_ecliptic_longitude_deg)
                && (-90.0..=90.0).contains(&p.altitude_deg)
                && p.distance_km > 0.0));
        let offset_time = CivilTimestamp::parse("2026-11-10T12:30:00Z").unwrap();
        let same = calculate(&mut eph, offset_time, milan).unwrap();
        near(result.tdb_julian_day, same.tdb_julian_day, 0.0);
        near(
            result.positions[1].mean_ecliptic_longitude_deg,
            same.positions[1].mean_ecliptic_longitude_deg,
            1e-10,
        );
        let opposite = calculate(
            &mut eph,
            time,
            Observer {
                latitude_deg: -45.4642,
                longitude_deg_east: -170.81,
                height_m: 0.0,
            },
        )
        .unwrap();
        assert!(
            (result.positions[1].mean_ecliptic_longitude_deg
                - opposite.positions[1].mean_ecliptic_longitude_deg)
                .abs()
                > 0.01
        );
    }
}
