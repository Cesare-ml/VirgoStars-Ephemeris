# SPK Type 2/3: select records using original binary64 ET boundaries

**Modification date:** 2026-11-14  
**Scope:** canonical Rust `spk_direct_state`, synthetic unit regression only.  
**Licensing:** original VirgoStars source stays CPAL-1.0; retain Exhibit A, [LICENSE](../LICENSE), [NOTICE](../NOTICE), and [LICENSING_AND_CREDITS.md](LICENSING_AND_CREDITS.md).

## Failure reproduced

A Type 2 or Type 3 SPK segment has `INIT` (initial epoch) and `INTLEN` (record length), both in **TDB ET seconds past J2000**. The previous record selection used:

```text
floor((et - INIT) / INTLEN)
```

At a record boundary that is exactly representable as binary64, subtraction and division can round the **original epoch immediately before the boundary** up to the next record's integer index.

For example, with `INIT = -10`, `INTLEN = 10`, and two records, the exact second-record boundary is ET `0`. Its immediately preceding binary64 value, `-f64::from_bits(1)` (the smallest negative subnormal), satisfies `floor((et + 10) / 10) == 1` due to rounding, even though `et < 0`. With distinct synthetic Chebyshev coefficients this can silently return a plausible **but incorrect** state from the second record.

## Correction

After existing `record_layout` validation, binary-search the **directory-defined start boundaries** `INIT + i * INTLEN` for `i = 1 .. N-1` and compare each directly to the original supplied `et`:

- `et < boundary` belongs to the preceding record;
- `et == boundary` belongs to the following record;
- the segment's final inclusive endpoint remains in the last record;
- a nonfinite computed boundary is rejected with `SpkStateError::InvalidSegment`.

Record lookup remains `O(log N)` and performs no coefficient reads during the search. The record `MID`/`RADIUS` coherence check introduced with #150 continues to run after selection. The fix does **not** recover precision already lost in the caller's single binary64 ET input, or guarantee distinguishability when consecutive directory boundaries themselves collapse at that precision.

## Regression contract

The unit fixture modifies the existing two-record synthetic SPK kernel to span ET `[-10, +10]` with its interior boundary at zero. In both `LTL-IEEE` and `BIG-IEEE`, and for both SPK Types 2 and 3, the test asserts:

1. The old quotient actually rounds the negative-subnormal predecessor to integer record index `1`.
2. At that original predecessor the returned X position is **110 km**, from the *first* record.
3. At ET zero and its immediately following positive subnormal the X position is **210 km**, from the *second* record.

The fixture supplies only geometric Cartesian coefficients (km) and km/s velocities in the fixed NAIF J2000/ICRF frame. No observer, astronomical time-scale conversion, aberration, light time or leap-second handling is changed.

## Verification and limits

```sh
cargo fmt --all -- --check
cargo test --workspace --locked spk_direct_state::tests::near_zero_record_boundary_uses_original_tdb_epoch
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Run the read-only Linux/Windows/macOS GitHub CI for the **exact PR head** before merging. This is a synthetic floating-point selection test; it is not independent numerical evidence against real NAIF/SPICE or NASA/JPL ephemeris kernels, and does not establish production readiness.
