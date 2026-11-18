// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Isolated experimental nutation components at caller-supplied TT epochs.
//! Input: one TT Julian date per line. No physical-accuracy claim.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::earth_orientation::{
    experimental_extrapolated_nutation, experimental_linear_argument_nutation,
};

fn evaluate(jd: f64) -> Result<[f64; 5], Box<dyn Error>> {
    let n = experimental_extrapolated_nutation(jd)?;
    let linear = experimental_linear_argument_nutation(jd)?;
    Ok([
        jd,
        n.d_psi_arcsec,
        n.d_epsilon_arcsec,
        linear.d_psi_arcsec,
        linear.d_epsilon_arcsec,
    ])
}

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().len() != 1 {
        return Err("usage: diagnose_experimental_nutation < tt-jd-lines".into());
    }
    for line in io::stdin().lock().lines() {
        let row = evaluate(line?.parse()?)?;
        println!(
            "{:.17},{:.15},{:.15},{:.15},{:.15}",
            row[0], row[1], row[2], row[3], row[4]
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::evaluate;
    #[test]
    fn checks_j2000_and_invalid_epochs() {
        let r = evaluate(2_451_545.0).unwrap();
        // At J2000 the full and linear fundamental arguments agree.
        assert!((r[1] - (-6.754_261_253_992_235e-5_f64).to_degrees() * 3600.0).abs() < 1e-9);
        assert!((r[2] - (-2.797_092_331_098_565_3e-5_f64).to_degrees() * 3600.0).abs() < 1e-9);
        assert!(evaluate(f64::NAN).is_err());
        assert!(evaluate(1e9).is_err());
        assert!(super::experimental_linear_argument_nutation(f64::NAN).is_err());
        assert!(super::experimental_linear_argument_nutation(1e9).is_err());
        for jd in [2_415_020.0, 2_451_545.0, 2_453_736.5, 2_488_069.0] {
            let experimental = super::experimental_linear_argument_nutation(jd).unwrap();
            let standard =
                virgostars_ephemeris_core::earth_orientation::iau2000b_nutation(jd).unwrap();
            assert_eq!(experimental, standard);
        }
    }
}
