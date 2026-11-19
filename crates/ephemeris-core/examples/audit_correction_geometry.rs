// SPDX-License-Identifier: CPAL-1.0. Exploratory geometry export, not an API.
//! stdin: body ID (1 Moon, 2 Mercury, 3 Venus, 5 Jupiter barycenter), JD TDB.
//! Outputs the existing light-time, deflection, aberration and barycentric
//! states for external comparison; it never implements a separate model.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::aberration_state::aberrate_light_time_state;
use virgostars_ephemeris_core::apparent_ecliptic_of_date::tdb_to_tt_jd;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::light_deflection::deflect_light_time_state_by_sun;
use virgostars_ephemeris_core::light_time::geocentric_light_time;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: audit_correction_geometry KERNEL < cases.csv".into());
    }
    let mut eph = De441::open(&args[1])?;
    for input in io::stdin().lock().lines() {
        let input = input?;
        let (id, jd) = input.split_once(',').ok_or("expected ID,JD")?;
        let target = match id {
            "1" => Body::Moon,
            "2" => Body::Mercury,
            "3" => Body::Venus,
            "5" => Body::JupiterBarycenter,
            _ => return Err("invalid ID".into()),
        };
        let t = JulianDateTdb(jd.parse()?);
        let light = geocentric_light_time(&mut eph, target, t)?;
        let earth = eph.state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, t)?;
        let sun = eph.state(Body::Sun, Body::SolarSystemBarycenter, t)?;
        let source = eph.state(target, Body::SolarSystemBarycenter, light.emission_epoch)?;
        let emitted_sun =
            eph.state(Body::Sun, Body::SolarSystemBarycenter, light.emission_epoch)?;
        let deflected = deflect_light_time_state_by_sun(&mut eph, &light)?;
        let apparent = aberrate_light_time_state(
            &deflected,
            earth.velocity_km_per_day,
            earth.acceleration_km_per_day2,
        )?;
        let values = [
            light.position_km,
            deflected.position_km,
            apparent.position_km,
            earth.position_km,
            earth.velocity_km_per_day,
            sun.position_km,
            source.position_km,
            emitted_sun.position_km,
        ];
        let mut output = vec![
            id.to_owned(),
            format!("{:.9}", t.0),
            format!("{:.9}", tdb_to_tt_jd(t.0)?),
        ];
        output.extend(values.iter().flatten().map(|x| format!("{x:.17e}")));
        println!("{}", output.join(","));
    }
    Ok(())
}
