// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Audit caller-supplied S5 spline and USNO snapshots; neither is bundled.
use std::{
    error::Error,
    fs,
    io::{self, BufRead},
};
use virgostars_ephemeris_core::{
    delta_t_spline::{DeltaTSpline, DeltaTSplineSegment},
    delta_t_table::DeltaTTable,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 5 {
        return Err("usage: EXE S5_ROWS MONTHLY HISTORIC PREDS".into());
    }
    let mut rows = Vec::new();
    for line in fs::read_to_string(&args[1])?.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 7 || fields[0].parse::<usize>()? != rows.len() + 1 {
            return Err("invalid S5 row".into());
        }
        let v = fields[1..]
            .iter()
            .map(|s| s.parse::<f64>())
            .collect::<Result<Vec<_>, _>>()?;
        rows.push(DeltaTSplineSegment {
            start_year: v[0],
            end_year: v[1],
            coefficients: [v[2], v[3], v[4], v[5]],
        });
    }
    if rows.len() != 58 || rows[0].start_year != -720.0 || rows.last().unwrap().end_year != 2019.0 {
        return Err("unexpected published S5 coverage".into());
    }
    let spline = DeltaTSpline::new(rows)?;
    let table = DeltaTTable::from_usno(
        &fs::read_to_string(&args[2])?,
        &fs::read_to_string(&args[3])?,
        Some(&fs::read_to_string(&args[4])?),
    )?;
    println!(
        "meta,{:.9},{:.9}",
        table.last_measured_jd_ut(),
        table.last_jd_ut()
    );
    for line in io::stdin().lock().lines() {
        let line = line?;
        let (kind, value) = line.split_once(',').ok_or("expected S,year or U,jd")?;
        let n: f64 = value.parse()?;
        match kind {
            "S" => match spline.seconds_at(n) {
                Ok(x) => println!("S,{n:.12},{x:.12}"),
                Err(_) => println!("S,{n:.12},ERR"),
            },
            "U" => match table.delta_t(n) {
                Ok(x) => println!(
                    "U,{n:.12},{:.12},{},{:?}",
                    x.delta_t_seconds,
                    x.uncertainty_seconds
                        .map_or("unknown".to_owned(), |u| format!("{u:.12}")),
                    x.kind
                ),
                Err(_) => println!("U,{n:.12},ERR"),
            },
            _ => return Err("unknown query kind".into()),
        }
    }
    Ok(())
}
