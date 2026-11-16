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

//! Solar eclipses on the whole Earth: when they happen, their kind and the instants at
//! which the shadow first and last touches the globe.
//!
//! No ephemeris here either: the caller supplies the apparent geocentric positions of
//! the Sun and the Moon (km, true equator of date) through a closure.
//!
//! # Geometry
//!
//! The shadow axis is the line from the Sun's centre through the Moon's centre; `u` is
//! the unit vector from the Moon to the Sun and `z = P·u` the coordinate of a point `P`
//! (geocentric) along it. With `D` the Sun–Moon distance, `R☉`, `R☾` the radii:
//!
//! * penumbra: apex on the Sun's side of the Moon, half-angle `sin f₁ = (R☉ + R☾)/D`,
//!   radius at `z`: `(z_p − z) tan f₁`;
//! * umbra: apex beyond the Moon, half-angle `sin f₂ = (R☉ − R☾)/D`, radius at `z`:
//!   `|z − z_u| tan f₂` (beyond the apex it is the *anti*umbra of an annular eclipse).
//!
//! The Earth is the WGS84 ellipsoid (the polar axis is the z axis of the equatorial
//! frame). A point `P` of the surface lies in the penumbra when `ρ(P) < r_p(z(P))`, with
//! `ρ` its distance from the axis. Contacts are zeros of
//! `min_P [ρ(P) − r(z(P))]`; the minimum over the ellipsoid is found by the fixed-point
//! iteration of the tangency condition (the gradient of `ρ + z tan f` is antiparallel to
//! the outward normal), which converges in a handful of steps.
//!
//! * `partial_begin/end`: the penumbra first/last touches the globe;
//! * `total_begin/end`: the umbra (or antumbra) first/last touches it;
//! * `center_begin/end`: the axis itself first/last meets the globe;
//! * `maximum`: the instant the axis is closest to the geocentre (the convention of the
//!   published canons).
//!
//! Local circumstances (what a given place sees) are not part of this module.

use crate::eclipse::{
    bisect, minimize, EclipseError, SunMoon, MOON_RADIUS_KM, SUN_RADIUS_KM, SYNODIC_MONTH_DAYS,
};
use crate::wgs84_ecef::{WGS84_EQUATORIAL_RADIUS_M, WGS84_INVERSE_FLATTENING};

const EQUATORIAL_KM: f64 = WGS84_EQUATORIAL_RADIUS_M / 1000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolarKind {
    Partial,
    Annular,
    Total,
    /// Annular at one end of the central line and total at the other.
    Hybrid,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolarEclipse {
    pub kind: SolarKind,
    /// The axis meets the globe (central), as opposed to only the penumbra (partial) or
    /// only the edge of the umbra (non-central total/annular).
    pub central: bool,
    pub maximum_ut: f64,
    pub partial_begin_ut: f64,
    pub partial_end_ut: f64,
    pub total_begin_ut: Option<f64>,
    pub total_end_ut: Option<f64>,
    pub center_begin_ut: Option<f64>,
    pub center_end_ut: Option<f64>,
    /// Closest approach of the axis to the geocentre in equatorial Earth radii.
    pub gamma: f64,
    /// The instant, near the maximum, at which the point of the globe under the axis is on
    /// the meridian of the Sun (local apparent noon there); `None` if there is no such instant.
    pub noon_ut: Option<f64>,
}

type V3 = [f64; 3];
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

/// The shadow at one instant.
#[derive(Clone, Copy, Debug)]
struct Shadow {
    /// Unit vector from the Moon to the Sun.
    u: V3,
    /// Vector from the geocentre to the nearest point of the axis (⟂ u).
    w: V3,
    /// Coordinate along `u` of the penumbra apex and of the umbra apex.
    z_penumbra_apex: f64,
    z_umbra_apex: f64,
    tan_f1: f64,
    tan_f2: f64,
}

fn shadow(p: &SunMoon) -> Shadow {
    let sm = sub(p.sun_km, p.moon_km);
    let d = norm(sm);
    let u = scale(sm, 1.0 / d);
    let zm = dot(p.moon_km, u);
    let w = sub(p.moon_km, scale(u, zm));
    let (rs, rm) = (SUN_RADIUS_KM, MOON_RADIUS_KM);
    let tan_of = |sin: f64| sin / (1.0 - sin * sin).sqrt();
    Shadow {
        u,
        w,
        z_penumbra_apex: zm + d * rm / (rs + rm),
        z_umbra_apex: zm - d * rm / (rs - rm),
        tan_f1: tan_of((rs + rm) / d),
        tan_f2: tan_of((rs - rm) / d),
    }
}

fn semi_minor_km() -> f64 {
    EQUATORIAL_KM * (1.0 - 1.0 / WGS84_INVERSE_FLATTENING)
}

/// Point of the ellipsoid whose outward normal is parallel to `n`.
fn support_point(n: V3) -> V3 {
    let (a, b) = (EQUATORIAL_KM, semi_minor_km());
    let q = (a * a * (n[0] * n[0] + n[1] * n[1]) + b * b * n[2] * n[2]).sqrt();
    [a * a * n[0] / q, a * a * n[1] / q, b * b * n[2] / q]
}

/// `min over the ellipsoid of ρ(P) + k z(P)`, by fixed-point iteration of tangency.
fn min_over_globe(s: &Shadow, k: f64) -> f64 {
    let f = |p: V3| {
        let z = dot(p, s.u);
        let perp = sub(sub(p, scale(s.u, z)), s.w);
        (norm(perp), z)
    };
    // Start at the point of the globe facing the axis.
    let mut p = support_point(if norm(s.w) > 1e-9 { s.w } else { s.u });
    let mut best = f64::INFINITY;
    for _ in 0..80 {
        let (rho, z) = f(p);
        let value = rho + k * z;
        best = best.min(value);
        let perp = sub(sub(p, scale(s.u, z)), s.w);
        let dir = if rho > 1e-9 {
            scale(perp, 1.0 / rho)
        } else {
            [0.0; 3]
        };
        // gradient of ρ + k z is dir + k u; the minimiser has the outward normal -(that).
        let want = scale(add(dir, scale(s.u, k)), -1.0);
        if norm(want) < 1e-12 {
            break;
        }
        let next = support_point(want);
        let moved = norm(sub(next, p));
        p = next;
        if moved < 1e-9 {
            let (rho, z) = f(p);
            best = best.min(rho + k * z);
            break;
        }
    }
    best
}

/// The two coordinates `z` at which the axis meets the globe, if it does.
fn axis_entries(s: &Shadow) -> Option<(f64, f64)> {
    let (a, b) = (EQUATORIAL_KM, semi_minor_km());
    let q = |v: V3, c: V3| (v[0] * c[0] + v[1] * c[1]) / (a * a) + v[2] * c[2] / (b * b);
    let (qa, qb, qc) = (q(s.u, s.u), q(s.w, s.u), q(s.w, s.w) - 1.0);
    let disc = qb * qb - qa * qc;
    if disc < 0.0 {
        return None;
    }
    let root = disc.sqrt();
    Some(((-qb - root) / qa, (-qb + root) / qa))
}

/// Margin of the penumbra over the globe: negative when it touches it.
fn penumbra_margin(s: &Shadow) -> f64 {
    // An axis through the globe has ρ = 0 at its two surface points, which the tangency
    // iteration (made for an axis outside the silhouette) cannot find.
    if let Some((z_far, _)) = axis_entries(s) {
        return s.tan_f1 * (z_far - s.z_penumbra_apex);
    }
    min_over_globe(s, s.tan_f1) - s.z_penumbra_apex * s.tan_f1
}

/// Margin of the umbra or antumbra over the globe.
fn umbra_margin(s: &Shadow) -> f64 {
    if let Some((z1, z2)) = axis_entries(s) {
        let reach = (z1 - s.z_umbra_apex).abs().max((z2 - s.z_umbra_apex).abs());
        return -reach * s.tan_f2;
    }
    // Points nearer the Moon than the apex see the umbra, farther ones the antumbra.
    let near = min_over_globe(s, -s.tan_f2) + s.z_umbra_apex * s.tan_f2;
    let far = min_over_globe(s, s.tan_f2) - s.z_umbra_apex * s.tan_f2;
    near.min(far)
}

/// Positive when the axis misses the globe, negative when it pierces it (km²-scaled).
fn axis_margin(s: &Shadow) -> f64 {
    let (a, b) = (EQUATORIAL_KM, semi_minor_km());
    // Points of the axis: w + s u. Quadratic in s from x²/a² + y²/a² + z²/b² = 1.
    let q = |v: V3, c: V3| (v[0] * c[0] + v[1] * c[1]) / (a * a) + v[2] * c[2] / (b * b);
    let qa = q(s.u, s.u);
    let qb = q(s.w, s.u);
    let qc = q(s.w, s.w) - 1.0;
    // discriminant of qa s² + 2 qb s + qc, scaled to a length²
    -(qb * qb - qa * qc) / qa
}

/// Minimum of `f` over `[center - half, center + half]`: a coarse scan (the contact
/// functions are not unimodal near the kinks) refined by golden section around the best sample.
fn scan_minimum<E>(
    f: &mut impl FnMut(f64) -> Result<f64, EclipseError<E>>,
    center: f64,
    half: f64,
) -> Result<(f64, f64), EclipseError<E>> {
    const STEPS: usize = 48;
    let step = 2.0 * half / STEPS as f64;
    let mut best = (center - half, f(center - half)?);
    let mut best_index = 0usize;
    for i in 1..=STEPS {
        let t = center - half + step * i as f64;
        let v = f(t)?;
        if v < best.1 {
            best = (t, v);
            best_index = i;
        }
    }
    let lo = center - half + step * best_index.saturating_sub(1) as f64;
    let hi = (center - half + step * (best_index + 1) as f64).min(center + half);
    let refined = minimize(f, lo, hi)?;
    Ok(if refined.1 < best.1 { refined } else { best })
}

fn eval<E, F>(positions: &mut F, t: f64) -> Result<SunMoon, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    positions(t).map_err(EclipseError::Position)
}

fn conjunction_offset<E, F>(positions: &mut F, t: f64) -> Result<f64, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    let p = eval(positions, t)?;
    let az = |v: V3| v[1].atan2(v[0]).to_degrees();
    Ok((az(p.moon_km) - az(p.sun_km) + 180.0).rem_euclid(360.0) - 180.0)
}

fn solve_conjunction<E, F>(positions: &mut F, mut t: f64) -> Result<f64, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    for _ in 0..12 {
        let f = conjunction_offset(positions, t)?;
        t -= f / 12.19;
        if f.abs() < 1e-7 {
            break;
        }
    }
    Ok(t)
}

/// Find the first solar eclipse whose greatest eclipse is after `start_ut_jd`.
pub fn next_solar_eclipse<E, F>(
    mut positions: F,
    start_ut_jd: f64,
    search_limit_days: f64,
    backward: bool,
) -> Result<SolarEclipse, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    if !start_ut_jd.is_finite() || !search_limit_days.is_finite() || search_limit_days <= 0.0 {
        return Err(EclipseError::NonFinite);
    }
    let sign = if backward { -1.0 } else { 1.0 };
    let limit = start_ut_jd + sign * search_limit_days;
    let mut conjunction = solve_conjunction(&mut positions, start_ut_jd)?;
    // Begin about a day on the near side of the start so that the first candidate may straddle it.
    while (conjunction - start_ut_jd) * sign < -1.0 {
        let next = solve_conjunction(&mut positions, conjunction + sign * SYNODIC_MONTH_DAYS)?;
        if (next - conjunction) * sign < 1.0 {
            return Err(EclipseError::NotFound);
        }
        conjunction = next;
    }
    while (conjunction - limit) * sign < 0.0 {
        if let Some(eclipse) = solar_candidate(&mut positions, conjunction, start_ut_jd, backward)?
        {
            return Ok(eclipse);
        }
        let next = solve_conjunction(&mut positions, conjunction + sign * SYNODIC_MONTH_DAYS)?;
        // A source that has no further syzygy (a synthetic one) must not loop forever.
        if (next - conjunction) * sign < 1.0 {
            break;
        }
        conjunction = next;
    }
    Err(EclipseError::NotFound)
}

fn solar_candidate<E, F>(
    positions: &mut F,
    conjunction: f64,
    start_ut_jd: f64,
    backward: bool,
) -> Result<Option<SolarEclipse>, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    let mut gamma_km =
        |t: f64| -> Result<f64, EclipseError<E>> { Ok(norm(shadow(&eval(positions, t)?).w)) };
    let (t_max, gamma) = minimize(&mut gamma_km, conjunction - 1.0, conjunction + 1.0)?;
    if (backward && t_max >= start_ut_jd) || (!backward && t_max <= start_ut_jd) {
        return Ok(None);
    }
    // Cheap exclusion: the penumbra radius on the fundamental plane is below about 4,000 km.
    if gamma > EQUATORIAL_KM + 4_500.0 {
        return Ok(None);
    }
    let mut pen = |t: f64| -> Result<f64, EclipseError<E>> {
        Ok(penumbra_margin(&shadow(&eval(positions, t)?)))
    };
    let (t_pen_min, pen_min) = scan_minimum(&mut pen, t_max, 0.25)?;
    if pen_min >= 0.0 {
        return Ok(None);
    }
    let partial_begin = bisect(&mut pen, t_pen_min - 0.4, t_pen_min)?;
    let partial_end = bisect(&mut pen, t_pen_min + 0.4, t_pen_min)?;

    let mut umb = |t: f64| -> Result<f64, EclipseError<E>> {
        Ok(umbra_margin(&shadow(&eval(positions, t)?)))
    };
    let (t_umb_min, umb_min) = scan_minimum(&mut umb, t_max, 0.25)?;
    let (mut total_begin, mut total_end) = (None, None);
    if umb_min < 0.0 {
        total_begin = Some(bisect(&mut umb, t_umb_min - 0.3, t_umb_min)?);
        total_end = Some(bisect(&mut umb, t_umb_min + 0.3, t_umb_min)?);
    }
    let mut axis =
        |t: f64| -> Result<f64, EclipseError<E>> { Ok(axis_margin(&shadow(&eval(positions, t)?))) };
    let (t_axis_min, axis_min) = scan_minimum(&mut axis, t_max, 0.25)?;
    let (mut center_begin, mut center_end) = (None, None);
    if axis_min < 0.0 {
        center_begin = Some(bisect(&mut axis, t_axis_min - 0.3, t_axis_min)?);
        center_end = Some(bisect(&mut axis, t_axis_min + 0.3, t_axis_min)?);
    }
    let central = center_begin.is_some();
    // Total (the umbra reaches the surface) or annular (the apex is short of it): look at
    // the surface point where the axis enters and leaves, or else at the umbral minimum.
    let kind = if total_begin.is_none() {
        SolarKind::Partial
    } else {
        let reaches = |positions: &mut F, t: f64| -> Result<bool, EclipseError<E>> {
            let s = shadow(&eval(positions, t)?);
            // z of the axis' entry point on the globe: the larger root of the line-ellipsoid quadratic
            let (a, b) = (EQUATORIAL_KM, semi_minor_km());
            let q = |v: V3, c: V3| (v[0] * c[0] + v[1] * c[1]) / (a * a) + v[2] * c[2] / (b * b);
            let (qa, qb, qc) = (q(s.u, s.u), q(s.w, s.u), q(s.w, s.w) - 1.0);
            let disc = (qb * qb - qa * qc).max(0.0).sqrt();
            let z_entry = (-qb + disc) / qa; // on the Sun side: the umbra meets it first
            Ok(z_entry >= s.z_umbra_apex)
        };
        match (center_begin, center_end) {
            (Some(b), Some(e)) => {
                let ends = (reaches(positions, b + 2e-6)?, reaches(positions, e - 2e-6)?);
                let middle = reaches(positions, t_max)?;
                match (ends.0, middle, ends.1) {
                    (true, true, true) => SolarKind::Total,
                    (false, false, false) => SolarKind::Annular,
                    _ => SolarKind::Hybrid,
                }
            }
            _ => {
                if reaches_at_minimum(positions, t_umb_min)? {
                    SolarKind::Total
                } else {
                    SolarKind::Annular
                }
            }
        }
    };
    Ok(Some(SolarEclipse {
        kind,
        central,
        maximum_ut: t_max,
        partial_begin_ut: partial_begin,
        partial_end_ut: partial_end,
        total_begin_ut: total_begin,
        total_end_ut: total_end,
        center_begin_ut: center_begin,
        center_end_ut: center_end,
        gamma: gamma / EQUATORIAL_KM,
        noon_ut: find_noon(positions, t_max)?,
    }))
}

/// For a non-central umbral eclipse: does the umbra (rather than the antumbra) touch the globe?
fn reaches_at_minimum<E, F>(positions: &mut F, t: f64) -> Result<bool, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    let s = shadow(&eval(positions, t)?);
    let near = min_over_globe(&s, -s.tan_f2) + s.z_umbra_apex * s.tan_f2;
    let far = min_over_globe(&s, s.tan_f2) - s.z_umbra_apex * s.tan_f2;
    Ok(near <= far)
}

/// The point of the globe under the axis: where the axis enters (Sun side) or, if it misses
/// the globe, the surface point nearest to it.
fn ground_point(s: &Shadow) -> V3 {
    match axis_entries(s) {
        Some((_, z_near)) => add(s.w, scale(s.u, z_near)),
        None => support_point(s.w),
    }
}

/// Local apparent noon at the ground point under the axis, nearest the maximum.
fn find_noon<E, F>(positions: &mut F, t_max: f64) -> Result<Option<f64>, EclipseError<E>>
where
    F: FnMut(f64) -> Result<SunMoon, E>,
{
    let mut g = |t: f64| -> Result<f64, EclipseError<E>> {
        let p = eval(positions, t)?;
        let ground = ground_point(&shadow(&p));
        let az = |v: V3| v[1].atan2(v[0]).to_degrees();
        Ok((az(ground) - az(p.sun_km) + 180.0).rem_euclid(360.0) - 180.0)
    };
    const STEPS: i32 = 288;
    let step = 1.0 / f64::from(STEPS);
    // Scan outward from the maximum for the nearest upward crossing of zero.
    let mut best: Option<(f64, f64, f64)> = None;
    let mut prev = (t_max - 0.5, g(t_max - 0.5)?);
    for i in 1..=2 * STEPS {
        let t = t_max - 0.5 + step * f64::from(i);
        let v = g(t)?;
        if prev.1 < 0.0 && v >= 0.0 && v - prev.1 < 170.0 {
            let distance = (t - t_max).abs();
            if best.is_none_or(|b| distance < (b.0 - t_max).abs()) {
                best = Some((t, prev.0, distance));
            }
        }
        prev = (t, v);
    }
    match best {
        None => Ok(None),
        Some((hi, lo, _)) => {
            // `bisect` expects h > 0 outside and h <= 0 inside: use -g on a bracket where g crosses upward.
            let mut h = |t: f64| g(t).map(|v| -v);
            Ok(Some(bisect(&mut h, lo, hi)?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    const AU_KM: f64 = 149_597_870.7;
    const T0: f64 = 2_451_545.0;
    const KM_PER_DAY: f64 = 1.0 * 86_400.0;

    /// The Sun fixed on +x at 1 AU, the Moon on the Sun's side of the Earth at `distance`
    /// km moving along y at 1 km/s, passing `impact` km from the Sun-Earth line along z.
    fn synthetic(distance: f64, impact: f64) -> impl FnMut(f64) -> Result<SunMoon, Infallible> {
        move |t| {
            Ok(SunMoon {
                sun_km: [AU_KM, 0.0, 0.0],
                moon_km: [distance, (t - T0) * KM_PER_DAY, impact],
            })
        }
    }

    fn find(distance: f64, impact: f64) -> Result<SolarEclipse, EclipseError<Infallible>> {
        next_solar_eclipse(synthetic(distance, impact), T0 - 2.0, 60.0, false)
    }

    #[test]
    fn a_near_moon_passing_close_gives_a_central_total_eclipse() {
        let e = find(360_000.0, 0.2 * EQUATORIAL_KM).unwrap();
        assert_eq!(e.kind, SolarKind::Total);
        assert!(e.central);
        assert!((e.gamma - 0.2).abs() < 5e-3, "{}", e.gamma);
        assert!((e.maximum_ut - T0).abs() < 1e-5);
        let (tb, te) = (e.total_begin_ut.unwrap(), e.total_end_ut.unwrap());
        let (cb, ce) = (e.center_begin_ut.unwrap(), e.center_end_ut.unwrap());
        assert!(e.partial_begin_ut < tb && tb <= cb && cb < e.maximum_ut);
        assert!(e.maximum_ut < ce && ce <= te && te < e.partial_end_ut);
        // The penumbra footprint is about 2 x 9,700 km long: 19,400 s at 1 km/s.
        let duration = (e.partial_end_ut - e.partial_begin_ut) * 86_400.0;
        assert!((duration - 19_400.0).abs() < 1_000.0, "{duration}");
    }

    #[test]
    fn a_far_moon_gives_an_annular_eclipse_and_a_wide_pass_a_partial_one() {
        let annular = find(400_000.0, 0.1 * EQUATORIAL_KM).unwrap();
        assert_eq!(annular.kind, SolarKind::Annular);
        assert!(annular.central && annular.total_begin_ut.is_some());
        let partial = find(360_000.0, 1.1 * EQUATORIAL_KM).unwrap();
        assert_eq!(partial.kind, SolarKind::Partial);
        assert!(!partial.central);
        assert!(partial.total_begin_ut.is_none() && partial.center_begin_ut.is_none());
        assert!((partial.gamma - 1.1).abs() < 5e-3);
    }

    #[test]
    fn a_pass_outside_the_penumbra_is_no_eclipse() {
        assert_eq!(
            find(360_000.0, 1.8 * EQUATORIAL_KM),
            Err(EclipseError::NotFound)
        );
    }

    #[test]
    fn an_axis_that_misses_the_globe_is_not_central() {
        // A pass a few kilometres outside the equatorial limb: the umbra (about 70 km wide)
        // may touch the limb, but the axis itself never meets the globe.
        let e = find(360_000.0, 1.0 * EQUATORIAL_KM + 5.0).unwrap();
        assert!(matches!(e.kind, SolarKind::Total | SolarKind::Partial));
        assert!(!e.central);
    }

    #[test]
    fn the_noon_instant_is_found_and_the_search_has_a_direction() {
        let e = find(360_000.0, 0.2 * EQUATORIAL_KM).unwrap();
        assert!(e.noon_ut.is_none() || (e.noon_ut.unwrap() - e.maximum_ut).abs() < 0.6);
        assert!(next_solar_eclipse(synthetic(360_000.0, 1_000.0), T0 + 0.5, 3.0, true).is_ok());
        assert_eq!(
            next_solar_eclipse(synthetic(360_000.0, 1_000.0), T0 + 0.5, 3.0, false),
            Err(EclipseError::NotFound)
        );
    }
}
