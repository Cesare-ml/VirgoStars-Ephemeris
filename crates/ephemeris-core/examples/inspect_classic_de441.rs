// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

// Modification record: 2026-11-13 — added a read-only classic DE441
// coverage diagnostic with synthetic parser and CLI argument regressions.

//! Inspect the declared epoch coverage of an externally supplied classic DE441
//! binary. This is header validation, NOT a scientific accuracy check.
//!
//! Run:
//! cargo run --offline --locked --example inspect_classic_de441 -- /path/to/de441.eph

use std::env;
use std::ffi::OsString;
use std::io::{Read, Seek};
use std::path::PathBuf;
use std::process::ExitCode;

use virgostars_ephemeris_core::de441::{De441, De441Error, JulianDateTdb};

const USAGE: &str = "Usage: inspect_classic_de441 <path-to-classic-de441-file>";

#[derive(Debug, PartialEq, Eq)]
enum Request {
    Help,
    File(PathBuf),
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Request, &'static str> {
    let mut args = args.into_iter();
    let path = args.next().ok_or("a local classic DE441 file path is required")?;
    if args.next().is_some() {
        return Err("exactly one file path is required");
    }
    if path == OsString::from("--help") || path == OsString::from("-h") {
        return Ok(Request::Help);
    }
    if path.is_empty() {
        return Err("file path cannot be empty");
    }
    Ok(Request::File(PathBuf::from(path)))
}

fn coverage_csv((start, end): (JulianDateTdb, JulianDateTdb)) -> String {
    // Decimal f64 display retains enough digits for a round trip. This is
    // header metadata, not a precision/accuracy assertion about the kernel.
    format!(
        "format,coverage_start_jd_tdb,coverage_end_jd_tdb\nclassic-de441,{},{}\n",
        start.0, end.0
    )
}

fn inspect<R: Read + Seek>(source: R) -> Result<String, De441Error> {
    let ephemeris = De441::new(source)?;
    Ok(coverage_csv(ephemeris.coverage()))
}

fn main() -> ExitCode {
    match parse_args(env::args_os().skip(1)) {
        Ok(Request::Help) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Request::File(path)) => match std::fs::File::open(&path) {
            Ok(file) => match inspect(file) {
                Ok(csv) => {
                    print!("{csv}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("DE441 inspection failed: {error}");
                    ExitCode::FAILURE
                }
            },
            Err(error) => {
                eprintln!("Cannot open classic DE441 file: {error}");
                ExitCode::FAILURE
            }
        },
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn put_f64(bytes: &mut [u8], at: usize, number: f64) {
        bytes[at..at + 8].copy_from_slice(&number.to_le_bytes());
    }

    fn put_i32(bytes: &mut [u8], at: usize, number: i32) {
        bytes[at..at + 4].copy_from_slice(&number.to_le_bytes());
    }

    // A synthetic, one-record *classic* DE441 layout, not a NASA/JPL fixture.
    fn synthetic_header_and_record() -> Vec<u8> {
        let mut bytes = vec![0u8; 3 * 3000];
        let title = b"JPL Planetary Ephemeris DE441";
        bytes[..title.len()].copy_from_slice(title);
        put_f64(&mut bytes, 2652, 1000.0);
        put_f64(&mut bytes, 2660, 1032.0);
        put_f64(&mut bytes, 2668, 32.0);
        put_f64(&mut bytes, 2688, 81.0);
        put_i32(&mut bytes, 2840, 441);
        // Unused 3-component series fixes the inferred record size at 3000 B.
        put_i32(&mut bytes, 2844, 373);
        put_i32(&mut bytes, 2848, 1);
        put_i32(&mut bytes, 2852, 1);
        put_f64(&mut bytes, 6000, 1000.0);
        put_f64(&mut bytes, 6008, 1032.0);
        bytes
    }

    #[test]
    fn accepts_one_path_or_help_only() {
        assert_eq!(
            parse_args([OsString::from("test.eph")]),
            Ok(Request::File(PathBuf::from("test.eph")))
        );
        assert_eq!(parse_args([OsString::from("--help")]), Ok(Request::Help));
        assert!(parse_args(Vec::<OsString>::new()).is_err());
        assert!(parse_args([OsString::from("a.eph"), OsString::from("b.eph")]).is_err());
        assert!(parse_args([OsString::new()]).is_err());
    }

    #[test]
    fn reports_validated_classic_header_coverage_without_state_interpolation() {
        let csv = inspect(Cursor::new(synthetic_header_and_record())).unwrap();
        assert_eq!(
            csv,
            "format,coverage_start_jd_tdb,coverage_end_jd_tdb\nclassic-de441,1000,1032\n"
        );
    }

    #[test]
    fn corrupt_and_short_files_fail_without_a_partial_csv_result() {
        let mut corrupt = synthetic_header_and_record();
        corrupt[0] = b'X';
        assert!(inspect(Cursor::new(corrupt)).is_err());
        let mut short = synthetic_header_and_record();
        short.truncate(8500);
        assert!(inspect(Cursor::new(short)).is_err());
    }
}
