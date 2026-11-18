// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
use virgostars_ephemeris_core::apparent_ecliptic_of_date::ApparentFrame;
use virgostars_ephemeris_core::calendar::{Calendar, TimeScale};
use virgostars_ephemeris_core::calendar_decimal_hours::{julday_f64, DecimalHourDateTime};
use virgostars_ephemeris_core::de441::De441;
use virgostars_ephemeris_core::lunar_points::lunar_point;
use virgostars_ephemeris_core::mvp_lunar::{lunar_points_ut1, LunarSetError, MVP_LUNAR_POINTS};
use virgostars_ephemeris_core::mvp_planets::PlanetaryTime;

#[test]
#[ignore = "requires DE441_PATH with a local classic DE441 file"]
fn long_term_lunar_set_uses_one_epoch_and_preserves_modern_guard() {
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
        let time = PlanetaryTime {
            jd_ut1: jd,
            delta_t_seconds: 100.0,
            uncertainty_seconds: 10.0,
            source: "synthetic",
        };
        let set = lunar_points_ut1(&mut eph, &MVP_LUNAR_POINTS, time).unwrap();
        assert_eq!(set.epoch.jd_ut1, jd);
        assert_eq!(set.delta_t_source, "synthetic");
        assert_eq!(set.positions.len(), 4);
        for (position, point) in set.positions.iter().zip(MVP_LUNAR_POINTS) {
            assert_eq!(position.point, point);
            for coordinates in [position.ecliptic, position.equatorial] {
                assert!((0.0..360.0).contains(&coordinates.longitude_deg));
                assert!(coordinates.latitude_deg.is_finite() && coordinates.distance_au > 0.0);
                assert!(coordinates.longitude_rate_deg_per_day.is_finite());
            }
            if year == 2000 {
                let modern = lunar_point(
                    Some(&mut eph),
                    point,
                    ApparentFrame::TrueEclipticOfDate,
                    set.epoch.jd_tdb.0,
                )
                .unwrap();
                let difference = (position.ecliptic.longitude_deg - modern.longitude_deg + 180.0)
                    .rem_euclid(360.0)
                    - 180.0;
                assert!(difference.abs() < 0.05, "{point:?}: {difference}");
            }
        }
        // Swiss 2.09.03 JPL, same TT; four samples, not a continuous guarantee.
        let reference = match year {
            -12999 => (
                82.68578521281252,
                84.95809235737663,
                21.04596812090592,
                6.16551291946851,
                -4.5305852366801345,
            ),
            0 => (
                278.15317428689684,
                278.18122240010126,
                261.0792235345096,
                257.3323554508416,
                -1.5144395199969698,
            ),
            2000 => (
                115.4026739148131,
                114.61709462648383,
                283.6707218074333,
                282.8482666337974,
                1.0489351082696834,
            ),
            3999 => (
                333.70278375140356,
                332.10720611684144,
                257.29312242563276,
                254.06759713837974,
                -5.00206993416884,
            ),
            _ => unreachable!(),
        };
        for (actual, expected) in [
            (set.positions[1].ecliptic.longitude_deg, reference.1),
            (set.positions[3].ecliptic.longitude_deg, reference.3),
        ] {
            let error = ((actual - expected + 180.0).rem_euclid(360.0) - 180.0).abs() * 3600.0;
            assert!(error < 1.0, "year {year}: {error} arcseconds");
        }
        for (actual, expected) in [
            (set.positions[0].ecliptic.longitude_deg, reference.0),
            (set.positions[2].ecliptic.longitude_deg, reference.2),
        ] {
            let error = ((actual - expected + 180.0).rem_euclid(360.0) - 180.0).abs() * 3600.0;
            assert!(
                error < 10.0,
                "mean longitude, year {year}: {error} arcseconds"
            );
        }
        assert!((set.positions[2].ecliptic.latitude_deg - reference.4).abs() * 3600.0 < 3.0);
        if year == 0 {
            assert!(lunar_point(
                Some(&mut eph),
                MVP_LUNAR_POINTS[0],
                ApparentFrame::TrueEclipticOfDate,
                set.epoch.jd_tdb.0
            )
            .is_err());
        }
    }
    let time = || PlanetaryTime {
        jd_ut1: 2_451_545.0,
        delta_t_seconds: 64.0,
        uncertainty_seconds: 2.0,
        source: "synthetic",
    };
    assert!(matches!(
        lunar_points_ut1(&mut eph, &[], time()),
        Err(LunarSetError::InvalidTargets)
    ));
    assert!(matches!(
        lunar_points_ut1(
            &mut eph,
            &[MVP_LUNAR_POINTS[0], MVP_LUNAR_POINTS[0]],
            time()
        ),
        Err(LunarSetError::InvalidTargets)
    ));
    assert!(matches!(
        lunar_points_ut1(
            &mut eph,
            &MVP_LUNAR_POINTS,
            PlanetaryTime {
                source: "",
                ..time()
            }
        ),
        Err(LunarSetError::InvalidSource)
    ));
}
