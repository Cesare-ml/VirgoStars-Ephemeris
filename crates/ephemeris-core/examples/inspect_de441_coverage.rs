/* EXHIBIT A. Common Public Attribution License Version 1.0.
The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

//! Read-only inspection of the declared classic JPL DE441 binary coverage.
//!
//! No ephemeris vectors are evaluated and no kernel is downloaded.
//! The reported coverage is header metadata, not NASA/JPL numerical validation.
//! Modification record: 2026-11-13 — initial inspector; added strict coverage gate.

use std::env;
use std::error::Error;
use std::io::{self, Read, Seek};
use virgostars_ephemeris_core::de441::De441;

const USAGE: &str =
    "usage: inspect_de441_coverage <classic-de441.eph> [--at-jd <JD_TDB> [--require-coverage]]";

struct Options {
    file: String,
    probe_jd_tdb: Option<f64>,
    require_coverage: bool,
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let file = match args.first() {
        Some(file) if !file.is_empty() && !file.starts_with("--") => file.clone(),
        _ => return Err(USAGE.to_owned()),
    };
    let (probe_text, require_coverage) = match &args[1..] {
        [] => (None, false),
        [flag, value] if flag == "--at-jd" => (Some(value.as_str()), false),
        [flag, value, strict] if flag == "--at-jd" && strict == "--require-coverage" => {
            (Some(value.as_str()), true)
        }
        _ => return Err(USAGE.to_owned()),
    };
    let probe_jd_tdb = probe_text
        .map(|value| {
            let jd = value
                .parse::<f64>()
                .map_err(|_| "JD_TDB must be a finite number".to_owned())?;
            if !jd.is_finite() {
                return Err("JD_TDB must be a finite number".to_owned());
            }
            Ok(jd)
        })
        .transpose()?;
    Ok(Options {
        file,
        probe_jd_tdb,
        require_coverage,
    })
}

fn csv_report<R: Read + Seek>(ephemeris: &De441<R>, probe_jd_tdb: Option<f64>) -> String {
    let (start, end) = ephemeris.coverage();
    let (probe, status) = match probe_jd_tdb {
        Some(jd) if jd >= start.0 && jd <= end.0 => (format!("{jd:.12}"), "covered"),
        Some(jd) => (format!("{jd:.12}"), "out_of_coverage"),
        None => (String::new(), "not_checked"),
    };
    format!(
        "format,epoch_scale,declared_start_jd,declared_end_jd,probe_jd,probe_status\n\
         classic-de441,TDB,{:.12},{:.12},{probe},{status}",
        start.0, end.0
    )
}

/// In strict mode, reject a non-covered epoch before emitting any CSV.
fn checked_csv_report<R: Read + Seek>(
    ephemeris: &De441<R>,
    probe_jd_tdb: Option<f64>,
    require_coverage: bool,
) -> Result<String, String> {
    if require_coverage {
        let jd = probe_jd_tdb.ok_or("strict coverage requires --at-jd")?;
        let (start, end) = ephemeris.coverage();
        if jd < start.0 || jd > end.0 {
            return Err(format!(
                "JD TDB {jd} is outside declared DE441 coverage [{}, {}] JD TDB",
                start.0, end.0
            ));
        }
    }
    Ok(csv_report(ephemeris, probe_jd_tdb))
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let options = parse_args(&args)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    // De441::open validates the classic header and declared file length.
    // This intentionally does not read or certify any coefficient values.
    let ephemeris = De441::open(&options.file)?;
    let report = checked_csv_report(&ephemeris, options.probe_jd_tdb, options.require_coverage)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    println!("{report}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("DE441 coverage inspection failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const RECORD_BYTES: usize = 3000;

    fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    // Minimal synthetic classic DE441 shape, not a scientific ephemeris.
    fn synthetic_de441() -> De441<Cursor<Vec<u8>>> {
        let mut bytes = vec![0u8; 3 * RECORD_BYTES];
        let title = b"JPL Planetary Ephemeris DE441";
        bytes[..title.len()].copy_from_slice(title);
        put_f64(&mut bytes, 2652, 1000.0);
        put_f64(&mut bytes, 2660, 1032.0);
        put_f64(&mut bytes, 2668, 32.0);
        put_f64(&mut bytes, 2688, 81.0);
        put_i32(&mut bytes, 2840, 441);
        put_i32(&mut bytes, 2844, 373);
        put_i32(&mut bytes, 2848, 1);
        put_i32(&mut bytes, 2852, 1);
        put_f64(&mut bytes, RECORD_BYTES * 2, 1000.0);
        put_f64(&mut bytes, RECORD_BYTES * 2 + 8, 1032.0);
        De441::new(Cursor::new(bytes)).unwrap()
    }

    #[test]
    fn reports_declared_coverage_without_probe() {
        let report = csv_report(&synthetic_de441(), None);
        assert_eq!(
            report,
            "format,epoch_scale,declared_start_jd,declared_end_jd,probe_jd,probe_status\n\
             classic-de441,TDB,1000.000000000000,1032.000000000000,,not_checked"
        );
    }

    #[test]
    fn inclusive_endpoints_and_interior_are_covered() {
        let ephemeris = synthetic_de441();
        for epoch in [1000.0, 1016.0, 1032.0] {
            assert!(csv_report(&ephemeris, Some(epoch)).ends_with(",covered"));
        }
    }

    #[test]
    fn out_of_range_dates_are_explicit_not_silent() {
        let ephemeris = synthetic_de441();
        for epoch in [999.999_999, 1032.000_001] {
            assert!(csv_report(&ephemeris, Some(epoch)).ends_with(",out_of_coverage"));
        }
    }

    #[test]
    fn requires_file_and_explicit_probe_flag() {
        for args in [
            vec![],
            vec!["--at-jd".to_owned()],
            vec!["file.eph".to_owned(), "2451545".to_owned()],
            vec!["file.eph".to_owned(), "--at-jd".to_owned()],
            vec![
                "file.eph".to_owned(),
                "--at-jd".to_owned(),
                "1".to_owned(),
                "extra".to_owned(),
            ],
        ] {
            assert!(parse_args(&args).is_err());
        }
    }

    #[test]
    fn finite_tdb_jd_is_required() {
        for value in ["NaN", "inf", "-inf", "abc"] {
            assert!(parse_args(&["file.eph".into(), "--at-jd".into(), value.into()]).is_err());
        }
        let args = vec!["file.eph".into(), "--at-jd".into(), "-3027215.5".into()];
        let parsed = parse_args(&args).unwrap();
        assert_eq!(parsed.file, "file.eph");
        assert_eq!(parsed.probe_jd_tdb, Some(-3_027_215.5));
        assert!(!parsed.require_coverage);
    }

    #[test]
    fn strict_flag_requires_one_explicit_tdb_probe() {
        let valid = vec![
            "file.eph".into(),
            "--at-jd".into(),
            "2451545.0".into(),
            "--require-coverage".into(),
        ];
        let options = parse_args(&valid).unwrap();
        assert_eq!(options.probe_jd_tdb, Some(2_451_545.0));
        assert!(options.require_coverage);

        for args in [
            vec!["file.eph".into(), "--require-coverage".into()],
            vec![
                "file.eph".into(),
                "--require-coverage".into(),
                "--at-jd".into(),
                "2451545".into(),
            ],
            vec![
                "file.eph".into(),
                "--at-jd".into(),
                "2451545".into(),
                "--require-coverage".into(),
                "--require-coverage".into(),
            ],
        ] {
            assert!(parse_args(&args).is_err());
        }
    }

    #[test]
    fn strict_mode_accepts_inclusive_endpoints_and_retains_csv_contract() {
        let ephemeris = synthetic_de441();
        for jd in [1000.0, 1016.0, 1032.0] {
            assert_eq!(
                checked_csv_report(&ephemeris, Some(jd), true).unwrap(),
                csv_report(&ephemeris, Some(jd))
            );
        }
    }

    #[test]
    fn strict_mode_rejects_both_sides_before_csv_but_diagnostic_mode_remains_compatible() {
        let ephemeris = synthetic_de441();
        for jd in [999.999_999, 1032.000_001] {
            let error = checked_csv_report(&ephemeris, Some(jd), true).unwrap_err();
            assert!(error.contains("outside declared DE441 coverage"));
            assert!(checked_csv_report(&ephemeris, Some(jd), false)
                .unwrap()
                .ends_with(",out_of_coverage"));
        }
        assert!(checked_csv_report(&ephemeris, None, true).is_err());
        assert!(checked_csv_report(&ephemeris, None, false)
            .unwrap()
            .ends_with(",not_checked"));
    }
}
