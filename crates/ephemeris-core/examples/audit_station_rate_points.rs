// SPDX-License-Identifier: CPAL-1.0. Diagnostic only; no new astronomy model.
//! Input on stdin: `ID,JD_TDB`; IDs 0–9 match the standard ten targets.
//! `tt,JD_TDB` returns the existing TDB-to-TT conversion without calculating
//! an apparent state. Errors are reported per row without a silent fallback.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    geocentric_apparent_six_fields, tdb_to_tt_jd,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: audit_station_rate_points KERNEL < cases.csv".into());
    }
    let targets = [
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
    let mut eph = De441::open(&args[1])?;
    for input in io::stdin().lock().lines() {
        let input = input?;
        let (id, value) = input.split_once(',').ok_or("expected ID,JD")?;
        let jd: f64 = value.parse()?;
        let tt = tdb_to_tt_jd(jd)?;
        if id == "tt" {
            println!("tt,{jd:.9},{tt:.9},time");
            continue;
        }
        let target = *targets.get(id.parse::<usize>()?).ok_or("invalid ID")?;
        match geocentric_apparent_six_fields(&mut eph, target, JulianDateTdb(jd)) {
            Ok(result) => {
                let p = result.fields;
                println!(
                    "{id},{jd:.9},{tt:.9},ok,{:.12},{:.12},{:.15},{:.12},{:.12},{:.15}",
                    p.longitude_deg,
                    p.latitude_deg,
                    p.distance_au,
                    p.longitude_speed_deg_per_tdb_day,
                    p.latitude_speed_deg_per_tdb_day,
                    p.distance_speed_au_per_tdb_day
                );
            }
            Err(error) => println!("{id},{jd:.9},{tt:.9},error,{error:?}"),
        }
    }
    Ok(())
}
