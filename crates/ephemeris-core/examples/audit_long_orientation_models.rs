// SPDX-License-Identifier: CPAL-1.0. Diagnostic export, no new frame model.
//! Read TT Julian dates from stdin; export existing experimental nutation and
//! long-term mean axes for independent comparison with caller-supplied ERFA.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::earth_orientation::experimental_extrapolated_nutation;
use virgostars_ephemeris_core::long_term_precession::mean_date_matrices;

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().len() != 1 {
        return Err("usage: audit_long_orientation_models < dates.txt".into());
    }
    for line in io::stdin().lock().lines() {
        let jd: f64 = line?.parse()?;
        let nut = experimental_extrapolated_nutation(jd)?;
        let (equator, ecliptic) = mean_date_matrices(jd)?;
        let mut fields = vec![
            format!("{jd:.9}"),
            format!("{:.17e}", nut.d_psi_rad),
            format!("{:.17e}", nut.d_epsilon_rad),
        ];
        fields.extend(
            equator
                .iter()
                .flatten()
                .chain(ecliptic[2].iter())
                .map(|x| format!("{x:.17e}")),
        );
        println!("{}", fields.join(","));
    }
    Ok(())
}
