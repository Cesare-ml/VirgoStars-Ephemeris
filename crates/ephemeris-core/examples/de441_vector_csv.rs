/*
The contents of this file are subject to the Common Public Attribution License
Version 1.0 (CPAL-1.0). See LICENSE, NOTICE and the full Exhibit A notice in
crates/ephemeris-core/src/de441.rs.
Original contribution: Cesare-ml (VirgoStars project), 2026.
Modification record: 2026-11-11 — add offline geometric vector export example; preserve full f64 precision for vector components; add deterministic bit-pattern serialization regressions.
*/

//! Offline CSV export for independently comparing classic DE441 states.
//! This is not a Horizons reference or a numerical accuracy certification.

use std::env;
use std::error::Error;
use std::io;

use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

fn body(name: &str) -> Option<Body> {
    match name {
        "ssb" => Some(Body::SolarSystemBarycenter),
        "mercury" => Some(Body::Mercury),
        "venus" => Some(Body::Venus),
        "emb" => Some(Body::EarthMoonBarycenter),
        "earth" => Some(Body::Earth),
        "moon" => Some(Body::Moon),
        "mars_barycenter" => Some(Body::MarsBarycenter),
        "jupiter_barycenter" => Some(Body::JupiterBarycenter),
        "saturn_barycenter" => Some(Body::SaturnBarycenter),
        "uranus_barycenter" => Some(Body::UranusBarycenter),
        "neptune_barycenter" => Some(Body::NeptuneBarycenter),
        "pluto_barycenter" => Some(Body::PlutoBarycenter),
        "sun" => Some(Body::Sun),
        _ => None,
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        return Err(invalid(
            "usage: cargo run --example de441_vector_csv -- <de441.eph> <JD_TDB> <target> <center>; names: ssb, mercury, venus, emb, earth, moon, mars_barycenter, jupiter_barycenter, saturn_barycenter, uranus_barycenter, neptune_barycenter, pluto_barycenter, sun",
        )
        .into());
    }
    let epoch: f64 = args[2].parse()?;
    if !epoch.is_finite() {
        return Err(invalid("JD_TDB must be finite").into());
    }
    let target = body(&args[3]).ok_or_else(|| invalid("unsupported target name"))?;
    let center = body(&args[4]).ok_or_else(|| invalid("unsupported center name"))?;
    let mut ephemeris = De441::open(&args[1])?;
    let state = ephemeris.state(target, center, JulianDateTdb(epoch))?;
    println!("jd_tdb,target,center,frame,corrections,x_km,y_km,z_km,vx_km_day,vy_km_day,vz_km_day");
    println!(
        "{:.16e},{},{},ICRF,GEOMETRIC,{:.16e},{:.16e},{:.16e},{:.16e},{:.16e},{:.16e}",
        epoch,
        args[3],
        args[4],
        state.position_km[0],
        state.position_km[1],
        state.position_km[2],
        state.velocity_km_per_day[0],
        state.velocity_km_per_day[1],
        state.velocity_km_per_day[2],
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn exported_epoch_roundtrips_without_nine_decimal_truncation() {
        let epoch = 2451545.123456789_f64;
        let encoded = format!("{epoch:.16e}");
        let decoded: f64 = encoded.parse().expect("valid floating-point JD");
        assert_eq!(decoded.to_bits(), epoch.to_bits());
    }

    #[test]
    fn vector_components_roundtrip_at_full_f64_precision() {
        let values = [
            0.0_f64,
            -0.0,
            1.2345678901234567,
            -1.2345678901234567e11,
            1.0e-120,
            f64::MIN_POSITIVE,
            f64::MAX,
        ];
        for value in values {
            let encoded = format!("{value:.16e}");
            let decoded: f64 = encoded.parse().expect("valid finite component");
            assert_eq!(
                decoded.to_bits(),
                value.to_bits(),
                "CSV serialization changed component {value:?}"
            );
        }
    }

    #[test]
    fn finite_bit_patterns_roundtrip_through_csv_format() {
        // Deterministic coverage across signs, exponents, mantissas and subnormals.
        // These values test serialization only, not astronomical accuracy.
        let mut bits = 0x9e37_79b9_7f4a_7c15_u64;
        let mut checked = 0_usize;
        for _ in 0..10_000 {
            bits ^= bits >> 12;
            bits ^= bits << 25;
            bits ^= bits >> 27;
            let value = f64::from_bits(bits.wrapping_mul(0x2545_f491_4f6c_dd1d));
            if !value.is_finite() {
                continue;
            }
            let encoded = format!("{value:.16e}");
            let decoded: f64 = encoded.parse().expect("formatted finite f64");
            assert_eq!(
                decoded.to_bits(),
                value.to_bits(),
                "CSV precision regression for bits {:016x}",
                value.to_bits()
            );
            checked += 1;
        }
        assert!(checked > 9_000, "insufficient finite regression samples");

        for value in [
            f64::from_bits(1), // Smallest positive subnormal.
            f64::from_bits(0x8000_0000_0000_0001), // Negative subnormal.
            f64::from_bits(0x7fef_ffff_ffff_ffff), // Largest finite.
            f64::from_bits(0xffef_ffff_ffff_ffff), // Most negative finite.
        ] {
            let encoded = format!("{value:.16e}");
            let decoded: f64 = encoded.parse().expect("finite boundary value");
            assert_eq!(decoded.to_bits(), value.to_bits());
        }
    }

    #[test]
    fn csv_body_names_are_unique_and_explicit_about_barycenters() {
        use virgostars_ephemeris_core::de441::Body;

        let cases = [
            ("ssb", Body::SolarSystemBarycenter),
            ("mercury", Body::Mercury),
            ("venus", Body::Venus),
            ("emb", Body::EarthMoonBarycenter),
            ("earth", Body::Earth),
            ("moon", Body::Moon),
            ("mars_barycenter", Body::MarsBarycenter),
            ("jupiter_barycenter", Body::JupiterBarycenter),
            ("saturn_barycenter", Body::SaturnBarycenter),
            ("uranus_barycenter", Body::UranusBarycenter),
            ("neptune_barycenter", Body::NeptuneBarycenter),
            ("pluto_barycenter", Body::PlutoBarycenter),
            ("sun", Body::Sun),
        ];
        for (index, (name, expected)) in cases.iter().enumerate() {
            assert_eq!(
                super::body(name),
                Some(*expected),
                "incorrect CSV body: {name}"
            );
            for (other_name, _) in &cases[..index] {
                assert_ne!(name, other_name, "duplicate CSV body identifier");
            }
        }
        for ambiguous in [
            "mars",
            "jupiter",
            "saturn",
            "uranus",
            "neptune",
            "pluto",
            "earth_moon_barycenter",
            "solar_system_barycenter",
            "",
        ] {
            assert!(
                super::body(ambiguous).is_none(),
                "ambiguous or undocumented name accepted: {ambiguous}"
            );
        }
    }

    #[test]
    fn unsupported_body_names_are_rejected() {
        assert!(super::body("mars").is_none());
        assert!(super::body("MERCURY").is_none());
        assert!(super::body("mars_barycenter").is_some());
    }
}
