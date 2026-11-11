# Offline classic-DE441 vector export (experimental)

The Rust example `de441_vector_csv` exports one geometric state as CSV so that a reviewer can compare it with an **independently obtained** NASA/JPL Horizons vector. It reads the **classic little-endian DE441 binary** (`.eph`), **not** an SPK `.bsp` file. It never downloads or redistributes ephemeris data.

```sh
cargo run --example de441_vector_csv -- /path/to/de441.eph 2451545.0 moon earth
```

Arguments are `<file> <JD_TDB> <target> <center>`. Supported lowercase names: `ssb`, `mercury`, `venus`, `emb`, `earth`, `moon`, `mars_barycenter`, `jupiter_barycenter`, `saturn_barycenter`, `uranus_barycenter`, `neptune_barycenter`, `pluto_barycenter`, `sun`. Planetary barycenters must **not** be confused with planetary centers. The example rejects malformed input, unknown names, and nonfinite epochs; the reader rejects dates outside the file coverage.

CSV metadata: `jd_tdb` is Julian Date in TDB, `frame=ICRF`, `corrections=GEOMETRIC`, position in km, velocity in km/day. No light time, aberration, precession, nutation, topocentric correction or UTC conversion is applied. The exported JD uses 17 significant decimal digits in scientific notation (`{:.16e}`), sufficient to round-trip the input IEEE-754 `f64` value without introducing an additional CSV rounding error. All six position and velocity components also use 17 significant decimal digits (`{:.16e}`) to preserve finite IEEE-754 `f64` values across CSV serialization and parsing; using 16 digits could silently change the least-significant bits. The original input is used for calculation. This formatting does not increase the physical precision of the epoch or the ephemeris.

For a scientifically meaningful comparison, record the external file's supplier, filename, SHA-256, coverage, the exact Horizons request (epoch/time scale, target, center, frame, correction mode and output units), and the reference response. Convert Horizons velocities from km/s to km/day by multiplying by 86400 if needed. Use an identical ephemeris solution, not merely a similar epoch. Do **not** call agreement with a different kernel proof of DE441 parity.

## Independent comparison protocol (not yet executed)

1. Select an epoch strictly inside the local DE441 header coverage, away from record boundaries for the first comparison; later repeat exactly at and on either side of a boundary. Preserve the full `jd_tdb` value rather than rounding it to a calendar timestamp.
2. Request a **vector** (not an observer-angle or osculating-element table) from Horizons, with geometric corrections (no light-time or aberration), matching target and center IDs, ICRF axes, TDB epoch, and output units. Save the complete request and response, including the Horizons ephemeris solution identifier. Check that an `emb`/outer-planet barycenter export is compared with the corresponding barycenter rather than a planet center.
3. Parse each of the three position and three velocity components independently. Normalize velocity units explicitly (km/s × 86400 = km/day), then calculate signed component residuals `our - reference`, Euclidean position residual in km and Euclidean velocity residual in km/day. Report the maximum absolute component error separately from vector norms.
4. Repeat for at least Earth, Moon relative to Earth, Sun relative to SSB, and a planetary barycenter; use multiple epochs spanning separate DE441 records. Preserve both successful and failed comparisons. Do not select tolerance thresholds after seeing the results: record the proposed error budget, reference-data precision, and acceptance threshold before evaluation.
5. If the Horizons solution is not confirmed to be DE441, classify the exercise as a **cross-ephemeris comparison**, not a DE441 implementation parity test. If frame, center, correction or time-scale metadata is missing, mark that case *inconclusive* rather than treating a small residual as validation.

No Horizons response has been imported or evaluated as part of these instructions. They define an auditable procedure, not a claim of measured accuracy.

This tool is a reproducibility aid, **not a scientific validation result**. No external JPL/Horizons vectors or accuracy tolerances are bundled or claimed. See [VALIDATION.md](VALIDATION.md) and [DE441.md](DE441.md).

Modification record: 2026-11-11 — added offline vector-export example and comparison instructions; preserved round-trip JD and six-vector-component precision and added example unit tests. Added a deterministic 10,000-pattern finite `f64` round-trip stress regression, including explicit subnormal and finite-extreme boundaries. This tests CSV serialization, **not** DE441 numerical accuracy. Run `cargo test --workspace --all-targets` to include example tests. CPAL-1.0, NOTICE and VirgoStars attribution remain applicable.
