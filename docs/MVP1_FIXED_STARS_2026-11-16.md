# MVP 1, row 4 — fixed stars: calculation engine (no catalog yet)

**Status: Locally tested algorithm; NO star catalog is bundled or approved.**
The calculation is ready; the data gate (real catalog, rights, identification)
is open. Branch: `integration/mvp1-apparent-pipeline-20261116`.

## What exists

`fixed_star` (Rust, `virgostars-ephemeris-core`) takes a **caller-supplied**
catalog entry and returns the apparent geocentric position in the true
ecliptic and equinox of date, with rates:

| Item | Value |
|---|---|
| Entry | ICRS RA/Dec (deg) at an explicit TDB epoch, μα* and μδ (mas/yr), parallax (mas), barycentric radial velocity (km/s) |
| Motion | constant rectilinear space motion from the catalog epoch (no stellar light-time, no nonlinear motion) |
| Observer | Earth from DE441: annual parallax by subtracting Earth's barycentric position |
| Corrections | first-order solar light deflection (`StellarSolarDeflection::PointMass`, default in the CLI; `Omit` to disable), special-relativistic annual aberration, IAU 2006 precession and IAU 2000B nutation (`earth_orientation`) |
| Rates | centered difference (±0.01 TDB day) of the whole selected pipeline |
| Errors | zero/negative/absurd parallax, poles, occulted ray, incoherent TT/TDB, epoch outside 1900–2100: all explicit errors, nothing invented |

CLI: `cargo run --release --example fixed_star_at -- [--no-solar-deflection] <de441.eph> <TDB-JD> <name> <ra_deg> <dec_deg> <pm_ra_mas_yr> <pm_dec_mas_yr> <parallax_mas> <rv_km_s> [epoch_tdb_jd]`.

The module was integrated from the experimental star branch
(`codex/…`/`feat/hipparcos-curated-import-20261115`, `fixed_star.rs`, unchanged)
because it depends only on modules that already exist on this branch.

## Evidence (2026-11-16)

- Unit tests: ERFA `pmpx`, `ldsun` and `atci13` conformance values (propagation
  to 1 nanoradian, solar deflection vector, near-Sun pipeline), invalid input,
  RA wrap. `cargo fmt`, `clippy -D warnings` and the whole workspace pass.
- Real kernel, ignored tests with `DE441_PATH`: the near-Sun ERFA pipeline check
  and `real_de441_solar_deflection_of_a_star_follows_the_cot_half_elongation_law`
  (a **fictional** star 1° from the Sun is displaced by the closed-form
  (2GM/c²/E)·cot(ψ/2) ≈ 0.48″ within 5 %).
- **Local comparison with an independent reference** (the Swiss Ephemeris C
  library in JPL mode on the same `de441.eph`, local only). Both engines were fed
  *identical caller inputs* for 16 bright stars on 41 dates between 1900 and
  2100, so this isolates the algorithm and says nothing about any catalog.
  Maximum differences, with Swiss defaults/flags matching ours in both settings:

  | Quantity | Max abs difference |
  |---|---|
  | longitude | 0.0011″ (Deneb), ≤ 0.0009″ for all other stars |
  | latitude | 0.00055″ |
  | distance | 1.2e-9 relative |
  | longitude / latitude rate vs centered difference of reference positions | ≤ 7e-6″/day, ≤ 2e-7″/day |

  Near the Sun (four dates with Regulus roughly 1°–4° from the Sun, inferred from
  the size of the deflection; deflection on and off) positions agree to ≤ 4e-5″, while deflection itself moves the star by up to
  0.5″.
  Aggregates only: `docs/evidence/mvp1-fixed-star-local-reference-receipt-2026-11-16.json`.
  No reference-library output rows, catalog values or code are stored. The
  comparison is for the maintainer's local use only.

## Not done — the data gate

1. **No catalog is bundled.** The reference library's own star file must not be
   used as a source (rights); the 81-star target list in the star branch lists
   names and HIP numbers only.
2. **Real data**: the planned source is the Hipparcos new reduction, CDS
   I/311/hip2 (van Leeuwen 2007; ICRS, epoch J1991.25 = JD 2448349.0625). It has
   to be downloaded by the maintainer or with explicit permission, its terms of
   reuse reviewed, and checksummed. The existing Node importer
   (`scripts/import-hipparcos-2007.mjs`) is JavaScript and was **not** reviewed or
   merged here; the long-term importer belongs in Rust.
3. **Not in Hipparcos**: V magnitude and barycentric radial velocity need
   separate, cited sources per star; companions, binaries and variables need
   special treatment (the importer's single-star solution filter helps).
4. **Names and aliases** must be cross-identified by HIP number from independent
   sources, never from the reference library's list.
5. `swe_fixstar_ut`/`swe_fixstar_mag`-shaped facade, magnitude lookup and
   missing-star errors belong to the Node/WASM binding (row 5).
6. No JPL Horizons or SOFA/NOVAS comparison beyond the ERFA fixtures; stars
   closer to the Sun than the deflection model's 1000× rule are rejected.
