// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Stream caller-supplied decimal years to the existing opt-in ΔT model.
//! No EOP, calendar, uncertainty or civil-time policy is implied.
use std::io::{self, BufRead};
use virgostars_ephemeris_core::delta_t_long_range::espenak_meeus_2006_long_range;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let line = line?;
        let year: f64 = line.trim().parse()?;
        let seconds = espenak_meeus_2006_long_range(year)?;
        println!("{year:.12},{seconds:.12}");
    }
    Ok(())
}
