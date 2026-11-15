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

//! Bounded experimental geometric horizon and meridian event search.
//! Added 2026-11-14. Not a visible rise/set or Swiss event replacement.

use crate::de441::{Body, De441, JulianDateTdb};
use crate::earth_orientation::{gast_iau2006, OrientationError};
use crate::equatorial_horizontal::{true_equatorial_to_horizontal, HorizontalConversionError};
use crate::first_sky::{observer_vector, Observer};
use crate::six_field_units::IAU_AU_KM;
use crate::true_ecliptic::{geocentric_true_equatorial, TrueEclipticError};
use std::error::Error;
use std::fmt;
use std::io::{Read, Seek};

/// Strict directional event; tangencies are not detected by sign sampling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizonEventKind {
    Rise,
    Set,
    UpperTransit,
    LowerTransit,
}

/// Explicit epochs for the same instant. No implicit UTC or Earth-orientation
/// conversion is performed. Offsets are propagated uniformly over the search.
#[derive(Clone, Copy, Debug)]
pub struct HorizonEventSearch {
    pub target: Body,
    pub kind: HorizonEventKind,
    pub start_ut1_jd: f64,
    pub start_tt_jd: f64,
    pub start_tdb: JulianDateTdb,
    pub latitude_deg: f64,
    pub longitude_deg_east: f64,
    /// Caller-selected geometric center altitude for rise/set, degrees.
    pub threshold_altitude_deg: f64,
    /// Search window in (0, 2] days; no event outside it is implied.
    pub window_days: f64,
    /// Sampling cadence in [1, 60] minutes. Crossings can be missed between samples.
    pub step_minutes: f64,
}

/// Inputs for the USNO-style conventional solar rise/set approximation.
/// Explicit UT1, TT and TDB epochs must represent the same initial instant.
#[derive(Clone, Copy, Debug)]
pub struct StandardSolarEventSearch {
    pub kind: HorizonEventKind,
    pub start_ut1_jd: f64,
    pub start_tt_jd: f64,
    pub start_tdb: JulianDateTdb,
    pub latitude_deg: f64,
    pub longitude_deg_east: f64,
    pub window_days: f64,
    pub step_minutes: f64,
}

/// Conventional unobstructed-horizon solar rise/set using a fixed 50 arcminute
/// depression of the geocentric solar center. This models average refraction
/// and mean solar semidiameter as a combined constant, not observed weather or
/// a physical solar disk. See <https://aa.usno.navy.mil/faq/RST_defs>.
/// This is approximate and does not reproduce Swiss rise/set semantics.
pub fn find_standard_solar_event<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    input: StandardSolarEventSearch,
) -> Result<HorizonEvent, HorizonEventError> {
    if !matches!(input.kind, HorizonEventKind::Rise | HorizonEventKind::Set) {
        return Err(HorizonEventError::InvalidInput);
    }
    find_geometric_event(
        ephemeris,
        HorizonEventSearch {
            target: Body::Sun,
            kind: input.kind,
            start_ut1_jd: input.start_ut1_jd,
            start_tt_jd: input.start_tt_jd,
            start_tdb: input.start_tdb,
            latitude_deg: input.latitude_deg,
            longitude_deg_east: input.longitude_deg_east,
            threshold_altitude_deg: -50.0 / 60.0,
            window_days: input.window_days,
            step_minutes: input.step_minutes,
        },
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizonEvent {
    pub kind: HorizonEventKind,
    pub target: Body,
    pub ut1_jd: f64,
    pub tt_jd: f64,
    pub tdb: JulianDateTdb,
    /// Final time bracket width in SI seconds, subject to f64 JD resolution.
    pub bracket_seconds: f64,
}

#[derive(Debug)]
pub enum HorizonEventError {
    InvalidInput,
    IncoherentEpoch,
    NotFound,
    InvalidSample,
    Ephemeris(TrueEclipticError),
    Horizontal(HorizontalConversionError),
    Orientation(OrientationError),
}
impl fmt::Display for HorizonEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => write!(f, "invalid event-search parameters"),
            Self::IncoherentEpoch => {
                write!(f, "UT1, TT and TDB must describe the same start instant")
            }
            Self::NotFound => write!(f, "no requested crossing found in the sampled window"),
            Self::InvalidSample => write!(f, "event function returned a non-finite value"),
            Self::Ephemeris(e) => write!(f, "ephemeris: {e}"),
            Self::Horizontal(e) => write!(f, "horizontal geometry: {e}"),
            Self::Orientation(e) => write!(f, "orientation: {e}"),
        }
    }
}
impl Error for HorizonEventError {}

// Search a continuous signed scalar; rising and upper-transit roots cross
// from negative to positive, setting and lower-transit from positive to negative.
fn search_crossing<F>(
    window: f64,
    step: f64,
    increasing: bool,
    mut sample: F,
) -> Result<(f64, f64), HorizonEventError>
where
    F: FnMut(f64) -> Result<f64, HorizonEventError>,
{
    let mut left = 0.0;
    let mut f_left = sample(left)?;
    if !f_left.is_finite() {
        return Err(HorizonEventError::InvalidSample);
    }
    while left < window {
        let mut right = (left + step).min(window);
        let f_right = sample(right)?;
        if !f_right.is_finite() {
            return Err(HorizonEventError::InvalidSample);
        }
        let crossed = if increasing {
            (f_left < 0.0 && f_right >= 0.0) || (f_left == 0.0 && f_right > 0.0)
        } else {
            (f_left > 0.0 && f_right <= 0.0) || (f_left == 0.0 && f_right < 0.0)
        };
        if crossed {
            while (right - left) * 86_400.0 > 0.25 {
                let mid = (left + right) / 2.0;
                let value = sample(mid)?;
                if !value.is_finite() {
                    return Err(HorizonEventError::InvalidSample);
                }
                if (increasing && value <= 0.0) || (!increasing && value >= 0.0) {
                    left = mid;
                } else {
                    right = mid;
                }
            }
            return Ok(((left + right) / 2.0, (right - left) * 86_400.0));
        }
        left = right;
        f_left = f_right;
    }
    Err(HorizonEventError::NotFound)
}

/// Search for the next sampled geometric altitude or meridian crossing.
/// Results project a geocentric light-time-plus-aberration direction to an ideal
/// horizon: no topocentric parallax, refraction, limb, terrain, polar motion,
/// UT1 interpolation or leap-second handling is performed. The caller
/// supplies coherent epochs and an explicit geometric threshold. Numerical
/// bracket width is not an accuracy guarantee and grazing events can be missed.
pub fn find_geometric_event<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    input: HorizonEventSearch,
) -> Result<HorizonEvent, HorizonEventError> {
    find_event(ephemeris, input, None)
}

/// Like `find_geometric_event`, but subtracts a WGS84 observer vector from
/// the geocentric direction. Height is geodetic metres; no observer light-time
/// or diurnal aberration is applied. Refraction and body limbs remain omitted.
pub fn find_topocentric_geometric_event<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    input: HorizonEventSearch,
    height_m: f64,
) -> Result<HorizonEvent, HorizonEventError> {
    if !height_m.is_finite() || !(-12_000.0..=100_000.0).contains(&height_m) {
        return Err(HorizonEventError::InvalidInput);
    }
    find_event(ephemeris, input, Some(height_m))
}

/// Subtract the WGS84 observer position in true-equator-of-date axes. This
/// is a geometric parallax correction of an already corrected direction,
/// not a complete topocentric apparent place calculation.
pub fn topocentric_equatorial_direction(
    ut1_jd: f64,
    tt_jd: f64,
    site: Observer,
    ra_deg: f64,
    dec_deg: f64,
    distance_au: f64,
) -> Result<(f64, f64), HorizonEventError> {
    if !ut1_jd.is_finite() || !tt_jd.is_finite() || (ut1_jd - tt_jd).abs() > 0.1 {
        return Err(HorizonEventError::IncoherentEpoch);
    }
    if !site.latitude_deg.is_finite()
        || site.latitude_deg.abs() > 90.0
        || !site.longitude_deg_east.is_finite()
        || site.longitude_deg_east.abs() > 180.0
        || !site.height_m.is_finite()
        || !(-12_000.0..=100_000.0).contains(&site.height_m)
        || !ra_deg.is_finite()
        || !(0.0..360.0).contains(&ra_deg)
        || !dec_deg.is_finite()
        || dec_deg.abs() > 90.0
        || !distance_au.is_finite()
        || distance_au <= 0.0
    {
        return Err(HorizonEventError::InvalidInput);
    }
    let gast = gast_iau2006(ut1_jd, tt_jd).map_err(HorizonEventError::Orientation)?;
    let observer = observer_vector(site, gast + site.longitude_deg_east.to_radians());
    let (sin_ra, cos_ra) = ra_deg.to_radians().sin_cos();
    let (sin_dec, cos_dec) = dec_deg.to_radians().sin_cos();
    let distance_km = distance_au * IAU_AU_KM;
    let xyz = [
        distance_km * cos_dec * cos_ra - observer[0],
        distance_km * cos_dec * sin_ra - observer[1],
        distance_km * sin_dec - observer[2],
    ];
    let xy = xyz[0].hypot(xyz[1]);
    let radius = xy.hypot(xyz[2]);
    if !radius.is_finite() || radius <= 0.0 || xy <= f64::EPSILON * radius {
        return Err(HorizonEventError::InvalidSample);
    }
    let ra = xyz[1].atan2(xyz[0]).to_degrees().rem_euclid(360.0);
    Ok((
        if ra >= 360.0 { 0.0 } else { ra },
        xyz[2].atan2(xy).to_degrees(),
    ))
}

fn find_event<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    input: HorizonEventSearch,
    height_m: Option<f64>,
) -> Result<HorizonEvent, HorizonEventError> {
    if !input.start_ut1_jd.is_finite()
        || !input.start_tt_jd.is_finite()
        || !input.start_tdb.0.is_finite()
        || (input.start_tt_jd - input.start_tdb.0).abs() > 1.0 / 86_400.0
        || (input.start_ut1_jd - input.start_tt_jd).abs() > 0.1
    {
        return Err(HorizonEventError::IncoherentEpoch);
    }
    if input.target == Body::Earth
        || !input.latitude_deg.is_finite()
        || input.latitude_deg.abs() >= 90.0
        || !input.longitude_deg_east.is_finite()
        || input.longitude_deg_east.abs() > 180.0
        || !input.threshold_altitude_deg.is_finite()
        || input.threshold_altitude_deg.abs() >= 90.0
        || !input.window_days.is_finite()
        || !(0.0..=2.0).contains(&input.window_days)
        || input.window_days == 0.0
        || !input.step_minutes.is_finite()
        || !(1.0..=60.0).contains(&input.step_minutes)
    {
        return Err(HorizonEventError::InvalidInput);
    }
    let increasing = matches!(
        input.kind,
        HorizonEventKind::Rise | HorizonEventKind::UpperTransit
    );
    let (elapsed, bracket_seconds) = search_crossing(
        input.window_days,
        input.step_minutes / 1440.0,
        increasing,
        |delta| {
            let ut1 = input.start_ut1_jd + delta;
            let tt = input.start_tt_jd + delta;
            let tdb = JulianDateTdb(input.start_tdb.0 + delta);
            let state = geocentric_true_equatorial(ephemeris, input.target, tdb, tt)
                .map_err(HorizonEventError::Ephemeris)?;
            let (ra, dec) = if let Some(height_m) = height_m {
                topocentric_equatorial_direction(
                    ut1,
                    tt,
                    Observer {
                        latitude_deg: input.latitude_deg,
                        longitude_deg_east: input.longitude_deg_east,
                        height_m,
                    },
                    state.right_ascension_deg,
                    state.declination_deg,
                    state.distance_au,
                )?
            } else {
                (state.right_ascension_deg, state.declination_deg)
            };
            match input.kind {
                HorizonEventKind::Rise | HorizonEventKind::Set => {
                    let horizontal = true_equatorial_to_horizontal(
                        ut1,
                        tt,
                        input.latitude_deg,
                        input.longitude_deg_east,
                        ra,
                        dec,
                    )
                    .map_err(HorizonEventError::Horizontal)?;
                    Ok(horizontal.true_altitude_deg - input.threshold_altitude_deg)
                }
                HorizonEventKind::UpperTransit | HorizonEventKind::LowerTransit => {
                    let gast = gast_iau2006(ut1, tt).map_err(HorizonEventError::Orientation)?;
                    Ok((gast + (input.longitude_deg_east - ra).to_radians()).sin())
                }
            }
        },
    )?;
    Ok(HorizonEvent {
        kind: input.kind,
        target: input.target,
        ut1_jd: input.start_ut1_jd + elapsed,
        tt_jd: input.start_tt_jd + elapsed,
        tdb: JulianDateTdb(input.start_tdb.0 + elapsed),
        bracket_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wgs84_parallax_has_expected_analytic_and_reference_behavior() {
        let ut1 = 2_451_545.0;
        let tt = 2_451_545.0;
        let angle = gast_iau2006(ut1, tt).unwrap();
        let ra = (angle.to_degrees() + 90.0).rem_euclid(360.0);
        let site = Observer {
            latitude_deg: 0.0,
            longitude_deg_east: 0.0,
            height_m: 0.0,
        };
        let (topo_ra, topo_dec) =
            topocentric_equatorial_direction(ut1, tt, site, ra, 0.0, 384_400.0 / IAU_AU_KM)
                .unwrap();
        let shift = (topo_ra - ra).rem_euclid(360.0);
        assert!((shift - (6378.137_f64 / 384_400.0).atan().to_degrees()).abs() < 1e-10);
        assert!(topo_dec.abs() < 1e-12);
        assert!(matches!(
            topocentric_equatorial_direction(ut1, tt, site, ra, 0.0, 0.0),
            Err(HorizonEventError::InvalidInput)
        ));
        // Independent Swiss 2.09.03 output, same DE441 and geocentric input, Rome
        // 2026-11-10 00 UT; includes diurnal aberration and other model differences.
        let rome = Observer {
            latitude_deg: 41.9028,
            longitude_deg_east: 12.4964,
            height_m: 20.0,
        };
        let (ra, dec) = topocentric_equatorial_direction(
            2_461_354.5,
            2_461_354.500_800_741,
            rome,
            231.981_750_755_068_53,
            -24.008_338_358_732_306,
            0.002_672_331_115_769_936_6,
        )
        .unwrap();
        assert!((ra - 232.105_567_496_268_7).abs() < 0.00015);
        assert!((dec + 24.285_660_002_690_335).abs() < 0.00015);
    }
    #[test]
    fn directional_crossings_and_no_crossing_are_explicit() {
        let wave = |d: f64| Ok((d * std::f64::consts::TAU).sin());
        let (rise, width) = search_crossing(1.0, 0.1, true, wave).unwrap();
        assert!(rise * 86_400.0 < 0.25);
        assert!(width <= 0.25);
        let (set, _) = search_crossing(1.0, 0.1, false, wave).unwrap();
        assert!((set - 0.5).abs() * 86_400.0 < 0.25);
        assert!(matches!(
            search_crossing(1.0, 0.1, true, |_| Ok(1.0)),
            Err(HorizonEventError::NotFound)
        ));
        assert!(matches!(
            search_crossing(1.0, 0.1, true, |_| Ok(f64::NAN)),
            Err(HorizonEventError::InvalidSample)
        ));
        let (end, _) = search_crossing(0.5, 0.1, true, |t| Ok(t - 0.5)).unwrap();
        assert!((end - 0.5).abs() * 86_400.0 < 0.25);
        let (end, _) = search_crossing(0.5, 0.1, false, |t| Ok(0.5 - t)).unwrap();
        assert!((end - 0.5).abs() * 86_400.0 < 0.25);
    }

    #[test]
    #[ignore = "requires a separately supplied DE441_PATH"]
    fn real_kernel_solar_rise_reaches_requested_geometric_altitude() {
        let path = std::env::var_os("DE441_PATH").expect("DE441_PATH required");
        let mut eph = De441::open(path).unwrap();
        let input = HorizonEventSearch {
            target: Body::Sun,
            kind: HorizonEventKind::Rise,
            start_ut1_jd: 2461354.5,
            start_tt_jd: 2_461_354.500_800_741,
            start_tdb: JulianDateTdb(2_461_354.500_800_725),
            latitude_deg: 41.9028,
            longitude_deg_east: 12.4964,
            threshold_altitude_deg: 0.0,
            window_days: 1.0,
            step_minutes: 10.0,
        };
        let event = find_geometric_event(&mut eph, input).unwrap();
        let direction =
            geocentric_true_equatorial(&mut eph, Body::Sun, event.tdb, event.tt_jd).unwrap();
        let horizontal = true_equatorial_to_horizontal(
            event.ut1_jd,
            event.tt_jd,
            input.latitude_deg,
            input.longitude_deg_east,
            direction.right_ascension_deg,
            direction.declination_deg,
        )
        .unwrap();
        assert!(horizontal.true_altitude_deg.abs() < 0.001);
        assert!(
            event.ut1_jd > input.start_ut1_jd
                && event.ut1_jd < input.start_ut1_jd + input.window_days
        );
        assert!(event.bracket_seconds <= 0.25);
    }

    #[test]
    #[ignore = "requires a separately supplied DE441_PATH"]
    fn lunar_center_events_are_near_recorded_swiss_geometric_reference() {
        let path = std::env::var_os("DE441_PATH").expect("DE441_PATH required");
        let mut eph = De441::open(path).unwrap();
        // Swiss 2.09.03, DE441 JPL flag, center/no-refraction, Rome, 20 m,
        // 2026-11-10 00:00 UT; UT1=UTC is an explicit test approximation.
        let mut input = HorizonEventSearch {
            target: Body::Moon,
            kind: HorizonEventKind::Rise,
            start_ut1_jd: 2_461_354.5,
            start_tt_jd: 2_461_354.500_800_741,
            start_tdb: JulianDateTdb(2_461_354.500_800_725),
            latitude_deg: 41.9028,
            longitude_deg_east: 12.4964,
            threshold_altitude_deg: 0.0,
            window_days: 2.0,
            step_minutes: 10.0,
        };
        let rise = find_topocentric_geometric_event(&mut eph, input, 20.0).unwrap();
        assert!((rise.ut1_jd - 2_461_354.805_781_344).abs() * 86_400.0 < 3.0);
        input.kind = HorizonEventKind::Set;
        let set = find_topocentric_geometric_event(&mut eph, input, 20.0).unwrap();
        assert!((set.ut1_jd - 2_461_355.170_165_578_4).abs() * 86_400.0 < 3.0);
        assert!(matches!(
            find_topocentric_geometric_event(&mut eph, input, f64::NAN),
            Err(HorizonEventError::InvalidInput)
        ));
    }
}
