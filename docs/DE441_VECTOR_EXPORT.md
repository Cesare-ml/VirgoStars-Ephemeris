# Offline classic-DE441 vector export (experimental)

The Rust example `de441_vector_csv` exports one geometric state as CSV so that a reviewer can compare it with an **independently obtained** NASA/JPL Horizons vector. It reads the **classic little-endian DE441 binary** (`.eph`), **not** an SPK `.bsp` file. It never downloads or redistributes ephemeris data.

```sh
cargo run --example de441_vector_csv -- /path/to/de441.eph 2451545.0 moon earth
```

Arguments are `<file> <JD_TDB> <target> <center>`. Supported lowercase names: `ssb`, `mercury`, `venus`, `emb`, `earth`, `moon`, `mars_barycenter`, `jupiter_barycenter`, `saturn_barycenter`, `uranus_barycenter`, `neptune_barycenter`, `pluto_barycenter`, `sun`. Planetary barycenters must **not** be confused with planetary centers. The example rejects malformed input, unknown names, and nonfinite epochs; the reader rejects dates outside the file coverage.

CSV metadata: `jd_tdb` is Julian Date in TDB, `frame=ICRF`, `corrections=GEOMETRIC`, position in km, velocity in km/day. No light time, aberration, precession, nutation, topocentric correction or UTC conversion is applied. The exported JD uses 17 significant decimal digits in scientific notation (`{:.16e}`), sufficient to round-trip the input IEEE-754 `f64` value without introducing an additional CSV rounding error. All six position and velocity components also use 17 significant decimal digits (`{:.16e}`) to preserve finite IEEE-754 `f64` values across CSV serialization and parsing; using 16 digits could silently change the least-significant bits. The original input is used for calculation. This formatting does not increase the physical precision of the epoch or the ephemeris.

For a scientifically meaningful comparison, record the external file's supplier, filename, SHA-256, coverage, the exact Horizons request (epoch/time scale, target, center, frame, correction mode and output units), and the reference response. Convert Horizons velocities from km/s to km/day by multiplying by 86400 if needed. Use an identical ephemeris solution, not merely a similar epoch. Do **not** call agreement with a different kernel proof of DE441 parity.

## Reproducible local evidence capture (not yet executed)

Keep the input binary **outside** the repository. On a POSIX host, the following commands create an audit directory containing the exact input digest, exported CSV, and Rust toolchain version. Replace the path with a legally obtained classic DE441 file; do not commit the binary or a raw Horizons response until its redistribution rights have been reviewed.

```sh
mkdir -p local-de441-audit
sha256sum /path/to/de441.eph > local-de441-audit/de441.sha256
rustc --version > local-de441-audit/rustc-version.txt
cargo run --quiet --example de441_vector_csv -- /path/to/de441.eph 2451545.0 moon earth > local-de441-audit/moon-earth-2451545.csv
```

The digest file records the **literal path** used for hashing and may need path normalization before sharing; the digest itself identifies the input bytes. Preserve the unmodified Horizons request/response, the kernel/solution identity reported by Horizons, and a separate comparison report. A CSV produced by this command alone is **not** evidence of agreement with Horizons. If the reference solution is not independently confirmed as DE441, label the result cross-ephemeris rather than implementation parity. Never set acceptance thresholds retroactively based on the measured residuals.

## Independent comparison protocol (not yet executed)

1. Select an epoch strictly inside the local DE441 header coverage, away from record boundaries for the first comparison; later repeat exactly at and on either side of a boundary. Preserve the full `jd_tdb` value rather than rounding it to a calendar timestamp.
2. Request a **vector** (not an observer-angle or osculating-element table) from Horizons, with geometric corrections (no light-time or aberration), matching target and center IDs, ICRF axes, TDB epoch, and output units. Save the complete request and response, including the Horizons ephemeris solution identifier. Check that an `emb`/outer-planet barycenter export is compared with the corresponding barycenter rather than a planet center.
3. Parse each of the three position and three velocity components independently. Normalize velocity units explicitly (km/s × 86400 = km/day), then calculate signed component residuals `our - reference`, Euclidean position residual in km and Euclidean velocity residual in km/day. Report the maximum absolute component error separately from vector norms.
4. Repeat for at least Earth, Moon relative to Earth, Sun relative to SSB, and a planetary barycenter; use multiple epochs spanning separate DE441 records. Preserve both successful and failed comparisons. Do not select tolerance thresholds after seeing the results: record the proposed error budget, reference-data precision, and acceptance threshold before evaluation.
5. If the Horizons solution is not confirmed to be DE441, classify the exercise as a **cross-ephemeris comparison**, not a DE441 implementation parity test. If frame, center, correction or time-scale metadata is missing, mark that case *inconclusive* rather than treating a small residual as validation.

## Horizons identity and request checklist (reviewer aid)

For the first reproducible comparison, use the following **NAIF/Horizons numeric IDs**, not ambiguous English names. The exported target and center must refer to the same physical objects as the reference query:

| CSV name | Horizons/NAIF ID | Meaning |
| --- | ---: | --- |
| `ssb` | 0 | Solar-system barycenter |
| `sun` | 10 | Sun center |
| `emb` | 3 | Earth–Moon barycenter |
| `earth` | 399 | Earth center |
| `moon` | 301 | Moon center |
| `mars_barycenter` | 4 | Mars-system barycenter, **not** Mars center (499) |
| `jupiter_barycenter` | 5 | Jupiter-system barycenter, **not** Jupiter center (599) |

In Horizons, choose **VECTORS**, geometric state (`VEC_CORR=NONE`), reference plane **FRAME** (`REF_PLANE=FRAME`, not ecliptic), and output units **KM-S** (`OUT_UNITS=KM-S`). Record the returned reference frame and time-scale header; do not assume that the label alone proves a matching ICRF/TDB solution. For example, the CSV pair `moon earth` means target 301 relative to center 399; `sun ssb` means target 10 relative to center 0. Confirm the Horizons center selection in its returned header, since center identifiers can require an explicit center syntax in the Horizons interface.

Keep the complete raw Horizons response with its `$SOE`/`$EOE` vector block and solution provenance. Compare `X,Y,Z` in km and `VX,VY,VZ` in km/s only **after** converting the latter to km/day. Do not compare a rounded calendar-date epoch with the exact `f64` JD exported by Rust without documenting the rounding. This checklist is a request specification, **not** evidence that a query or a real-kernel comparison has been run.

No Horizons response has been imported or evaluated as part of these instructions. They define an auditable procedure, not a claim of measured accuracy.

This tool is a reproducibility aid, **not a scientific validation result**. No external JPL/Horizons vectors or accuracy tolerances are bundled or claimed. See [VALIDATION.md](VALIDATION.md) and [DE441.md](DE441.md).

## Offline residual calculator (Python standard library)

After independently preparing a **single-row** reference CSV with the same column names and units as the Rust exporter, run:

```sh
python3 tools/compare_de441_vectors.py local-de441-audit/moon-earth-2451545.csv /path/to/independent-reference.csv
```

The reference CSV must contain `jd_tdb,target,center,frame,corrections,x_km,y_km,z_km,vx_km_day,vy_km_day,vz_km_day`. Convert Horizons velocity values from km/s to km/day **before** creating it. The comparator refuses missing fields, multiple rows, non-finite values, or mismatched epoch/body/center/frame/corrections, and prints position and velocity Euclidean residual norms and maximum component magnitudes. This is a numerical comparison helper, **not** a parser for raw Horizons output or a guarantee that the supplied reference really uses DE441. Verify and archive its provenance separately; no acceptance thresholds are assumed.

Modification record: 2026-11-12 — added a reproducible local evidence-capture command sequence with input SHA-256 and Rust toolchain provenance (documentation only; not executed). 2026-11-12 — added explicit Horizons body-ID and vector-request checklist (documentation only; no scientific run). 2026-11-11 — added offline vector-export example and comparison instructions; preserved round-trip JD and six-vector-component precision and added example unit tests. Added a deterministic 10,000-pattern finite `f64` round-trip stress regression, including explicit subnormal and finite-extreme boundaries. This tests CSV serialization, **not** DE441 numerical accuracy. Run `cargo test --workspace --all-targets` to include example tests. CPAL-1.0, NOTICE and VirgoStars attribution remain applicable.
