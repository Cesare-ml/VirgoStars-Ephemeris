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

//! Real-kernel checks for `fixed_star` with a **fictional** star. Needs a
//! classic DE441 file in `DE441_PATH`; no catalog data is used or bundled.

use virgostars_ephemeris_core::apparent_ecliptic_of_date::tdb_to_tt_jd;
use virgostars_ephemeris_core::de441::{De441, JulianDateTdb};
use virgostars_ephemeris_core::fixed_star::{
    calculate_star_with_solar_deflection, FixedStarCatalogEntry, StellarSolarDeflection,
};

/// A fictional, motionless-looking star 1 degree east of the Sun in ecliptic
/// longitude at J2000, on the ecliptic.
fn fictional_star_near_sun() -> FixedStarCatalogEntry {
    let obliquity = 23.439_291_f64.to_radians();
    let lambda = 281.37_f64.to_radians();
    let ra = (lambda.sin() * obliquity.cos()).atan2(lambda.cos());
    let dec = (lambda.sin() * obliquity.sin()).asin();
    FixedStarCatalogEntry {
        epoch_tdb: JulianDateTdb(2_451_545.0),
        right_ascension_deg: ra.to_degrees().rem_euclid(360.0),
        declination_deg: dec.to_degrees(),
        proper_motion_ra_cos_dec_mas_per_year: 0.0,
        proper_motion_dec_mas_per_year: 0.0,
        parallax_mas: 10.0,
        radial_velocity_km_per_second: 0.0,
    }
}

#[test]
#[ignore = "requires a legally obtained classic DE441 file in DE441_PATH"]
fn real_de441_solar_deflection_of_a_star_follows_the_cot_half_elongation_law() {
    let path = std::env::var_os("DE441_PATH").expect("DE441_PATH is not set");
    let mut eph = De441::open(path).unwrap();
    let star = fictional_star_near_sun();
    let tdb = JulianDateTdb(2_451_545.0);
    let tt = tdb_to_tt_jd(tdb.0).unwrap();
    let on = calculate_star_with_solar_deflection(
        &mut eph,
        star,
        tt,
        tdb,
        StellarSolarDeflection::PointMass,
    )
    .unwrap();
    let off =
        calculate_star_with_solar_deflection(&mut eph, star, tt, tdb, StellarSolarDeflection::Omit)
            .unwrap();
    let shift_arcsec = ((on.longitude_deg - off.longitude_deg).powi(2)
        + (on.latitude_deg - off.latitude_deg).powi(2))
    .sqrt()
        * 3600.0;
    // Elongation ≈ 1 degree (Sun at ≈ 280.37 deg), E ≈ 0.9833 AU:
    // (2GM/c²/E)·cot(psi/2) ≈ 2.04e-8 rad · 114.6 ≈ 0.48 arcsec.
    let elongation = (281.37_f64 - 280.368_2).to_radians();
    let expected = 1.974_125_743_36e-8 / 0.983_33 / (elongation / 2.0).tan() * 206_264.806_247;
    assert!(
        (shift_arcsec - expected).abs() < 0.05 * expected,
        "{shift_arcsec} vs {expected}"
    );
    // The star is east of the Sun, so deflection pushes it to larger longitude.
    assert!(on.longitude_deg > off.longitude_deg);
    assert_eq!(on.distance_au, off.distance_au);
}
