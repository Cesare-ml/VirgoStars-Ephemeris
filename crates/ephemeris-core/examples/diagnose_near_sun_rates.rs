// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Offline diagnostic only: compare reported rates to derivatives of each
//! ICRF stage's positions. Binary-exact offsets reduce JD-step rounding noise.
//! Not an independent physical accuracy assessment.
use std::error::Error;
use virgostars_ephemeris_core::aberration_state::aberrate_light_time_state;
use virgostars_ephemeris_core::apparent_ecliptic_of_date::experimental_apparent_icrf;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::light_deflection::deflect_light_time_state_by_sun;
use virgostars_ephemeris_core::light_time::geocentric_light_time;

type State = ([f64; 3], [f64; 3]);

fn stages(eph: &mut De441<std::fs::File>, jd: f64) -> Result<[State; 4], Box<dyn Error>> {
    let t = JulianDateTdb(jd);
    let earth = eph.state_with_acceleration(Body::Earth, Body::SolarSystemBarycenter, t)?;
    let lt = geocentric_light_time(eph, Body::UranusBarycenter, t)?;
    let def = deflect_light_time_state_by_sun(eph, &lt)?;
    let aberr = aberrate_light_time_state(
        &def,
        earth.velocity_km_per_day,
        earth.acceleration_km_per_day2,
    )?;
    let full = experimental_apparent_icrf(eph, Body::UranusBarycenter, t)?;
    Ok([
        (lt.position_km, lt.velocity_km_per_day),
        (def.position_km, def.velocity_km_per_day),
        (aberr.position_km, aberr.velocity_km_per_tdb_day),
        (full.position_km, full.velocity_km_per_tdb_day),
    ])
}
fn angle(p: [f64; 3]) -> f64 {
    p[1].atan2(p[0])
}
fn rate((p, v): State) -> f64 {
    (p[0] * v[1] - p[1] * v[0]) / (p[0] * p[0] + p[1] * p[1])
}
fn wrap(x: f64) -> f64 {
    (x + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: diagnose_near_sun_rates <classic-de441> <tdb-jd>".into());
    }
    let jd: f64 = args[2].parse()?;
    if !jd.is_finite() {
        return Err("invalid epoch".into());
    }
    let mut eph = De441::open(&args[1])?;
    let center = stages(&mut eph, jd)?;
    println!("jd_tdb,half_step_days,stage,rate_minus_position_derivative_arcsec_per_day");
    for n in [11, 10, 9, 8, 7, 6] {
        let h = 2f64.powi(-n);
        let m2 = stages(&mut eph, jd - 2.0 * h)?;
        let m1 = stages(&mut eph, jd - h)?;
        let p1 = stages(&mut eph, jd + h)?;
        let p2 = stages(&mut eph, jd + 2.0 * h)?;
        for (i, label) in [
            "light_time",
            "deflection",
            "aberration",
            "experimental_full",
        ]
        .iter()
        .enumerate()
        {
            let a = angle(center[i].0);
            let diff = (wrap(angle(m2[i].0) - a) - 8.0 * wrap(angle(m1[i].0) - a)
                + 8.0 * wrap(angle(p1[i].0) - a)
                - wrap(angle(p2[i].0) - a))
                / (12.0 * h);
            println!(
                "{jd},{h},{label},{:.12}",
                (rate(center[i]) - diff) * 206_264.806_247_096_36
            );
        }
        // Isolate the deflection contribution and compare its reported rate
        // with a two-point derivative of the deflection-only angle.
        let correction = |s: [State; 4]| wrap(angle(s[1].0) - angle(s[0].0));
        let centered = wrap(correction(p1) - correction(m1)) / (2.0 * h);
        let reported = rate(center[1]) - rate(center[0]);
        println!(
            "{jd},{h},deflection_only_centered,{:.12}",
            (reported - centered) * 206_264.806_247_096_36
        );
    }
    Ok(())
}
