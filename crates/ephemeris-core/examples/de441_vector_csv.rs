/*
The contents of this file are subject to the Common Public Attribution License
Version 1.0 (CPAL-1.0). See LICENSE, NOTICE and the full Exhibit A notice in
crates/ephemeris-core/src/de441.rs.
Original contribution: Cesare-ml (VirgoStars project), 2026.
Modification record: 2026-11-11 — add offline geometric vector export example.
*/

//! Offline CSV export for independently comparing classic DE441 states.
//! This is not a Horizons reference or a numerical accuracy certification.

use std::env;
use std::error::Error;
use std::io;

use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

fn body(name: &str) -> Option<Body> {
    match name {
        "ssb" => Some(Body::SolarSystemBarycenter),
        "mercury" => Some(Body::Mercury),
        "venus" => Some(Body::Venus),
        "emb" => Some(Body::EarthMoonBarycenter),
        "earth" => Some(Body::Earth),
        "moon" => Some(Body::Moon),
        "mars_barycenter" => Some(Body::MarsBarycenter),
        "jupiter_barycenter" => Some(Body::JupiterBarycenter),
        "saturn_barycenter" => Some(Body::SaturnBarycenter),
        "uranus_barycenter" => Some(Body::UranusBarycenter),
        "neptune_barycenter" => Some(Body::NeptuneBarycenter),
        "pluto_barycenter" => Some(Body::PlutoBarycenter),
        "sun" => Some(Body::Sun),
        _ => None,
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        return Err(invalid(
            "usage: cargo run --example de441_vector_csv -- <de441.eph> <JD_TDB> <target> <center>; names: ssb, mercury, venus, emb, earth, moon, mars_barycenter, jupiter_barycenter, saturn_barycenter, uranus_barycenter, neptune_barycenter, pluto_barycenter, sun",
        )
        .into());
    }
    let epoch: f64 = args[2].parse()?;
    if !epoch.is_finite() {
        return Err(invalid("JD_TDB must be finite").into());
    }
    let target = body(&args[3]).ok_or_else(|| invalid("unsupported target name"))?;
    let center = body(&args[4]).ok_or_else(|| invalid("unsupported center name"))?;
    let mut ephemeris = De441::open(&args[1])?;
    let state = ephemeris.state(target, center, JulianDateTdb(epoch))?;
    println!("jd_tdb,target,center,frame,corrections,x_km,y_km,z_km,vx_km_day,vy_km_day,vz_km_day");
    println!(
        "{:.9},{},{},ICRF,GEOMETRIC,{:.15e},{:.15e},{:.15e},{:.15e},{:.15e},{:.15e}",
        epoch,
        args[3],
        args[4],
        state.position_km[0],
        state.position_km[1],
        state.position_km[2],
        state.velocity_km_per_day[0],
        state.velocity_km_per_day[1],
        state.velocity_km_per_day[2],
    );
    Ok(())
}
