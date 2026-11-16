# MVP 1, row 4 — fixed stars: calculation engine and Hipparcos reader

**Status: Locally tested algorithm and data path; NO star catalog is bundled or
approved for redistribution.** The calculation and the reader for the genuine
Hipparcos file work; the remaining data gates (rights, radial velocity,
magnitude, names, quality policy) are open. Branch: `integration/mvp1-apparent-pipeline-20261116`.

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

## Hipparcos reader (added the same day)

`hipparcos` parses the CDS I/311 `hip2.dat` fixed-width records (276 bytes,
117,955 records) strictly: wrong length, non-ASCII, blank/non-finite fields,
out-of-range RA/Dec, duplicate HIP numbers and a wrong record count are all
errors. `Hip2Record::to_catalog_entry(RadialVelocity)` converts a record to the
engine's entry at epoch J1991.25 (JD 2448349.0625); the catalog contains **no
radial velocity**, so the caller must pass `Measured(km/s)` or an explicit
`AssumedZero` (never a silent default). Zero or negative parallaxes never become
a distance. CLI: `hip_star_at`.

The maintainer authorized downloading `hip2.dat.gz` (8,977,907 bytes) on
2026-11-16; it is kept **outside** the repository. Provenance (URL, sizes,
SHA-256 of the archive and of the extracted file, checks done) is in
`docs/evidence/hip2-source-provenance-2026-11-16.json`. The ignored test
`genuine_hip2_validates_and_known_stars_have_sane_values` (`HIP2_PATH`) reads the
whole file and checks Regulus' parallax and declination.

**End-to-end check with the genuine catalog (local reference, aggregates only,
`docs/evidence/mvp1-hipparcos-star-path-local-reference-receipt-2026-11-16.json`).**
For 16 bright stars on 41 dates (1900–2100) the engine fed with the genuine
hip2 astrometry differs from the local reference, which uses its own catalog
values, by at most 0.007″ (Altair); most stars stay below 0.002″ in longitude.
This mixes catalog-value and algorithm differences and says nothing about the
absolute accuracy of the catalog.

**Quality policy (maintainer decision, 2026-11-16).** `Hip2QualityPolicy::STRICT`
(single star, `Sn = 5`, parallax S/N ≥ 5) rejects nine of those 16 familiar
stars: Aldebaran, Antares, Vega and Betelgeuse (`Sn = 1`, stochastic solution),
Spica (`Sn = 7`), Algol, Capella and Procyon (`Sn = 75`), and Arcturus
(`Sn = 55`, two components). The maintainer accepted these solutions for the
traditional named stars, so `Hip2QualityPolicy::TRADITIONAL_STARS` allows
`Sn ∈ {1, 5, 7, 55, 75}` and `Nc ∈ {1, 2}` with S/N ≥ 5; `STRICT` stays
available. The catalog still flags those solutions as less reliable, and
their linear-motion positions were within a few milliarcseconds of the
independent engine in the local comparison. The ignored test
`genuine_hip2_traditional_policy_accepts_all_sixteen_comparison_stars` confirms on
the genuine file that 7 of the 16 pass `STRICT` and all 16 pass the new policy.
Other stars are not covered by this decision; each new star still needs its own
check of radial velocity, magnitude and identity.

## Not done — remaining data gates

1. **Redistribution terms** of the Hipparcos data have not been reviewed, so no
   derived catalog (curated list, names, magnitudes) is committed.
2. **Radial velocities and V magnitudes** are not in hip2 and need cited sources
   per star. The reference library's catalog must not be the source (rights).
3. **Names and aliases** must be cross-identified by HIP number from
   independent sources; the 81-name HIP target list on the star branch is a
   starting point, unreviewed.
4. **Quality policy** is decided only for the traditional stars above.
5. The Node facade now provides `swe_fixstar_ut` and `swe_fixstar_mag` with
   caller-supplied names, aliases, HIP identifiers, explicit radial velocity and
   magnitude; no reviewed catalog is included. A null radial velocity is an
   explicit zero assumption; a null magnitude produces `E_STAR_MAGNITUDE`.
   `fixstarUt` uses the engine's selected UT conversion policy. The facade is
   tested locally but is not a claim of full Swiss star-name compatibility.
6. No JPL Horizons or SOFA/NOVAS comparison beyond the ERFA fixtures; rays
   closer to the Sun than the deflection model's 1000× rule are rejected.
