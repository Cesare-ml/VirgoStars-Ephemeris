# Classic DE441 declared-coverage inspector

**Status (2026-11-13):** experimental, offline, read-only Rust example; **not** a scientific accuracy or provenance certification.

The `inspect_de441_coverage` example reads a **caller-supplied classic little-endian JPL DE441 binary** (`.eph`) using the existing `De441::open` parser. It reports the declared start/end header interval in **TDB Julian days (JD TDB)**, inclusive. It optionally labels one explicitly supplied JD TDB as `covered` or `out_of_coverage` according to those metadata bounds.

```sh
cargo run --offline --locked --example inspect_de441_coverage -- /path/to/de441.eph
cargo run --offline --locked --example inspect_de441_coverage -- /path/to/de441.eph --at-jd 2451545.0
cargo test --workspace --example inspect_de441_coverage --locked
```

Standard output is a two-line CSV without a header banner:

```csv
format,epoch_scale,declared_start_jd,declared_end_jd,probe_jd,probe_status
classic-de441,TDB,1000.000000000000,1032.000000000000,1016.000000000000,covered
```

The sample above is from a **synthetic test fixture**, not an official JPL kernel. When no `--at-jd` is passed, `probe_jd` is empty and `probe_status` is `not_checked`; values outside the inclusive interval are reported as `out_of_coverage`. An out-of-coverage probe is a **valid diagnostic result** and exits successfully; consumers that require coverage must explicitly check `probe_status`. Missing/broken kernels, invalid flags and nonfinite probes cause a nonzero exit and an error on standard error before any CSV output.

**Scientific contract and limitations**

- The timescale is **TDB**. This tool **does not convert UTC, TT or local civil dates** to TDB, and it does not select default epochs.
- The coverage is **declared file-level header metadata**; the existing reader checks the classic header, record layout and declared file length. This tool does **not inspect every coefficient record**, prove per-body series availability, interpolate vectors, or compare states with independent JPL Horizons/NAIF output.
- It cannot read NAIF SPK `.bsp`, compressed Astrodienst `.se1` or any arbitrary JPL binary format. SPK inspection is a **separate** workstream.
- No reference frame, target/center, apparent correction, physical model, velocity or numerical tolerance is inferred from a coverage row.
- A `covered` row **does not establish scientific validity**, real-kernel provenance or usable numerical precision. Existing validation gates, including independently sourced reference data, remain mandatory under [issue #3](https://github.com/Cesare-ml/VirgoStars-Ephemeris/issues/3).

## Verification

Five original deterministic tests cover the known synthetic header, inclusive boundaries, immediately out-of-bounds epochs, CLI argument shape and rejection of nonfinite TDB probes. CI is expected to run these specifically with `cargo test --workspace --example inspect_de441_coverage --locked`; review the status at the PR commit. There is **no bundled third-party ephemeris dataset** and no network access required.

## License and modifications

Original Rust example and test fixture: CPAL-1.0, consistent with [LICENSE](../LICENSE) and [NOTICE](../NOTICE). Added on 2026-11-13. No NASA/JPL file, SPICE Toolkit code, Swiss Ephemeris source or Astrodienst dataset was incorporated. Required downstream VirgoStars attribution is described in [licensing and credits](LICENSING_AND_CREDITS.md); the original project is [VirgoStars](https://www.virgostars.it/).
