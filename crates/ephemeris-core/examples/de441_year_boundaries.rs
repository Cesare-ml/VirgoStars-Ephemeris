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

// Modification (2026-11-19): expose canonical year boundaries for offline verification.

//! Print proleptic Gregorian year boundaries in JD TDB; no time conversion.

use std::{env, error::Error, io};
use virgostars_ephemeris_core::calendar::{Calendar, CalendarDateTime, TimeScale};

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.is_empty() || args.len() > 512 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected 1 to 512 years").into());
    }
    for arg in args {
        let year = arg.parse::<i32>()?;
        let jd = CalendarDateTime {
            year,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0.0,
            scale: TimeScale::Tdb,
        }
        .to_julian_date(Calendar::ProlepticGregorian)?
        .to_de441_epoch()?;
        println!("{year} {}", jd.0);
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("de441_year_boundaries: {error}");
        std::process::exit(2);
    }
}
