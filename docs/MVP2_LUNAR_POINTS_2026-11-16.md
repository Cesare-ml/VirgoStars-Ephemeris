# MVP 2 — lunar nodes and apogees (2026-11-16)

**Status: Locally tested against the installed `swisseph` 0.5.17. The
"interpolated" apsides are NOT included. Not integrated.** Bodies 10–13 of the
backend's default chart array: mean node, true node, mean apogee ("mean Lilith") and
osculating apogee.

## What exists

`lunar_points` (Rust core) and the Node calls `swe_calc`/`swe_calc_ut` with body
numbers 10–13, ecliptic and `SEFLG_EQUATORIAL` output, rates, names
(`mean Node`, `true Node`, `mean Apogee`, `osc. Apogee`).

| Point | Definition (all in the true ecliptic and equinox of date) |
|---|---|
| Mean node (10) | Simon et al. (1994) polynomial for the ascending node plus the IAU 2000B nutation in longitude; latitude 0; distance 384,400 km |
| True node (11) | ascending node of the Moon's **instantaneous orbital plane**: normal `r × v` from the inertial geocentric DE441 state at the light-time retarded epoch, rotated to the ecliptic of date; latitude 0; distance = radius of the osculating Kepler ellipse at the node, `p/(1+e cos ω)` |
| Mean apogee (12) | the point at orbital longitude `ϖ + 180°` on the **mean lunar orbit** (node = the mean node, inclination 5.145396°), projected on the ecliptic, hence with a latitude (up to ±5.1°); `ϖ` from Simon et al. (1994); distance `a(1+e)`, `a = 384,400 km`, `e = 0.054900489` |
| Osculating apogee (13) | apoapsis direction `−e` of the osculating ellipse of the Moon about the Earth (Laplace–Runge–Lenz vector), `μ = GM_earth + GM_moon = 403,503.235625 km³/s²`, light-time retarded Moon state; distance `a(1+e)` |

These are mathematical points: no aberration and no solar deflection. Rates are
centered differences (±0.01 day) of the final coordinates, so they include the
motion of the output frame. A heliocentric request returns the zero vector with
`rflag 1801`, as the reference does. Equatorial output rotates the ecliptic position
by the true obliquity (`rflag 2305` family). `SEFLG_NOGDEFL` has no effect on them.

## Findings that shaped the implementation

- The mean apogee has a **latitude** in the reference (3.4° at J2000): it is the
  point of the mean orbit, not a point of the ecliptic. An earlier prototype that
  put it on the ecliptic was off by 0.11°.
- The mean node's distance is the nominal lunar distance and the mean apogee's is
  `a(1+e)` with Meeus's mean eccentricity; both reproduce the reference to 1e-15.
- Taking the Moon's state at the light-time retarded epoch halves the residual of the
  osculating apogee (0.34″ → 0.19″) and of the true node (0.013″ → 0.006″).

## Evidence

- Core: 4 offline tests (mean node = polynomial + nutation and retrograde rate, mean
  apogee on the inclined orbit with the documented distance, equatorial = rotated
  ecliptic, refusals) and 2 real-kernel tests (the true node equals the independent
  re-derivation to 1e-9° and stays within 2° of the mean node; rates equal exact-step
  centered differences for all four points in both frames over 1900–2100). All pass.
- Node: 41 tests, all pass with kernel, catalog, USNO files and the real `swisseph`
  package (11 pass, 30 skip without data), including key sets and `rflag` equality
  with the reference for bodies 10–13 in both frames.
- Against the reference over 99 TDB dates, 1900–2100 (receipt
  `docs/evidence/mvp2-lunar-points-local-reference-receipt-2026-11-16.json`):

  | Point | Longitude | Latitude | Distance |
  |---|---|---|---|
  | mean node | 0.20″ | 0 | 4e-16 |
  | true node | 0.006″ | 0 | 7e-8 |
  | mean apogee | 0.22″ | 0.004″ | 7e-16 |
  | osculating apogee | 0.19″ | 0.015″ | 7e-8 |

  The 0.2″ on the mean points is the difference between the published series used
  here and the reference's own series. The osculating apogee turns by 1.6° per day,
  so 0.19″ is a 0.1 s time offset.

## Not done

- **Chiron (15)** is not part of this document: see [MVP2_CHIRON_2026-11-16.md](MVP2_CHIRON_2026-11-16.md).
- **Interpolated apogee and perigee (21, 22).** No published definition; not offered.
- Other flags for these points (`NONUT`, `TRUEPOS`, `J2000`).
