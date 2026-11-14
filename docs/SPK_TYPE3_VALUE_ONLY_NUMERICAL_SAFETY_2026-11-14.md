# SPK Type-3 value-only interpolation: avoid false derivative overflows

**Modification date:** 2026-11-14  
**Original developer:** Cesare-ml (VirgoStars project). Original Rust source and its modifications remain subject to [CPAL-1.0](../LICENSE), [NOTICE](../NOTICE) and [Licensing and Credits](LICENSING_AND_CREDITS.md).

## Problem

The existing Rust `direct_spk_state` used the same Clenshaw function to evaluate Chebyshev polynomials for both SPK Type 2 and Type 3. That function returns the polynomial value **and its derivative**, rejecting a result whenever either is non-finite.

- **Type 2:** coefficients encode Cartesian **position in km**. The interpolated velocity **in km per TDB second** must be calculated from the position polynomial derivative divided by the record radius (seconds). The derivative is required and its overflow must remain an error.
- **Type 3:** coefficients encode Cartesian **position and velocity as six independent series**. The requested state needs **only the polynomial values** of all six series. The derivative of any series (especially the three velocity series) is **not** a required result. If an unneeded derivative exceeds binary64 range while the requested value is finite, rejecting the state is a spurious failure.

A new crate-internal `evaluate_chebyshev_value` performs the same validated Chebyshev Clenshaw value recurrence **without** computing the unused derivative. It enforces the existing coefficient validation, normalized domain `[-1,1]`, and non-finite output protection. All Type-2 paths continue calling the original value+derivative evaluator; Type-3 paths call the value-only evaluator.

This changes neither the public Rust API nor the SPK file format, endian support, segment precedence, reference frame (type 2/3, frame ID 1 only), km/km/s units, nor the mathematical result of ordinary finite Type-3 values.

## Deterministic tests

- In `chebyshev.rs`, compare the value-only result with the original evaluator for a finite high-order series at `x=-1,-0.75,0,0.125,0.9,1`. Reject nonfinite/out-of-domain inputs as before.
- Reproduce the false failure with `c_3=8e307` and all lower coefficients zero at normalized `x=0`. The value is **T₃(0)=0**; its derivative `c₃*T₃'(0)` is not representable in binary64. The new value-only result is correctly `0`; the original value+derivative evaluator correctly rejects the derivative when explicitly requested.
- In `spk_direct_state.rs`, build an entirely synthetic **four-coefficient Type-3 SPK record** in both little- and big-endian DAF encodings. Verify the actual reader returns finite XYZ position and independently encoded XYZ velocity at ET=5s with the otherwise-overflowing position derivative.

Test coefficients are deliberately numerically extreme and **not physically plausible ephemeris data**. The regression checks that a numerical quantity is not a prerequisite for an API that does not return that quantity. It is not a demonstration of NASA/JPL/SPICE numerical parity or actual DE441 accuracy.

## Verification

From the repository root, on the PR's **exact head**:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Read-only CI also runs native Rust tests on Windows and macOS. Do not infer success until the latest GitHub Actions run is complete.

## Scientific provenance

The Type-2 and Type-3 model distinctions follow the [NAIF SPK Required Reading](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html). No NAIF/SPICE algorithms, copyrighted kernel data, third-party code, new dependency or external download is introduced. Existing repository attribution remains in all modified source files.
