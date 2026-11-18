// SPDX-License-Identifier: CPAL-1.0. Exploratory comparison, no app activation.
//! stdin: `body-id,JD_TDB` or `tt,JD_TDB`. The existing experimental and
//! standard Rust paths are called explicitly; a standard error is recorded.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    experimental_long_term_apparent_ecliptic, geocentric_apparent_six_fields, tdb_to_tt_jd,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::six_field_units::FixedSixFieldsDegreesAu;

fn values(p: FixedSixFieldsDegreesAu) -> String {
    format!(
        "{:.12},{:.12},{:.15},{:.12},{:.12},{:.15}",
        p.longitude_deg,
        p.latitude_deg,
        p.distance_au,
        p.longitude_speed_deg_per_tdb_day,
        p.latitude_speed_deg_per_tdb_day,
        p.distance_speed_au_per_tdb_day
    )
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: audit_pack_orientation_candidate KERNEL < cases.csv".into());
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
        let target = *targets.get(id.parse::<usize>()?).ok_or("unknown target")?;
        let long = experimental_long_term_apparent_ecliptic(&mut eph, target, JulianDateTdb(jd))?;
        match geocentric_apparent_six_fields(&mut eph, target, JulianDateTdb(jd)) {
            Ok(standard) => println!(
                "{id},{jd:.9},{tt:.9},ok,{},{}",
                values(long),
                values(standard.fields)
            ),
            Err(error) => println!("{id},{jd:.9},{tt:.9},error,{}, {error:?}", values(long)),
        }
    }
    Ok(())
}
