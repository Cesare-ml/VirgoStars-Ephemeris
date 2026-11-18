// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Compare two experimental nutation arguments on the same ICRF direction.
//! Input per line: TT JD, right ascension degrees, declination degrees.
//! This is a frame sensitivity study, not a physical accuracy determination.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::earth_orientation::{
    experimental_extrapolated_nutation, experimental_linear_argument_nutation,
    iau2006_bias_precession_matrix, matrix_multiply, matrix_vector_multiply, x_rotation,
    z_rotation, NutationAngles, J2000_TT_JD,
};
use virgostars_ephemeris_core::long_term_precession::mean_date_matrices;

fn project(tt: f64, ra: f64, dec: f64) -> Result<[f64; 12], Box<dyn Error>> {
    if ![tt, ra, dec].iter().all(|x| x.is_finite()) || !(-90.0..=90.0).contains(&dec) {
        return Err("invalid direction".into());
    }
    let (sr, cr) = ra.to_radians().sin_cos();
    let (sd, cd) = dec.to_radians().sin_cos();
    let v = [cd * cr, cd * sr, sd];
    let (eq, ecl) = mean_date_matrices(tt)?;
    let bias = iau2006_bias_precession_matrix(J2000_TT_JD)?;
    let eps = eq[2]
        .iter()
        .zip(ecl[2])
        .map(|(a, b)| a * b)
        .sum::<f64>()
        .clamp(-1.0, 1.0)
        .acos();
    let equatorial = |n: NutationAngles| {
        let nut = matrix_multiply(
            matrix_multiply(x_rotation(-eps - n.d_epsilon_rad), z_rotation(-n.d_psi_rad)),
            x_rotation(eps),
        );
        matrix_vector_multiply(matrix_multiply(nut, matrix_multiply(eq, bias)), v)
    };
    let ecliptic = |n: NutationAngles| {
        matrix_vector_multiply(
            matrix_multiply(z_rotation(-n.d_psi_rad), matrix_multiply(ecl, bias)),
            v,
        )
    };
    let full = experimental_extrapolated_nutation(tt)?;
    let linear = experimental_linear_argument_nutation(tt)?;
    let a = equatorial(full);
    let b = equatorial(linear);
    let c = ecliptic(full);
    let d = ecliptic(linear);
    Ok([
        a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], d[0], d[1], d[2],
    ])
}

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().len() != 1 {
        return Err("usage: diagnose_nutation_direction_impact < input.csv".into());
    }
    for line in io::stdin().lock().lines() {
        let line = line?;
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 3 {
            return Err("expected TT-JD,RA-deg,Dec-deg".into());
        }
        let out = project(fields[0].parse()?, fields[1].parse()?, fields[2].parse()?)?;
        println!(
            "{}",
            out.iter()
                .map(|v| format!("{v:.17}"))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::project;
    #[test]
    fn equal_at_j2000_and_rejects_invalid_input() {
        let p = project(2451545.0, 40.0, 20.0).unwrap();
        for i in 0..3 {
            assert!((p[i] - p[i + 3]).abs() < 1e-14);
            assert!((p[i + 6] - p[i + 9]).abs() < 1e-14);
        }
        assert!(project(f64::NAN, 0.0, 0.0).is_err());
        assert!(project(2451545.0, 0.0, 91.0).is_err());
    }
}
