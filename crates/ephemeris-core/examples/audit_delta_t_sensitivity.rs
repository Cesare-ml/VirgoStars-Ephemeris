// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Owner-local, fixed-UT1 Moon sensitivity samples using the existing provider.
use std::{
    error::Error,
    io::{self, BufRead},
};
use virgostars_ephemeris_core::{
    de441::Body, de441_pack_manager::De441PackManager,
    historical_delta_t_scenarios::historical_delta_t_scenarios_with_provider,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().ok_or("usage: EXE PACK_DIR")?;
    if args.next().is_some() {
        return Err("usage: EXE PACK_DIR".into());
    }
    let mut manager = De441PackManager::from_mobile_directory(path)?;
    for line in io::stdin().lock().lines() {
        let text = line?;
        let values = text
            .split(',')
            .map(str::parse::<f64>)
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() != 3 {
            return Err("expected jd_ut1,delta_t,perturbation".into());
        }
        let scenarios = historical_delta_t_scenarios_with_provider(
            &mut manager,
            Body::Moon,
            values[0],
            values[1],
            values[2],
            "reference-sensitivity-example",
        )
        .map_err(|error| io::Error::other(format!("{error:?}")))?;
        let fields =
            |x: virgostars_ephemeris_core::historical_delta_t_scenarios::HistoricalScenario| {
                (
                    x.fields.longitude_deg,
                    x.fields.latitude_deg,
                    x.fields.distance_au,
                )
            };
        let (a, b, c) = (
            fields(scenarios.lower),
            fields(scenarios.nominal),
            fields(scenarios.upper),
        );
        println!(
            "{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",
            values[0], values[1], values[2], a.0, a.1, a.2, b.0, b.1, b.2, c.0, c.1, c.2
        );
    }
    Ok(())
}
