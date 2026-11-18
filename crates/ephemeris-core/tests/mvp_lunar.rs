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
