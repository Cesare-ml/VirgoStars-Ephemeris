// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Four experimental apparent ICRF paths, with no physical-accuracy claim.
//! Input lines are `body-id,jd-tdb`; output is a CSV of angles and rates.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::aberration_state::{
    aberrate_light_time_state, AberratedGeocentricState,
};
use virgostars_ephemeris_core::apparent_ecliptic_of_date::experimental_apparent_icrf;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::experimental_analytic_apparent::{
    experimental_analytic_apparent_icrf, experimental_analytic_apparent_icrf_refined,
};
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
fn angles(state: AberratedGeocentricState) -> Result<[f64; 4], Box<dyn Error>> {
    let p = state.position_km;
    let v = state.velocity_km_per_tdb_day;
    let r = p[0].hypot(p[1]).hypot(p[2]);
    if !r.is_finite() || r <= 0.0 {
        return Err("invalid range".into());
    }
    let u = p.map(|x| x / r);
    let radial: f64 = u.iter().zip(v).map(|(a, b)| a * b).sum();
    let d: [f64; 3] = std::array::from_fn(|i| (v[i] - u[i] * radial) / r);
    let rho = u[0].hypot(u[1]);
    if !rho.is_finite() || rho < 1e-10 {
        return Err("undefined right ascension".into());
    }
    let arcsec = 206_264.806_247_096_36;
    Ok([
        u[1].atan2(u[0]).to_degrees().rem_euclid(360.0),
        u[2].atan2(rho).to_degrees(),
        (u[0] * d[1] - u[1] * d[0]) / rho * arcsec,
        d[2] / rho * arcsec,
    ])
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: compare_emission_deflection_matrix DE441 < cases.csv".into());
    }
    let mut eph = De441::open(&args[1])?;
    println!("body,jd_tdb,old_unref_ra,old_unref_dec,old_unref_dra,old_unref_ddec,analytic_unref_ra,analytic_unref_dec,analytic_unref_dra,analytic_unref_ddec,old_ref_ra,old_ref_dec,old_ref_dra,old_ref_ddec,analytic_ref_ra,analytic_ref_dec,analytic_ref_dra,analytic_ref_ddec");
    for line in io::stdin().lock().lines() {
        let line = line?;
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 2 {
            return Err("expected body-id,jd-tdb".into());
        }
        let id: u8 = fields[0].parse()?;
        let target = body(id).ok_or("unsupported body")?;
        let epoch = JulianDateTdb(fields[1].parse()?);
        if !epoch.0.is_finite() {
            return Err("invalid epoch".into());
        }
        let earth = eph.state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, epoch)?;
        let lt = geocentric_light_time(&mut eph, target, epoch)?;
        let lt = deflect_light_time_state_by_sun(&mut eph, &lt)?;
        let old = aberrate_light_time_state(
            &lt,
            earth.velocity_km_per_day,
            earth.acceleration_km_per_day2,
        )?;
        let values = [
            angles(old)?,
            angles(experimental_analytic_apparent_icrf(
                &mut eph, target, epoch,
            )?)?,
            angles(experimental_apparent_icrf(&mut eph, target, epoch)?)?,
            angles(experimental_analytic_apparent_icrf_refined(
                &mut eph, target, epoch,
            )?)?,
        ];
        print!("{id},{:.17}", epoch.0);
        for x in values.into_iter().flatten() {
            print!(",{x:.15}");
        }
        println!();
    }
    Ok(())
}
