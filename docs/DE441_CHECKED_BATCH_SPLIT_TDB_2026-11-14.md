# Checked batch geometric DE441 lookup from split TDB dates

**Date:** 2026-11-14. **Status:** experimental, dependent on PR #136 then PR #139.
This is **not** a feature of `main`, not an independently certified astronomical engine,
and not a complete Swiss Ephemeris replacement.

## Motivation and API

A single sky calculation commonly needs several target/center geometric
states at the **same** TDB instant. Existing draft PR #139 checks split-JD
quantization for each scalar lookup. This opt-in batch API checks the supplied
epoch **once**, then reads each pair through the existing classic DE441 reader,
whose last coefficient record is cached. It neither changes the reader nor
duplicates any astronomical interpolation algorithm.

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::de441::{Body, De441};
use virgostars_ephemeris_core::de441_checked_batch::checked_states_at_split_tdb;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut reader = De441::open("/path/to/legally-obtained/de441.eph")?;
let epoch = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb)?;
let requests = [
    (Body::Moon, Body::Earth),
    (Body::Sun, Body::Earth),
    (Body::Mercury, Body::Earth),
];
let states = checked_states_at_split_tdb(&mut reader, &requests, epoch, 0.001)?;
for result in states {
    println!(
        "{:?} relative to {:?}: {:?} km; JD input rounding {} s",
        result.state.target,
        result.state.center,
        result.state.position_km,
        result.quantization_error_seconds
    );
}
# Ok(())
# }
```

Return type is `Result<Vec<CheckedDe441State>, CheckedStateError>`.
Results preserve the order and duplicates of `requests`. For every pair,
state metadata includes the **same rounded TDB JD** as the other pairs,
position in **fixed ICRF km**, velocity in **fixed ICRF km per TDB day**, and
a shared measured **split-JD-to-f64 input rounding loss in seconds**.
Mars through Pluto may denote *system barycenters*, not planet centers.

Inputs must already be on the **TDB** scale. The caller supplies a finite,
nonnegative precision budget measured in seconds. An invalid budget or time
scale, or unacceptable quantization, is rejected before reading any
coefficient records. For empty requests, validate the epoch/budget and
return an empty vector without record I/O.

Any missing target series, invalid kernel, I/O error or out-of-coverage
epoch is returned as an error, without a partial vector or approximate
fallback. The reader's existing cache may still contain prior data after
an error, the same as for scalar lookups. No atomic I/O rollback is claimed.

## Verification

Four new deterministic Rust tests reuse the synthetic one-record classic
DE441 fixture in `de441_checked_state.rs`:

1. Three requests (including a duplicate and zero relative state) match
   scalar results and require only **one** coefficient-record read.
2. A strict quantization budget fails before record I/O, while a relaxed
   budget permits multiple states sharing the same reported precision loss.
3. Empty batches still reject non-TDB epochs, invalid budgets and lost
   precision, without record reads.
4. A missing series in the middle of a batch fails the whole call, and
   out-of-coverage errors are propagated.

Execute `cargo fmt --all -- --check`, `cargo test --workspace --locked`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`, and
the repository's permitted read-only Linux/Windows/macOS CI.

These tests use **synthetic values only**; they do not verify independent
NASA/JPL/Horizons numerical accuracy or show production readiness. The
single-f64 JD used internally by the classic reader remains a precision
boundary. Input rounding tolerance is **not** an astronomical
position/velocity error bound. No UTC↔TDB conversion, light-time,
aberration, observer position, SPK support or apparent-position calculation
is performed here.

## Dependency and integration order

- **PR #136** defines the validated split-JD → classic single-f64 JD bridge.
- **PR #139** defines the opt-in checked scalar geometric lookup.
- **This PR** reuses those two interfaces; it must not merge before both
  have been reviewed and integrated, in that order.
- PR #171 adds a separate ET → split-JD conversion and precision audit on
  `main`. No code from it is duplicated or assumed present in this branch.
- An actual high-precision reader and independently verified JPL/NAIF
  reference matrices remain separate future work.

This PR targets `dev/ephemeris` **only** to exercise the existing
read-only CI; its cumulative GitHub diff includes the dependencies.
The independent incremental review diff must be compared against
`feat/de441-checked-split-state-20261112`.

## License and change record

Modified 2026-11-14: added `de441_checked_batch.rs`, exported it through
`lib.rs`, added four synthetic tests to the existing checked scalar-state
test module, and created this note. Original code remains CPAL-1.0,
including the required `Powered by VirgoStars` attribution and
[NOTICE](../NOTICE). No dependencies, external kernels, Swiss/NAIF code,
credentials, releases, deployment or production migrations were added.
