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

//! Offline CLI: one Hipparcos (CDS I/311 hip2) star at one TDB epoch → true
//! ecliptic of date.
//!
//! Usage: `hip_star_at [--no-solar-deflection] <de441.eph> <hip2.dat> <TDB-JD>
//! <HIP> <radial-velocity-km-s | assumed-zero>`
//!
//! Both data files are supplied by the caller; nothing is bundled or fetched.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

use virgostars_ephemeris_core::apparent_ecliptic_of_date::tdb_to_tt_jd;
use virgostars_ephemeris_core::de441::{De441, JulianDateTdb};
use virgostars_ephemeris_core::fixed_star::{
    calculate_star_with_solar_deflection, StellarSolarDeflection,
};
use virgostars_ephemeris_core::hipparcos::{read_hip2_selected, RadialVelocity};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<String> = env::args().collect();
    let mut deflection = StellarSolarDeflection::PointMass;
    if let Some(pos) = args.iter().position(|a| a == "--no-solar-deflection") {
        args.remove(pos);
        deflection = StellarSolarDeflection::Omit;
    }
    if args.len() != 6 {
        return Err("usage: hip_star_at [--no-solar-deflection] <de441.eph> <hip2.dat> <TDB-JD> <HIP> <rv-km-s|assumed-zero>".into());
    }
    let tdb = JulianDateTdb(args[3].parse()?);
    let hip: u32 = args[4].parse()?;
    let rv = if args[5] == "assumed-zero" {
        RadialVelocity::AssumedZero
    } else {
        RadialVelocity::Measured(args[5].parse()?)
    };
    let record = read_hip2_selected(BufReader::new(File::open(&args[2])?), &[hip])?[0];
    let entry = record.to_catalog_entry(rv)?;
    let mut ephemeris = De441::open(&args[1])?;
    let tt = tdb_to_tt_jd(tdb.0)?;
    let p = calculate_star_with_solar_deflection(&mut ephemeris, entry, tt, tdb, deflection)?;
    println!(
        "HIP{hip},{:.9},{:.12},{:.12},{:.15},{:.12},{:.12},{:.15}",
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
