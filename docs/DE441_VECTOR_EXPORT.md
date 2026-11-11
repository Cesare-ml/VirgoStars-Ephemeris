# Offline classic-DE441 vector export (experimental)

The Rust example `de441_vector_csv` exports one geometric state as CSV so that a reviewer can compare it with an **independently obtained** NASA/JPL Horizons vector. It reads the **classic little-endian DE441 binary** (`.eph`), **not** an SPK `.bsp` file. It never downloads or redistributes ephemeris data.

```sh
cargo run --example de441_vector_csv -- /path/to/de441.eph 2451545.0 moon earth
```

Arguments are `<file> <JD_TDB> <target> <center>`. Supported lowercase names: `ssb`, `mercury`, `venus`, `emb`, `earth`, `moon`, `mars_barycenter`, `jupiter_barycenter`, `saturn_barycenter`, `uranus_barycenter`, `neptune_barycenter`, `pluto_barycenter`, `sun`. Planetary barycenters must **not** be confused with planetary centers. The example rejects malformed input, unknown names, and nonfinite epochs; the reader rejects dates outside the file coverage.

CSV metadata: `jd_tdb` is Julian Date in TDB, `frame=ICRF`, `corrections=GEOMETRIC`, position in km, velocity in km/day. No light time, aberration, precession, nutation, topocentric correction or UTC conversion is applied. The exported JD uses 17 significant decimal digits in scientific notation (`{:.16e}`), sufficient to round-trip the input IEEE-754 `f64` value without introducing an additional CSV rounding error. All six position and velocity components also use 17 significant decimal digits (`{:.16e}`) to preserve finite IEEE-754 `f64` values across CSV serialization and parsing; using 16 digits could silently change the least-significant bits. The original input is used for calculation. This formatting does not increase the physical precision of the epoch or the ephemeris.

For a scientifically meaningful comparison, record the external file's supplier, filename, SHA-256, coverage, the exact Horizons request (epoch/time scale, target, center, frame, correction mode and output units), and the reference response. Convert Horizons velocities from km/s to km/day by multiplying by 86400 if needed. Use an identical ephemeris solution, not merely a similar epoch. Do **not** call agreement with a different kernel proof of DE441 parity.

This tool is a reproducibility aid, **not a scientific validation result**. No external JPL/Horizons vectors or accuracy tolerances are bundled or claimed. See [VALIDATION.md](VALIDATION.md) and [DE441.md](DE441.md).

Modification record: 2026-11-11 — added offline vector-export example and comparison instructions; preserved round-trip JD and six-vector-component precision and added example unit tests. Run `cargo test --workspace --all-targets` to include example tests. CPAL-1.0, NOTICE and VirgoStars attribution remain applicable.
