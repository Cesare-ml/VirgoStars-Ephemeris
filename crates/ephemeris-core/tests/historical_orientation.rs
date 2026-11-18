// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
use virgostars_ephemeris_core::calendar::{Calendar, TimeScale};
use virgostars_ephemeris_core::calendar_decimal_hours::{julday_f64, DecimalHourDateTime};
use virgostars_ephemeris_core::historical_orientation::{
    experimental_gast, experimental_house_angles,
};
use virgostars_ephemeris_core::house_angles::house_angles;

#[test]
fn sampled_gast_is_near_swiss_and_modern_api_keeps_its_guard() {
    // Swiss 2.09.03, proleptic Gregorian July 1 12 UT1, ΔT in days;
    // local reference at identical UT1/TT. Eight samples, not a range guarantee.
    let cases = [
        (-12999, 8.247982728695737, 106.66879203081051),
        (-9999, 5.245191672948886, 104.20975515325219),
        (-4999, 1.7430663040793812, 100.01040943203097),
        (0, 0.12212644332679122, 99.48219535955094),
        (1000, 0.016910577532843193, 99.09592069607574),
        (2000, 0.0007403796199061666, 99.84455831214966),
        (3000, 0.03814358968827741, 99.7298808091851),
        (3999, 0.15087310111040808, 99.98095979460831),
    ];
    for (year, delta, expected) in cases {
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
        let actual = experimental_gast(jd, jd + delta).unwrap().to_degrees();
        let error = ((actual - expected + 180.0).rem_euclid(360.0) - 180.0).abs() * 3600.0;
        assert!(error < 10.0, "year {year}: {error} arcseconds");
        let angles = experimental_house_angles(jd, jd + delta, 41.9, 12.5).unwrap();
        assert!((0.0..360.0).contains(&angles.ascendant_deg));
        assert!((0.0..360.0).contains(&angles.midheaven_deg));
        if year == 0 {
            assert!(house_angles(jd, jd + delta, 41.9, 12.5).is_err());
        }
    }
    assert!(experimental_gast(f64::NAN, 2_451_545.0).is_err());
    assert!(experimental_house_angles(2_451_545.0, 2_451_545.0, 90.0, 0.0).is_err());
}

#[test]
fn sampled_house_angles_are_near_swiss() {
    // Swiss 2.09.03, 41.9 N 12.5 E, July 1 12 UT1; four sample dates.
    let cases = [
        (
            -12999,
            8.247982728695737,
            202.76022388280927,
            117.03835640192875,
        ),
        (
            0,
            0.12212644332679122,
            197.19363350228852,
            110.28604851705026,
        ),
        (
            2000,
            0.0007403796199061666,
            197.50376369742781,
            110.6626845517255,
        ),
        (
            3999,
            0.15087310111040808,
            197.63799220783926,
            110.826880925406,
        ),
    ];
    for (year, delta, asc, mc) in cases {
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
        let angles = experimental_house_angles(jd, jd + delta, 41.9, 12.5).unwrap();
        for (actual, reference) in [(angles.ascendant_deg, asc), (angles.midheaven_deg, mc)] {
            let error = ((actual - reference + 180.0).rem_euclid(360.0) - 180.0).abs() * 3600.0;
            assert!(error < 20.0, "year {year}: {error} arcseconds");
        }
    }
}

#[test]
fn sidereal_samples_include_other_seasons_and_hours() {
    let cases = [
        (-12999, 1, 15, 3.0, 8.24849306057613, 166.70423430597003),
        (-9999, 4, 10, 18.0, 5.245390484305693, 113.64798426560189),
        (-4999, 10, 9, 6.0, 1.7429263099205488, 108.33036388513933),
        (0, 1, 3, 23.0, 0.12218549307950437, 87.51741426611757),
        (2000, 4, 7, 4.0, 0.0007396510054746614, 255.7358001680444),
        (3999, 10, 12, 19.0, 0.15091564305662822, 306.79012258188334),
    ];
    for (year, month, day, hour, delta, expected) in cases {
        let jd = julday_f64(
            DecimalHourDateTime {
                year,
                month,
                day,
                decimal_hour: hour,
                scale: TimeScale::Ut1,
            },
            Calendar::ProlepticGregorian,
        )
        .unwrap();
        let actual = experimental_gast(jd, jd + delta).unwrap().to_degrees();
        let error = ((actual - expected + 180.0).rem_euclid(360.0) - 180.0).abs() * 3600.0;
        assert!(error < 10.0, "year {year}: {error} arcseconds");
    }
}
