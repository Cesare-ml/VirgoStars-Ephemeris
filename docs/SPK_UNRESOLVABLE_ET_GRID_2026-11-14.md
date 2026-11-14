# SPK record grids and binary64 ET resolution

**Modification date:** 2026-11-14  
**Scope:** Rust DAF/SPK Type-2/3 direct geometric state reader. Original code retains its CPAL-1.0 source header and attribution; see [LICENSE](../LICENSE), [NOTICE](../NOTICE) and [LICENSING_AND_CREDITS.md](LICENSING_AND_CREDITS.md).

## Failure mode

A Type-2/3 SPK segment directory declares the first record epoch `INIT`, a record interval `INTLEN` in **TDB/ET seconds past J2000**, and the number of records `N`. The end epoch is calculated as `INIT + N * INTLEN`. The Rust direct-state routine uses an `f64` ET epoch for record selection and interpolation.

Even though `INTLEN` is finite and strictly positive, its duration can disappear at the magnitude of the declared epochs when rounded to binary64. A two-record grid with:

- `INIT = 2^53 = 9007199254740992` seconds
- `INTLEN = 1` second
- `N = 2` records

has a finite end epoch (`INIT+2`) but `INIT+1 == INIT` in `f64`. A state requested at `INIT` could otherwise appear valid even though the time partition is ambiguous at that precision.

This modification rejects a directory if **either** `INIT + INTLEN == INIT` or `END - INTLEN == END`. The existing directory address, numeric finiteness, record metadata and coverage checks remain in place. The check is *fail-closed*: it never silently substitutes a different epoch, changes the chosen record, or reduces an SPK precision requirement. Type-2 vs Type-3 interpolation math remains unchanged.

## Regression fixtures

Unit tests build synthetic two-record DAF/SPK binaries for **both Type 2 and Type 3**, each in **little-endian and big-endian IEEE** form:

1. **Rejected grid:** `INIT=2^53`, `INTLEN=1s`, `N=2`; the directory and each record carry matching, finite binary64 epoch/radius values. The direct reader must return a named `InvalidSegment` error rather than outputting an apparently valid Cartesian position and velocity.
2. **Accepted nearby grid:** `INIT=2^53`, `INTLEN=4s`, `N=2`; both end intervals remain distinguishable. Evaluate the two record midpoints and verify the expected finite XYZ positions, preserving existing support rather than rejecting all large ET magnitudes.

These deliberately unphysical epochs are numerical safety fixtures, not historical/astronomical validation. Real SPK kernels generally use much smaller ET magnitudes. The reference frame remains NAIF frame 1 J2000/ICRF, with positions in **km** and velocities in **km/TDB second**. The reader does not implement other frames or SPK record types.

## Verification

```bash
cargo fmt --all -- --check
cargo test --workspace spk_direct_state::tests::rejects_spk_record_grid_with_unresolvable_tdb_seconds
cargo test --workspace spk_direct_state::tests::accepts_spk_record_grid_with_resolvable_large_tdb_epochs
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Require successful **exact-head** GitHub Actions on Linux, native Windows, and native macOS before review/merge. No kernel is bundled or downloaded. Passing these tests does **not** demonstrate independent numerical agreement with NASA/JPL, NAIF/SPICE or Horizons; that validation remains separate.
