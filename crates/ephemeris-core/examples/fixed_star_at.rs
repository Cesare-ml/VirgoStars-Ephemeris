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

//! Offline CLI: one caller-supplied catalog star at one TDB epoch → true
//! ecliptic of date, with rates.
//!
//! Usage: `fixed_star_at [--no-solar-deflection] <de441.eph> <TDB-JD> <name>
//! <ra_deg> <dec_deg> <pm_ra_cos_dec_mas_yr> <pm_dec_mas_yr> <parallax_mas>
//! <rv_km_s> [<catalog_epoch_tdb_jd, default 2451545.0>]`
//!
//! No star catalog is bundled; the caller owns the provenance and rights of
//! the values it passes in.

use std::env;
use std::error::Error;

use virgostars_ephemeris_core::apparent_ecliptic_of_date::tdb_to_tt_jd;
use virgostars_ephemeris_core::de441::{De441, JulianDateTdb};
use virgostars_ephemeris_core::fixed_star::{
    calculate_star_with_solar_deflection, FixedStarCatalogEntry, StellarSolarDeflection,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<String> = env::args().collect();
    let mut deflection = StellarSolarDeflection::PointMass;
    if let Some(pos) = args.iter().position(|a| a == "--no-solar-deflection") {
        args.remove(pos);
        deflection = StellarSolarDeflection::Omit;
    }
    if args.len() < 10 {
        return Err("usage: fixed_star_at [--no-solar-deflection] <de441.eph> <TDB-JD> <name> <ra_deg> <dec_deg> <pm_ra_mas_yr> <pm_dec_mas_yr> <parallax_mas> <rv_km_s> [epoch_tdb_jd]".into());
    }
    let number = |i: usize| -> Result<f64, Box<dyn Error>> { Ok(args[i].parse()?) };
    let tdb = JulianDateTdb(number(2)?);
    let entry = FixedStarCatalogEntry {
        epoch_tdb: JulianDateTdb(if args.len() > 10 {
            number(10)?
        } else {
            2_451_545.0
        }),
        right_ascension_deg: number(4)?,
        declination_deg: number(5)?,
        proper_motion_ra_cos_dec_mas_per_year: number(6)?,
        proper_motion_dec_mas_per_year: number(7)?,
        parallax_mas: number(8)?,
        radial_velocity_km_per_second: number(9)?,
    };
    let mut ephemeris = De441::open(&args[1])?;
    let tt = tdb_to_tt_jd(tdb.0)?;
    let p = calculate_star_with_solar_deflection(&mut ephemeris, entry, tt, tdb, deflection)?;
    println!(
        "{},{:.9},{:.12},{:.12},{:.15},{:.12},{:.12},{:.15}",
        args[3],
        tdb.0,
        p.longitude_deg,
        p.latitude_deg,
        p.distance_au,
        p.longitude_speed_deg_per_tdb_day,
        p.latitude_speed_deg_per_tdb_day,
        p.distance_speed_au_per_tdb_day
    );
    Ok(())
}
