# Classic DE441 — permanent combined boundary regressions

**Modified:** 2026-11-14. Original project source remains under CPAL-1.0.
**Branch base:** `dev/ephemeris` at `3854e7792d40740e2dfe1369d9db6406063a8aef`.
**File:** `crates/ephemeris-core/tests/de441_combined_boundary_guards.rs`.

## Why this change is needed

The five fixes previously verified together by integration-only PR
[#204](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/204)
are **already merged into `dev/ephemeris`** as original reviewed
PRs #174 (record-size limit), #178 (velocity arithmetic),
#182 (negative TDB epochs), #187 (granule representability and
normalization), and #193 (principal-record boundary selection).

However, the **three cross-feature regression tests** that exercised
multiple fixes together were only present on the deliberately unmergeable
PR #204. This test-only follow-up preserves those exact three already
tested functions as a normal, independently reviewable addition to
the integrated Rust core. No production reader logic, APIs, dependencies,
GitHub workflows, external ephemeris binaries or licensing requirements
are changed.

## Regressions

1. `signed_zero_neighborhood_obeys_both_record_and_granule_boundaries`
   chooses the correct synthetic Mercury series at TDB Julian dates
   around -16, 0 and +16, including the neighboring binary64 values
   around those boundaries, the final inclusive coverage endpoint and
   backward cached-record traversal.
2. `distant_negative_tdb_record_and_granule_boundaries_are_distinct`
   verifies analogous behavior using a synthetic archival-scale negative
   Julian date and rejects epochs outside the file's declared coverage.
3. `record_and_granule_transitions_preserve_extreme_finite_velocity`
   combines those boundary transitions with a large finite Chebyshev
   coefficient (T1 = 1e308). The 16-TDB-day granule must produce a
   finite 1.25e307 km/TDB-day derivative; it checks the analytical
   Cartesian states and cached-record transitions.

The fixture's values are deliberately discontinuous **synthetic
Chebyshev polynomials**, not astronomical positions. The interface contract
is a classic little-endian DE441 binary, explicit JD in **TDB**, geometric
**ICRF** Mercury relative to the **Solar System Barycenter**, XYZ in
**kilometers**, velocities in **kilometers per TDB day**. No UTC conversion,
light-time, nutation, aberration, topocentric correction or SPK support
is implied.

## Verification and provenance

The three original functions were copied unchanged from the exact
integration-only PR #204 head
`ee103d17f96070bb058117c7e185f9123f71421b`, where GitHub Actions
[run #43599084666](https://github.com/Cesare-ml/VirgoStars-Ephemeris/actions/runs/43599084666)
reported 3/3 passing in the `de441_combined_boundary_guards.rs` test
target, with full Linux, macOS and Windows CI successful.

**That prior CI was on the pre-integration test branch and does not
by itself certify the current destination `dev/ephemeris` head.**
The proposed permanent test-only PR must pass its **own exact-head CI**
against the already-integrated runtime, including:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked --test de441_combined_boundary_guards
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Native Windows/macOS CI should also pass. There is no local Cargo
execution in the GitHub connector session. All tests are structural and
arithmetic; **independent real NASA/JPL/NAIF/Horizons scientific-vector
validation is still an outstanding separate release gate.**

## Integration safety

This PR adds only the three regression functions and this document;
its target is `dev/ephemeris`. Do not merge the duplicate integration
PRs #198 or #204. They remain historical evidence and can be closed
as superseded after reviewing this permanent test-only PR.

Keep `main`, deployments, package publishing and real-kernel
distribution unchanged. Retain the required "Powered by VirgoStars"
attribution: https://www.virgostars.it/.
