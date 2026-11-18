// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Modification (2026-11-18): diagnose normalized-position rate and guard crossings.
// Modification (2026-11-18): distinguish sampled guard crossings by stencil.
// Modification (2026-11-18): compare complete experimental ICRF rates.
//! Exploratory derivatives of the existing deflection model, not a production
//! correction or a physical accuracy claim. Input CSV: body ID,TDB JD.
use std::error::Error;
use std::io::{self, BufRead};
use virgostars_ephemeris_core::aberration_state::aberrate_light_time_state;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::light_deflection::{
    deflect_light_time_state_by_sun, solar_deflection_delta, SUN_SCHWARZSCHILD_RADIUS_AU,
};
use virgostars_ephemeris_core::light_time::{geocentric_light_time, GeocentricLightTimeState};

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
    lt: GeocentricLightTimeState,
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
        lt,
    })
}

// Differentiate the actual normalized output R * normalize(p + delta),
// returning the unit-vector correction to p and its time derivative.
fn normalized_correction(p: V, pd: V, delta: V, dd: V) -> (V, V) {
    let w = add(p, delta);
    let n = norm(w);
    let u = scale(w, 1.0 / n);
    let wd = add(pd, dd);
    let ud = scale(sub(wd, scale(u, dot(u, wd))), 1.0 / n);
    (sub(u, p), sub(ud, pd))
}

fn normalized_delta(g: &Geometry) -> V {
    let (p, _, _, _) = unit_rate(g.lt.position_km, g.lt.velocity_km_per_day);
    let w = add(p, g.delta);
    sub(scale(w, 1.0 / norm(w)), p)
}

fn fourth_normalized(a: &[Geometry; 4], h: f64) -> V {
    let values = a.each_ref().map(normalized_delta);
    std::array::from_fn(|i| {
        (values[0][i] - 8.0 * values[1][i] + 8.0 * values[2][i] - values[3][i]) / (12.0 * h)
    })
}

// Keep the existing position, replace only the deflection derivative in an
// isolated candidate, then pass it through the same aberration calculation.
fn apparent_icrf(
    eph: &mut De441<std::fs::File>,
    g: &Geometry,
    candidate: bool,
) -> Result<(V, V), Box<dyn Error>> {
    let mut def = deflect_light_time_state_by_sun(eph, &g.lt)?;
    if candidate {
        let dd = g.analytic.ok_or("candidate unavailable at floor")?;
        let (p, pd, r, rd) = unit_rate(g.lt.position_km, g.lt.velocity_km_per_day);
        let (correction, correction_rate) = normalized_correction(p, pd, g.delta, dd);
        def.velocity_km_per_day = add(
            scale(add(p, correction), rd),
            scale(add(pd, correction_rate), r),
        );
    }
    let earth = eph.state_with_acceleration(
        Body::Earth,
        Body::SolarSystemBarycenter,
        g.lt.observation_epoch,
    )?;
    let state = aberrate_light_time_state(
        &def,
        earth.velocity_km_per_day,
        earth.acceleration_km_per_day2,
    )?;
    let (u, ud, _, _) = unit_rate(state.position_km, state.velocity_km_per_tdb_day);
    Ok((u, ud))
}

fn apparent_without_deflection(
    eph: &mut De441<std::fs::File>,
    g: &Geometry,
) -> Result<(V, V), Box<dyn Error>> {
    let earth = eph.state_with_acceleration(
        Body::Earth,
        Body::SolarSystemBarycenter,
        g.lt.observation_epoch,
    )?;
    let state = aberrate_light_time_state(
        &g.lt,
        earth.velocity_km_per_day,
        earth.acceleration_km_per_day2,
    )?;
    let (u, ud, _, _) = unit_rate(state.position_km, state.velocity_km_per_tdb_day);
    Ok((u, ud))
}

fn fourth_apparent_increment(
    eph: &mut De441<std::fs::File>,
    samples: &[Geometry; 4],
    h: f64,
) -> Result<V, Box<dyn Error>> {
    let mut values = [[0.0; 3]; 4];
    for (i, g) in samples.iter().enumerate() {
        values[i] = sub(
            apparent_icrf(eph, g, false)?.0,
            apparent_without_deflection(eph, g)?.0,
        );
    }
    Ok(std::array::from_fn(|i| {
        (values[0][i] - 8.0 * values[1][i] + 8.0 * values[2][i] - values[3][i]) / (12.0 * h)
    }))
}

fn fourth_apparent(
    eph: &mut De441<std::fs::File>,
    samples: &[Geometry; 4],
    h: f64,
) -> Result<V, Box<dyn Error>> {
    let values = [
        apparent_icrf(eph, &samples[0], false)?.0,
        apparent_icrf(eph, &samples[1], false)?.0,
        apparent_icrf(eph, &samples[2], false)?.0,
        apparent_icrf(eph, &samples[3], false)?.0,
    ];
    Ok(std::array::from_fn(|i| {
        (values[0][i] - 8.0 * values[1][i] + 8.0 * values[2][i] - values[3][i]) / (12.0 * h)
    }))
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
    if args.len() != 2
        && !(args.len() == 3
            && [
                "--normalized",
                "--floor-details",
                "--apparent-icrf",
                "--apparent-increment",
            ]
            .contains(&args[2].as_str()))
    {
        return Err(
            "usage: compare_deflection_rate_candidates <de441> [--normalized|--floor-details|--apparent-icrf|--apparent-increment] < cases.csv".into(),
        );
    }
    let normalized = args.len() == 3;
    let floor_details = args.len() == 3 && args[2] == "--floor-details";
    let apparent = args.len() == 3 && args[2] == "--apparent-icrf";
    let increment = args.len() == 3 && args[2] == "--apparent-increment";
    let mut eph = De441::open(&args[1])?;
    if floor_details {
        println!("body,jd_tdb,denominator,old_crosses,fine_crosses,finer_crosses,old_error_as_per_day,analytic_error_as_per_day,reference_disagreement_as_per_day");
    } else {
        println!("body,jd_tdb,elongation_deg,denominator,crosses_floor,old_error_as_per_day,analytic_error_as_per_day,reference_disagreement_as_per_day");
    }
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
        let mut crossing = crosses_floor(&old, c.denom)
            || crosses_floor(&fine, c.denom)
            || crosses_floor(&finer, c.denom);
        let Some(candidate) = c.analytic else {
            continue;
        };
        let (old_error, candidate_error, reference_disagreement) = if increment {
            let a = fourth_apparent_increment(&mut eph, &fine, 1.0 / 2048.0)?;
            let b = fourth_apparent_increment(&mut eph, &finer, 1.0 / 4096.0)?;
            let (u, old_velocity) = apparent_icrf(&mut eph, &c, false)?;
            let (v, new_velocity) = apparent_icrf(&mut eph, &c, true)?;
            let (_, unperturbed) = apparent_without_deflection(&mut eph, &c)?;
            if norm(sub(u, v)) > 1e-14 {
                return Err("candidate changed apparent position".into());
            }
            (
                norm(sub(sub(old_velocity, unperturbed), a)),
                norm(sub(sub(new_velocity, unperturbed), a)),
                norm(sub(a, b)),
            )
        } else if apparent {
            let coarse = sample(&mut eph, target, jd, 1.0 / 512.0)?;
            let refined = sample(&mut eph, target, jd, 1.0 / 1024.0)?;
            crossing |= crosses_floor(&coarse, c.denom) || crosses_floor(&refined, c.denom);
            let a = fourth_apparent(&mut eph, &coarse, 1.0 / 512.0)?;
            let b = fourth_apparent(&mut eph, &refined, 1.0 / 1024.0)?;
            let (u, old_velocity) = apparent_icrf(&mut eph, &c, false)?;
            let (v, new_velocity) = apparent_icrf(&mut eph, &c, true)?;
            if norm(sub(u, v)) > 1e-14 {
                return Err("candidate changed apparent position".into());
            }
            (
                norm(sub(old_velocity, a)),
                norm(sub(new_velocity, a)),
                norm(sub(a, b)),
            )
        } else if normalized {
            let (p, pd, range, _) = unit_rate(c.lt.position_km, c.lt.velocity_km_per_day);
            let (_, candidate_rate) = normalized_correction(p, pd, c.delta, candidate);
            let def = deflect_light_time_state_by_sun(&mut eph, &c.lt)?;
            let (_, actual_ud, _, _) = unit_rate(def.position_km, def.velocity_km_per_day);
            let old_correction_rate = sub(actual_ud, pd);
            let fine_ref = fourth_normalized(&fine, 1.0 / 2048.0);
            let finer_ref = fourth_normalized(&finer, 1.0 / 4096.0);
            if !range.is_finite() || range <= 0.0 {
                return Err("invalid range".into());
            }
            (
                norm(sub(old_correction_rate, fine_ref)),
                norm(sub(candidate_rate, fine_ref)),
                norm(sub(fine_ref, finer_ref)),
            )
        } else {
            (
                norm(sub(old_rate, reference)),
                norm(sub(candidate, reference)),
                norm(sub(reference, check)),
            )
        };
        if floor_details {
            println!(
                "{id},{jd},{:.14e},{},{},{},{:.12},{:.12},{:.12}",
                c.denom,
                crosses_floor(&old, c.denom),
                crosses_floor(&fine, c.denom),
                crosses_floor(&finer, c.denom),
                old_error * AS_PER_RAD,
                candidate_error * AS_PER_RAD,
                reference_disagreement * AS_PER_RAD
            );
        } else {
            println!(
                "{id},{jd},{:.12},{:.14e},{crossing},{:.12},{:.12},{:.12}",
                c.angle,
                c.denom,
                old_error * AS_PER_RAD,
                candidate_error * AS_PER_RAD,
                reference_disagreement * AS_PER_RAD
            );
        }
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

    #[test]
    fn normalized_position_derivative_matches_independent_difference() {
        let p = [0.6, 0.8, 0.0];
        let pd = [-0.008, 0.006, 0.002];
        let d = [1.0e-5, -7.5e-6, 3.0e-6];
        let dd = [2.0e-6, -1.5e-6, -1.0e-6];
        let (_, actual) = normalized_correction(p, pd, d, dd);
        let value = |t: f64| {
            let p_raw = add(p, scale(pd, t));
            let pp = scale(p_raw, 1.0 / norm(p_raw));
            let w = add(pp, add(d, scale(dd, t)));
            sub(scale(w, 1.0 / norm(w)), pp)
        };
        let h = 1e-4;
        let reference = scale(sub(value(h), value(-h)), 1.0 / (2.0 * h));
        assert!(norm(sub(actual, reference)) * AS_PER_RAD < 1e-6);
    }

    #[test]
    fn normalized_cartesian_velocity_includes_range_rate() {
        let p = [0.6, 0.8, 0.0];
        let pd = [-0.008, 0.006, 0.002];
        let d = [1.0e-5, -7.5e-6, 3.0e-6];
        let dd = [2.0e-6, -1.5e-6, -1.0e-6];
        let (r, rd) = (1000.0, 3.0);
        let (correction, correction_rate) = normalized_correction(p, pd, d, dd);
        let expected_velocity = add(
            scale(add(p, correction), rd),
            scale(add(pd, correction_rate), r),
        );
        let position = |t: f64| {
            let raw = add(p, scale(pd, t));
            let pt = scale(raw, 1.0 / norm(raw));
            let w = add(pt, add(d, scale(dd, t)));
            scale(w, (r + rd * t) / norm(w))
        };
        let h = 1e-4;
        let numeric = scale(sub(position(h), position(-h)), 1.0 / (2.0 * h));
        assert!(norm(sub(expected_velocity, numeric)) < 1e-8);
    }

    #[test]
    #[ignore = "requires classic DE441 in DE441_PATH; numerical consistency only"]
    fn real_de441_apparent_rate_candidate_is_consistent_for_sampled_near_sun_case() {
        let path = std::env::var_os("DE441_PATH").expect("DE441_PATH required");
        let mut eph = De441::open(path).unwrap();
        let target = Body::JupiterBarycenter;
        let jd = -1_474_550.0;
        let center = geometry(&mut eph, target, jd).unwrap();
        let a = sample(&mut eph, target, jd, 1.0 / 512.0).unwrap();
        let b = sample(&mut eph, target, jd, 1.0 / 1024.0).unwrap();
        assert!(!crosses_floor(&a, center.denom) && !crosses_floor(&b, center.denom));
        let ra = fourth_apparent(&mut eph, &a, 1.0 / 512.0).unwrap();
        let rb = fourth_apparent(&mut eph, &b, 1.0 / 1024.0).unwrap();
        let (u, old) = apparent_icrf(&mut eph, &center, false).unwrap();
        let (v, new) = apparent_icrf(&mut eph, &center, true).unwrap();
        let old_error = norm(sub(old, ra));
        assert!(norm(sub(u, v)) < 1e-14);
        assert!(norm(sub(ra, rb)) < old_error * 0.05);
        assert!(norm(sub(new, ra)) < old_error * 0.05);
    }
}
