// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification (2026-11-18): connect an explicit historical local clock to the snapshot.
//! Practical opt-in snapshot at one explicit UT1 and ΔT.
//!
//! Long-term lunar points are experimental. The default snapshot retains the
//! modern house and solar guards; the named experimental snapshot opts into
//! long-term orientation for those calculations.
//! House cusps remain in astrological-functions. An explicit historical local
//! clock may be converted to UT1 without inferring a time zone or UTC.

use crate::de441::De441;
use crate::historical_civil_time::{
    local_to_ut1, CivilTimeError, HistoricalCivilEpoch, HistoricalCivilInput,
};
use crate::mvp_lunar::{lunar_points_ut1, LunarSet, LunarSetError, MVP_LUNAR_POINTS};
use crate::mvp_planets::{planets_ut1, PlanetaryError, PlanetarySet, PlanetaryTime, MVP_PLANETS};
use crate::mvp_site::{
    experimental_house_angles_ut1, experimental_solar_event_ut1, house_angles_ut1, solar_event_ut1,
    ExplicitHouseAngles, SiteError, SolarEvent,
};
use crate::rise_transit::{Atmosphere, Event, Site};
use std::{
    error::Error,
    fmt,
    io::{Read, Seek},
};

/// Request sunrise and sunset after the snapshot time, using the given atmosphere.
#[derive(Clone, Copy, Debug)]
pub struct SolarSearch {
    pub atmosphere: Atmosphere,
    pub search_days: f64,
}

/// Planet and lunar failure stops the snapshot. Site-dependent results remain
/// separate so their absence or unsupported epoch is never mistaken for data.
#[derive(Debug)]
pub struct MvpSnapshot {
    pub planets: PlanetarySet,
    pub lunar: LunarSet,
    pub house_angles: Result<ExplicitHouseAngles, SiteError>,
    /// `None` means not requested; `Some(Ok(...))` may contain no event.
    pub sunrise: Option<Result<SolarEvent, SiteError>>,
    pub sunset: Option<Result<SolarEvent, SiteError>>,
}

#[derive(Debug)]
pub enum SnapshotError {
    Civil(CivilTimeError),
    Planets(PlanetaryError),
    Lunar(LunarSetError),
}
impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Civil(e) => write!(f, "civil time: {e}"),
            Self::Planets(e) => write!(f, "planets: {e}"),
            Self::Lunar(e) => write!(f, "lunar points: {e}"),
        }
    }
}
impl Error for SnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Civil(e) => Some(e),
            Self::Planets(e) => Some(e),
            Self::Lunar(e) => Some(e),
        }
    }
}

/// A snapshot together with the caller-selected local-clock conversion.
#[derive(Debug)]
pub struct HistoricalCivilSnapshot {
    pub civil: HistoricalCivilEpoch,
    pub snapshot: MvpSnapshot,
}

/// Convert a local clock using the declared convention, then compute the same
/// experimental snapshot. Delta T and its separate uncertainty remain explicit.
pub fn experimental_snapshot_from_civil<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    civil_input: HistoricalCivilInput<'_>,
    delta_t_seconds: f64,
    delta_t_uncertainty_seconds: f64,
    delta_t_source: &str,
    site: &Site,
    solar: Option<SolarSearch>,
) -> Result<HistoricalCivilSnapshot, SnapshotError> {
    let civil = local_to_ut1(civil_input).map_err(SnapshotError::Civil)?;
    let time = PlanetaryTime {
        jd_ut1: civil.jd_ut1,
        delta_t_seconds,
        uncertainty_seconds: delta_t_uncertainty_seconds,
        source: delta_t_source,
    };
    let snapshot = experimental_snapshot_ut1(ephemeris, time, site, solar)?;
    Ok(HistoricalCivilSnapshot { civil, snapshot })
}

/// Calculate supported planets, lunar points and house angles for a site.
/// Optionally search for sunrise and sunset after the same UT1 instant.
/// The caller must inspect every site-dependent result: missing events and
/// unsupported epochs are not replaced. Input is UT1, not UTC or local time.
pub fn snapshot_ut1<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    time: PlanetaryTime<'_>,
    site: &Site,
    solar: Option<SolarSearch>,
) -> Result<MvpSnapshot, SnapshotError> {
    snapshot_with_orientation(ephemeris, time, site, solar, false)
}

/// Opt into experimental long-term lunar, house and solar orientation.
/// No physical accuracy or civil-time guarantee is implied by date coverage.
pub fn experimental_snapshot_ut1<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    time: PlanetaryTime<'_>,
    site: &Site,
    solar: Option<SolarSearch>,
) -> Result<MvpSnapshot, SnapshotError> {
    snapshot_with_orientation(ephemeris, time, site, solar, true)
}

fn snapshot_with_orientation<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    time: PlanetaryTime<'_>,
    site: &Site,
    solar: Option<SolarSearch>,
    historical: bool,
) -> Result<MvpSnapshot, SnapshotError> {
    let planets = planets_ut1(ephemeris, &MVP_PLANETS, time).map_err(SnapshotError::Planets)?;
    let lunar =
        lunar_points_ut1(ephemeris, &MVP_LUNAR_POINTS, time).map_err(SnapshotError::Lunar)?;
    let house_angles = if historical {
        experimental_house_angles_ut1(time, site.latitude_deg, site.longitude_deg_east)
    } else {
        house_angles_ut1(time, site.latitude_deg, site.longitude_deg_east)
    };
    let sunrise = solar.map(|search| {
        let calculate = if historical {
            experimental_solar_event_ut1
        } else {
            solar_event_ut1
        };
        calculate(
            ephemeris,
            time,
            Event::Rise,
            site,
            &search.atmosphere,
            search.search_days,
        )
    });
    let sunset = solar.map(|search| {
        let calculate = if historical {
            experimental_solar_event_ut1
        } else {
            solar_event_ut1
        };
        calculate(
            ephemeris,
            time,
            Event::Set,
            site,
            &search.atmosphere,
            search.search_days,
        )
    });
    Ok(MvpSnapshot {
        planets,
        lunar,
        house_angles,
        sunrise,
        sunset,
    })
}
