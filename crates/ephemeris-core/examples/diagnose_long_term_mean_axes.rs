// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Experimental mean equator and ecliptic matrices for caller-supplied TT JD.
//! Output matrices map mean J2000 equatorial axes to mean axes of date.
//! No bias, nutation or physical accuracy claim is included.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::long_term_precession::mean_date_matrices;

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().len() != 1 {
        return Err("usage: diagnose_long_term_mean_axes < tt-jd-lines".into());
    }
    for line in io::stdin().lock().lines() {
        let jd: f64 = line?.parse()?;
        let (eq, ecl) = mean_date_matrices(jd)?;
        let values = std::iter::once(jd)
            .chain(eq.into_iter().flatten())
            .chain(ecl.into_iter().flatten());
        println!(
            "{}",
            values
                .map(|x| format!("{x:.17}"))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    Ok(())
}
