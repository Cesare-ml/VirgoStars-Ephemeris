// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
use std::io::Cursor;
use virgostars_ephemeris_core::apparent_ecliptic_of_date::ApparentFrame;
use virgostars_ephemeris_core::calendar::{Calendar, TimeScale};
use virgostars_ephemeris_core::calendar_decimal_hours::{julday_f64, DecimalHourDateTime};
use virgostars_ephemeris_core::lunar_points::{experimental_long_term_lunar_point, LunarPoint};

#[test]
fn mean_lunar_fit_covers_target_boundaries_and_rejects_far_extrapolation() {
    let date = |year, month, day, hour| {
        julday_f64(
            DecimalHourDateTime {
                year,
                month,
                day,
                decimal_hour: hour,
                scale: TimeScale::Ut1,
            },
            Calendar::ProlepticGregorian,
        )
        .unwrap()
    };
    for jd in [date(-12999, 1, 1, 0.0), date(3999, 12, 31, 23.0)] {
        for point in [LunarPoint::MeanNode, LunarPoint::MeanApogee] {
            let result = experimental_long_term_lunar_point::<Cursor<Vec<u8>>>(
                None,
                point,
                ApparentFrame::TrueEclipticOfDate,
                jd,
            )
            .unwrap();
            assert!((0.0..360.0).contains(&result.longitude_deg));
            assert!(
                result.latitude_deg.is_finite() && result.longitude_rate_deg_per_day.is_finite()
            );
        }
    }
    assert!(experimental_long_term_lunar_point::<Cursor<Vec<u8>>>(
        None,
        LunarPoint::MeanNode,
        ApparentFrame::TrueEclipticOfDate,
        date(5000, 1, 1, 0.0)
    )
    .is_err());
}
