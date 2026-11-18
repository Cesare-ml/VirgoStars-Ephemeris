// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Diagnostic projection of an external ICRF direction through experimental
//! long-range orientation. TT JD, RA deg, declination deg on each stdin line.
//! Mixed-frame and extrapolated nutation assumptions are not accuracy claims.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::earth_orientation::{
    experimental_extrapolated_nutation, iau2006_bias_precession_matrix, matrix_multiply,
    matrix_vector_multiply, x_rotation, z_rotation, J2000_TT_JD,
};
use virgostars_ephemeris_core::long_term_precession::{
    experimental_direct_obliquity, mean_date_matrices,
};

fn angles(v: [f64; 3]) -> (f64, f64) {
    (
        v[1].atan2(v[0]).to_degrees().rem_euclid(360.0),
        v[2].atan2(v[0].hypot(v[1])).to_degrees(),
    )
}
fn project(tt: f64, ra: f64, dec: f64) -> Result<[f64; 10], Box<dyn Error>> {
    if ![tt, ra, dec].iter().all(|x| x.is_finite()) || !(-90.0..=90.0).contains(&dec) {
        return Err("invalid input".into());
    }
    let (sr, cr) = ra.to_radians().sin_cos();
    let (sd, cd) = dec.to_radians().sin_cos();
    let v = [cd * cr, cd * sr, sd];
    let (eq, ecl) = mean_date_matrices(tt)?;
    let bias = iau2006_bias_precession_matrix(J2000_TT_JD)?;
    let nut = experimental_extrapolated_nutation(tt)?;
    let pole_eps = eq[2]
        .iter()
        .zip(ecl[2])
        .map(|(a, b)| a * b)
        .sum::<f64>()
        .clamp(-1.0, 1.0)
        .acos();
    let direct_eps = experimental_direct_obliquity(tt)?;
    // Matching the existing IAU-style rotation order, with long-range inputs.
    let n = matrix_multiply(
        matrix_multiply(
            x_rotation(-pole_eps - nut.d_epsilon_rad),
            z_rotation(-nut.d_psi_rad),
        ),
        x_rotation(pole_eps),
    );
    let (mean_ra, mean_dec) = angles(matrix_vector_multiply(matrix_multiply(eq, bias), v));
    let (true_ra, true_dec) = angles(matrix_vector_multiply(
        matrix_multiply(n, matrix_multiply(eq, bias)),
        v,
    ));
    let (pole_lon, pole_lat) = angles(matrix_vector_multiply(
        matrix_multiply(z_rotation(-nut.d_psi_rad), matrix_multiply(ecl, bias)),
        v,
    ));
    let (direct_lon, direct_lat) = angles(matrix_vector_multiply(
        matrix_multiply(
            z_rotation(-nut.d_psi_rad),
            matrix_multiply(matrix_multiply(x_rotation(direct_eps), eq), bias),
        ),
        v,
    ));
    Ok([
        mean_ra,
        mean_dec,
        true_ra,
        true_dec,
        pole_lon,
        pole_lat,
        direct_lon,
        direct_lat,
        (pole_eps + nut.d_epsilon_rad).to_degrees(),
        (direct_eps + nut.d_epsilon_rad).to_degrees(),
    ])
}
fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().len() != 1 {
        return Err("usage: diagnose_horizons_orientation_components < input.csv".into());
    }
    for line in io::stdin().lock().lines() {
        let line = line?;
        let f: Vec<_> = line.split(',').collect();
        if f.len() != 3 {
            return Err("expected TT-JD,RA-deg,Dec-deg".into());
        }
        let v = project(f[0].parse()?, f[1].parse()?, f[2].parse()?)?;
        println!(
            "{}",
            v.iter()
                .map(|x| format!("{x:.15}"))
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
    fn wrapped_ra_and_bad_inputs() {
        let a = project(2451545.0, 45.0, 20.0).unwrap();
        let b = project(2451545.0, 405.0, 20.0).unwrap();
        for i in 0..10 {
            assert!((a[i] - b[i]).abs() < 1e-10);
        }
        assert!(project(2451545.0, 0.0, 91.0).is_err());
    }

    #[test]
    fn true_equator_rotates_to_pole_ecliptic_on_ancient_epochs() {
        let to_vec = |lon: f64, lat: f64| {
            let (s, c) = lon.to_radians().sin_cos();
            let (z, r) = lat.to_radians().sin_cos();
            [r * c, r * s, z]
        };
        for tt in [-1930453.0, -104971.0, 2451727.0] {
            let out = project(tt, 73.0, -14.0).unwrap();
            let eq = to_vec(out[2], out[3]);
            let ec = to_vec(out[4], out[5]);
            let (s, c) = out[8].to_radians().sin_cos();
            let rotated = [eq[0], c * eq[1] + s * eq[2], -s * eq[1] + c * eq[2]];
            for i in 0..3 {
                assert!((rotated[i] - ec[i]).abs() < 2e-14, "{tt} axis {i}");
            }
        }
    }
}
