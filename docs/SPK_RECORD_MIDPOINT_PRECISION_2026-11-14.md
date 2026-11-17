# SPK Type-2/3 midpoint precision bound — 2026-11-14

**Modification record (CPAL-1.0 §3.3).** Original VirgoStars-Ephemeris source notices and the *Powered by VirgoStars* attribution remain unchanged. No dependencies or externally sourced kernels were added.

## Problem

The existing Rust direct Type-2/Type-3 SPK evaluator checks that a coefficient record's `MID` and `RADIUS` agree with the DAF/SPK directory's `INIT` and `INTLEN`. The midpoint check previously allowed an absolute difference of `32 * f64::EPSILON * max(|INIT|, |MID_expected|, |INTLEN|)` seconds. For extremely large but finite ET values, that rounding allowance can exceed a **large fraction of the record width**, accepting a shifted record that returns numerically finite but wrong positions.

A second edge case is a representable record *interval* whose intended midpoint rounds exactly onto the record start or end. The existing interval-width guard does not guarantee a distinct representable midpoint.

## Change

For the **selected** record only:

1. Reject the directory grid when the computed midpoint is not *strictly between* its computed start and end in binary64 TDB/ET seconds.
2. Cap the allowed midpoint/record discrepancy at the smaller of the existing `32 * epsilon * epoch scale` tolerance and **one quarter of `INTLEN`**. The existing radius tolerance, segment selection, byte-order support, coefficient interpolation and velocity units remain unchanged.

Both conditions return `SpkStateError::InvalidSegment`, preserving the existing public enum and API. Valid ordinary epochs are unaffected; the tightened behavior rejects only previously accepted numerically degenerate/corrupt records.

## Synthetic regression cases

- `INIT = 2^53` TDB seconds past J2000, `INTLEN = 4` seconds, two records: shifting the first record's midpoint by +2 seconds is less than the old absolute tolerance (about 64 s), yet one half-record out of phase. Even though its normalized Chebyshev coordinate would be exactly -1 and finite, the new check rejects it.
- The same `INIT`, `INTLEN = 2` seconds: both record midpoints round to their respective endpoints and now fail closed.
- Both SPK Type 2 and Type 3, both IEEE little- and big-endian, using in-memory **synthetic DAF/SPK** fixtures.
- Existing positive large-epoch `INTLEN = 4` test and sub-ULP-scale directory-roundoff test remain in the same module to prevent indiscriminate rejection of valid records.

This is **numeric safety validation, not astronomical accuracy validation**. It does not demonstrate real-kernel interoperability, Horizons/NAIF vector agreement, or production readiness.

## Reproduce and review

```sh
cargo fmt --all -- --check
cargo test --workspace --locked spk_direct_state::tests::rejects_midpoint_shift_that_exceeds_record_fraction_but_not_old_ulp_tolerance
cargo test --workspace --locked spk_direct_state::tests::rejects_unrepresentable_record_midpoints_on_both_sides_of_grid
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Review the read-only GitHub Actions results on the **exact PR head**, for Linux plus native Windows/macOS. This change does not modify the time scale (TDB seconds since J2000), the supported NAIF frame (1/J2000-ICRF), the direct geometric target/center definition, or the output units (km and km/s).

Related existing design: [Type-2/3 record coherence](RUST_SPK_TYPE23_RECORD_GRID_COHERENCE_2026-11-14.md), [binary64 record grid resolution](SPK_UNRESOLVABLE_ET_GRID_2026-11-14.md).
