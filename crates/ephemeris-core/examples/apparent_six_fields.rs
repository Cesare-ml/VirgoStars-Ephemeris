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

//! Offline CLI: classic DE441 → apparent geocentric ecliptic of date.
//!
//! Usage: `apparent_six_fields [--no-solar-deflection] [--heliocentric] [--equatorial]
//! <de441.eph> <TDB-JD> [Body ...]` (Earth is a valid body with `--heliocentric`)
//!
//! The DE441 file is supplied by the caller and never copied or downloaded.
//! Output is CSV on stdout with explicit frame/correction provenance, in
//! degrees, AU and per **TDB** day. No Swiss-parity claim is made.

use std::env;
use std::error::Error;

use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    applied_corrections, geocentric_apparent_six_fields_many_with_options, not_applied_corrections,
    ApparentCenter, ApparentFrame, ApparentOptions,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

const DEFAULT_TARGETS: [(&str, Body); 11] = [
    ("Sun", Body::Sun),
    ("Moon", Body::Moon),
    ("Mercury", Body::Mercury),
    ("Venus", Body::Venus),
    ("MarsBarycenter", Body::MarsBarycenter),
    ("JupiterBarycenter", Body::JupiterBarycenter),
    ("SaturnBarycenter", Body::SaturnBarycenter),
    ("UranusBarycenter", Body::UranusBarycenter),
    ("NeptuneBarycenter", Body::NeptuneBarycenter),
    ("PlutoBarycenter", Body::PlutoBarycenter),
    ("Earth", Body::Earth),
];

fn main() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<String> = env::args().collect();
    let mut options = ApparentOptions::default();
    if let Some(pos) = args.iter().position(|a| a == "--heliocentric") {
        args.remove(pos);
        options.center = ApparentCenter::Heliocentric;
    }
    if let Some(pos) = args.iter().position(|a| a == "--equatorial") {
        args.remove(pos);
        options.frame = ApparentFrame::TrueEquatorOfDate;
    }
    if let Some(pos) = args.iter().position(|a| a == "--no-solar-deflection") {
        args.remove(pos);
        options.solar_light_deflection = false;
    }
    if args.len() < 3 {
        return Err("usage: apparent_six_fields <de441.eph> <TDB-JD> [Body ...]".into());
    }
    let jd: f64 = args[2].parse()?;
    if !jd.is_finite() {
        return Err("TDB Julian date must be finite".into());
    }
    let selected: Vec<(&str, Body)> = if args.len() > 3 {
        args[3..]
            .iter()
            .map(|name| {
                DEFAULT_TARGETS
                    .iter()
                    .copied()
                    .find(|(n, _)| n == name)
                    .ok_or_else(|| format!("unsupported target: {name}"))
            })
            .collect::<Result<_, _>>()?
    } else {
        DEFAULT_TARGETS
            .iter()
            .copied()
            .filter(|(_, b)| *b != Body::Earth)
            .collect()
    };
    let bodies: Vec<Body> = selected.iter().map(|(_, b)| *b).collect();
    let mut ephemeris = De441::open(&args[1])?;
    let rows = geocentric_apparent_six_fields_many_with_options(
        &mut ephemeris,
        &bodies,
        JulianDateTdb(jd),
        options,
    )?;
    println!("target,tdb_jd,tt_jd,longitude_deg,latitude_deg,distance_au,longitude_speed_deg_per_tdb_day,latitude_speed_deg_per_tdb_day,distance_speed_au_per_tdb_day,frame,applied,not_applied");
    let frame_label = format!(
        "{}-{}",
        match options.frame {
            ApparentFrame::TrueEclipticOfDate => "true-ecliptic-and-equinox-of-date",
            ApparentFrame::TrueEquatorOfDate => "true-equator-and-equinox-of-date",
        },
        match options.center {
            ApparentCenter::Geocentric => "geocentric",
            ApparentCenter::Heliocentric => "heliocentric",
        }
    );
    for ((name, _), row) in selected.iter().zip(rows.iter()) {
        println!(
            "{name},{jd:.9},{:.9},{:.12},{:.12},{:.15},{:.12},{:.12},{:.15},{frame_label},{},{}",
            row.tt_jd,
            row.fields.longitude_deg,
            row.fields.latitude_deg,
            row.fields.distance_au,
            row.fields.longitude_speed_deg_per_tdb_day,
            row.fields.latitude_speed_deg_per_tdb_day,
            row.fields.distance_speed_au_per_tdb_day,
            applied_corrections(options).join(";"),
            not_applied_corrections(options).join(";"),
        );
    }
    Ok(())
}
