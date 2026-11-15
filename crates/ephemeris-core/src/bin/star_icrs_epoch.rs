// Original VirgoStars source, CPAL-1.0; see repository LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// Experimental explicit-input ICRS astrometric exporter, no catalog/kernel I/O.
use std::env;
use std::process;
use virgostars_ephemeris_core::de441::JulianDateTdb;
use virgostars_ephemeris_core::fixed_star::{
    propagate_star_icrs_astrometric, FixedStarCatalogEntry,
};

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 9 {
        return Err("usage: star_icrs_epoch <catalog_tdb_jd> <ra_deg> <dec_deg> <pm_ra_cos_dec_mas_yr> <pm_dec_mas_yr> <parallax_mas> <barycentric_rv_km_s> <target_tdb_jd> <hip_id>".into());
    }
    let values: Vec<f64> = args[..8]
        .iter()
        .map(|v| {
            v.parse::<f64>()
                .map_err(|_| "invalid finite stellar argument".to_string())
        })
        .collect::<Result<_, _>>()?;
    if values.iter().any(|v| !v.is_finite()) {
        return Err("all stellar inputs and epochs must be finite".into());
    }
    let hip: u32 = args[8]
        .parse()
        .map_err(|_| "HIP ID must be a positive integer".to_string())?;
    if hip == 0 || hip > 120_416 {
        return Err("HIP ID outside accepted source identifier range".into());
    }
    let entry = FixedStarCatalogEntry {
        epoch_tdb: JulianDateTdb(values[0]),
        right_ascension_deg: values[1],
        declination_deg: values[2],
        proper_motion_ra_cos_dec_mas_per_year: values[3],
        proper_motion_dec_mas_per_year: values[4],
        parallax_mas: values[5],
        radial_velocity_km_per_second: values[6],
    };
    let result = propagate_star_icrs_astrometric(entry, JulianDateTdb(values[7]))
        .map_err(|error| format!("invalid astrometric source: {error}"))?;
    println!("hip,tdb_jd,ra_icrs_deg,dec_icrs_deg,distance_au");
    println!(
        "{hip},{:.12},{:.15},{:.15},{:.12}",
        result.tdb_julian_day,
        result.right_ascension_deg,
        result.declination_deg,
        result.distance_au
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
