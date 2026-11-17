# SPK Type 2/3 Chebyshev record/directory time coherence — 2026-11-14

CPAL-1.0 §3.3 modification record: Cesare-ml (VirgoStars project), original CPAL-covered source; modifications preserve the original source notices, attribution and credits. This change is stacked on the **unmerged** Type 2/3 SPK evaluator PR #150, itself dependent on DAF/SPK index PR #148.

## Problem and correction

The SPK Type 2 and Type 3 data-segment **directory** stores INIT (first-record start ET) and INTLEN (duration in seconds of each record), while every coefficient record stores its own MID (time midpoint) and RADIUS (half-duration). A corrupt record with a shifted MID or altered RADIUS could previously yield a plausible finite `x=(ET-MID)/RADIUS` in [-1,1], silently mapping an otherwise valid ET to an incorrect part of the Chebyshev polynomial.

Before interpolating, the Rust evaluator now verifies that the selected record agrees with its directory:

- `expected_midpoint = INIT + (record_index + 0.5) * INTLEN`
- `expected_radius = 0.5 * INTLEN`
- `|MID - expected_midpoint| <= 32 * f64::EPSILON * max(|INIT|, |expected_midpoint|, |INTLEN|)`
- `|RADIUS - expected_radius| <= 32 * f64::EPSILON * |INTLEN|`

This tolerance accommodates small binary64 encoding/operation differences at the ET scale; it does not accept second-scale shifts or duration errors in typical planetary kernels. Degenerate/unrepresentable derived midpoints or radii fail closed. `SpkStateError::InvalidSegment` describes a mismatch, without changing the public API.

New deterministic unit regressions exercise both Type 2 and Type 3, both little- and big-endian DAF data, both records of a synthetic segment, shifted MID and short/long RADIUS, plus accepted small binary64 roundoff. The tests assert an error **even when an altered record would still interpolate to finite numerical values**.

## Scientific contract and validation limits

- **Time:** ET seconds past J2000 on TDB scale; this is *not* UTC, TT or a time-scale conversion.
- **Frame:** type 2/3 SPK evaluator supports only NAIF frame 1 (J2000/ICRF axes) and only direct target-versus-segment-center geometric states.
- **Units:** km positions, km/s velocities; no aberration, light time, precession/nutation, observer transformations or apparent positions.
- **Limits:** these are synthetic malformed-file tests, not proof of compatibility with a real JPL `.bsp`, NAIF Toolkit or Horizons. The stated tolerance is a local engineering preflight and must still be evaluated against authentic kernels and independent vectors before production use.

Source format: [NAIF SPK Required Reading — Type 2/3](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html).

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The authoring environment has no Rust compiler; execution must be checked through the permitted read-only CI. In particular, **do not merge or publish this stacked draft as a standalone working SPK reader**: review parent #148, #150 and their downstream stacks in order, and independently validate real NASA/JPL kernels before a release.
