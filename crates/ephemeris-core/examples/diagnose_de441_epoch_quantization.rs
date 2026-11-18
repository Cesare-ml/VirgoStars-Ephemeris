// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Measure conversion of a split, already-TDB date to the DE441 f64 epoch.
//! Input: a representable base JD and an offset in seconds, one pair per line.
//! The caller supplies the offset; no time-scale conversion occurs here.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::de441_epoch_bridge::checked_de441_epoch;
use virgostars_ephemeris_core::time::SECONDS_PER_DAY;

fn measure(base: f64, seconds: f64) -> Result<[f64; 3], Box<dyn Error>> {
    if !seconds.is_finite() || seconds.abs() > 1.0 {
        return Err("invalid offset".into());
    }
    let epoch = JulianDate::from_f64(base, TimeScale::Tdb)?;
    let mut day = epoch.day_number();
    let mut fraction = epoch.midnight_fraction() + seconds / SECONDS_PER_DAY;
    if fraction < 0.0 {
        day -= 1;
        fraction += 1.0;
    } else if fraction >= 1.0 {
        day += 1;
        fraction -= 1.0;
    }
    let split = JulianDate::new(day, fraction, TimeScale::Tdb)?;
    let checked = checked_de441_epoch(split, 1.0)?;
    let (first, second) = split.parts()?;
    let signed_seconds = ((checked.epoch.0 - first) - second) * SECONDS_PER_DAY;
    Ok([
        checked.epoch.0,
        signed_seconds,
        checked.quantization_error_seconds,
    ])
}

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().len() != 1 {
        return Err("usage: diagnose_de441_epoch_quantization < base-jd,offset-seconds".into());
    }
    for line in io::stdin().lock().lines() {
        let line = line?;
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 2 {
            return Err("expected base-jd,offset-seconds".into());
        }
        let r = measure(fields[0].parse()?, fields[1].parse()?)?;
        println!("{:.17},{:.17},{:.17}", r[0], r[1], r[2]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::measure;
    #[test]
    fn reports_sign_and_validates_offset() {
        assert_eq!(measure(2451545.0, 0.0).unwrap(), [2451545.0, 0.0, 0.0]);
        let p = measure(2451545.0, 0.000_001).unwrap();
        assert!(p[1] < 0.0 && (p[1].abs() - p[2]).abs() < 1e-15);
        let n = measure(2451545.0, -0.000_001).unwrap();
        assert!(n[1] > 0.0 && (n[1].abs() - n[2]).abs() < 1e-15);
        assert!(measure(f64::NAN, 0.0).is_err());
        assert!(measure(2451545.0, f64::NAN).is_err());
        assert!(measure(2451545.0, 2.0).is_err());
    }
}
