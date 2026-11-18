// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
use virgostars_ephemeris_core::calendar::{Calendar, TimeScale};
use virgostars_ephemeris_core::calendar_decimal_hours::{julday_f64, DecimalHourDateTime};
use virgostars_ephemeris_core::de441::De441;
use virgostars_ephemeris_core::mvp_lunar::{lunar_points_ut1, MVP_LUNAR_POINTS};
use virgostars_ephemeris_core::mvp_planets::{planets_ut1, PlanetaryTime, MVP_PLANETS};
use virgostars_ephemeris_core::mvp_site::{house_angles_ut1, solar_event_ut1, SiteError};
use virgostars_ephemeris_core::rise_transit::{Atmosphere, Event, Site};

#[test]
#[ignore = "requires DE441_PATH with a local classic DE441 file"]
fn solar_events_use_explicit_time_and_report_polar_no_event() {
    let mut eph =
        De441::open(std::env::var_os("DE441_PATH").expect("DE441_PATH required")).unwrap();
    let date = |year, month, day| {
        julday_f64(
            DecimalHourDateTime {
                year,
                month,
                day,
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
    let rome = Site {
        latitude_deg: 41.9,
        longitude_deg_east: 12.5,
        height_m: 0.0,
    };
    let atmosphere = Atmosphere {
        pressure_hpa: 1013.25,
        temperature_c: 15.0,
    };
    let start = date(2024, 3, 20);
    let common = time(start);
    let planets = planets_ut1(&mut eph, &MVP_PLANETS, common).unwrap();
    let lunar = lunar_points_ut1(&mut eph, &MVP_LUNAR_POINTS, common).unwrap();
    let houses = house_angles_ut1(common, rome.latitude_deg, rome.longitude_deg_east).unwrap();
    assert_eq!(planets.epoch, lunar.epoch);
    assert_eq!(planets.epoch, houses.epoch);
    assert_eq!(planets.delta_t_source, lunar.delta_t_source);
    assert_eq!(planets.delta_t_source, houses.delta_t_source);
    let rise =
        solar_event_ut1(&mut eph, time(start), Event::Rise, &rome, &atmosphere, 1.0).unwrap();
    let set = solar_event_ut1(&mut eph, time(start), Event::Set, &rome, &atmosphere, 1.0).unwrap();
    assert!((0.15..0.35).contains(&(rise.jd_ut1.unwrap() - start)));
    assert!((0.60..0.80).contains(&(set.jd_ut1.unwrap() - start)));
    assert_eq!(rise.delta_t_source, "synthetic");
    let arctic = Site {
        latitude_deg: 69.65,
        longitude_deg_east: 18.96,
        height_m: 0.0,
    };
    let midnight_sun = solar_event_ut1(
        &mut eph,
        time(date(2024, 6, 21)),
        Event::Set,
        &arctic,
        &atmosphere,
        1.0,
    )
    .unwrap();
    assert!(midnight_sun.jd_ut1.is_none());
    assert!(matches!(
        solar_event_ut1(
            &mut eph,
            time(date(0, 1, 1)),
            Event::Rise,
            &rome,
            &atmosphere,
            1.0
        ),
        Err(SiteError::Solar(_))
    ));
    assert!(matches!(
        solar_event_ut1(
            &mut eph,
            time(start),
            Event::UpperTransit,
            &rome,
            &atmosphere,
            1.0
        ),
        Err(SiteError::InvalidEvent)
    ));
}
