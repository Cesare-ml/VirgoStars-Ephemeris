/*
EXHIBIT A. Common Public Attribution License Version 1.0.

The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

//! Offline, non-overwriting classic-DE441 slice exporter. No kernel downloads.

use std::env;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use virgostars_ephemeris_core::calendar::{Calendar, CalendarDateTime, TimeScale};
use virgostars_ephemeris_core::de441::{De441, JulianDateTdb};

fn usage() -> &'static str {
    "usage: cargo run --offline --locked -p virgostars-ephemeris-core --example slice_de441_classic -- \
     <source_de441.eph> <new_output.eph> <first_year> <last_year_inclusive>\n\
     Years use astronomical numbering (0 = 1 BCE), proleptic Gregorian Jan 1 at 00:00 TDB.\n\
     Existing output files are never overwritten. Output is NOT an SPK .bsp file."
}

fn first_jd_of_year(year: i32) -> Result<JulianDateTdb, Box<dyn Error>> {
    Ok(CalendarDateTime {
        year,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
        scale: TimeScale::Tdb,
    }
    .to_julian_date(Calendar::ProlepticGregorian)?
    .to_de441_epoch()?)
}

fn input_error(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.to_owned())
}

fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 5 {
        return Err(input_error(usage()).into());
    }
    let first_year = args[3]
        .parse::<i32>()
        .map_err(|_| input_error("invalid first_year"))?;
    let last_year = args[4]
        .parse::<i32>()
        .map_err(|_| input_error("invalid last_year_inclusive"))?;
    if first_year > last_year {
        return Err(input_error("first_year must not exceed last_year_inclusive").into());
    }
    let end_exclusive_year = last_year
        .checked_add(1)
        .ok_or_else(|| input_error("last_year_inclusive overflow"))?;
    let start = first_jd_of_year(first_year)?;
    let end = first_jd_of_year(end_exclusive_year)?;

    let mut ephemeris = De441::open(&args[1])?;
    // An output path must be new. In particular, never overwrite the source.
    let destination = PathBuf::from(&args[2]);
    if destination.exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "output exists").into());
    }
    let file_name = destination
        .file_name()
        .ok_or_else(|| input_error("output must name a file"))?;
    let temp_name = format!(
        ".{}.partial-{}",
        file_name.to_string_lossy(),
        std::process::id()
    );
    let temporary = destination.with_file_name(temp_name);
    let mut partial = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> Result<_, Box<dyn Error>> {
        let summary = ephemeris.write_slice(start, end, &mut partial)?;
        partial.flush()?;
        partial.sync_all()?;
        let verified = De441::<File>::open(&temporary)?;
        if verified.coverage() != (summary.stored_start_tdb, summary.stored_end_tdb)
            || partial.metadata()?.len() != summary.bytes_written
        {
            return Err(input_error("slice re-open/length verification failed").into());
        }
        // Hard-link in the same directory refuses an existing destination
        // atomically. Remove only the temporary file created by this command.
        fs::hard_link(&temporary, &destination)?;
        Ok(summary)
    })();
    drop(partial);
    let cleanup = fs::remove_file(&temporary);
    if let Err(error) = cleanup {
        eprintln!("temporary file cleanup warning: {error}");
    }
    let summary = result?;
    println!(
        "stored_jd_tdb=[{}, {}] records={} bytes={} output={}",
        summary.stored_start_tdb.0,
        summary.stored_end_tdb.0,
        summary.coefficient_records,
        summary.bytes_written,
        destination.display()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run(&env::args().collect::<Vec<_>>()) {
        eprintln!("slice_de441_classic: {error}");
        std::process::exit(2);
    }
}
