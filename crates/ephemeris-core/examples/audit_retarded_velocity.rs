// SPDX-License-Identifier: CPAL-1.0
//! Offline diagnostic: light-time velocity and the unretarded-velocity hybrid.
//! Neither state includes stellar aberration or gravitational deflection.
//! The hybrid is deliberately not the derivative of the returned position.
//! Usage: `audit_retarded_velocity KERNEL earth|ssb sun|moon|mercury|jupiter JD_TDB`
use std::error::Error;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::light_time::light_time_with_states;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("usage: KERNEL earth|ssb sun|moon|mercury|jupiter JD_TDB".into());
    }
    let observer = match args[2].as_str() {
        "earth" => Body::Earth,
        "ssb" => Body::SolarSystemBarycenter,
        _ => return Err("unknown observer".into()),
    };
    let target = match args[3].as_str() {
        "sun" => Body::Sun,
        "moon" => Body::Moon,
        "mercury" => Body::Mercury,
        "jupiter" => Body::JupiterBarycenter,
        _ => return Err("unknown target".into()),
    };
    let jd = JulianDateTdb(args[4].parse()?);
    let mut eph = De441::open(&args[1])?;
    let light = light_time_with_states(observer, target, jd, |body, epoch| {
        eph.state(body, Body::SolarSystemBarycenter, epoch)
    })?;
    let observer_state = eph.state(observer, Body::SolarSystemBarycenter, jd)?;
    let emitted = eph.state(target, Body::SolarSystemBarycenter, light.emission_epoch)?;
    let hybrid: [f64; 3] = std::array::from_fn(|i| {
        emitted.velocity_km_per_day[i] - observer_state.velocity_km_per_day[i]
    });
    println!(
        "x,y,z,correct_vx,correct_vy,correct_vz,hybrid_vx,hybrid_vy,hybrid_vz,light_time_rate"
    );
    println!(
        "{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
        light.position_km[0],
        light.position_km[1],
        light.position_km[2],
        light.velocity_km_per_day[0],
        light.velocity_km_per_day[1],
        light.velocity_km_per_day[2],
        hybrid[0],
        hybrid[1],
        hybrid[2],
        light.light_time_rate
    );
    Ok(())
}
