# Offline classic-DE441 vector export (experimental)

The Rust example `de441_vector_csv` exports one geometric state as CSV so that a reviewer can compare it with an **independently obtained** NASA/JPL Horizons vector. It reads the **classic little-endian DE441 binary** (`.eph`), **not** an SPK `.bsp` file. It never downloads or redistributes ephemeris data.

```sh
cargo run --example de441_vector_csv -- /path/to/de441.eph 2451545.0 moon earth
```

Arguments are `<file> <JD_TDB> <target> <center>`. Supported lowercase names: `ssb`, `mercury`, `venus`, `emb`, `earth`, `moon`, `mars_barycenter`, `jupiter_barycenter`, `saturn_barycenter`, `uranus_barycenter`, `neptune_barycenter`, `pluto_barycenter`, `sun`. Planetary barycenters must **not** be confused with planetary centers. The example rejects malformed input, unknown names, and nonfinite epochs; the reader rejects dates outside the file coverage.

CSV metadata: `jd_tdb` is Julian Date in TDB, `frame=ICRF`, `corrections=GEOMETRIC`, position in km, velocity in km/day. No light time, aberration, precession, nutation, topocentric correction or UTC conversion is applied. The printed JD has nine decimal places; the original input is used for calculation.

For a scientifically meaningful comparison, record the external file's supplier, filename, SHA-256, coverage, the exact Horizons request (epoch/time scale, target, center, frame, correction mode and output units), and the reference response. Convert Horizons velocities from km/s to km/day by multiplying by 86400 if needed. Use an identical ephemeris solution, not merely a similar epoch. Do **not** call agreement with a different kernel proof of DE441 parity.

This tool is a reproducibility aid, **not a scientific validation result**. No external JPL/Horizons vectors or accuracy tolerances are bundled or claimed. See [VALIDATION.md](VALIDATION.md) and [DE441.md](DE441.md).

Modification record: 2026-11-11 — added offline vector-export example and comparison instructions. CPAL-1.0, NOTICE and VirgoStars attribution remain applicable.
