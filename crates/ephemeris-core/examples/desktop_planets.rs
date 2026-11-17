// SPDX-License-Identifier: CPAL-1.0
//! Offline desktop integration example. All input is caller supplied.
//! Usage: cargo run --example desktop_planets -- <classic-de441> <TDB-JD>
//!
//! This deliberately accepts TDB, not a civil date. Converting a desktop's
//! UTC/UT1 input, computing houses and handling unavailable objects belong to
//! the consumer's explicit policies, not to this example.
use std::{env, error::Error};
use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    geocentric_apparent_six_fields_many_with_options, ApparentFrame, ApparentOptions,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

const TARGETS: [(&str, Body); 10] = [
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
];

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: desktop_planets <classic-de441> <TDB-JD>".into());
    }
    let jd: f64 = args[2].parse()?;
    if !jd.is_finite() {
        return Err("TDB Julian date must be finite".into());
    }
    let mut file = De441::open(&args[1])?;
    let targets: Vec<_> = TARGETS.iter().map(|(_, body)| *body).collect();
    let ecliptic = geocentric_apparent_six_fields_many_with_options(
        &mut file,
        &targets,
        JulianDateTdb(jd),
        ApparentOptions::default(),
    )?;
    let equatorial = geocentric_apparent_six_fields_many_with_options(
        &mut file,
        &targets,
        JulianDateTdb(jd),
        ApparentOptions {
            frame: ApparentFrame::TrueEquatorOfDate,
            ..ApparentOptions::default()
        },
    )?;
    println!("target,jd_tdb,tt_jd,longitude_deg,latitude_deg,distance_au,longitude_speed_deg_per_tdb_day,latitude_speed_deg_per_tdb_day,distance_speed_au_per_tdb_day,right_ascension_deg,declination_deg");
    for ((name, _), (ecl, equ)) in TARGETS.iter().zip(ecliptic.iter().zip(equatorial.iter())) {
        let a = &ecl.fields;
        let b = &equ.fields;
        println!(
            "{name},{jd:.9},{:.9},{:.12},{:.12},{:.15},{:.12},{:.12},{:.15},{:.12},{:.12}",
            ecl.tt_jd,
            a.longitude_deg,
            a.latitude_deg,
            a.distance_au,
            a.longitude_speed_deg_per_tdb_day,
            a.latitude_speed_deg_per_tdb_day,
            a.distance_speed_au_per_tdb_day,
            b.longitude_deg,
            b.latitude_deg
        );
    }
    Ok(())
}
