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

//! Experimental geometric time-and-place demonstration; no data is bundled.

use std::env;
use std::error::Error;
use std::io;
use virgostars_ephemeris_core::de441::De441;
use virgostars_ephemeris_core::first_sky::{calculate, CivilTimestamp, Observer};

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 5 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
            "usage: planets_at <de441.eph> <YYYY-MM-DDTHH:MM:SSZ|±HH:MM> <latitude-deg> <longitude-east-deg> <height-m>").into());
    }
    let civil = CivilTimestamp::parse(&args[1])?;
    let site = Observer {
        latitude_deg: args[2].parse()?,
        longitude_deg_east: args[3].parse()?,
        height_m: args[4].parse()?,
    };
    let mut ephemeris = De441::open(&args[0])?;
    let result = calculate(&mut ephemeris, civil, site)?;
    println!("EXPERIMENTAL: topocentric geometric positions, mean equator/ecliptic of date.");
    println!(
        "UT1 approximated by UTC; no nutation, light-time, aberration, deflection or refraction."
    );
    println!("Mars through Pluto are system barycenters. Results are not apparent Swiss Ephemeris positions.");
    println!("JD TDB {:.9}; JD TT {:.9}; TAI-UTC {} s; WGS84 site lat {:.5}°, lon {:.5}° E, height {:.1} m",
        result.tdb_julian_day, result.tt_julian_day, result.tai_minus_utc_seconds,
        result.observer.latitude_deg, result.observer.longitude_deg_east, result.observer.height_m);
    println!(
        "{:<18} {:>12} {:>12} {:>12} {:>12} {:>16}",
        "body", "ecl lon °", "ecl lat °", "alt °", "az °", "distance km"
    );
    for p in result.positions {
        let azimuth = p
            .azimuth_deg
            .map(|v| format!("{v:.6}"))
            .unwrap_or_else(|| "undefined".to_string());
        println!(
            "{:<18} {:>12.6} {:>12.6} {:>12.6} {:>12} {:>16.3}",
            format!("{:?}", p.body),
            p.mean_ecliptic_longitude_deg,
            p.mean_ecliptic_latitude_deg,
            p.altitude_deg,
            azimuth,
            p.distance_km
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
