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

//! Offline source-vs-slice regression. NOT independent NASA/JPL verification.
//! Modification 2026-11-18: verify every sliced DE441 record at three epochs.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, ErrorKind};

use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

const PAIRS: &[(Body, Body)] = &[
    (Body::Mercury, Body::SolarSystemBarycenter),
    (Body::Venus, Body::SolarSystemBarycenter),
    (Body::EarthMoonBarycenter, Body::SolarSystemBarycenter),
    (Body::Earth, Body::SolarSystemBarycenter),
    (Body::Moon, Body::SolarSystemBarycenter),
    (Body::MarsBarycenter, Body::SolarSystemBarycenter),
    (Body::JupiterBarycenter, Body::SolarSystemBarycenter),
    (Body::SaturnBarycenter, Body::SolarSystemBarycenter),
    (Body::UranusBarycenter, Body::SolarSystemBarycenter),
    (Body::NeptuneBarycenter, Body::SolarSystemBarycenter),
    (Body::PlutoBarycenter, Body::SolarSystemBarycenter),
    (Body::Sun, Body::SolarSystemBarycenter),
    (Body::Moon, Body::Earth),
    (Body::Earth, Body::Sun),
];

fn input_error(message: impl Into<String>) -> io::Error {
    io::Error::new(ErrorKind::InvalidInput, message.into())
}

fn validate_pair(
    parent: &mut De441<File>,
    slice: &mut De441<File>,
    epoch: JulianDateTdb,
    pair: (Body, Body),
) -> Result<(), Box<dyn Error>> {
    let expected = parent.state(pair.0, pair.1, epoch)?;
    let actual = slice.state(pair.0, pair.1, epoch)?;
    if expected != actual {
        return Err(input_error(format!(
            "DE441 source/slice mismatch at JD TDB {} for {:?} relative to {:?}: expected={:?} actual={:?}",
            epoch.0, pair.0, pair.1, expected, actual
        ))
        .into());
    }
    Ok(())
}

fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 3 {
        return Err(input_error(
            "usage: verify_de441_classic_slice <source_classic_de441> <generated_classic_slice>",
        )
        .into());
    }
    let mut source = De441::open(&args[1])?;
    let mut slice = De441::open(&args[2])?;
    let (source_start, source_end) = source.coverage();
    let (slice_start, slice_end) = slice.coverage();
    let step = slice.record_step_days();
    let records = slice.coefficient_record_count();
    if source_start.0 > slice_start.0
        || source_end.0 < slice_end.0
        || source.record_step_days() != step
        || records == 0
    {
        return Err(input_error("invalid slice geometry or record count").into());
    }

    let mut epochs = 0usize;
    for record in 0..records {
        let record_start = slice_start.0 + record as f64 * step;
        // Keep the final endpoint exclusive: the unsliced source selects its
        // next record there whereas a slice necessarily selects its last.
        for fraction in [0.0, 0.5, 0.875] {
            let jd = JulianDateTdb(record_start + fraction * step);
            if jd.0 >= slice_end.0 {
                return Err(input_error("sample crossed slice coverage").into());
            }
            for pair in PAIRS {
                validate_pair(&mut source, &mut slice, jd, *pair)?;
            }
            epochs += 1;
        }
    }
    if slice_end == source_end {
        // Only when the source also ends here can this endpoint select the
        // same last polynomial record on both sides.
        for pair in PAIRS {
            validate_pair(&mut source, &mut slice, slice_end, *pair)?;
        }
        epochs += 1;
    }
    println!(
        "parity_ok records={} epochs={} state_pairs={} jd_tdb=[{}, {}]",
        records,
        epochs,
        epochs * PAIRS.len(),
        slice_start.0,
        slice_end.0
    );
    Ok(())
}

fn main() {
    if let Err(error) = run(&env::args().collect::<Vec<_>>()) {
        eprintln!("verify_de441_classic_slice: {error}");
        std::process::exit(2);
    }
}
