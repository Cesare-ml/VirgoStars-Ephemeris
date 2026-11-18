// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification (2026-11-18): check historical and modern equatorial outputs.
use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    geocentric_apparent_six_fields_many_with_options, ApparentFrame, ApparentOptions,
};
use virgostars_ephemeris_core::calendar::{Calendar, TimeScale};
use virgostars_ephemeris_core::calendar_decimal_hours::{julday_f64, DecimalHourDateTime};
use virgostars_ephemeris_core::de441::{Body, De441};
use virgostars_ephemeris_core::historical_rate_candidate::historical_three_rate_candidate;
use virgostars_ephemeris_core::mvp_planets::{
    planets_ut1, PlanetaryError, PlanetaryTime, MVP_PLANETS,
};

#[test]
#[ignore = "requires DE441_PATH with a local classic DE441 file"]
fn full_set_reuses_existing_long_range_results_and_rejects_bad_input() {
    let path = std::env::var_os("DE441_PATH").expect("DE441_PATH required");
    let mut eph = De441::open(path).expect("classic DE441 required");
    for year in [-12999, 0, 2000, 3999] {
        let jd = julday_f64(
            DecimalHourDateTime {
                year,
                month: 7,
                day: 1,
                decimal_hour: 12.0,
                scale: TimeScale::Ut1,
            },
            Calendar::ProlepticGregorian,
        )
        .unwrap();
        // Deliberately fictional: this checks the API wiring, not historical ΔT.
        let time = PlanetaryTime {
            jd_ut1: jd,
            delta_t_seconds: 100.0,
            uncertainty_seconds: 10.0,
            source: "synthetic",
        };
        let set = planets_ut1(&mut eph, &MVP_PLANETS, time).unwrap();
        assert_eq!(set.positions.len(), 10);
        assert_eq!(set.epoch.jd_ut1, jd);
        assert_eq!(set.delta_t_source, "synthetic");
        for (position, target) in set.positions.iter().zip(MVP_PLANETS) {
            assert_eq!(position.body, target);
            assert!(position.rates.base.longitude_deg.is_finite());
            assert!((0.0..360.0).contains(&position.equatorial.right_ascension_deg));
            assert!((-90.0..=90.0).contains(&position.equatorial.declination_deg));
        }
        let expected =
            historical_three_rate_candidate(&mut eph, Body::Moon, jd, 100.0, 10.0, "synthetic")
                .unwrap();
        assert_eq!(set.positions[1].rates.base, expected.rates.base);
        assert_eq!(
            set.positions[1].rates.longitude_deg_per_tdb_day,
            expected.rates.longitude_deg_per_tdb_day
        );
    }
    let model = PlanetaryTime::espenak_meeus_2006(2_451_545.0, 2.0).unwrap();
    let modeled = planets_ut1(&mut eph, &[Body::Sun, Body::Moon], model).unwrap();
    assert_eq!(modeled.delta_t_source, "Espenak-Meeus 2006");
    assert_eq!(modeled.positions.len(), 2);
    // A modern-date sanity check against the separate existing equatorial path.
    // This is a same-engine comparison, not an independent accuracy validation.
    let eq = geocentric_apparent_six_fields_many_with_options(
        &mut eph,
        &[Body::Sun, Body::Moon],
        modeled.epoch.jd_tdb,
        ApparentOptions {
            frame: ApparentFrame::TrueEquatorOfDate,
            ..ApparentOptions::default()
        },
    )
    .unwrap();
    for (position, reference) in modeled.positions.iter().zip(eq) {
        let ra_diff = (position.equatorial.right_ascension_deg - reference.fields.longitude_deg
            + 180.0)
            .rem_euclid(360.0)
            - 180.0;
        assert!(ra_diff.abs() < 0.01);
        assert!((position.equatorial.declination_deg - reference.fields.latitude_deg).abs() < 0.01);
    }
    let time = || PlanetaryTime {
        jd_ut1: 2_451_545.0,
        delta_t_seconds: 64.0,
        uncertainty_seconds: 2.0,
        source: "synthetic",
    };
    assert!(matches!(
        planets_ut1(&mut eph, &[Body::Sun, Body::Sun], time()),
        Err(PlanetaryError::InvalidTargets)
    ));
    assert!(matches!(
        planets_ut1(
            &mut eph,
            &[Body::Sun],
            PlanetaryTime {
                source: " ",
                ..time()
            }
        ),
        Err(PlanetaryError::InvalidSource)
    ));
    assert!(matches!(
        planets_ut1(
            &mut eph,
            &[Body::Sun],
            PlanetaryTime {
                uncertainty_seconds: -1.0,
                ..time()
            }
        ),
        Err(PlanetaryError::Time(_))
    ));
}
