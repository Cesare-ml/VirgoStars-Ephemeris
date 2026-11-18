// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification (2026-11-18): exercise the explicit historical local clock path.
use virgostars_ephemeris_core::calendar::CalendarDateTime;
use virgostars_ephemeris_core::calendar::{Calendar, TimeScale};
use virgostars_ephemeris_core::calendar_decimal_hours::{julday_f64, DecimalHourDateTime};
use virgostars_ephemeris_core::de441::De441;
use virgostars_ephemeris_core::historical_civil_time::{HistoricalCivilInput, HistoricalOffset};
use virgostars_ephemeris_core::mvp_planets::PlanetaryTime;
use virgostars_ephemeris_core::mvp_snapshot::{
    experimental_snapshot_from_civil, experimental_snapshot_ut1, snapshot_ut1, SolarSearch,
};
use virgostars_ephemeris_core::rise_transit::{Atmosphere, Site};

#[test]
#[ignore = "requires DE441_PATH with a local classic DE441 file"]
fn snapshot_reports_results_and_unsupported_site_data_explicitly() {
    let mut eph =
        De441::open(std::env::var_os("DE441_PATH").expect("DE441_PATH required")).unwrap();
    let jd = |year| {
        julday_f64(
            DecimalHourDateTime {
                year,
                month: 3,
                day: 20,
                decimal_hour: 0.0,
                scale: TimeScale::Ut1,
            },
            Calendar::ProlepticGregorian,
        )
        .unwrap()
    };
    let time = |jd_ut1| PlanetaryTime {
        jd_ut1,
        delta_t_seconds: 69.0,
        uncertainty_seconds: 2.0,
        source: "synthetic",
    };
    let site = Site {
        latitude_deg: 41.9,
        longitude_deg_east: 12.5,
        height_m: 0.0,
    };
    let solar = SolarSearch {
        atmosphere: Atmosphere {
            pressure_hpa: 1013.25,
            temperature_c: 15.0,
        },
        search_days: 1.0,
    };
    let modern = snapshot_ut1(&mut eph, time(jd(2024)), &site, Some(solar)).unwrap();
    assert_eq!(modern.planets.positions.len(), 10);
    assert_eq!(modern.lunar.positions.len(), 4);
    assert_eq!(modern.planets.epoch, modern.lunar.epoch);
    assert_eq!(
        modern.house_angles.as_ref().unwrap().epoch,
        modern.planets.epoch
    );
    assert!(modern.sunrise.unwrap().unwrap().jd_ut1.is_some());
    assert!(modern.sunset.unwrap().unwrap().jd_ut1.is_some());
    let ancient = snapshot_ut1(&mut eph, time(jd(-12999)), &site, Some(solar)).unwrap();
    assert_eq!(ancient.planets.positions.len(), 10);
    assert_eq!(ancient.lunar.positions.len(), 4);
    assert!(ancient.house_angles.is_err());
    assert!(ancient.sunrise.unwrap().is_err());
    assert!(ancient.sunset.unwrap().is_err());
    for year in [-12999, 0, 2000, 3999] {
        let snapshot =
            experimental_snapshot_ut1(&mut eph, time(jd(year)), &site, Some(solar)).unwrap();
        assert_eq!(snapshot.planets.positions.len(), 10);
        assert_eq!(snapshot.lunar.positions.len(), 4);
        assert!(snapshot.house_angles.is_ok(), "house angles: {year}");
        assert!(
            snapshot.sunrise.unwrap().unwrap().jd_ut1.is_some(),
            "sunrise: {year}"
        );
        assert!(
            snapshot.sunset.unwrap().unwrap().jd_ut1.is_some(),
            "sunset: {year}"
        );
        let civil = experimental_snapshot_from_civil(
            &mut eph,
            HistoricalCivilInput {
                local: CalendarDateTime {
                    year,
                    month: 3,
                    day: 20,
                    hour: 1,
                    minute: 0,
                    second: 0.0,
                    scale: TimeScale::Ut1,
                },
                calendar: Calendar::ProlepticGregorian,
                offset: HistoricalOffset::LocalMeanSolar {
                    longitude_deg_east: 15.0,
                },
                uncertainty_seconds: 60.0,
            },
            69.0,
            2.0,
            "synthetic",
            &site,
            None,
        )
        .unwrap();
        assert_eq!(civil.civil.jd_ut1, jd(year));
        assert_eq!(civil.snapshot.planets.epoch, snapshot.planets.epoch);
        assert_eq!(
            civil.snapshot.planets.positions[0].rates.base.longitude_deg,
            snapshot.planets.positions[0].rates.base.longitude_deg
        );
    }
    let without_solar = snapshot_ut1(&mut eph, time(jd(2000)), &site, None).unwrap();
    assert!(without_solar.sunrise.is_none() && without_solar.sunset.is_none());
}
