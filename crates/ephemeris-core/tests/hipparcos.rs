// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

//! Parser tests with **fictional** fixed-width records, plus an ignored test
//! on a caller-supplied genuine `hip2.dat` (`HIP2_PATH`). No catalog data is
//! stored in the repository.

use std::io::{BufReader, Cursor};

use virgostars_ephemeris_core::fixed_star::propagate_star_icrs_astrometric;
use virgostars_ephemeris_core::hipparcos::{
    accepts_experimental_hip2, parse_hip2_record, read_hip2_selected, Hip2Error, Hip2QualityPolicy,
    RadialVelocity, HIP2_EPOCH_JD, HIP2_RECORD_LENGTH,
};

#[test]
fn experimental_policy_requires_explicit_finite_positive_parallax_and_known_solution() {
    let mut r = parse_hip2_record(&fictional()).unwrap();
    assert!(Hip2QualityPolicy::STRICT.accepts(&r));
    assert!(accepts_experimental_hip2(&r));
    r.solution_type = 15;
    r.components = 2;
    r.parallax_mas = 1.0;
    assert!(!Hip2QualityPolicy::TRADITIONAL_STARS.accepts(&r));
    assert!(accepts_experimental_hip2(&r));
    r.parallax_mas = 0.0;
    assert!(!accepts_experimental_hip2(&r));
    r.parallax_mas = -1.0;
    assert!(!accepts_experimental_hip2(&r));
    r.parallax_mas = 1.0;
    r.e_parallax_mas = 0.0;
    assert!(!accepts_experimental_hip2(&r));
    r.e_parallax_mas = 1.0;
    r.solution_type = 42;
    assert!(!accepts_experimental_hip2(&r));
    r.solution_type = 15;
    r.components = 0;
    assert!(!accepts_experimental_hip2(&r));
}

/// Build a 276-byte record; each value is right-aligned in its CDS columns.
#[allow(clippy::too_many_arguments)]
fn record(
    hip: u32,
    sn: u32,
    nc: u32,
    ra: &str,
    dec: &str,
    plx: &str,
    pmra: &str,
    pmde: &str,
) -> String {
    let mut bytes = vec![b' '; HIP2_RECORD_LENGTH];
    let mut put = |from: usize, to: usize, text: &str| {
        let width = to - from + 1;
        assert!(text.len() <= width, "{text}");
        let start = to - text.len();
        bytes[start..to].copy_from_slice(text.as_bytes());
    };
    put(1, 6, &hip.to_string());
    put(8, 10, &sn.to_string());
    put(14, 14, &nc.to_string());
    put(16, 28, ra);
    put(30, 42, dec);
    put(44, 50, plx);
    put(52, 59, pmra);
    put(61, 68, pmde);
    for (from, to, text) in [
        (70, 75, "1.00"),
        (77, 82, "0.50"),
        (84, 89, "2.00"),
        (91, 96, "1.50"),
        (98, 103, "0.80"),
    ] {
        put(from, to, text);
    }
    String::from_utf8(bytes).unwrap()
}

fn fictional() -> String {
    record(
        900_001,
        5,
        1,
        "1.0000000000",
        "-0.5000000000",
        "20.00",
        "-100.00",
        "50.00",
    )
}

#[test]
fn parses_a_fictional_record_with_its_units() {
    let r = parse_hip2_record(&fictional()).unwrap();
    assert_eq!(r.hip, 900_001);
    assert_eq!((r.solution_type, r.components), (5, 1));
    assert_eq!(r.ra_rad, 1.0);
    assert_eq!(r.dec_rad, -0.5);
    assert_eq!(r.parallax_mas, 20.0);
    assert_eq!(r.pm_ra_cos_dec_mas_per_year, -100.0);
    assert_eq!(r.pm_dec_mas_per_year, 50.0);
    assert_eq!(r.e_parallax_mas, 2.0);
    assert_eq!(r.e_pm_dec_mas_per_year, 0.8);
}

#[test]
fn conversion_keeps_catalog_epoch_degrees_and_the_rv_decision() {
    let r = parse_hip2_record(&fictional()).unwrap();
    let e = r.to_catalog_entry(RadialVelocity::Measured(-12.5)).unwrap();
    assert_eq!(e.epoch_tdb.0, HIP2_EPOCH_JD);
    assert!((e.right_ascension_deg - 1.0_f64.to_degrees()).abs() < 1e-12);
    assert!((e.declination_deg + 0.5_f64.to_degrees()).abs() < 1e-12);
    assert_eq!(e.proper_motion_ra_cos_dec_mas_per_year, -100.0);
    assert_eq!(e.radial_velocity_km_per_second, -12.5);
    let zero = r.to_catalog_entry(RadialVelocity::AssumedZero).unwrap();
    assert_eq!(zero.radial_velocity_km_per_second, 0.0);
    assert!(r
        .to_catalog_entry(RadialVelocity::Measured(f64::NAN))
        .is_err());
    assert!(r.to_catalog_entry(RadialVelocity::Measured(3.0e5)).is_err());
    // The epoch is exactly J1991.25 by the Julian-epoch definition.
    assert_eq!(2_451_545.0 + (1991.25 - 2000.0) * 365.25, HIP2_EPOCH_JD);
}

#[test]
fn rejects_malformed_records_explicitly() {
    let good = fictional();
    assert_eq!(
        parse_hip2_record(&good[..275]),
        Err(Hip2Error::BadRecordLength(275))
    );
    assert_eq!(
        parse_hip2_record(&format!("{good} ")),
        Err(Hip2Error::BadRecordLength(277))
    );
    let mut blank = good.clone().into_bytes();
    blank[43..50].fill(b' ');
    assert!(matches!(
        parse_hip2_record(&String::from_utf8(blank).unwrap()),
        Err(Hip2Error::BadField("Plx"))
    ));
    let bad_ra = record(
        1,
        5,
        1,
        "7.0000000000",
        "0.1000000000",
        "10.00",
        "1.00",
        "1.00",
    );
    assert_eq!(
        parse_hip2_record(&bad_ra),
        Err(Hip2Error::OutOfRange("RArad"))
    );
    let bad_dec = record(1, 5, 1, "1.0", "2.0000000000", "10.00", "1.00", "1.00");
    assert_eq!(
        parse_hip2_record(&bad_dec),
        Err(Hip2Error::OutOfRange("DErad"))
    );
    assert_eq!(
        parse_hip2_record(&good.replace('1', "é")),
        Err(Hip2Error::NonAscii)
    );
}

#[test]
fn non_positive_parallax_is_never_turned_into_a_distance() {
    let r = parse_hip2_record(&record(1, 5, 1, "1.0", "0.1", "-0.50", "1.00", "1.00")).unwrap();
    assert!(r.to_catalog_entry(RadialVelocity::AssumedZero).is_err());
}

#[test]
fn quality_policy_is_explicit_and_not_a_catalog_rule() {
    let mk = |sn, nc, plx: &str| {
        parse_hip2_record(&record(1, sn, nc, "1.0", "0.1", plx, "1.00", "1.00")).unwrap()
    };
    let strict = Hip2QualityPolicy::STRICT;
    assert!(strict.accepts(&mk(5, 1, "20.00")));
    assert!(!strict.accepts(&mk(7, 1, "20.00")), "7-parameter solution");
    assert!(!strict.accepts(&mk(5, 2, "20.00")), "two components");
    assert!(!strict.accepts(&mk(5, 1, "9.00")), "S/N 4.5 < 5");
    let relaxed = Hip2QualityPolicy::TRADITIONAL_STARS;
    assert!(relaxed.accepts(&mk(7, 1, "20.00")));
    assert!(relaxed.accepts(&mk(1, 1, "20.00")));
    assert!(relaxed.accepts(&mk(55, 2, "20.00")));
    assert!(relaxed.accepts(&mk(75, 1, "20.00")));
    // Still rejected: unknown solution types, three components, low S/N.
    assert!(!relaxed.accepts(&mk(9, 1, "20.00")));
    assert!(!relaxed.accepts(&mk(5, 3, "20.00")));
    assert!(!relaxed.accepts(&mk(1, 1, "9.00")));
}

#[test]
fn selection_requires_the_full_published_record_count() {
    let lines = [
        fictional(),
        record(900_002, 5, 1, "2.0", "0.2", "10.00", "1.00", "1.00"),
    ];
    let text = format!("{}\n{}\n", lines[0], lines[1]);
    let err = read_hip2_selected(BufReader::new(Cursor::new(text)), &[900_001]).unwrap_err();
    assert_eq!(err, Hip2Error::WrongRecordCount(2));
    let dup = format!("{}\n{}\n", lines[0], lines[0]);
    assert_eq!(
        read_hip2_selected(BufReader::new(Cursor::new(dup)), &[900_001]).unwrap_err(),
        Hip2Error::DuplicateHip(900_001)
    );
}

#[test]
fn catalog_epoch_astrometric_position_reproduces_the_record() {
    let r = parse_hip2_record(&fictional()).unwrap();
    let e = r.to_catalog_entry(RadialVelocity::AssumedZero).unwrap();
    let p = propagate_star_icrs_astrometric(e, e.epoch_tdb).unwrap();
    assert!((p.right_ascension_deg - 1.0_f64.to_degrees()).abs() < 1e-10);
    assert!((p.declination_deg + 0.5_f64.to_degrees()).abs() < 1e-10);
}

#[test]
#[ignore = "requires a caller-supplied genuine hip2.dat in HIP2_PATH"]
fn genuine_hip2_validates_and_known_stars_have_sane_values() {
    let path = std::env::var_os("HIP2_PATH").expect("HIP2_PATH is not set");
    let file = std::fs::File::open(path).unwrap();
    // Regulus, Aldebaran, Spica, Antares (HIP numbers are public identifiers).
    let wanted = [49_669, 21_421, 65_474, 80_763];
    let stars = read_hip2_selected(BufReader::new(file), &wanted).unwrap();
    let regulus = &stars[0];
    assert!((regulus.parallax_mas - 41.13).abs() < 0.05, "{regulus:?}");
    assert!((regulus.dec_rad.to_degrees() - 11.967).abs() < 0.01);
    assert!(regulus.pm_ra_cos_dec_mas_per_year < -240.0);
    for star in &stars {
        assert!(star.parallax_mas > 5.0 && star.e_parallax_mas > 0.0);
    }
    // Regulus passes both policies; Aldebaran (Sn 1), Spica (Sn 7) and
    // Antares (Sn 1) pass only the traditional-stars policy.
    assert!(Hip2QualityPolicy::STRICT.accepts(&stars[0]));
    for star in &stars[1..] {
        assert!(!Hip2QualityPolicy::STRICT.accepts(star), "{}", star.hip);
        assert!(
            Hip2QualityPolicy::TRADITIONAL_STARS.accepts(star),
            "{}",
            star.hip
        );
    }
}

#[test]
#[ignore = "requires a caller-supplied genuine hip2.dat in HIP2_PATH"]
fn genuine_hip2_traditional_policy_accepts_all_sixteen_comparison_stars() {
    let path = std::env::var_os("HIP2_PATH").expect("HIP2_PATH is not set");
    let file = std::fs::File::open(path).unwrap();
    let wanted = [
        21_421, 49_669, 65_474, 80_763, 37_826, 91_262, 69_673, 113_368, 14_576, 27_989, 24_608,
        37_279, 30_438, 102_098, 97_649, 11_767,
    ];
    let stars = read_hip2_selected(BufReader::new(file), &wanted).unwrap();
    let strict = stars
        .iter()
        .filter(|s| Hip2QualityPolicy::STRICT.accepts(s))
        .count();
    let traditional = stars
        .iter()
        .filter(|s| Hip2QualityPolicy::TRADITIONAL_STARS.accepts(s))
        .count();
    assert_eq!((strict, traditional), (7, 16));
}
