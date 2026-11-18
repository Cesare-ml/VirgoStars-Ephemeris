// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Practical opt-in snapshot at one explicit UT1 and ΔT.
//!
//! Long-term lunar points are experimental. House angles and solar events
//! retain their modern orientation guards and report errors for older dates.
//! This module neither supplies house cusps nor converts civil time to UT1.

use crate::de441::De441;
use crate::mvp_lunar::{lunar_points_ut1, LunarSet, LunarSetError, MVP_LUNAR_POINTS};
use crate::mvp_planets::{planets_ut1, PlanetaryError, PlanetarySet, PlanetaryTime, MVP_PLANETS};
use crate::mvp_site::{
    house_angles_ut1, solar_event_ut1, ExplicitHouseAngles, SiteError, SolarEvent,
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
    Planets(PlanetaryError),
    Lunar(LunarSetError),
}
impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Planets(e) => write!(f, "planets: {e}"),
            Self::Lunar(e) => write!(f, "lunar points: {e}"),
        }
    }
}
impl Error for SnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Planets(e) => Some(e),
            Self::Lunar(e) => Some(e),
        }
    }
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
    let planets = planets_ut1(ephemeris, &MVP_PLANETS, time).map_err(SnapshotError::Planets)?;
    let lunar =
        lunar_points_ut1(ephemeris, &MVP_LUNAR_POINTS, time).map_err(SnapshotError::Lunar)?;
    let house_angles = house_angles_ut1(time, site.latitude_deg, site.longitude_deg_east);
    let sunrise = solar.map(|search| {
        solar_event_ut1(
            ephemeris,
            time,
            Event::Rise,
            site,
            &search.atmosphere,
            search.search_days,
        )
    });
    let sunset = solar.map(|search| {
        solar_event_ut1(
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
