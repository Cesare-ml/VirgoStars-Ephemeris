// SPDX-License-Identifier: CPAL-1.0
//! Offline projection for comparing printed Horizons ICRF positions.
//! Input: TT Julian day, ICRF right ascension degrees, declination degrees.
//! Uses the engine's experimental orientation, so it is not an independent
//! test of long-range precession or nutation.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::earth_orientation::{
    experimental_extrapolated_nutation, iau2006_bias_precession_matrix, matrix_multiply,
    matrix_vector_multiply, z_rotation, J2000_TT_JD,
};
use virgostars_ephemeris_core::long_term_precession::mean_date_matrices;

fn project(tt: f64, ra: f64, dec: f64) -> Result<(f64, f64), Box<dyn Error>> {
    if ![tt, ra, dec].iter().all(|x| x.is_finite()) || !(-90.0..=90.0).contains(&dec) {
        return Err("invalid ICRF coordinates".into());
    }
    let (sr, cr) = ra.to_radians().sin_cos();
    let (sd, cd) = dec.to_radians().sin_cos();
    let bias = iau2006_bias_precession_matrix(J2000_TT_JD)?;
    let mean = matrix_multiply(mean_date_matrices(tt)?.1, bias);
    let dpsi = experimental_extrapolated_nutation(tt)?.d_psi_rad;
    let v = matrix_vector_multiply(
        matrix_multiply(z_rotation(-dpsi), mean),
        [cd * cr, cd * sr, sd],
    );
    let lon = v[1].atan2(v[0]).to_degrees().rem_euclid(360.0);
    let lat = v[2].atan2(v[0].hypot(v[1])).to_degrees();
    Ok((lon, lat))
}

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().len() != 1 {
        return Err("usage: diagnose_icrf_ecliptic_projection < input.csv".into());
    }
    for line in io::stdin().lock().lines() {
        let line = line?;
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 3 {
            return Err("expected TT-JD,RA-deg,Dec-deg".into());
        }
        let (lon, lat) = project(fields[0].parse()?, fields[1].parse()?, fields[2].parse()?)?;
        println!("{lon:.17e},{lat:.17e}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::project;

    #[test]
    fn projection_wraps_longitude_and_rejects_invalid_inputs() {
        let a = project(2451545.0, 45.0, 20.0).unwrap();
        let b = project(2451545.0, 405.0, 20.0).unwrap();
        assert!((a.0 - b.0).abs() < 1e-10);
        assert!((a.1 - b.1).abs() < 1e-10);
        assert!(project(2451545.0, 0.0, 91.0).is_err());
        assert!(project(f64::NAN, 0.0, 0.0).is_err());
    }
}
