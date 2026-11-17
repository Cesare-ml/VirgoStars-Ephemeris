// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Exploratory derivatives of the existing deflection model, not a production
//! correction or a physical accuracy claim. Input CSV: body ID,TDB JD.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::light_deflection::{
    solar_deflection_delta, SUN_SCHWARZSCHILD_RADIUS_AU,
};
use virgostars_ephemeris_core::light_time::geocentric_light_time;

const AU_KM: f64 = 149_597_870.7;
const FLOOR: f64 = 1e-6;
const AS_PER_RAD: f64 = 206_264.806_247_096_36;
type V = [f64; 3];
fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: V, s: f64) -> V {
    a.map(|v| v * s)
}
fn dot(a: V, b: V) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: V) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn unit_rate(position: V, velocity: V) -> (V, V, f64, f64) {
    let r = norm(position);
    let u = scale(position, 1.0 / r);
    let rd = dot(u, velocity);
    (u, scale(sub(velocity, scale(u, rd)), 1.0 / r), r, rd)
}

// Direct differentiation of p × (e × q) and its scalar denominator.
// At the guard boundary the model has a kink: the derivative is undefined.
fn derivative((p, pd): (V, V), (q, qd): (V, V), (e, ed): (V, V), (r, rd): (f64, f64)) -> Option<V> {
    let d = dot(q, add(q, e));
    if !r.is_finite() || r <= 0.0 || (d - FLOOR).abs() < 1e-12 {
        return None;
    }
    let dd = if d > FLOOR {
        dot(qd, add(scale(q, 2.0), e)) + dot(q, ed)
    } else {
        0.0
    };
    let denom = d.max(FLOOR);
    let w = SUN_SCHWARZSCHILD_RADIUS_AU / r / denom;
    let wd = -w * (rd / r + dd / denom);
    let eq = cross(e, q);
    let f = cross(p, eq);
    let fd = add(cross(pd, eq), cross(p, add(cross(ed, q), cross(e, qd))));
    let out = add(scale(f, wd), scale(fd, w));
    out.iter().all(|v| v.is_finite()).then_some(out)
}
fn body(id: u8) -> Option<Body> {
    Some(match id {
        2 => Body::Mercury,
        3 => Body::Venus,
        4 => Body::MarsBarycenter,
        5 => Body::JupiterBarycenter,
        6 => Body::SaturnBarycenter,
        7 => Body::UranusBarycenter,
        8 => Body::NeptuneBarycenter,
        9 => Body::PlutoBarycenter,
        _ => return None,
    })
}
struct Geometry {
    delta: V,
    analytic: Option<V>,
    angle: f64,
    denom: f64,
}
fn geometry(
    eph: &mut De441<std::fs::File>,
    target: Body,
    jd: f64,
) -> Result<Geometry, Box<dyn Error>> {
    let t = JulianDateTdb(jd);
    let lt = geocentric_light_time(eph, target, t)?;
    let earth = eph.state(Body::Earth, Body::SolarSystemBarycenter, t)?;
    let sun = eph.state(Body::Sun, Body::SolarSystemBarycenter, t)?;
    let src = eph.state(target, Body::SolarSystemBarycenter, lt.emission_epoch)?;
    let emitted_sun = eph.state(Body::Sun, Body::SolarSystemBarycenter, lt.emission_epoch)?;
    let (p, pd, _, _) = unit_rate(lt.position_km, lt.velocity_km_per_day);
    let (q, qd, _, _) = unit_rate(
        sub(src.position_km, emitted_sun.position_km),
        scale(
            sub(src.velocity_km_per_day, emitted_sun.velocity_km_per_day),
            1.0 - lt.light_time_rate,
        ),
    );
    let (e, ed, r, rd) = unit_rate(
        sub(earth.position_km, sun.position_km),
        sub(earth.velocity_km_per_day, sun.velocity_km_per_day),
    );
    let delta = solar_deflection_delta(p, q, e, r / AU_KM)?;
    Ok(Geometry {
        delta,
        analytic: derivative((p, pd), (q, qd), (e, ed), (r / AU_KM, rd / AU_KM)),
        angle: (-dot(p, e)).clamp(-1.0, 1.0).acos().to_degrees(),
        denom: dot(q, add(q, e)),
    })
}
fn sample(
    eph: &mut De441<std::fs::File>,
    target: Body,
    jd: f64,
    h: f64,
) -> Result<[Geometry; 4], Box<dyn Error>> {
    Ok([
        geometry(eph, target, jd - 2.0 * h)?,
        geometry(eph, target, jd - h)?,
        geometry(eph, target, jd + h)?,
        geometry(eph, target, jd + 2.0 * h)?,
    ])
}
fn fourth(a: &[Geometry; 4], h: f64) -> V {
    std::array::from_fn(|i| {
        (a[0].delta[i] - 8.0 * a[1].delta[i] + 8.0 * a[2].delta[i] - a[3].delta[i]) / (12.0 * h)
    })
}
fn crosses_floor(a: &[Geometry; 4], center: f64) -> bool {
    a.iter().any(|x| (x.denom > FLOOR) != (center > FLOOR))
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: compare_deflection_rate_candidates <de441> < cases.csv".into());
    }
    let mut eph = De441::open(&args[1])?;
    println!("body,jd_tdb,elongation_deg,denominator,crosses_floor,old_error_as_per_day,analytic_error_as_per_day,reference_disagreement_as_per_day");
    for line in io::stdin().lock().lines() {
        let line = line?;
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 2 {
            return Err("expected body-id,TDB-JD".into());
        }
        let id: u8 = fields[0].parse()?;
        let target = body(id).ok_or("unsupported body")?;
        let jd: f64 = fields[1].parse()?;
        if !jd.is_finite() {
            return Err("invalid epoch".into());
        }
        let c = geometry(&mut eph, target, jd)?;
        let old = sample(&mut eph, target, jd, 1.0 / 256.0)?;
        let fine = sample(&mut eph, target, jd, 1.0 / 2048.0)?;
        let finer = sample(&mut eph, target, jd, 1.0 / 4096.0)?;
        let reference = fourth(&fine, 1.0 / 2048.0);
        let check = fourth(&finer, 1.0 / 4096.0);
        let old_rate = scale(sub(old[2].delta, old[1].delta), 128.0);
        let crossing = crosses_floor(&old, c.denom)
            || crosses_floor(&fine, c.denom)
            || crosses_floor(&finer, c.denom);
        let Some(candidate) = c.analytic else {
            continue;
        };
        println!(
            "{id},{jd},{:.12},{:.14e},{crossing},{:.12},{:.12},{:.12}",
            c.angle,
            c.denom,
            norm(sub(old_rate, reference)) * AS_PER_RAD,
            norm(sub(candidate, reference)) * AS_PER_RAD,
            norm(sub(reference, check)) * AS_PER_RAD
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analytic_derivative_matches_synthetic_varying_vectors() {
        for degrees in [0.04_f64, 0.3, 2.0, 30.0, 120.0] {
            let psi = degrees.to_radians();
            let speed = 0.017;
            let p = [-psi.cos(), psi.sin(), 0.0];
            let pd = [psi.sin() * speed, psi.cos() * speed, 0.0];
            let e = [1.0, 0.0, 0.0];
            let zero = [0.0; 3];
            let got = derivative((p, pd), (p, pd), (e, zero), (1.0, 0.0)).unwrap();
            let value = |dt: f64| {
                let a = psi + speed * dt;
                let v = [-a.cos(), a.sin(), 0.0];
                solar_deflection_delta(v, v, e, 1.0).unwrap()
            };
            let h = 1e-5;
            let expected = scale(sub(value(h), value(-h)), 1.0 / (2.0 * h));
            assert!(norm(sub(got, expected)) * AS_PER_RAD < 1e-5, "{degrees}");
        }
    }

    #[test]
    fn rejects_nondifferentiable_floor_boundary() {
        let psi = (1.0 - FLOOR).acos();
        let p = [-psi.cos(), psi.sin(), 0.0];
        let e = [1.0, 0.0, 0.0];
        let zero = [0.0; 3];
        assert!(derivative((p, zero), (p, zero), (e, zero), (1.0, 0.0)).is_none());
    }

    #[test]
    fn analytic_derivative_handles_independent_vector_and_distance_motion() {
        let direction = |a: f64| [a.cos(), a.sin(), 0.0];
        let tangent = |a: f64, w: f64| [-a.sin() * w, a.cos() * w, 0.0];
        let (a, b, c) = (2.4, 2.5, 0.2);
        let (u, v, w) = (0.02, -0.003, 0.015);
        let (r, rd) = (0.98, 0.004);
        let got = derivative(
            (direction(a), tangent(a, u)),
            (direction(b), tangent(b, v)),
            (direction(c), tangent(c, w)),
            (r, rd),
        )
        .unwrap();
        let value = |t: f64| {
            solar_deflection_delta(
                direction(a + u * t),
                direction(b + v * t),
                direction(c + w * t),
                r + rd * t,
            )
            .unwrap()
        };
        let h = 1e-4;
        let expected = scale(sub(value(h), value(-h)), 1.0 / (2.0 * h));
        assert!(norm(sub(got, expected)) * AS_PER_RAD < 1e-7);
    }
}
