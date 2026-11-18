// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
use virgostars_ephemeris_core::calendar::{Calendar, TimeScale};
use virgostars_ephemeris_core::calendar_decimal_hours::{julday_f64, DecimalHourDateTime};
use virgostars_ephemeris_core::de441::De441;
use virgostars_ephemeris_core::mvp_planets::PlanetaryTime;
use virgostars_ephemeris_core::mvp_site::experimental_solar_event_ut1;
use virgostars_ephemeris_core::rise_transit::{Atmosphere, Event, Site};

#[test]
#[ignore = "requires DE441_PATH with a local classic DE441 file"]
fn sampled_sunrise_sunset_are_near_swiss() {
    // Swiss 2.09.03 JPL DE441, Rome, upper limb, refraction, 1013.25 hPa, 15 C.
    // ΔT is Swiss's value at start, held constant here. Four dates only.
    let cases = [
        (
            -12999,
            8.248298301738588,
            -3026649.29554565,
            -3026648.781339076,
        ),
        (
            0,
            0.12216048192544914,
            1721138.7179789168,
            1721139.2236109653,
        ),
        (
            2000,
            0.0007394892239141244,
            2451623.717477461,
            2451624.223835616,
        ),
        (
            3999,
            0.15083047939433186,
            3181742.716268985,
            3181743.2216380066,
        ),
    ];
    let mut eph =
        De441::open(std::env::var_os("DE441_PATH").expect("DE441_PATH required")).unwrap();
    let site = Site {
        latitude_deg: 41.9,
        longitude_deg_east: 12.5,
        height_m: 0.0,
    };
    let atmosphere = Atmosphere {
        pressure_hpa: 1013.25,
        temperature_c: 15.0,
    };
    for (year, delta_days, rise_ref, set_ref) in cases {
        let jd = julday_f64(
            DecimalHourDateTime {
                year,
                month: 3,
                day: 20,
                decimal_hour: 0.0,
                scale: TimeScale::Ut1,
            },
            Calendar::ProlepticGregorian,
        )
        .unwrap();
        let time = PlanetaryTime {
            jd_ut1: jd,
            delta_t_seconds: delta_days * 86400.0,
            uncertainty_seconds: 0.0,
            source: "Swiss comparison",
        };
        for (event, reference) in [(Event::Rise, rise_ref), (Event::Set, set_ref)] {
            let result =
                experimental_solar_event_ut1(&mut eph, time, event, &site, &atmosphere, 1.0)
                    .unwrap();
            let error = (result.jd_ut1.unwrap() - reference).abs() * 86400.0;
            eprintln!("{year} {event:?}: {error:.3} seconds");
            assert!(error < 120.0, "{year} {event:?}: {error} seconds");
        }
    }
}
