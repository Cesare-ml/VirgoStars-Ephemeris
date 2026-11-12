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

//! Offline, experimental fixed-ICRF CSV export from the *single* Rust
//! astronomical core. No JPL file is bundled or automatically downloaded.
//!
//! Explicit `tdb:<JD>` avoids an implicit timescale conversion in
//! numerical comparisons against independent JPL Horizons references.

use std::env;
use std::error::Error;
use std::io::{self, BufWriter, Write};

use virgostars_ephemeris_core::aberrated_spherical::{
    geocentric_aberrated_fixed_six_fields_many, AberratedFixedSixFieldsDegreesAu,
};
use virgostars_ephemeris_core::civil_aberrated::geocentric_aberrated_fixed_six_fields_many_from_civil;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::first_sky::CivilTimestamp;

const HEADER: &str = "target,center,reception_tdb_jd,emission_tdb_jd,fixed_tilt_rad,longitude_deg,latitude_deg,distance_au,longitude_speed_deg_per_tdb_day,latitude_speed_deg_per_tdb_day,distance_speed_au_per_tdb_day,light_time_seconds,light_time_rate,observer_acceleration_source,frame,corrections";

const DEFAULT_TARGETS: [Body; 10] = [
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

fn usage_error(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn parse_body(input: &str) -> Result<Body, io::Error> {
    match input {
        "Sun" => Ok(Body::Sun),
        "Moon" => Ok(Body::Moon),
        "Mercury" => Ok(Body::Mercury),
        "Venus" => Ok(Body::Venus),
        "MarsBarycenter" => Ok(Body::MarsBarycenter),
        "JupiterBarycenter" => Ok(Body::JupiterBarycenter),
        "SaturnBarycenter" => Ok(Body::SaturnBarycenter),
        "UranusBarycenter" => Ok(Body::UranusBarycenter),
        "NeptuneBarycenter" => Ok(Body::NeptuneBarycenter),
        "PlutoBarycenter" => Ok(Body::PlutoBarycenter),
        _ => Err(usage_error(format!(
            "unsupported geocentric target: {input} (use the exact Body variant spelling)"
        ))),
    }
}

#[derive(Debug)]
enum EpochInput {
    Civil(CivilTimestamp),
    Tdb(JulianDateTdb),
}

fn parse_epoch_input(input: &str) -> Result<EpochInput, Box<dyn Error>> {
    if let Some(jd) = input.strip_prefix("tdb:") {
        let value: f64 = jd.parse()?;
        if !value.is_finite() {
            return Err(usage_error("TDB Julian date must be finite").into());
        }
        return Ok(EpochInput::Tdb(JulianDateTdb(value)));
    }
    // No silently assumed timezone. The existing parser requires an
    // explicit Z or ±HH:MM offset and enforces its supported UTC range.
    Ok(EpochInput::Civil(CivilTimestamp::parse(input)?))
}

/// Writes a single metadata-explicit, machine-readable CSV table.
///
/// No observations or numerical approximations are implemented here:
/// each field comes from the existing Rust core. The row labels and
/// order are stable for reproducible offline comparisons.
/// The full batch has already succeeded before this writer is called.
fn write_csv<W: Write>(
    output: &mut W,
    epoch: JulianDateTdb,
    positions: &[AberratedFixedSixFieldsDegreesAu],
) -> io::Result<()> {
    writeln!(output, "{HEADER}")?;
    for item in positions {
        let f = item.fields;
        writeln!(
            output,
            "{:?},{:?},{:.12},{:.12},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:?},fixed_ICRF_equatorial,LT_plus_annual_SR_aberration",
            f.target,
            f.center,
            epoch.0,
            item.emission_epoch.0,
            f.fixed_tilt_rad,
            f.longitude_deg,
            f.latitude_deg,
            f.distance_au,
            f.longitude_speed_deg_per_tdb_day,
            f.latitude_speed_deg_per_tdb_day,
            f.distance_speed_au_per_tdb_day,
            item.light_time_seconds,
            item.light_time_rate,
            item.observer_acceleration_source,
        )?;
    }
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 2 {
        return Err(usage_error(
            "usage: aberrated_csv <classic-de441.eph> <ISO-civil-with-offset|tdb:JD> [Sun Moon Mercury Venus MarsBarycenter JupiterBarycenter ...]",
        )
        .into());
    }
    let instant = parse_epoch_input(&args[1])?;
    let targets: Vec<Body> = if args.len() == 2 {
        DEFAULT_TARGETS.to_vec()
    } else {
        args[2..]
            .iter()
            .map(|argument| parse_body(argument))
            .collect::<Result<_, _>>()?
    };

    let mut eph = De441::open(&args[0])?;
    let (epoch, results) = match instant {
        EpochInput::Civil(civil) => {
            let result =
                geocentric_aberrated_fixed_six_fields_many_from_civil(&mut eph, &targets, civil, 0.0)?;
            (result.reception_epoch, result.results)
        }
        EpochInput::Tdb(epoch) => (
            epoch,
            geocentric_aberrated_fixed_six_fields_many(&mut eph, &targets, epoch, 0.0)?,
        ),
    };

    // Compute the entire batch *before* writing stdout. Invalid targets,
    // malformed civil times or missing file records cannot cause partial
    // scientifically plausible CSV output.
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    write_csv(&mut writer, epoch, &results)?;
    writer.flush()?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("aberrated_csv: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use virgostars_ephemeris_core::aberration_state::ObserverAccelerationSource;
    use virgostars_ephemeris_core::six_field_units::FixedSixFieldsDegreesAu;

    #[test]
    fn parsing_requires_an_explicit_time_scale_or_civil_offset() {
        assert!(matches!(
            parse_epoch_input("tdb:2461354.5").unwrap(),
            EpochInput::Tdb(JulianDateTdb(2_461_354.5))
        ));
        assert!(parse_epoch_input("tdb:NaN").is_err());
        assert!(parse_epoch_input("tdb:Infinity").is_err());
        assert!(parse_epoch_input("2026-11-10T12:30:00").is_err());
        assert!(matches!(
            parse_epoch_input("2026-11-10T12:30:00Z").unwrap(),
            EpochInput::Civil(_)
        ));
        assert!(matches!(
            parse_epoch_input("2026-11-10T13:30:00+01:00").unwrap(),
            EpochInput::Civil(_)
        ));
    }

    #[test]
    fn body_parser_retains_system_barycenter_identity_and_rejects_earth() {
        assert_eq!(parse_body("MarsBarycenter").unwrap(), Body::MarsBarycenter);
        assert_eq!(parse_body("JupiterBarycenter").unwrap(), Body::JupiterBarycenter);
        assert_eq!(parse_body("Moon").unwrap(), Body::Moon);
        assert!(parse_body("Earth").is_err());
        assert!(parse_body("Mars").is_err());
        assert!(parse_body("Unknown").is_err());
        assert_eq!(DEFAULT_TARGETS.len(), 10);
    }

    #[test]
    fn csv_contains_scientific_provenance_columns_without_losing_signed_rates() {
        // Writer-only fixture. These values do NOT represent a planet.
        let fake = AberratedFixedSixFieldsDegreesAu {
            fields: FixedSixFieldsDegreesAu {
                target: Body::Mercury,
                center: Body::Earth,
                epoch: JulianDateTdb(2_461_354.5),
                fixed_tilt_rad: 0.0,
                longitude_deg: 359.5,
                latitude_deg: -1.0,
                distance_au: 0.85,
                longitude_speed_deg_per_tdb_day: -0.75,
                latitude_speed_deg_per_tdb_day: 0.002,
                distance_speed_au_per_tdb_day: -0.012,
            },
            emission_epoch: JulianDateTdb(2_461_354.5 - 2.0 / 86_400.0),
            light_time_seconds: 2.0,
            light_time_rate: -1.0e-5,
            observer_acceleration_source: ObserverAccelerationSource::AnalyticDe441Chebyshev,
        };
        let mut output = Vec::new();
        write_csv(&mut output, JulianDateTdb(2_461_354.5), &[fake, fake]).unwrap();
        let text = String::from_utf8(output).unwrap();
        let rows: Vec<&str> = text.lines().collect();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], HEADER);
        assert_eq!(rows[1], rows[2]); // duplicates are intentional
        let cells: Vec<&str> = rows[1].split(',').collect();
        assert_eq!(cells.len(), 16);
        assert_eq!(cells[0], "Mercury");
        assert_eq!(cells[1], "Earth");
        assert_eq!(cells[7].parse::<f64>().unwrap(), 0.85);
        assert_eq!(cells[8].parse::<f64>().unwrap(), -0.75);
        assert_eq!(cells[10].parse::<f64>().unwrap(), -0.012);
        assert_eq!(cells[13], "AnalyticDe441Chebyshev");
        assert_eq!(cells[14], "fixed_ICRF_equatorial");
        assert_eq!(cells[15], "LT_plus_annual_SR_aberration");
    }
}
