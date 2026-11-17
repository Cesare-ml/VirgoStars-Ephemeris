// SPDX-License-Identifier: CPAL-1.0
//! Offline experiment to isolate rate contributions; not a production model.
//! Input lines on stdin: `<body-id>,<TDB-JD>`. Output is CSV Cartesian states.
//! The intentionally incomplete alternatives must never be used as an apparent
//! ephemeris: they test hypotheses about another implementation's rate output.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::aberration_state::{
    aberrate_light_time_state, AberratedGeocentricState,
};
use virgostars_ephemeris_core::apparent_ecliptic_of_date::experimental_apparent_icrf;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::light_deflection::deflect_light_time_state_by_sun;
use virgostars_ephemeris_core::light_time::geocentric_light_time;

fn body(id: u8) -> Option<Body> {
    Some(match id {
        0 => Body::Sun,
        1 => Body::Moon,
        2 => Body::Mercury,
        3 => Body::Venus,
        4 => Body::MarsBarycenter,
        5 => Body::JupiterBarycenter,
        6 => Body::SaturnBarycenter,
        7 => Body::UranusBarycenter,
        8 => Body::NeptuneBarycenter,
        9 => Body::PlutoBarycenter,
        _ => return None,
    })
}
fn print_state(id: u8, jd: f64, label: &str, state: AberratedGeocentricState) {
    let p = state.position_km;
    let v = state.velocity_km_per_tdb_day;
    println!(
        "{id},{jd:.9},{label},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
        p[0], p[1], p[2], v[0], v[1], v[2]
    );
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: diagnose_icrf_rate_components <classic-de441> < input.csv".into());
    }
    let mut kernel = De441::open(&args[1])?;
    for line in io::stdin().lock().lines() {
        let line = line?;
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 2 {
            return Err("expected body-id,TDB-JD".into());
        }
        let id: u8 = fields[0].parse()?;
        let target = body(id).ok_or("unknown body")?;
        let jd: f64 = fields[1].parse()?;
        if !jd.is_finite() {
            return Err("nonfinite TDB JD".into());
        }
        let t = JulianDateTdb(jd);
        let earth = kernel.state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, t)?;
        let lt = geocentric_light_time(&mut kernel, target, t)?;
        let emitted = kernel.state(target, Body::SolarSystemBarycenter, lt.emission_epoch)?;
        let deflected = deflect_light_time_state_by_sun(&mut kernel, &lt)?;
        let mut no_retarded_rate = lt;
        no_retarded_rate.velocity_km_per_day =
            std::array::from_fn(|i| emitted.velocity_km_per_day[i] - earth.velocity_km_per_day[i]);
        no_retarded_rate.light_time_rate = 0.0;
        let deflected_no_retard = deflect_light_time_state_by_sun(&mut kernel, &no_retarded_rate)?;
        print_state(
            id,
            jd,
            "full",
            experimental_apparent_icrf(&mut kernel, target, t)?,
        );
        print_state(
            id,
            jd,
            "base",
            aberrate_light_time_state(
                &deflected,
                earth.velocity_km_per_day,
                earth.acceleration_km_per_day2,
            )?,
        );
        print_state(
            id,
            jd,
            "no_earth_acceleration",
            aberrate_light_time_state(&deflected, earth.velocity_km_per_day, [0.0; 3])?,
        );
        print_state(
            id,
            jd,
            "no_retarded_rate",
            aberrate_light_time_state(
                &deflected_no_retard,
                earth.velocity_km_per_day,
                earth.acceleration_km_per_day2,
            )?,
        );
        print_state(
            id,
            jd,
            "neither",
            aberrate_light_time_state(&deflected_no_retard, earth.velocity_km_per_day, [0.0; 3])?,
        );
        // Deliberately mix the aberrated position with an untransformed rate
        // to test a convention hypothesis; this is not a physical state.
        let mut untransformed = aberrate_light_time_state(
            &deflected,
            earth.velocity_km_per_day,
            earth.acceleration_km_per_day2,
        )?;
        untransformed.velocity_km_per_tdb_day = deflected.velocity_km_per_day;
        print_state(id, jd, "untransformed", untransformed);
        untransformed.velocity_km_per_tdb_day = deflected_no_retard.velocity_km_per_day;
        print_state(id, jd, "untransformed_no_retard", untransformed);
        // Also omit the derivative of solar deflection, retaining the same
        // apparent position. This is another intentionally nonphysical state.
        untransformed.velocity_km_per_tdb_day = no_retarded_rate.velocity_km_per_day;
        print_state(
            id,
            jd,
            "untransformed_no_retard_no_deflection_rate",
            untransformed,
        );
    }
    Ok(())
}
