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

//! Instantaneous geometric sightline from a DE441 Earth-relative target
//! to a separately supplied observer **in the same DE441 ICRF axes**.
//!
//! This deliberately DOES NOT accept the integrated WGS84 observer GCRS
//! position. A physical, independently validated GCRS/ICRF frame and
//! epoch alignment must be established before those inputs can be mixed.
//! No aberration, light-time, relativity, Earth orientation, refraction,
//! precession, nutation or apparent-position corrections are applied.
//!
//! The classic DE441 reader supports TDB epochs; positions and velocities
//! are assumed equatorial ICRF kilometers and km/TDB day respectively.
//! Published independent angular oracle: ERFA t_p2s.
//! <https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c>
//!
//! Created 2026-11-12. Original VirgoStars code, CPAL-1.0.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;
use std::io::{Read, Seek};

use crate::de441::{Body, De441, De441Error, JulianDateTdb};

/// Geometric, simultaneous target-minus-observer direction in the
/// *input DE441 equatorial ICRF axes*, at a common TDB coordinate epoch.
///
/// The geocentric target position and its time derivative come from
/// the classic DE441 reader at `epoch_tdb`. The observer position is
/// supplied by the caller in the **same Earth-centered ICRF km axes**,
/// including all necessary frame and origin conversions upstream.
/// The velocity is that of the geocentric target only, not a complete
/// observer-relative velocity or light-time-corrected state.
///
/// Right ascension is an ICRF fixed-axis azimuth in [0, 2π), with
/// `None` at an exact celestial pole. Declination is in
/// [-π/2,π/2]. No equator/equinox of date or apparent correction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct De441IcrfObserverSightline {
    pub target: Body,
    pub epoch_tdb: JulianDateTdb,
    pub target_geocentric_icrf_km: [f64; 3],
    pub target_geocentric_velocity_icrf_km_per_tdb_day: [f64; 3],
    pub observer_geocentric_icrf_km: [f64; 3],
    pub displacement_icrf_km: [f64; 3],
    pub range_km: f64,
    pub unit_direction_icrf: [f64; 3],
    pub right_ascension_icrf_rad: Option<f64>,
    pub declination_icrf_rad: f64,
}

/// Fail-closed errors, without a silent GCRS≈ICRF approximation.
#[derive(Debug)]
pub enum De441IcrfObserverError {
    /// An Earth-centered sightline directed at Earth's own center is not
    /// a useful target state for this geometric observer API.
    EarthIsNotAValidTarget,
    InvalidTdbEpoch,
    InvalidObserver,
    Ephemeris(De441Error),
    NonFiniteTargetState,
    NonFiniteDisplacement,
    CoincidentTargetAndObserver,
    InvalidRange,
}

impl fmt::Display for De441IcrfObserverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EarthIsNotAValidTarget => write!(f, "target Earth has no supported sightline"),
            Self::InvalidTdbEpoch => write!(f, "DE441 epoch must be a finite TDB Julian date"),
            Self::InvalidObserver => {
                write!(f, "observer must be finite geocentric ICRF kilometers")
            }
            Self::Ephemeris(error) => write!(f, "DE441 geometric target state: {error}"),
            Self::NonFiniteTargetState => write!(f, "non-finite DE441 target position or velocity"),
            Self::NonFiniteDisplacement => {
                write!(f, "non-finite ICRF target-observer displacement")
            }
            Self::CoincidentTargetAndObserver => {
                write!(f, "target and observer positions coincide")
            }
            Self::InvalidRange => write!(f, "unrepresentable ICRF observer-target range"),
        }
    }
}

impl Error for De441IcrfObserverError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Ephemeris(error) => Some(error),
            _ => None,
        }
    }
}

/// Sample a DE441 geometric Earth-relative target and subtract an
/// **already Earth-centered ICRF** observer position at the same TDB
/// coordinate epoch.
///
/// This API intentionally does **not** take `ObserverGcrsPosition`:
/// its GCRS position is not proven to coincide with the DE441 ICRF
/// barycentric axes by an Earth-orientation numerical fixture alone.
/// The caller is responsible for an externally validated
/// geocentric-to-ICRF observer position before invoking this function.
///
/// The classic DE441 reader may return planetary **system barycenters**
/// (e.g. JupiterBarycenter), not the individual planet center. The
/// returned target and center semantics must be preserved.
/// Physical travel-time and apparent directions require separate stages.
pub fn de441_geometric_observer_sightline_icrf<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    target: Body,
    epoch_tdb: JulianDateTdb,
    observer_geocentric_icrf_km: [f64; 3],
) -> Result<De441IcrfObserverSightline, De441IcrfObserverError> {
    if target == Body::Earth {
        return Err(De441IcrfObserverError::EarthIsNotAValidTarget);
    }
    if !epoch_tdb.0.is_finite() {
        return Err(De441IcrfObserverError::InvalidTdbEpoch);
    }
    if observer_geocentric_icrf_km
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(De441IcrfObserverError::InvalidObserver);
    }
    let state = ephemeris
        .state(target, Body::Earth, epoch_tdb)
        .map_err(De441IcrfObserverError::Ephemeris)?;
    if state
        .position_km
        .iter()
        .chain(state.velocity_km_per_day.iter())
        .any(|value| !value.is_finite())
    {
        return Err(De441IcrfObserverError::NonFiniteTargetState);
    }
    let displacement_icrf_km: [f64; 3] =
        std::array::from_fn(|i| state.position_km[i] - observer_geocentric_icrf_km[i]);
    if displacement_icrf_km.iter().any(|value| !value.is_finite()) {
        return Err(De441IcrfObserverError::NonFiniteDisplacement);
    }
    let [x, y, z] = displacement_icrf_km;
    let planar_km = x.hypot(y);
    let range_km = planar_km.hypot(z);
    if range_km == 0.0 {
        return Err(De441IcrfObserverError::CoincidentTargetAndObserver);
    }
    if !range_km.is_finite() {
        return Err(De441IcrfObserverError::InvalidRange);
    }
    let unit_direction_icrf = [x / range_km, y / range_km, z / range_km];
    let declination_icrf_rad = z.atan2(planar_km);
    let right_ascension_icrf_rad = if planar_km == 0.0 {
        None
    } else {
        let ra = y.atan2(x).rem_euclid(TAU);
        Some(if ra == 0.0 || ra >= TAU { 0.0 } else { ra })
    };
    if unit_direction_icrf.iter().any(|value| !value.is_finite())
        || !declination_icrf_rad.is_finite()
    {
        return Err(De441IcrfObserverError::InvalidRange);
    }
    Ok(De441IcrfObserverSightline {
        target,
        epoch_tdb,
        target_geocentric_icrf_km: state.position_km,
        target_geocentric_velocity_icrf_km_per_tdb_day: state.velocity_km_per_day,
        observer_geocentric_icrf_km,
        displacement_icrf_km,
        range_km,
        unit_direction_icrf,
        right_ascension_icrf_rad,
        declination_icrf_rad,
    })
}
