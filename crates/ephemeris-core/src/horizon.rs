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

//! Atmospheric refraction and horizon coordinates.
//!
//! # Refraction
//!
//! Two published refraction models are provided, both as functions of
//! atmospheric pressure `P` (hPa) and temperature `T` (°C):
//!
//! * **Plain** (`swe_refrac` behaviour): for a *true* altitude `h` (degrees),
//!   `h ≤ 15°`: Sæmundsson (1986), `R = 1.02′ / tan(h + 10.3/(h + 5.11))`;
//!   `h > 15°`: `R = 58.276″ tan z − 0.0824″ tan³z` with `z = 90° − h`; both
//!   scaled by `(P/1010)(283/(273 + T))`. Applied only when the refracted
//!   altitude is above the horizon.
//! * **Extended** (`swe_azalt`/`swe_rise_trans` behaviour): the formula of the
//!   *Astronomical Almanac* as a function of the **apparent** altitude `a`:
//!   `R = 0.97′ / tan a` for `a > 17.904104638432°`, otherwise
//!   `R = (34.46 + 4.23a + 0.004a²) / (1 + 0.505a + 0.0845a²)` arcminutes;
//!   scaled by `((P − 80)/930) / (1 + 8·10⁻⁵ (R + 39)(T − 10))`. True → apparent
//!   solves `a = h + R(a)` by iteration; apparent → true is `h = a − R(a)`.
//!   Refraction is applied only if the apparent altitude is not below the
//!   **horizon dip** of the observer's height.
//!
//! The dip is `−acos(1/(1 + H/R⊕)) · sqrt(d)` with
//! `d = 1 − 1.848 κ P / (273.15 + T)²`, `κ = (0.0342 + λ)/(0.154 · 0.0238)` and
//! lapse rate `λ` (K/m), after Schaefer & Liller (1990).
//!
//! A pressure of zero means "unknown" in the reference convention and is
//! resolved by the caller with [`estimated_pressure_hpa`]
//! (`1013.25 (1 − 0.0065 H / 288)^5.255`).
//!
//! # Horizon coordinates
//!
//! [`equatorial_to_horizontal`] turns an equatorial-of-date direction and the
//! local apparent sidereal time into altitude and azimuth. Azimuth is measured
//! here from **north through east**; the Swiss-shaped facade converts to the
//! south-based convention. Geodetic latitude is used as given.

use std::error::Error;
use std::fmt;

use crate::earth_orientation::{ecliptic_nutation, OrientationError};
use crate::house_angles::{house_angles, HouseAnglesError};

/// Standard sea-level pressure, hPa.
pub const STANDARD_PRESSURE_HPA: f64 = 1013.25;
/// Standard tropospheric lapse rate, K/m.
pub const STANDARD_LAPSE_RATE: f64 = 0.0065;
const EXTENDED_SWITCH_ALTITUDE_DEG: f64 = 17.904_104_638_432;
const PLAIN_SWITCH_ALTITUDE_DEG: f64 = 15.0;
const DIP_EARTH_RADIUS_M: f64 = 6_378_136.6;

/// Pressure estimated from the height above sea level, hPa.
pub fn estimated_pressure_hpa(height_m: f64) -> f64 {
    STANDARD_PRESSURE_HPA * (1.0 - 0.0065 * height_m / 288.0).max(0.0).powf(5.255)
}

fn valid_atmosphere(pressure_hpa: f64, temperature_c: f64) -> bool {
    pressure_hpa.is_finite()
        && pressure_hpa > 0.0
        && temperature_c.is_finite()
        && temperature_c > -273.0
}

// ---------------------------------------------------------------- plain model

fn plain_refraction_deg(true_altitude_deg: f64, pressure_hpa: f64, temperature_c: f64) -> f64 {
    let h = true_altitude_deg;
    let arcmin = if h <= PLAIN_SWITCH_ALTITUDE_DEG {
        1.02 / (h + 10.3 / (h + 5.11)).to_radians().tan()
    } else {
        let t = (90.0 - h).to_radians().tan();
        (58.276 * t - 0.0824 * t * t * t) / 60.0
    };
    arcmin / 60.0 * (pressure_hpa / 1010.0) * (283.0 / (273.0 + temperature_c))
}

/// Apparent altitude (degrees) for a true altitude with the plain model.
pub fn plain_true_to_apparent(
    true_altitude_deg: f64,
    pressure_hpa: f64,
    temperature_c: f64,
) -> f64 {
    if !true_altitude_deg.is_finite()
        || !valid_atmosphere(pressure_hpa, temperature_c)
        || true_altitude_deg <= -5.0
    {
        return true_altitude_deg;
    }
    let apparent =
        true_altitude_deg + plain_refraction_deg(true_altitude_deg, pressure_hpa, temperature_c);
    if apparent > 0.0 {
        apparent
    } else {
        true_altitude_deg
    }
}

/// True altitude (degrees) for an apparent altitude with the plain model, as the
/// numerical inverse of [`plain_true_to_apparent`]; unchanged below the horizon.
pub fn plain_apparent_to_true(
    apparent_altitude_deg: f64,
    pressure_hpa: f64,
    temperature_c: f64,
) -> f64 {
    if !apparent_altitude_deg.is_finite()
        || !valid_atmosphere(pressure_hpa, temperature_c)
        || apparent_altitude_deg <= 0.0
    {
        return apparent_altitude_deg;
    }
    let mut true_altitude = apparent_altitude_deg;
    for _ in 0..60 {
        let next = apparent_altitude_deg
            - plain_refraction_deg(true_altitude, pressure_hpa, temperature_c);
        let done = (next - true_altitude).abs() < 1e-13;
        true_altitude = next;
        if done {
            break;
        }
    }
    if true_altitude > 0.0 {
        true_altitude
    } else {
        apparent_altitude_deg
    }
}

// ------------------------------------------------------------- extended model

/// Refraction (degrees) of the extended model as a function of the apparent altitude.
fn extended_refraction_deg(
    apparent_altitude_deg: f64,
    pressure_hpa: f64,
    temperature_c: f64,
) -> f64 {
    let a = apparent_altitude_deg;
    let arcmin = if a > EXTENDED_SWITCH_ALTITUDE_DEG {
        0.97 / a.to_radians().tan()
    } else {
        (34.46 + 4.23 * a + 0.004 * a * a) / (1.0 + 0.505 * a + 0.0845 * a * a)
    };
    let scaled =
        (pressure_hpa - 80.0) / 930.0 / (1.0 + 8e-5 * (arcmin + 39.0) * (temperature_c - 10.0))
            * arcmin;
    scaled / 60.0
}

/// True altitude (degrees, negative) of a body whose apparent altitude is exactly
/// zero: minus the extended-model refraction at the horizon. This is the horizon
/// used for rising and setting; `0.0` for an unphysical atmosphere.
pub fn apparent_horizon_true_altitude_deg(pressure_hpa: f64, temperature_c: f64) -> f64 {
    if !valid_atmosphere(pressure_hpa, temperature_c) {
        return 0.0;
    }
    -extended_refraction_deg(0.0, pressure_hpa, temperature_c)
}

/// Horizon dip (degrees, ≤ 0) for an observer `height_m` above the surface.
pub fn horizon_dip_deg(
    height_m: f64,
    pressure_hpa: f64,
    temperature_c: f64,
    lapse_rate: f64,
) -> f64 {
    let krefr = (0.0342 + lapse_rate) / (0.154 * 0.0238);
    let t = 273.15 + temperature_c;
    let d = 1.0 - 1.848 * krefr * pressure_hpa / t / t;
    if !(d.is_finite() && d > 0.0 && height_m.is_finite()) || height_m < 0.0 {
        return 0.0;
    }
    -(1.0 / (1.0 + height_m / DIP_EARTH_RADIUS_M))
        .acos()
        .to_degrees()
        * d.sqrt()
}

/// Result of an extended-model conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtendedRefraction {
    pub true_altitude_deg: f64,
    pub apparent_altitude_deg: f64,
    pub dip_deg: f64,
}

/// True → apparent altitude with the extended model, for an observer at
/// `height_m`. Unchanged when the atmosphere is unphysical or the refracted
/// altitude would fall below the horizon dip.
pub fn extended_true_to_apparent(
    true_altitude_deg: f64,
    height_m: f64,
    pressure_hpa: f64,
    temperature_c: f64,
    lapse_rate: f64,
) -> ExtendedRefraction {
    let dip = horizon_dip_deg(height_m, pressure_hpa, temperature_c, lapse_rate);
    let unchanged = ExtendedRefraction {
        true_altitude_deg,
        apparent_altitude_deg: true_altitude_deg,
        dip_deg: dip,
    };
    if !true_altitude_deg.is_finite()
        || !valid_atmosphere(pressure_hpa, temperature_c)
        || true_altitude_deg < -10.0
    {
        return unchanged;
    }
    let h = true_altitude_deg.min(90.0);
    let mut apparent = h;
    for _ in 0..60 {
        let next = h + extended_refraction_deg(apparent, pressure_hpa, temperature_c);
        let done = (next - apparent).abs() < 1e-13;
        apparent = next;
        if done {
            break;
        }
    }
    if apparent >= dip {
        ExtendedRefraction {
            true_altitude_deg,
            apparent_altitude_deg: apparent,
            dip_deg: dip,
        }
    } else {
        unchanged
    }
}

/// Apparent → true altitude with the extended model.
pub fn extended_apparent_to_true(
    apparent_altitude_deg: f64,
    height_m: f64,
    pressure_hpa: f64,
    temperature_c: f64,
    lapse_rate: f64,
) -> ExtendedRefraction {
    let dip = horizon_dip_deg(height_m, pressure_hpa, temperature_c, lapse_rate);
    let unchanged = ExtendedRefraction {
        true_altitude_deg: apparent_altitude_deg,
        apparent_altitude_deg,
        dip_deg: dip,
    };
    if !apparent_altitude_deg.is_finite()
        || !valid_atmosphere(pressure_hpa, temperature_c)
        || apparent_altitude_deg <= dip
        || apparent_altitude_deg < -10.0
    {
        return unchanged;
    }
    let a = apparent_altitude_deg.min(90.0);
    ExtendedRefraction {
        true_altitude_deg: apparent_altitude_deg
            - extended_refraction_deg(a, pressure_hpa, temperature_c),
        apparent_altitude_deg,
        dip_deg: dip,
    }
}

// ---------------------------------------------------------- horizon coordinates

#[derive(Debug, PartialEq)]
pub enum HorizonError {
    NonFinite,
    InvalidSite,
    Orientation(OrientationError),
    HouseAngles(HouseAnglesError),
}

impl fmt::Display for HorizonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => write!(f, "non-finite input"),
            Self::InvalidSite => write!(
                f,
                "latitude must satisfy |lat| <= 90 and longitude be finite"
            ),
            Self::Orientation(e) => write!(f, "orientation: {e}"),
            Self::HouseAngles(e) => write!(f, "sidereal time: {e}"),
        }
    }
}
impl Error for HorizonError {}
impl From<OrientationError> for HorizonError {
    fn from(e: OrientationError) -> Self {
        Self::Orientation(e)
    }
}
impl From<HouseAnglesError> for HorizonError {
    fn from(e: HouseAnglesError) -> Self {
        Self::HouseAngles(e)
    }
}

/// True altitude and azimuth (from north through east) of a direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Horizontal {
    pub azimuth_north_east_deg: f64,
    pub true_altitude_deg: f64,
}

/// Altitude and azimuth of an equatorial-of-date direction for a local apparent
/// sidereal time `armc_deg` (Greenwich apparent sidereal time plus east longitude).
pub fn equatorial_to_horizontal(
    right_ascension_deg: f64,
    declination_deg: f64,
    armc_deg: f64,
    latitude_deg: f64,
) -> Result<Horizontal, HorizonError> {
    if ![right_ascension_deg, declination_deg, armc_deg, latitude_deg]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err(HorizonError::NonFinite);
    }
    if latitude_deg.abs() > 90.0 {
        return Err(HorizonError::InvalidSite);
    }
    let hour_angle = (armc_deg - right_ascension_deg).to_radians();
    let (dec, lat) = (declination_deg.to_radians(), latitude_deg.to_radians());
    let sin_alt = lat.sin() * dec.sin() + lat.cos() * dec.cos() * hour_angle.cos();
    let altitude = sin_alt.clamp(-1.0, 1.0).asin().to_degrees();
    // Components in the local frame: north, east, up.
    let north = dec.sin() * lat.cos() - dec.cos() * lat.sin() * hour_angle.cos();
    let east = -dec.cos() * hour_angle.sin();
    let mut azimuth = east.atan2(north).to_degrees().rem_euclid(360.0);
    if azimuth >= 360.0 {
        azimuth = 0.0;
    }
    Ok(Horizontal {
        azimuth_north_east_deg: azimuth,
        true_altitude_deg: altitude,
    })
}

/// Inverse of [`equatorial_to_horizontal`]: right ascension and declination of
/// the direction with the given azimuth (north through east) and true altitude.
pub fn horizontal_to_equatorial(
    azimuth_north_east_deg: f64,
    true_altitude_deg: f64,
    armc_deg: f64,
    latitude_deg: f64,
) -> Result<(f64, f64), HorizonError> {
    if ![
        azimuth_north_east_deg,
        true_altitude_deg,
        armc_deg,
        latitude_deg,
    ]
    .iter()
    .all(|v| v.is_finite())
    {
        return Err(HorizonError::NonFinite);
    }
    if latitude_deg.abs() > 90.0 {
        return Err(HorizonError::InvalidSite);
    }
    let (az, alt, lat) = (
        azimuth_north_east_deg.to_radians(),
        true_altitude_deg.to_radians(),
        latitude_deg.to_radians(),
    );
    let (n, e, u) = (alt.cos() * az.cos(), alt.cos() * az.sin(), alt.sin());
    // Rotate (north, east, up) to the equatorial axes (meridian, east, pole).
    let to_meridian = -lat.sin() * n + lat.cos() * u;
    let to_pole = lat.cos() * n + lat.sin() * u;
    // meridian axis points to the local meridian on the equator: hour angle 0
    let hour_angle = (-e).atan2(to_meridian);
    let declination = to_pole.clamp(-1.0, 1.0).asin().to_degrees();
    let ra = (armc_deg - hour_angle.to_degrees()).rem_euclid(360.0);
    Ok((if ra >= 360.0 { 0.0 } else { ra }, declination))
}

/// Ecliptic-of-date longitude/latitude to equatorial-of-date right ascension and
/// declination (degrees) with the true obliquity at `tt_jd`.
pub fn ecliptic_to_equatorial(
    tt_jd: f64,
    longitude_deg: f64,
    latitude_deg: f64,
) -> Result<(f64, f64), HorizonError> {
    if !longitude_deg.is_finite() || !latitude_deg.is_finite() {
        return Err(HorizonError::NonFinite);
    }
    let eps = ecliptic_nutation(tt_jd)?.true_obliquity_rad;
    let (lon, lat) = (longitude_deg.to_radians(), latitude_deg.to_radians());
    let (x, y, z) = (lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin());
    let ye = y * eps.cos() - z * eps.sin();
    let ze = y * eps.sin() + z * eps.cos();
    let ra = ye.atan2(x).to_degrees().rem_euclid(360.0);
    Ok((
        if ra >= 360.0 { 0.0 } else { ra },
        ze.atan2(x.hypot(ye)).to_degrees(),
    ))
}

/// Local apparent sidereal time in degrees (the ARMC): Greenwich apparent sidereal
/// time of `ut1_jd`/`tt_jd` plus the east longitude.
pub fn local_apparent_sidereal_deg(
    ut1_jd: f64,
    tt_jd: f64,
    longitude_deg_east: f64,
) -> Result<f64, HorizonError> {
    // The latitude only matters for the Ascendant, which is not used here.
    Ok(house_angles(ut1_jd, tt_jd, 0.0, longitude_deg_east)?.armc_deg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_estimate_is_standard_at_sea_level_and_falls_with_height() {
        assert_eq!(estimated_pressure_hpa(0.0), 1013.25);
        let p = estimated_pressure_hpa(2000.0);
        assert!((p - 794.88).abs() < 0.01, "{p}");
    }

    #[test]
    fn plain_model_values() {
        // Published check values of the formulas (computed independently).
        let a = plain_true_to_apparent(10.0, 1013.25, 15.0);
        assert!((a - 10.088_848_272).abs() < 1e-8, "{a}");
        let b = plain_true_to_apparent(45.0, 1010.0, 10.0);
        assert!((b - 45.0 - 58.276 / 3600.0 + 0.0824 / 3600.0).abs() < 1e-7);
        // Below the horizon after refraction: unchanged.
        assert_eq!(plain_true_to_apparent(-3.0, 1013.25, 15.0), -3.0);
        // No atmosphere: unchanged.
        assert_eq!(plain_true_to_apparent(10.0, 0.0, 15.0), 10.0);
    }

    #[test]
    fn extended_model_round_trips_and_switches_continuously() {
        for &(p, t) in &[(1013.25, 15.0), (1013.25, 0.0), (900.0, 25.0)] {
            for k in 0..=180 {
                let true_alt = -0.3 + 0.5 * f64::from(k);
                if true_alt > 90.0 {
                    break;
                }
                let up = extended_true_to_apparent(true_alt, 0.0, p, t, STANDARD_LAPSE_RATE);
                let back = extended_apparent_to_true(
                    up.apparent_altitude_deg,
                    0.0,
                    p,
                    t,
                    STANDARD_LAPSE_RATE,
                );
                assert!(
                    (back.true_altitude_deg - true_alt).abs() < 1e-9,
                    "{true_alt} {p} {t}: {back:?}"
                );
            }
        }
        // The formula is continuous at its switch point.
        let below = extended_refraction_deg(EXTENDED_SWITCH_ALTITUDE_DEG - 1e-9, 1013.25, 15.0);
        let above = extended_refraction_deg(EXTENDED_SWITCH_ALTITUDE_DEG + 1e-9, 1013.25, 15.0);
        assert!((below - above).abs() < 1e-9, "{below} {above}");
    }

    #[test]
    fn extended_model_refuses_to_lift_a_body_that_stays_below_the_horizon() {
        let down = extended_true_to_apparent(-0.6, 0.0, 1013.25, 15.0, STANDARD_LAPSE_RATE);
        assert_eq!(down.apparent_altitude_deg, -0.6);
        let up = extended_true_to_apparent(-0.5, 0.0, 1013.25, 15.0, STANDARD_LAPSE_RATE);
        assert!(up.apparent_altitude_deg > 0.0);
        // A raised observer sees below the astronomical horizon: the dip lets the lift happen.
        let high = extended_true_to_apparent(-0.6, 2000.0, 1013.25, 15.0, STANDARD_LAPSE_RATE);
        assert!(
            high.apparent_altitude_deg > -0.6 && high.dip_deg < -1.2,
            "{high:?}"
        );
    }

    #[test]
    fn the_refracted_horizon_is_about_thirty_four_arcminutes_down() {
        let h = apparent_horizon_true_altitude_deg(1013.25, 15.0);
        assert!((h + 0.5599).abs() < 1e-4, "{h}");
        // Colder, denser air bends more; no atmosphere bends nothing.
        assert!(apparent_horizon_true_altitude_deg(1013.25, -10.0) < h);
        assert_eq!(apparent_horizon_true_altitude_deg(0.0, 15.0), 0.0);
    }

    #[test]
    fn dip_grows_with_the_square_root_of_height() {
        let d1 = horizon_dip_deg(100.0, 1013.25, 15.0, STANDARD_LAPSE_RATE);
        let d4 = horizon_dip_deg(400.0, 1013.25, 15.0, STANDARD_LAPSE_RATE);
        assert!((d4 / d1 - 2.0).abs() < 1e-3);
        assert_eq!(
            horizon_dip_deg(0.0, 1013.25, 15.0, STANDARD_LAPSE_RATE),
            0.0
        );
    }

    #[test]
    fn meridian_passage_geometry() {
        // A body on the meridian south of the zenith: altitude 90 - (lat - dec), azimuth 180.
        let h = equatorial_to_horizontal(30.0, 10.0, 30.0, 45.0).unwrap();
        assert!((h.true_altitude_deg - 55.0).abs() < 1e-12);
        assert!((h.azimuth_north_east_deg - 180.0).abs() < 1e-9);
        // North of the zenith (dec above latitude): azimuth 0.
        let n = equatorial_to_horizontal(30.0, 60.0, 30.0, 45.0).unwrap();
        assert!((n.true_altitude_deg - 75.0).abs() < 1e-12);
        assert!(n.azimuth_north_east_deg < 1e-9 || n.azimuth_north_east_deg > 360.0 - 1e-9);
        // Body 90 degrees west of the meridian on the equator, observer on the equator: setting due west.
        let w = equatorial_to_horizontal(0.0, 0.0, 90.0, 0.0).unwrap();
        assert!(w.true_altitude_deg.abs() < 1e-12);
        assert!((w.azimuth_north_east_deg - 270.0).abs() < 1e-9);
    }

    #[test]
    fn horizontal_round_trip() {
        for &(ra, dec, armc, lat) in &[
            (10.0, 20.0, 100.0, 41.9),
            (250.0, -30.0, 12.0, -33.0),
            (359.0, 5.0, 359.0, 60.0),
        ] {
            let h = equatorial_to_horizontal(ra, dec, armc, lat).unwrap();
            let (ra2, dec2) =
                horizontal_to_equatorial(h.azimuth_north_east_deg, h.true_altitude_deg, armc, lat)
                    .unwrap();
            let dra = ((ra2 - ra + 540.0).rem_euclid(360.0) - 180.0).abs();
            assert!(
                dra < 1e-9 && (dec2 - dec).abs() < 1e-9,
                "{ra} {dec}: {ra2} {dec2}"
            );
        }
    }

    #[test]
    fn ecliptic_equator_conversion_uses_the_true_obliquity() {
        // The vernal point and the solstice point.
        let (ra, dec) = ecliptic_to_equatorial(2_451_545.0, 0.0, 0.0).unwrap();
        assert!(ra.abs() < 1e-9 || (360.0 - ra).abs() < 1e-9);
        assert!(dec.abs() < 1e-9);
        let (_, dec) = ecliptic_to_equatorial(2_451_545.0, 90.0, 0.0).unwrap();
        assert!((dec - 23.437).abs() < 0.01, "{dec}");
    }

    #[test]
    fn refusals() {
        assert_eq!(
            equatorial_to_horizontal(f64::NAN, 0.0, 0.0, 0.0),
            Err(HorizonError::NonFinite)
        );
        assert_eq!(
            equatorial_to_horizontal(0.0, 0.0, 0.0, 91.0),
            Err(HorizonError::InvalidSite)
        );
    }
}
