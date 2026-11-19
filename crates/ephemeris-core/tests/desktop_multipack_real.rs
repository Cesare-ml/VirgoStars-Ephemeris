/*
EXHIBIT A. Common Public Attribution License Version 1.0.

The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

// Modification (2026-11-19): same-source Desktop multipack boundary validation.

use std::path::PathBuf;
use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    experimental_geocentric_apparent_with_provider, experimental_long_term_apparent_ecliptic,
    experimental_long_term_apparent_with_provider, experimental_small_body_apparent_with_provider,
    geocentric_apparent_six_fields_many_with_options, small_body_apparent_six_fields,
    ApparentCenter, ApparentError, ApparentFrame, ApparentOptions,
    ExperimentalProviderApparentError,
};
use virgostars_ephemeris_core::calendar::{Calendar, CalendarDateTime, TimeScale};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::de441_pack_manager::De441PackError;
use virgostars_ephemeris_core::de441_pack_manager::{De441PackManager, De441PackSpec};
use virgostars_ephemeris_core::earth_orientation::OrientationError;
use virgostars_ephemeris_core::fixed_star::{
    calculate_star_with_provider, calculate_star_with_solar_deflection, FixedStarCatalogEntry,
    FixedStarError, StellarSolarDeflection,
};
use virgostars_ephemeris_core::historical_civil_time::{HistoricalCivilInput, HistoricalOffset};
use virgostars_ephemeris_core::lunar_points::{
    experimental_long_term_lunar_point, experimental_long_term_lunar_point_with_provider,
    LunarPoint, LunarPointError,
};
use virgostars_ephemeris_core::mvp_lunar::{
    lunar_points_ut1, lunar_points_ut1_with_provider, MVP_LUNAR_POINTS,
};
use virgostars_ephemeris_core::mvp_planets::{
    planets_ut1, planets_ut1_with_provider, PlanetaryTime, MVP_PLANETS,
};
use virgostars_ephemeris_core::mvp_snapshot::{
    experimental_snapshot_from_civil, experimental_snapshot_from_civil_with_provider,
};
use virgostars_ephemeris_core::rise_transit::Site;
use virgostars_ephemeris_core::six_field_units::FixedSixFieldsDegreesAu;
use virgostars_ephemeris_core::spk_type21::SmallBodySpk;

fn fields(f: FixedSixFieldsDegreesAu) -> [f64; 6] {
    [
        f.longitude_deg,
        f.latitude_deg,
        f.distance_au,
        f.longitude_speed_deg_per_tdb_day,
        f.latitude_speed_deg_per_tdb_day,
        f.distance_speed_au_per_tdb_day,
    ]
}

#[test]
#[ignore = "requires locally verified classic DE441 and three private Desktop slices"]
fn apparent_provider_matches_source_at_desktop_boundaries() {
    let source = PathBuf::from(std::env::var_os("DE441_PATH").expect("DE441_PATH"));
    let directory = PathBuf::from(
        std::env::var_os("DE441_DESKTOP_TEST_PACKS").expect("DE441_DESKTOP_TEST_PACKS"),
    );
    let specs = [
        (
            "history",
            "de441-v2-history-00800-00999.eph",
            2013253.5,
            2086302.5,
            false,
        ),
        (
            "base",
            "de441-v2-desktop-core-1000-2200.eph",
            2086302.5,
            2524958.5,
            true,
        ),
        (
            "future",
            "de441-v2-future-02201-02400.eph",
            2524958.5,
            2598007.5,
            false,
        ),
    ]
    .into_iter()
    .map(|(id, file, start, end, required)| {
        De441PackSpec::new(
            id,
            directory.join(file),
            JulianDateTdb(start),
            JulianDateTdb(end),
            required,
        )
    })
    .collect();
    let mut manager = De441PackManager::from_specs(specs).unwrap();
    let mut original = De441::open(source).unwrap();
    let bodies = [
        Body::Sun,
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::MarsBarycenter,
        Body::JupiterBarycenter,
        Body::SaturnBarycenter,
        Body::UranusBarycenter,
        Body::NeptuneBarycenter,
        Body::PlutoBarycenter,
    ];
    let limits = [1e-8, 1e-8, 1e-12, 1e-8, 1e-8, 1e-12];
    let mut supported = 0;
    let mut unsupported = 0;
    for boundary in [2086302.5, 2451545.0, 2524958.5] {
        for offset in [-0.08, -0.02, -1.0 / 86400.0, 0.0, 1.0 / 86400.0, 0.02, 0.08] {
            let epoch = JulianDateTdb(boundary + offset);
            for body in bodies {
                let actual = fields(
                    experimental_long_term_apparent_with_provider(&mut manager, body, epoch)
                        .unwrap(),
                );
                let expected = fields(
                    experimental_long_term_apparent_ecliptic(&mut original, body, epoch).unwrap(),
                );
                for i in 0..6 {
                    let diff = if i == 0 {
                        ((actual[i] - expected[i] + 180.0).rem_euclid(360.0) - 180.0).abs()
                    } else {
                        (actual[i] - expected[i]).abs()
                    };
                    assert!(
                        diff <= limits[i],
                        "boundary={boundary} offset={offset} body={body:?} field={i} diff={diff}"
                    );
                }
                for frame in [
                    ApparentFrame::TrueEclipticOfDate,
                    ApparentFrame::TrueEquatorOfDate,
                ] {
                    let result = experimental_geocentric_apparent_with_provider(
                        &mut manager,
                        body,
                        epoch,
                        frame,
                    );
                    let standard = geocentric_apparent_six_fields_many_with_options(
                        &mut original,
                        &[body],
                        epoch,
                        ApparentOptions {
                            frame,
                            ..ApparentOptions::default()
                        },
                    );
                    let (result, standard) = match (result, standard) {
                        (Ok(result), Ok(standard)) => {
                            supported += 1;
                            (result, standard)
                        }
                        (
                            Err(ExperimentalProviderApparentError::Apparent(
                                ApparentError::Orientation(OrientationError::UnsupportedEpoch {
                                    ..
                                }),
                            )),
                            Err(ApparentError::Orientation(OrientationError::UnsupportedEpoch {
                                ..
                            })),
                        ) => {
                            unsupported += 1;
                            continue;
                        }
                        other => panic!("unexpected provider/source results: {other:?}"),
                    };
                    let actual = fields(result.fields);
                    let expected = fields(standard[0].fields);
                    for i in 0..6 {
                        let diff = if i == 0 {
                            ((actual[i] - expected[i] + 180.0).rem_euclid(360.0) - 180.0).abs()
                        } else {
                            (actual[i] - expected[i]).abs()
                        };
                        assert!(diff <= limits[i], "frame={frame:?} boundary={boundary} offset={offset} body={body:?} field={i} diff={diff}");
                    }
                }
            }
        }
        let time = PlanetaryTime {
            jd_ut1: boundary,
            delta_t_seconds: 69.0,
            uncertainty_seconds: 2.0,
            source: "synthetic test",
        };
        let actual = lunar_points_ut1_with_provider(&mut manager, &MVP_LUNAR_POINTS, time).unwrap();
        let expected = lunar_points_ut1(&mut original, &MVP_LUNAR_POINTS, time).unwrap();
        for (a, b) in actual.positions.iter().zip(&expected.positions) {
            assert_eq!(a.ecliptic, b.ecliptic);
            assert_eq!(a.equatorial, b.equatorial);
        }
        let actual = planets_ut1_with_provider(&mut manager, &MVP_PLANETS, time).unwrap();
        let expected = planets_ut1(&mut original, &MVP_PLANETS, time).unwrap();
        for (a, b) in actual.positions.iter().zip(&expected.positions) {
            assert_eq!(a.body, b.body);
            assert_eq!(a.rates.base, b.rates.base);
            assert_eq!(
                a.rates.longitude_deg_per_tdb_day,
                b.rates.longitude_deg_per_tdb_day
            );
            assert_eq!(
                a.rates.latitude_deg_per_tdb_day,
                b.rates.latitude_deg_per_tdb_day
            );
            assert_eq!(a.rates.radial_au_per_tdb_day, b.rates.radial_au_per_tdb_day);
            assert_eq!(a.equatorial, b.equatorial);
        }
    }
    assert!(supported > 0 && unsupported > 0);
}

#[test]
#[ignore = "requires locally verified classic DE441 and three private Desktop slices"]
fn lunar_points_use_the_correct_pack_for_every_sample() {
    let source = PathBuf::from(std::env::var_os("DE441_PATH").expect("DE441_PATH"));
    let directory =
        PathBuf::from(std::env::var_os("DE441_DESKTOP_TEST_PACKS").expect("pack directory"));
    let base = directory.join("de441-v2-desktop-core-1000-2200.eph");
    let specs = vec![
        De441PackSpec::new(
            "history",
            directory.join("de441-v2-history-00800-00999.eph"),
            JulianDateTdb(2013253.5),
            JulianDateTdb(2086302.5),
            false,
        ),
        De441PackSpec::new(
            "base",
            &base,
            JulianDateTdb(2086302.5),
            JulianDateTdb(2524958.5),
            true,
        ),
        De441PackSpec::new(
            "future",
            directory.join("de441-v2-future-02201-02400.eph"),
            JulianDateTdb(2524958.5),
            JulianDateTdb(2598007.5),
            false,
        ),
    ];
    let mut manager = De441PackManager::from_specs(specs).unwrap();
    let mut original = De441::open(source).unwrap();
    let points = [
        LunarPoint::MeanNode,
        LunarPoint::TrueNode,
        LunarPoint::MeanApogee,
        LunarPoint::OsculatingApogee,
    ];
    for boundary in [2086302.5, 2451545.0, 2524958.5] {
        for offset in [-0.08, -0.02, -1.0 / 86400.0, 0.0, 1.0 / 86400.0, 0.02, 0.08] {
            for point in points {
                for frame in [
                    ApparentFrame::TrueEclipticOfDate,
                    ApparentFrame::TrueEquatorOfDate,
                ] {
                    let epoch = boundary + offset;
                    let actual = experimental_long_term_lunar_point_with_provider(
                        &mut manager,
                        point,
                        frame,
                        epoch,
                    )
                    .unwrap();
                    let expected = experimental_long_term_lunar_point(
                        Some(&mut original),
                        point,
                        frame,
                        epoch,
                    )
                    .unwrap();
                    assert_eq!(
                        actual, expected,
                        "epoch={epoch} point={point:?} frame={frame:?}"
                    );
                }
            }
        }
    }
    let mut incomplete = De441PackManager::from_specs(vec![
        De441PackSpec::new(
            "missing-history",
            directory.join("deliberately-absent.eph"),
            JulianDateTdb(2013253.5),
            JulianDateTdb(2086302.5),
            false,
        ),
        De441PackSpec::new(
            "base",
            base,
            JulianDateTdb(2086302.5),
            JulianDateTdb(2524958.5),
            true,
        ),
    ])
    .unwrap();
    assert!(matches!(
        experimental_long_term_lunar_point_with_provider(&mut incomplete, LunarPoint::TrueNode, ApparentFrame::TrueEclipticOfDate, 2086302.505),
        Err(LunarPointError::Pack(De441PackError::MissingPack { id, .. })) if id == "missing-history"
    ));
}

#[test]
#[ignore = "requires the local DE441 source and two private modern slices"]
fn stars_match_source_across_modern_pack_boundary() {
    let source = std::env::var_os("DE441_PATH").expect("DE441_PATH");
    let directory =
        PathBuf::from(std::env::var_os("DE441_MODERN_TEST_PACKS").expect("pack directory"));
    let before = directory.join("before.eph");
    let after = directory.join("after.eph");
    let specs = vec![
        De441PackSpec::new(
            "before",
            &before,
            JulianDateTdb(2378496.5),
            JulianDateTdb(2451544.5),
            false,
        ),
        De441PackSpec::new(
            "after",
            &after,
            JulianDateTdb(2451544.5),
            JulianDateTdb(2524593.5),
            true,
        ),
    ];
    let mut manager = De441PackManager::from_specs(specs).unwrap();
    let mut original = De441::open(source).unwrap();
    let star = FixedStarCatalogEntry {
        epoch_tdb: JulianDateTdb(2451545.0),
        right_ascension_deg: 90.0,
        declination_deg: 20.0,
        proper_motion_ra_cos_dec_mas_per_year: 10.0,
        proper_motion_dec_mas_per_year: -20.0,
        parallax_mas: 10.0,
        radial_velocity_km_per_second: 5.0,
    };
    let boundary = 2451544.5;
    for offset in [-0.02, -0.01, -1.0 / 86400.0, 0.0, 1.0 / 86400.0, 0.01, 0.02] {
        let epoch = JulianDateTdb(boundary + offset);
        let tt =
            virgostars_ephemeris_core::apparent_ecliptic_of_date::tdb_to_tt_jd(epoch.0).unwrap();
        for correction in [
            StellarSolarDeflection::Omit,
            StellarSolarDeflection::PointMass,
        ] {
            let actual =
                calculate_star_with_provider(&mut manager, star, tt, epoch, correction).unwrap();
            let expected =
                calculate_star_with_solar_deflection(&mut original, star, tt, epoch, correction)
                    .unwrap();
            assert_eq!(
                actual, expected,
                "epoch={} correction={correction:?}",
                epoch.0
            );
        }
    }
    let mut incomplete = De441PackManager::from_specs(vec![
        De441PackSpec::new(
            "missing-before",
            directory.join("absent.eph"),
            JulianDateTdb(2378496.5),
            JulianDateTdb(boundary),
            false,
        ),
        De441PackSpec::new(
            "after",
            after,
            JulianDateTdb(boundary),
            JulianDateTdb(2524593.5),
            true,
        ),
    ])
    .unwrap();
    let epoch = JulianDateTdb(boundary + 0.005);
    let tt = virgostars_ephemeris_core::apparent_ecliptic_of_date::tdb_to_tt_jd(epoch.0).unwrap();
    assert!(
        matches!(calculate_star_with_provider(&mut incomplete, star, tt, epoch, StellarSolarDeflection::PointMass),
        Err(FixedStarError::Pack(De441PackError::MissingPack { id, .. })) if id == "missing-before")
    );
}

#[test]
#[ignore = "requires DE441, two modern slices, and a separately held Chiron SPK"]
fn small_body_matches_source_across_modern_pack_boundary() {
    let source = std::env::var_os("DE441_PATH").expect("DE441_PATH");
    let directory =
        PathBuf::from(std::env::var_os("DE441_MODERN_TEST_PACKS").expect("pack directory"));
    let spk_path = std::env::var_os("DE441_TEST_CHIRON_SPK").expect("Chiron SPK path");
    let spk = SmallBodySpk::from_bytes(std::fs::read(spk_path).unwrap()).unwrap();
    let boundary = 2451544.5;
    let after = directory.join("after.eph");
    let mut manager = De441PackManager::from_specs(vec![
        De441PackSpec::new(
            "before",
            directory.join("before.eph"),
            JulianDateTdb(2378496.5),
            JulianDateTdb(boundary),
            false,
        ),
        De441PackSpec::new(
            "after",
            &after,
            JulianDateTdb(boundary),
            JulianDateTdb(2524593.5),
            true,
        ),
    ])
    .unwrap();
    let mut original = De441::open(source).unwrap();
    for offset in [-0.08, -0.02, -1.0 / 86400.0, 0.0, 1.0 / 86400.0, 0.02, 0.08] {
        let epoch = JulianDateTdb(boundary + offset);
        for center in [ApparentCenter::Geocentric, ApparentCenter::Heliocentric] {
            for frame in [
                ApparentFrame::TrueEclipticOfDate,
                ApparentFrame::TrueEquatorOfDate,
            ] {
                let options = ApparentOptions {
                    center,
                    frame,
                    ..ApparentOptions::default()
                };
                let actual = experimental_small_body_apparent_with_provider(
                    &mut manager,
                    &spk,
                    epoch,
                    options,
                )
                .unwrap();
                let expected =
                    small_body_apparent_six_fields(&mut original, &spk, epoch, options).unwrap();
                assert_eq!(
                    actual, expected,
                    "epoch={} center={center:?} frame={frame:?}",
                    epoch.0
                );
            }
        }
    }
    let mut incomplete = De441PackManager::from_specs(vec![
        De441PackSpec::new(
            "missing-before",
            directory.join("absent.eph"),
            JulianDateTdb(2378496.5),
            JulianDateTdb(boundary),
            false,
        ),
        De441PackSpec::new(
            "after",
            after,
            JulianDateTdb(boundary),
            JulianDateTdb(2524593.5),
            true,
        ),
    ])
    .unwrap();
    assert!(matches!(experimental_small_body_apparent_with_provider(
        &mut incomplete, &spk, JulianDateTdb(boundary + 0.005), ApparentOptions::default()),
        Err(ExperimentalProviderApparentError::Provider(De441PackError::MissingPack { id, .. })) if id == "missing-before"));
}

#[test]
#[ignore = "requires locally verified classic DE441 and three Desktop slices"]
fn historical_snapshot_matches_source_near_boundaries() {
    let source = std::env::var_os("DE441_PATH").expect("DE441_PATH");
    let directory =
        PathBuf::from(std::env::var_os("DE441_DESKTOP_TEST_PACKS").expect("pack directory"));
    let mut manager = De441PackManager::from_specs(vec![
        De441PackSpec::new(
            "history",
            directory.join("de441-v2-history-00800-00999.eph"),
            JulianDateTdb(2013253.5),
            JulianDateTdb(2086302.5),
            false,
        ),
        De441PackSpec::new(
            "base",
            directory.join("de441-v2-desktop-core-1000-2200.eph"),
            JulianDateTdb(2086302.5),
            JulianDateTdb(2524958.5),
            true,
        ),
        De441PackSpec::new(
            "future",
            directory.join("de441-v2-future-02201-02400.eph"),
            JulianDateTdb(2524958.5),
            JulianDateTdb(2598007.5),
            false,
        ),
    ])
    .unwrap();
    let mut original = De441::open(source).unwrap();
    let site = Site {
        latitude_deg: 41.9,
        longitude_deg_east: 12.5,
        height_m: 0.0,
    };
    for year in [1000, 2000, 2201] {
        let make_civil = || HistoricalCivilInput {
            local: CalendarDateTime {
                year,
                month: 1,
                day: 1,
                hour: 12,
                minute: 0,
                second: 0.0,
                scale: TimeScale::Ut1,
            },
            calendar: Calendar::ProlepticGregorian,
            offset: HistoricalOffset::LocalMeanSolar {
                longitude_deg_east: 12.5,
            },
            uncertainty_seconds: 60.0,
        };
        let actual = experimental_snapshot_from_civil_with_provider(
            &mut manager,
            make_civil(),
            69.0,
            2.0,
            "synthetic",
            &site,
        )
        .unwrap();
        let expected = experimental_snapshot_from_civil(
            &mut original,
            make_civil(),
            69.0,
            2.0,
            "synthetic",
            &site,
            None,
        )
        .unwrap();
        assert_eq!(actual.civil.jd_ut1, expected.civil.jd_ut1);
        for (a, b) in actual
            .snapshot
            .planets
            .positions
            .iter()
            .zip(&expected.snapshot.planets.positions)
        {
            assert_eq!(a.rates.base, b.rates.base);
            assert_eq!(
                a.rates.longitude_deg_per_tdb_day,
                b.rates.longitude_deg_per_tdb_day
            );
            assert_eq!(
                a.rates.latitude_deg_per_tdb_day,
                b.rates.latitude_deg_per_tdb_day
            );
            assert_eq!(a.rates.radial_au_per_tdb_day, b.rates.radial_au_per_tdb_day);
            assert_eq!(a.equatorial, b.equatorial);
        }
        for (a, b) in actual
            .snapshot
            .lunar
            .positions
            .iter()
            .zip(&expected.snapshot.lunar.positions)
        {
            assert_eq!(a.ecliptic, b.ecliptic);
            assert_eq!(a.equatorial, b.equatorial);
        }
        let a = actual.snapshot.house_angles.unwrap().angles;
        let b = expected.snapshot.house_angles.unwrap().angles;
        assert_eq!(a.ascendant_deg, b.ascendant_deg);
        assert_eq!(a.midheaven_deg, b.midheaven_deg);
        assert!(actual.snapshot.sunrise.is_none() && actual.snapshot.sunset.is_none());
    }
}
