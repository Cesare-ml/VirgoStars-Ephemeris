# Manual official SPK scientific validation — pending

The project **has not yet successfully read and compared a real NASA/JPL kernel**.

## Source and scope

Official NAIF planet SPK archive: https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/
Technical spec: https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html
DAF spec: https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/daf.html
JPL Horizons: https://ssd.jpl.nasa.gov/horizons/

1. Download an unmodified, modern, compact official JPL SPK, e.g. `de440s.bsp`, **manually**. Confirm source and rights; do not commit the binary into Git.
2. Run `node scripts/inspect-spk.mjs /path/to/de440s.bsp` and keep the SHA-256, segment list and range in an internal validation receipt.
3. Choose an epoch strictly inside a relevant segment's documented TDB coverage and compute the TDB seconds since JD 2451545.0 **with an independent, explicit time converter**.
4. Run `node scripts/inspect-spk.mjs /path/to/de440s.bsp ET_TDB_SECONDS 10 399` for geometric Sun relative to Earth. Do not mistake body 3 (Earth–Moon barycenter) for 399 (Earth center).
5. Request matching NAIF/Horizons geometric vectors at the same epoch, target, center, frame and correction flags. Compare all Cartesian km and km/s, across multiple epochs/segment boundaries.
6. Record worst-case deviations, expected rounding, actual SPK type coverage, source hashes and test environment. Mark failures; do not simply widen tolerances.

**Important:** the current prototype reads SPK type 2 and frame ID 1 only, and holds the entire binary in memory. Only geometric vectors exist. It must not claim zodiac/apparent/house parity or publish to production based on this smoke.

No GitHub Actions, no production modification, no automated downloads.
