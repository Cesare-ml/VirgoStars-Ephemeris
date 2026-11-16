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

//! Lunar eclipses: when they happen, their kind and their contacts.
//!
//! Like [`crate::rise_transit`] this module has no ephemeris: the caller supplies a
//! closure returning the **apparent geocentric** positions of the Sun and the Moon
//! (kilometres, any fixed equatorial or ecliptic frame) for a UT Julian date.
//!
//! # Geometry
//!
//! The Earth's shadow is the pair of cones tangent to the Sun and to the Earth
//! (taken as a sphere of the equatorial radius, enlarged below). With `D` the Sun distance,
//! `sin f_u = (R☉ − R⊕)/D` and `sin f_p = (R☉ + R⊕)/D`, the cone radius in the plane
//! through the Moon perpendicular to the axis (the line from the Sun through the
//! Earth's centre), at distance `z` behind the Earth, is
//! `R⊕/cos f_u − z tan f_u` for the umbra and `R⊕/cos f_p + z tan f_p` for the
//! penumbra. With `ρ` the distance of the Moon's centre from the axis and `R☾` the
//! Moon's radius:
//!
//! * penumbral eclipse: `ρ < r_p + R☾`; partial: `ρ < r_u + R☾`; total: `ρ < r_u − R☾`;
//! * contacts are the instants `ρ = r ± R☾`; the maximum is the minimum of `ρ`.
//!
//! The Earth's radius is enlarged by 1 % for the atmosphere (`R⊕ / 0.99`, the Danjon-style
//! convention of the reference; measured: it removes a constant 64.6 km bias in the shadow
//! radii of 83 eclipses, leaving 1 km scatter). Radii: Sun 696,000 km, Earth 6,378.14 km
//! (equatorial), Moon 1,738.15 km.

use std::error::Error;
use std::fmt;

pub const SUN_RADIUS_KM: f64 = 696_000.0;
pub const EARTH_RADIUS_KM: f64 = 6_378.14;
pub const MOON_RADIUS_KM: f64 = 1_738.15;
/// The Earth radius used for its shadow: the equatorial radius enlarged by 1 % for the atmosphere.
pub const SHADOW_EARTH_RADIUS_KM: f64 = EARTH_RADIUS_KM / 0.99;
/// Mean synodic month, days.
pub const SYNODIC_MONTH_DAYS: f64 = 29.530_588_9;

/// Apparent geocentric positions of the Sun and the Moon in a common frame, km.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SunMoon {
    pub sun_km: [f64; 3],
    pub moon_km: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LunarKind {
    Penumbral,
    Partial,
    Total,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LunarEclipse {
    pub kind: LunarKind,
    /// UT of the greatest eclipse (the Moon's centre closest to the shadow axis).
    pub maximum_ut: f64,
    pub penumbral_begin_ut: f64,
    pub penumbral_end_ut: f64,
    pub partial_begin_ut: Option<f64>,
    pub partial_end_ut: Option<f64>,
    pub total_begin_ut: Option<f64>,
    pub total_end_ut: Option<f64>,
    /// Fraction of the Moon's diameter inside the umbra at maximum (0 if none; above 1 when total).
    pub umbral_magnitude: f64,
    pub penumbral_magnitude: f64,
}

#[derive(Debug, PartialEq)]
pub enum EclipseError<E> {
    Position(E),
    NonFinite,
    /// The search ran past its limit without finding an eclipse.
    NotFound,
}

impl<E: fmt::Display> fmt::Display for EclipseError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Position(e) => write!(f, "position: {e}"),
            Self::NonFinite => write!(f, "non-finite input"),
            Self::NotFound => write!(f, "no eclipse within the search limit"),
        }
    }
}
impl<E: fmt::Debug + fmt::Display> Error for EclipseError<E> {}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// The Moon relative to the Earth's shadow at one instant.
#[derive(Clone, Copy, Debug)]
struct ShadowState {
    /// Distance of the Moon's centre from the axis, km.
    rho: f64,
    umbra_radius: f64,
    penumbra_radius: f64,
}

fn shadow_state(p: &SunMoon) -> ShadowState {
    let d = norm(p.sun_km);
    let axis = scale(p.sun_km, -1.0 / d);
    let z = dot(p.moon_km, axis);
    let off = sub(p.moon_km, scale(axis, z));
    let re = SHADOW_EARTH_RADIUS_KM;
    let sin_u = (SUN_RADIUS_KM - re) / d;
    let sin_p = (SUN_RADIUS_KM + re) / d;
    let (cos_u, cos_p) = ((1.0 - sin_u * sin_u).sqrt(), (1.0 - sin_p * sin_p).sqrt());
    ShadowState {
        rho: norm(off),
        umbra_radius: re / cos_u - z * sin_u / cos_u,
        penumbra_radius: re / cos_p + z * sin_p / cos_p,
    }
}

fn wrap_pm180(x: f64) -> f64 {
    (x + 180.0).rem_euclid(360.0) - 180.0
}

/// Right-ascension-like longitude of a vector about the frame's z axis, degrees.
fn azimuth(v: [f64; 3]) -> f64 {
    v[1].atan2(v[0]).to_degrees()
}

fn eval<E, F>(positions: &mut F, t: f64) -> Result<SunMoon, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    positions(t).map_err(EclipseError::Position)
}

/// Opposition offset in degrees: the Moon's longitude about the frame's z axis minus
/// the Sun's minus 180°, wrapped; it grows through zero at each full moon.
fn opposition_offset<E, F>(positions: &mut F, t: f64) -> Result<f64, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    let p = eval(positions, t)?;
    Ok(wrap_pm180(azimuth(p.moon_km) - azimuth(p.sun_km) - 180.0))
}

/// Newton iteration (mean elongation rate 12.19°/day) to the opposition nearest `t`.
fn solve_opposition<E, F>(positions: &mut F, mut t: f64) -> Result<f64, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    for _ in 0..12 {
        let f = opposition_offset(positions, t)?;
        t -= f / 12.19;
        if f.abs() < 1e-7 {
            break;
        }
    }
    Ok(t)
}

/// Golden-section minimum of `g` on `[a, b]`.
pub(crate) fn minimize<E>(
    g: &mut impl FnMut(f64) -> Result<f64, EclipseError<E>>,
    mut a: f64,
    mut b: f64,
) -> Result<(f64, f64), EclipseError<E>> {
    const INV_PHI: f64 = 0.618_033_988_749_894_9;
    let mut c = b - INV_PHI * (b - a);
    let mut d = a + INV_PHI * (b - a);
    let (mut gc, mut gd) = (g(c)?, g(d)?);
    for _ in 0..80 {
        if (b - a).abs() < 1e-9 {
            break;
        }
        if gc < gd {
            b = d;
            d = c;
            gd = gc;
            c = b - INV_PHI * (b - a);
            gc = g(c)?;
        } else {
            a = c;
            c = d;
            gc = gd;
            d = a + INV_PHI * (b - a);
            gd = g(d)?;
        }
    }
    Ok(if gc < gd { (c, gc) } else { (d, gd) })
}

/// Root of `h` between `outside` (where `h > 0`) and `inside` (where `h <= 0`), by bisection.
pub(crate) fn bisect<E>(
    h: &mut impl FnMut(f64) -> Result<f64, EclipseError<E>>,
    mut outside: f64,
    mut inside: f64,
) -> Result<f64, EclipseError<E>> {
    for _ in 0..60 {
        if (inside - outside).abs() < 1e-9 {
            break;
        }
        let mid = 0.5 * (outside + inside);
        if h(mid)? > 0.0 {
            outside = mid;
        } else {
            inside = mid;
        }
    }
    Ok(0.5 * (outside + inside))
}

/// Find the first lunar eclipse whose maximum is after `start_ut_jd`.
///
/// `search_limit_days` bounds the search (an eclipse occurs at least every six
/// lunations, so 400 days is generous). Penumbral eclipses are included.
pub fn next_lunar_eclipse<E, F>(
    mut positions: F,
    start_ut_jd: f64,
    search_limit_days: f64,
    backward: bool,
) -> Result<LunarEclipse, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    if !start_ut_jd.is_finite() || !search_limit_days.is_finite() || search_limit_days <= 0.0 {
        return Err(EclipseError::NonFinite);
    }
    let sign = if backward { -1.0 } else { 1.0 };
    let limit = start_ut_jd + sign * search_limit_days;
    let mut opposition = solve_opposition(&mut positions, start_ut_jd)?;
    // Begin about a day on the near side of the start so that the first candidate may straddle it.
    while (opposition - start_ut_jd) * sign < -1.0 {
        let next = solve_opposition(&mut positions, opposition + sign * SYNODIC_MONTH_DAYS)?;
        if (next - opposition) * sign < 1.0 {
            return Err(EclipseError::NotFound);
        }
        opposition = next;
    }
    while (opposition - limit) * sign < 0.0 {
        if let Some(eclipse) = lunar_candidate(&mut positions, opposition, start_ut_jd, backward)? {
            return Ok(eclipse);
        }
        let next = solve_opposition(&mut positions, opposition + sign * SYNODIC_MONTH_DAYS)?;
        // A source that has no further syzygy (a synthetic one) must not loop forever.
        if (next - opposition) * sign < 1.0 {
            break;
        }
        opposition = next;
    }
    Err(EclipseError::NotFound)
}

fn lunar_candidate<E, F>(
    positions: &mut F,
    opposition: f64,
    start_ut_jd: f64,
    backward: bool,
) -> Result<Option<LunarEclipse>, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    let mut rho =
        |t: f64| -> Result<f64, EclipseError<E>> { Ok(shadow_state(&eval(positions, t)?).rho) };
    let (t_max, _) = minimize(&mut rho, opposition - 1.0, opposition + 1.0)?;
    if (backward && t_max >= start_ut_jd) || (!backward && t_max <= start_ut_jd) {
        return Ok(None);
    }
    let at_max = shadow_state(&eval(positions, t_max)?);
    let (rm, ru, rp) = (MOON_RADIUS_KM, at_max.umbra_radius, at_max.penumbra_radius);
    if at_max.rho >= rp + rm {
        return Ok(None);
    }
    let kind = if at_max.rho < ru - rm {
        LunarKind::Total
    } else if at_max.rho < ru + rm {
        LunarKind::Partial
    } else {
        LunarKind::Penumbral
    };
    // Contact functions: positive outside the contact, negative inside.
    let mut contact = |t: f64, which: u8| -> Result<f64, EclipseError<E>> {
        let s = shadow_state(&eval(positions, t)?);
        Ok(match which {
            0 => s.rho - (s.penumbra_radius + MOON_RADIUS_KM),
            1 => s.rho - (s.umbra_radius + MOON_RADIUS_KM),
            _ => s.rho - (s.umbra_radius - MOON_RADIUS_KM),
        })
    };
    let mut pair = |which: u8| -> Result<(f64, f64), EclipseError<E>> {
        let mut h = |t: f64| contact(t, which);
        let begin = bisect(&mut h, t_max - 0.5, t_max)?;
        let end = bisect(&mut h, t_max + 0.5, t_max)?;
        Ok((begin, end))
    };
    let (pen_begin, pen_end) = pair(0)?;
    let partial = if kind != LunarKind::Penumbral {
        Some(pair(1)?)
    } else {
        None
    };
    let total = if kind == LunarKind::Total {
        Some(pair(2)?)
    } else {
        None
    };
    Ok(Some(LunarEclipse {
        kind,
        maximum_ut: t_max,
        penumbral_begin_ut: pen_begin,
        penumbral_end_ut: pen_end,
        partial_begin_ut: partial.map(|p| p.0),
        partial_end_ut: partial.map(|p| p.1),
        total_begin_ut: total.map(|p| p.0),
        total_end_ut: total.map(|p| p.1),
        umbral_magnitude: ((ru + rm - at_max.rho) / (2.0 * rm)).max(0.0),
        penumbral_magnitude: (rp + rm - at_max.rho) / (2.0 * rm),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    const AU_KM: f64 = 149_597_870.7;
    const T0: f64 = 2_451_545.0;
    const KM_PER_DAY: f64 = 0.95 * 86_400.0;

    /// The Sun fixed on +x at 1 AU, the Moon opposite at 384,400 km moving along y at
    /// 0.95 km/s (eastward as seen from the Earth, so the elongation grows) with an offset
    /// `impact` from the shadow axis along z.
    fn synthetic(impact_km: f64) -> impl FnMut(f64) -> Result<SunMoon, Infallible> {
        move |t| {
            Ok(SunMoon {
                sun_km: [AU_KM, 0.0, 0.0],
                moon_km: [-384_400.0, -(t - T0) * KM_PER_DAY, impact_km],
            })
        }
    }

    #[test]
    fn classifies_by_impact_and_orders_the_contacts() {
        for &(impact, kind) in &[
            (0.0, LunarKind::Total),
            (1_500.0, LunarKind::Total),
            (4_500.0, LunarKind::Partial),
            (7_500.0, LunarKind::Penumbral),
        ] {
            let e = next_lunar_eclipse(synthetic(impact), T0 - 2.0, 60.0, false).unwrap();
            assert_eq!(e.kind, kind, "impact {impact}");
            assert!(
                (e.maximum_ut - T0).abs() < 1e-6,
                "maximum at the closest approach"
            );
            assert!(e.penumbral_begin_ut < e.maximum_ut && e.maximum_ut < e.penumbral_end_ut);
            match kind {
                LunarKind::Total => {
                    let (a, b, c, d) = (
                        e.partial_begin_ut.unwrap(),
                        e.total_begin_ut.unwrap(),
                        e.total_end_ut.unwrap(),
                        e.partial_end_ut.unwrap(),
                    );
                    assert!(e.penumbral_begin_ut < a && a < b && b < e.maximum_ut);
                    assert!(e.maximum_ut < c && c < d && d < e.penumbral_end_ut);
                    assert!(e.umbral_magnitude > 1.0);
                }
                LunarKind::Partial => {
                    assert!(e.total_begin_ut.is_none() && e.partial_begin_ut.is_some());
                    assert!(e.umbral_magnitude > 0.0 && e.umbral_magnitude < 1.0);
                }
                LunarKind::Penumbral => {
                    assert!(e.partial_begin_ut.is_none());
                    assert_eq!(e.umbral_magnitude, 0.0);
                    assert!(e.penumbral_magnitude > 0.0 && e.penumbral_magnitude < 1.0);
                }
            }
        }
    }

    #[test]
    fn the_shadow_is_symmetric_about_the_maximum() {
        let e = next_lunar_eclipse(synthetic(2_000.0), T0 - 2.0, 60.0, false).unwrap();
        let before = e.maximum_ut - e.penumbral_begin_ut;
        let after = e.penumbral_end_ut - e.maximum_ut;
        assert!((before - after).abs() < 1e-6, "{before} {after}");
        // Penumbral half-width about (r_p + R_moon) / v: roughly 9,500 km / 0.95 km/s = 0.115 day.
        assert!((0.10..0.13).contains(&before), "{before}");
    }

    #[test]
    fn no_eclipse_within_the_limit_is_an_error_and_bad_input_is_refused() {
        let miss = next_lunar_eclipse(synthetic(15_000.0), T0 - 2.0, 40.0, false);
        assert_eq!(miss, Err(EclipseError::NotFound));
        assert_eq!(
            next_lunar_eclipse(synthetic(0.0), f64::NAN, 40.0, false),
            Err(EclipseError::NonFinite)
        );
        assert_eq!(
            next_lunar_eclipse(synthetic(0.0), T0, -1.0, false),
            Err(EclipseError::NonFinite)
        );
    }

    #[test]
    fn the_search_is_strictly_after_or_before_the_start() {
        // The maximum is exactly T0: from just before it the search finds it, from just after it does not.
        assert!(next_lunar_eclipse(synthetic(0.0), T0 - 0.01, 3.0, false).is_ok());
        assert_eq!(
            next_lunar_eclipse(synthetic(0.0), T0 + 0.01, 3.0, false),
            Err(EclipseError::NotFound)
        );
        assert!(next_lunar_eclipse(synthetic(0.0), T0 + 0.01, 3.0, true).is_ok());
    }
}
