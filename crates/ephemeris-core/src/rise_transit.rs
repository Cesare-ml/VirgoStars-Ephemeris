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

// Modification 2026-11-18: permit an explicit sidereal provider for event searches.

//! Rising, setting and meridian transit of a body.
//!
//! The module is deliberately free of any ephemeris: the caller supplies a closure
//! that returns the **apparent geocentric equatorial-of-date** position of the
//! body (right ascension, declination, distance and the TT of the same instant)
//! for a UT Julian date. The binding feeds it from the DE441 pipeline; tests feed
//! it synthetic bodies.
//!
//! # Definitions
//!
//! * The observer is on the WGS84 ellipsoid at the given geodetic latitude,
//!   longitude and height. The local apparent sidereal time is the IAU 2006/2000B
//!   Greenwich apparent sidereal time plus the east longitude (UT1 is taken equal
//!   to the supplied UT: |UT1 − UTC| < 0.9 s is not modelled).
//! * With `topocentric` set, the observer's position (in the equatorial frame of
//!   date) is subtracted from the geocentric vector: this is the diurnal parallax.
//!   Diurnal aberration (≤ 0.3″) and polar motion are not modelled.
//! * **Rise / set**: the instant the chosen limb of the disc reaches the horizon
//!   going up / down. The horizon is the *apparent* horizon of
//!   the extended refraction model of [`crate::horizon`] (`h*`, the true altitude whose
//!   apparent altitude is 0, about −34′) unless refraction is disabled (`h* = 0`).
//!   The threshold for the **centre** is `h* − s` for the upper limb, `h*` for the
//!   centre and `h* + s` for the lower limb, with `s = asin(radius / distance)`
//!   the topocentric semi-diameter. A twilight request replaces the whole
//!   threshold by a fixed depression of the centre.
//! * **Upper / lower transit**: the hour angle of the same (topocentric or
//!   geocentric) direction is 0° / 180°.
//! * Events are searched **after** the start instant; the search covers
//!   `search_days` days in steps of half an hour and each bracket is refined to
//!   better than a millisecond. No event in that window is `Ok(None)`, never an
//!   error (polar day/night, circumpolar bodies).

use std::error::Error;
use std::fmt;

use crate::horizon::{
    apparent_horizon_true_altitude_deg, local_apparent_sidereal_deg, HorizonError,
};
use crate::wgs84_ecef::wgs84_geodetic_to_ecef;

const KM_PER_AU: f64 = 149_597_870.7;
const SCAN_STEP_DAYS: f64 = 1.0 / 48.0;
const REFINE_TOLERANCE_DAYS: f64 = 1e-9;

/// Apparent geocentric equatorial-of-date position at one instant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyPosition {
    pub tt_jd: f64,
    pub right_ascension_deg: f64,
    pub declination_deg: f64,
    pub distance_au: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Site {
    pub latitude_deg: f64,
    pub longitude_deg_east: f64,
    pub height_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Atmosphere {
    /// Resolved pressure in hPa (the caller applies the "0 = estimate" convention).
    pub pressure_hpa: f64,
    pub temperature_c: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Rise,
    Set,
    UpperTransit,
    LowerTransit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limb {
    Upper,
    Center,
    Lower,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Disc {
    /// Physical radius in km; the semi-diameter follows the distance.
    Radius { km: f64, limb: Limb },
    /// A fixed semi-diameter in degrees (the reference's fixed-disc-size option).
    FixedSemiDiameter { degrees: f64, limb: Limb },
    /// A point: no disc.
    Point,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Request {
    pub event: Event,
    pub disc: Disc,
    /// Apply atmospheric refraction to the horizon.
    pub refraction: bool,
    /// Use the topocentric (parallax-corrected) direction.
    pub topocentric: bool,
    /// Fixed centre altitude (degrees) replacing every other horizon term, for
    /// twilight definitions.
    pub fixed_centre_altitude_deg: Option<f64>,
    pub search_days: f64,
}

#[derive(Debug, PartialEq)]
pub enum RiseError<E> {
    Site(HorizonError),
    Position(E),
    InvalidRequest(&'static str),
}

impl<E: fmt::Display> fmt::Display for RiseError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Site(e) => write!(f, "{e}"),
            Self::Position(e) => write!(f, "position: {e}"),
            Self::InvalidRequest(m) => write!(f, "invalid request: {m}"),
        }
    }
}
impl<E: fmt::Debug + fmt::Display> Error for RiseError<E> {}

/// Observer position in the true equator of date, in AU, for a local sidereal angle.
fn observer_vector(site: &Site, armc_deg: f64) -> Result<[f64; 3], HorizonError> {
    let ecef = wgs84_geodetic_to_ecef(0.0, site.latitude_deg.to_radians(), site.height_m)
        .map_err(|_| HorizonError::InvalidSite)?;
    let (rho_xy, z) = (ecef.x_m / 1000.0 / KM_PER_AU, ecef.z_m / 1000.0 / KM_PER_AU);
    let (s, c) = armc_deg.to_radians().sin_cos();
    Ok([rho_xy * c, rho_xy * s, z])
}

struct Observed {
    right_ascension_deg: f64,
    declination_deg: f64,
    distance_au: f64,
    armc_deg: f64,
}

fn observe<E, F, S>(
    position: &mut F,
    sidereal: &mut S,
    site: &Site,
    topocentric: bool,
    ut_jd: f64,
) -> Result<Observed, RiseError<E>>
where
    F: FnMut(f64) -> Result<SkyPosition, E>,
    S: FnMut(f64, f64, f64) -> Result<f64, HorizonError>,
{
    let sky = position(ut_jd).map_err(RiseError::Position)?;
    let armc = sidereal(ut_jd, sky.tt_jd, site.longitude_deg_east).map_err(RiseError::Site)?;
    if !armc.is_finite() {
        return Err(RiseError::Site(HorizonError::NonFinite));
    }
    let (ra, dec) = (
        sky.right_ascension_deg.to_radians(),
        sky.declination_deg.to_radians(),
    );
    let mut v = [
        sky.distance_au * dec.cos() * ra.cos(),
        sky.distance_au * dec.cos() * ra.sin(),
        sky.distance_au * dec.sin(),
    ];
    if topocentric {
        let o = observer_vector(site, armc).map_err(RiseError::Site)?;
        for i in 0..3 {
            v[i] -= o[i];
        }
    }
    let d = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    Ok(Observed {
        right_ascension_deg: v[1].atan2(v[0]).to_degrees().rem_euclid(360.0),
        declination_deg: (v[2] / d).clamp(-1.0, 1.0).asin().to_degrees(),
        distance_au: d,
        armc_deg: armc,
    })
}

fn wrap_pm180(x: f64) -> f64 {
    (x + 180.0).rem_euclid(360.0) - 180.0
}

/// Signed function whose upward zero crossing is the event.
fn event_function<E, F, S>(
    position: &mut F,
    sidereal: &mut S,
    site: &Site,
    request: &Request,
    horizon_true_altitude_deg: f64,
    ut_jd: f64,
) -> Result<f64, RiseError<E>>
where
    F: FnMut(f64) -> Result<SkyPosition, E>,
    S: FnMut(f64, f64, f64) -> Result<f64, HorizonError>,
{
    let o = observe(position, sidereal, site, request.topocentric, ut_jd)?;
    match request.event {
        Event::UpperTransit => Ok(wrap_pm180(o.armc_deg - o.right_ascension_deg)),
        Event::LowerTransit => Ok(wrap_pm180(o.armc_deg - o.right_ascension_deg - 180.0)),
        Event::Rise | Event::Set => {
            let (dec, lat) = (
                o.declination_deg.to_radians(),
                site.latitude_deg.to_radians(),
            );
            let h = (o.armc_deg - o.right_ascension_deg).to_radians();
            let altitude = (lat.sin() * dec.sin() + lat.cos() * dec.cos() * h.cos())
                .clamp(-1.0, 1.0)
                .asin()
                .to_degrees();
            let threshold = match request.fixed_centre_altitude_deg {
                Some(fixed) => fixed,
                None => {
                    let semi = match request.disc {
                        Disc::Point => (0.0, Limb::Center),
                        Disc::Radius { km, limb } => (
                            (km / (o.distance_au * KM_PER_AU))
                                .clamp(-1.0, 1.0)
                                .asin()
                                .to_degrees(),
                            limb,
                        ),
                        Disc::FixedSemiDiameter { degrees, limb } => (degrees, limb),
                    };
                    match semi.1 {
                        Limb::Upper => horizon_true_altitude_deg - semi.0,
                        Limb::Center => horizon_true_altitude_deg,
                        Limb::Lower => horizon_true_altitude_deg + semi.0,
                    }
                }
            };
            let signed = altitude - threshold;
            Ok(if request.event == Event::Rise {
                signed
            } else {
                -signed
            })
        }
    }
}

/// Find the first event strictly after `start_ut_jd`.
///
/// `position` maps a UT Julian date to the apparent geocentric equatorial
/// position of the body. Returns the UT of the event, or `None` when nothing
/// happens within `request.search_days`.
pub fn find_event<E, F>(
    position: F,
    site: &Site,
    atmosphere: &Atmosphere,
    request: &Request,
    start_ut_jd: f64,
) -> Result<Option<f64>, RiseError<E>>
where
    F: FnMut(f64) -> Result<SkyPosition, E>,
{
    find_event_with_sidereal(
        position,
        local_apparent_sidereal_deg,
        site,
        atmosphere,
        request,
        start_ut_jd,
    )
}

/// Search with a caller-selected local apparent sidereal-time provider.
/// The position and sidereal providers must use compatible equatorial frames.
/// Existing `find_event` continues to use its guarded modern sidereal model.
pub fn find_event_with_sidereal<E, F, S>(
    mut position: F,
    mut sidereal: S,
    site: &Site,
    atmosphere: &Atmosphere,
    request: &Request,
    start_ut_jd: f64,
) -> Result<Option<f64>, RiseError<E>>
where
    F: FnMut(f64) -> Result<SkyPosition, E>,
    S: FnMut(f64, f64, f64) -> Result<f64, HorizonError>,
{
    if !start_ut_jd.is_finite()
        || !site.latitude_deg.is_finite()
        || !site.longitude_deg_east.is_finite()
        || !site.height_m.is_finite()
    {
        return Err(RiseError::Site(HorizonError::NonFinite));
    }
    if site.latitude_deg.abs() >= 90.0 {
        return Err(RiseError::Site(HorizonError::InvalidSite));
    }
    if !(request.search_days.is_finite()
        && request.search_days > 0.0
        && request.search_days <= 31.0)
    {
        return Err(RiseError::InvalidRequest("search_days must be in (0, 31]"));
    }
    // True altitude whose apparent altitude is zero (the refracted horizon).
    let horizon = if request.refraction {
        apparent_horizon_true_altitude_deg(atmosphere.pressure_hpa, atmosphere.temperature_c)
    } else {
        0.0
    };
    let mut f = |t: f64| event_function(&mut position, &mut sidereal, site, request, horizon, t);
    let end = start_ut_jd + request.search_days;
    let steps = (request.search_days / SCAN_STEP_DAYS).ceil() as usize;
    let grazing_possible = matches!(request.event, Event::Rise | Event::Set);
    // The sample two steps back, if any, and the one step back.
    let mut older: Option<(f64, f64)> = None;
    let mut previous = (start_ut_jd, f(start_ut_jd)?);
    for k in 1..=steps {
        let t_cur = (start_ut_jd + k as f64 * SCAN_STEP_DAYS).min(end);
        let f_cur = f(t_cur)?;
        let (t_prev, f_prev) = previous;
        // An hour-angle function jumps by 360° where it wraps: never a crossing.
        if f_prev < 0.0 && f_cur >= 0.0 && f_cur - f_prev < 90.0 {
            return Ok(Some(refine(&mut f, t_prev, f_prev, t_cur, f_cur)?));
        }
        // A body that crosses the horizon and comes back within one sample step shows up
        // only as a local extremum of the otherwise one-signed function: a peak among
        // negative samples (a brief rise above the horizon) or a trough among
        // non-negative ones (a brief dip below it). Find the extremum and test it.
        if grazing_possible {
            if let Some((t_old, f_old)) = older {
                if f_old < 0.0 && f_prev < 0.0 && f_cur < 0.0 && f_prev > f_old && f_prev > f_cur {
                    let (t_peak, f_peak) = maximize(&mut f, t_old, t_cur)?;
                    if f_peak >= 0.0 {
                        return Ok(Some(refine(&mut f, t_old, f_old, t_peak, f_peak)?));
                    }
                }
                if f_old >= 0.0 && f_prev >= 0.0 && f_cur >= 0.0 && f_prev < f_old && f_prev < f_cur
                {
                    let (t_trough, neg_trough) =
                        maximize(&mut |t: f64| f(t).map(|v| -v), t_old, t_cur)?;
                    if neg_trough > 0.0 {
                        return Ok(Some(refine(&mut f, t_trough, -neg_trough, t_cur, f_cur)?));
                    }
                }
            }
        }
        older = Some((t_prev, f_prev));
        previous = (t_cur, f_cur);
        if t_cur >= end {
            break;
        }
    }
    Ok(None)
}

/// Golden-section search for the maximum of a unimodal function on `[a, b]`.
fn maximize<E>(
    f: &mut impl FnMut(f64) -> Result<f64, RiseError<E>>,
    mut a: f64,
    mut b: f64,
) -> Result<(f64, f64), RiseError<E>> {
    const INV_PHI: f64 = 0.618_033_988_749_894_9;
    let mut c = b - INV_PHI * (b - a);
    let mut d = a + INV_PHI * (b - a);
    let (mut fc, mut fd) = (f(c)?, f(d)?);
    for _ in 0..60 {
        if (b - a).abs() < REFINE_TOLERANCE_DAYS {
            break;
        }
        if fc > fd {
            b = d;
            d = c;
            fd = fc;
            c = b - INV_PHI * (b - a);
            fc = f(c)?;
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + INV_PHI * (b - a);
            fd = f(d)?;
        }
    }
    Ok(if fc > fd { (c, fc) } else { (d, fd) })
}

/// Illinois (modified regula falsi) refinement of a bracketed upward crossing.
fn refine<E>(
    f: &mut impl FnMut(f64) -> Result<f64, RiseError<E>>,
    mut a: f64,
    mut fa: f64,
    mut b: f64,
    mut fb: f64,
) -> Result<f64, RiseError<E>> {
    let mut side = 0i8;
    for _ in 0..100 {
        if (b - a).abs() < REFINE_TOLERANCE_DAYS || fb == fa {
            break;
        }
        let c = (a * fb - b * fa) / (fb - fa);
        let fc = f(c)?;
        if fc == 0.0 {
            return Ok(c);
        }
        if fc < 0.0 {
            a = c;
            fa = fc;
            if side == -1 {
                fb *= 0.5;
            }
            side = -1;
        } else {
            b = c;
            fb = fc;
            if side == 1 {
                fa *= 0.5;
            }
            side = 1;
        }
    }
    Ok(0.5 * (a + b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::{equatorial_to_horizontal, local_apparent_sidereal_deg};
    use std::convert::Infallible;

    const J2000_UT: f64 = 2_451_545.0;

    fn star(ra: f64, dec: f64) -> impl FnMut(f64) -> Result<SkyPosition, Infallible> {
        move |ut| {
            Ok(SkyPosition {
                tt_jd: ut + 64.0 / 86_400.0,
                right_ascension_deg: ra,
                declination_deg: dec,
                distance_au: 1.0e9,
            })
        }
    }

    fn request(event: Event) -> Request {
        Request {
            event,
            disc: Disc::Point,
            refraction: false,
            topocentric: false,
            fixed_centre_altitude_deg: None,
            search_days: 2.0,
        }
    }

    const SITE: Site = Site {
        latitude_deg: 41.9,
        longitude_deg_east: 12.5,
        height_m: 0.0,
    };
    const AIR: Atmosphere = Atmosphere {
        pressure_hpa: 1013.25,
        temperature_c: 15.0,
    };

    fn altitude_at(ra: f64, dec: f64, ut: f64, site: &Site) -> f64 {
        let armc =
            local_apparent_sidereal_deg(ut, ut + 64.0 / 86_400.0, site.longitude_deg_east).unwrap();
        equatorial_to_horizontal(ra, dec, armc, site.latitude_deg)
            .unwrap()
            .true_altitude_deg
    }

    #[test]
    fn a_star_rises_at_zero_altitude_and_sets_after_it() {
        let (ra, dec) = (101.0, -16.7);
        let rise = find_event(star(ra, dec), &SITE, &AIR, &request(Event::Rise), J2000_UT)
            .unwrap()
            .expect("rises");
        let set = find_event(star(ra, dec), &SITE, &AIR, &request(Event::Set), J2000_UT)
            .unwrap()
            .expect("sets");
        assert!(altitude_at(ra, dec, rise, &SITE).abs() < 1e-6);
        assert!(altitude_at(ra, dec, rise + 0.01, &SITE) > 0.0);
        assert!(altitude_at(ra, dec, set, &SITE).abs() < 1e-6);
        assert!(altitude_at(ra, dec, set - 0.01, &SITE) > 0.0);
        assert!(rise > J2000_UT && set > J2000_UT);
        assert!(rise - J2000_UT < 1.01 && set - J2000_UT < 1.01);
    }

    #[test]
    fn explicit_sidereal_provider_preserves_default_and_rejects_nonfinite() {
        let a = find_event(
            star(101.0, -16.7),
            &SITE,
            &AIR,
            &request(Event::Rise),
            J2000_UT,
        )
        .unwrap();
        let b = find_event_with_sidereal(
            star(101.0, -16.7),
            local_apparent_sidereal_deg,
            &SITE,
            &AIR,
            &request(Event::Rise),
            J2000_UT,
        )
        .unwrap();
        assert_eq!(a, b);
        let bad = find_event_with_sidereal(
            star(101.0, -16.7),
            |_, _, _| Ok(f64::NAN),
            &SITE,
            &AIR,
            &request(Event::Rise),
            J2000_UT,
        );
        assert!(matches!(bad, Err(RiseError::Site(HorizonError::NonFinite))));
    }

    #[test]
    fn transits_are_hour_angle_zero_and_half_a_sidereal_day_apart() {
        let (ra, dec) = (250.0, 20.0);
        let upper = find_event(
            star(ra, dec),
            &SITE,
            &AIR,
            &request(Event::UpperTransit),
            J2000_UT,
        )
        .unwrap()
        .unwrap();
        let lower = find_event(
            star(ra, dec),
            &SITE,
            &AIR,
            &request(Event::LowerTransit),
            J2000_UT,
        )
        .unwrap()
        .unwrap();
        let armc = |t: f64| {
            local_apparent_sidereal_deg(t, t + 64.0 / 86_400.0, SITE.longitude_deg_east).unwrap()
        };
        assert!(wrap_pm180(armc(upper) - ra).abs() < 1e-6);
        assert!(wrap_pm180(armc(lower) - ra - 180.0).abs() < 1e-6);
        // Meridian altitude is 90 - |lat - dec| at the upper transit.
        assert!((altitude_at(ra, dec, upper, &SITE) - (90.0 - (41.9 - 20.0))).abs() < 1e-6);
        // Half a sidereal day (11.967 h) between the two.
        let gap = (lower - upper).rem_euclid(0.997_269_566_4);
        assert!((gap - 0.498_634_783).abs() < 1e-5, "{gap}");
    }

    #[test]
    fn circumpolar_and_never_visible_bodies_have_no_rise_or_set() {
        // Dec +80 at latitude 41.9: circumpolar. Dec -80: never rises.
        assert_eq!(
            find_event(
                star(0.0, 80.0),
                &SITE,
                &AIR,
                &request(Event::Rise),
                J2000_UT
            )
            .unwrap(),
            None
        );
        assert_eq!(
            find_event(
                star(0.0, -80.0),
                &SITE,
                &AIR,
                &request(Event::Set),
                J2000_UT
            )
            .unwrap(),
            None
        );
        // They still transit.
        assert!(find_event(
            star(0.0, 80.0),
            &SITE,
            &AIR,
            &request(Event::LowerTransit),
            J2000_UT
        )
        .unwrap()
        .is_some());
    }

    #[test]
    fn refraction_and_the_upper_limb_make_a_body_rise_earlier() {
        let (ra, dec) = (101.0, -16.7);
        let plain = find_event(star(ra, dec), &SITE, &AIR, &request(Event::Rise), J2000_UT)
            .unwrap()
            .unwrap();
        let refracted = Request {
            refraction: true,
            ..request(Event::Rise)
        };
        let with_refraction = find_event(star(ra, dec), &SITE, &AIR, &refracted, J2000_UT)
            .unwrap()
            .unwrap();
        // About 34 arcminutes of lift: a few minutes of time at this latitude.
        let minutes = (plain - with_refraction) * 1440.0;
        assert!((1.5..4.5).contains(&minutes), "{minutes}");
        let disc = Request {
            disc: Disc::FixedSemiDiameter {
                degrees: 0.2666,
                limb: Limb::Upper,
            },
            ..refracted
        };
        let upper = find_event(star(ra, dec), &SITE, &AIR, &disc, J2000_UT)
            .unwrap()
            .unwrap();
        assert!(upper < with_refraction);
        let lower = Request {
            disc: Disc::FixedSemiDiameter {
                degrees: 0.2666,
                limb: Limb::Lower,
            },
            ..refracted
        };
        let lower_t = find_event(star(ra, dec), &SITE, &AIR, &lower, J2000_UT)
            .unwrap()
            .unwrap();
        assert!(lower_t > with_refraction);
    }

    #[test]
    fn parallax_delays_the_rise_of_a_near_body() {
        // A body at the Moon's distance on the equator: parallax lowers it by about a degree.
        let near = |ut: f64| -> Result<SkyPosition, Infallible> {
            Ok(SkyPosition {
                tt_jd: ut + 64.0 / 86_400.0,
                right_ascension_deg: 101.0,
                declination_deg: 5.0,
                distance_au: 384_400.0 / KM_PER_AU,
            })
        };
        let geo = Request {
            topocentric: false,
            ..request(Event::Rise)
        };
        let topo = Request {
            topocentric: true,
            ..request(Event::Rise)
        };
        let t_geo = find_event(near, &SITE, &AIR, &geo, J2000_UT)
            .unwrap()
            .unwrap();
        let t_topo = find_event(near, &SITE, &AIR, &topo, J2000_UT)
            .unwrap()
            .unwrap();
        let minutes = (t_topo - t_geo) * 1440.0;
        assert!((2.0..8.0).contains(&minutes), "{minutes}");
    }

    #[test]
    fn a_body_that_clears_the_horizon_for_minutes_is_not_missed() {
        // Culminates 0.03 degrees above the horizon: above it for about eleven minutes,
        // shorter than the half-hour scan step, wherever the scan happens to start.
        let (ra, dec) = (200.0, -48.07);
        for k in 0..60 {
            let start = J2000_UT + f64::from(k) * 0.0131;
            let rise = find_event(star(ra, dec), &SITE, &AIR, &request(Event::Rise), start)
                .unwrap()
                .unwrap_or_else(|| panic!("grazing rise missed from offset {k}"));
            let set = find_event(star(ra, dec), &SITE, &AIR, &request(Event::Set), start)
                .unwrap()
                .expect("grazing set");
            assert!(rise > start && set > start);
            assert!(altitude_at(ra, dec, rise, &SITE).abs() < 1e-5, "{k}");
            assert!(altitude_at(ra, dec, set, &SITE).abs() < 1e-5, "{k}");
            // When the scan starts before the passage the set follows the rise within minutes.
            if set > rise {
                assert!(set - rise < 0.02, "{k}");
            }
        }
    }

    #[test]
    fn events_beyond_the_search_window_are_not_reported() {
        let (ra, dec) = (101.0, -16.7);
        let narrow = Request {
            search_days: 0.01,
            ..request(Event::Rise)
        };
        // From a start where the rise is hours away, a quarter-hour window sees nothing.
        let found = find_event(star(ra, dec), &SITE, &AIR, &narrow, J2000_UT).unwrap();
        assert!(found.is_none() || found.unwrap() - J2000_UT <= 0.01);
    }

    #[test]
    fn invalid_requests_are_refused() {
        let bad_site = Site {
            latitude_deg: 90.0,
            ..SITE
        };
        assert!(find_event(
            star(0.0, 0.0),
            &bad_site,
            &AIR,
            &request(Event::Rise),
            J2000_UT
        )
        .is_err());
        let bad = Request {
            search_days: 0.0,
            ..request(Event::Rise)
        };
        assert!(find_event(star(0.0, 0.0), &SITE, &AIR, &bad, J2000_UT).is_err());
        assert!(find_event(star(0.0, 0.0), &SITE, &AIR, &request(Event::Rise), f64::NAN).is_err());
    }
}
