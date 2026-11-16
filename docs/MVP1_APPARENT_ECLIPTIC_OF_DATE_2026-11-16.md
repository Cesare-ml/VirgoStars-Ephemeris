# MVP 1, row 1 — geocentric apparent ecliptic of date, with rates

**Status: Locally tested on a real classic DE441 file; compared locally with an
independent reference (see below). NOT integrated, NOT released, NOT a complete
MVP 1.** Branch: `integration/mvp1-apparent-pipeline-20261116` (based on
`dev/ephemeris`, no change to `main`).

This closes the main technical gap named in the
[three-MVP document](THREE_MVP_SWISS_REPLACEMENT_2026-11-11.md): the aberrated
six-field stack only produced *fixed-axis* (ICRF) coordinates. The new module
`apparent_ecliptic_of_date` rotates that state into the **true ecliptic and
equinox of date** and converts it to the six consumer fields.

## Scientific contract

| Item | Value |
|---|---|
| Input epoch | TDB Julian date (`JulianDateTdb`) |
| Center / observer | Earth center (geocentric) |
| Targets | Sun, Moon, Mercury, Venus, and the *system barycenters* of Mars–Pluto; a barycenter is never relabeled a planet center |
| Frame of output | true ecliptic and equinox of date: IAU 2006 bias-precession (Fukushima–Williams), IAU 2000B nutation (77 terms), true obliquity |
| Corrections applied | one-way light-time, special-relativistic annual aberration (analytic Earth acceleration), frame rotation |
| Corrections NOT applied | gravitational light deflection, topocentric parallax / diurnal aberration, barycenter→body-center offset |
| Units | degrees, AU, and degrees (or AU) per **TDB** day |
| Rates | `v' = M v + (dM/dt) r`; `dM/dt` is a central finite difference (half-step 0.01 TT day) of the declared matrix |
| TT for the matrix | `TDB − 0.001657 s · sin(g)` (leading periodic term; omitted terms < 30 µs) |
| Validated range | 1900–2100 TT. Outside it the call returns `ApparentError::Orientation` (fail closed, no extrapolation) |

Entry points: `geocentric_apparent_six_fields`,
`geocentric_apparent_six_fields_many` (Earth read once, all-or-error) and the
CLI `cargo run --release --example apparent_six_fields -- <de441.eph> <TDB-JD> [Body ...]`.

## Defect found and fixed on the real kernel

`light_time::geocentric_light_time` required the fixed-point iteration to
reach *exactly* equal `f64` epochs. For the Moon (≈ 1.3 s delay) at JD ≈ 2.4e6 the
iteration alternates between two adjacent representable JDs (1 ulp ≈ 40 µs) and
returned `DidNotConverge` — for example at TDB JD 2436036.6. The synthetic tests
never exercised this. Convergence is now accepted at ≤ 4 ulps of the observation
epoch; the returned state is the one evaluated at the final emission epoch. The
regression test is `real_de441_moon_light_time_converges_across_1900_2100`.

## Evidence actually obtained (2026-11-16, Windows 11, Rust 1.99.1)

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo test --workspace`: pass on the integrated branch.
- With a locally held classic `de441.eph` (not in this repository, not
  downloaded by the code): `DE441_PATH=<file> cargo test --release --test apparent_ecliptic_of_date -- --ignored`
  → 3 passed. They check 10 bodies on 60 dates in 1900–2100 TDB against
  exact-step (2⁻⁸ day) centered differences of the module's own positions
  (this verifies rate consistency, not absolute accuracy) and J2000 Sun/Moon
  longitudes against public almanac values at the 1e-3° level.
- **Independent reference, local only.** A throw-away probe (never committed)
  evaluated the Swiss Ephemeris C library, AGPL, in its JPL mode on the *same*
  `de441.eph`, with the TDB epoch passed as its ephemeris time, at 199 dates
  from JD 2415100.5 to 2488000 (step 367.3 d), for the same ten targets.
  With Swiss gravitational deflection disabled (`SEFLG_NOGDEFL`), the maximum
  difference over all bodies and dates was:

  | Quantity | Max |Δ| |
  |---|---|
  | longitude | 0.00067 arcsec |
  | latitude | 0.00055 arcsec |
  | distance | 2.4 m worst case (Mercury); Sun below 1 mm |
  | rates vs centered difference (±0.001 day) of Swiss *positions* | longitude ≤ 0.002 arcsec/day, latitude ≤ 0.0003 arcsec/day, distance ≤ 1.3 km/day for all bodies except the Moon (longitude 0.28, latitude 0.02 arcsec/day, distance 0.5 km/day) |

  Swiss's own reported speeds disagree with the centered difference of its own
  positions by up to 0.08 arcsec/day (latitude) and ~1e4 km/day (distance), so
  they are not usable as the rate reference; the positions are.
  With Swiss deflection **enabled** the position difference of the planets grows to 0.07–0.5″ in longitude and
  0.03–0.9″ in latitude (largest: Uranus), while Sun and Moon stay below 0.001″. That is the not-implemented deflection and is the largest known
  residual of this module.
- The Moon's rate difference (0.28 arcsec/day ≈ 1e-5 relative) was not
  separated from the finite-difference noise of the comparison; it is recorded,
  not explained.

These residuals describe agreement between two implementations on one kernel.
They are not a JPL Horizons validation and do not certify absolute accuracy.
The Swiss comparison is for the maintainer's local use only (no professional
Swiss licence is held); no Swiss code, binary or output file is part of this
change.

## Still open for MVP 1 (not done here)

1. Gravitational light deflection (Sun first) with its rate; until then the
   comparison flag set is Swiss `NOGDEFL`, not Swiss default.
2. Independent JPL Horizons reference comparison of the apparent quantities.
3. Civil UTC/UT1 → TDB/TT front door for this module (the existing civil adapter
   still emits fixed-axis fields).
4. Row 2: `swe_julday`/`swe_revjul`-shaped calendar contract and historical ΔT.
5. Row 3: houses (`astrological-functions`) — separate repository.
6. Row 4: fixed stars (catalog rights review first).
7. Row 5: Node/WASM binding and load interface. Branches exist
   (`codex/node-*`) but were not reviewed or merged here.
8. Epochs outside 1900–2100, and non-DE441 data (`.bsp`), are unsupported.
9. Integration gate: parallel run, documented rollback, owner approval. Nothing
   here is a production switch.

## Provenance

The 77-term IAU 2000B coefficients and ERFA conformance values come from ERFA
(BSD-3) and are listed in `THIRD_PARTY_NOTICES.md`. The orientation module was
taken unchanged from `feat/apparent-geocentric-20261110`; the apparent module
composes existing stages and adds no new astronomical constants other than the
single-term TT−TDB expression above.
