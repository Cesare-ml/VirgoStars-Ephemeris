# DE441: precision of Earth/Moon states relative to the Earth–Moon barycenter

Date: 2026-11-10
Status: **experimental implementation merged** into `dev/ephemeris` by [PR #29](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/29) and into `main` by [PR #32](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/32). Synthetic CI checks passed; independent NASA/JPL/Horizons numerical validation is still outstanding.

## Issue and calculation

The classic JPL DE441 file contains a geometric Moon-relative-to-Earth
position/velocity series as well as the Earth–Moon barycenter (EMB) series.
Previously the implementation calculated `Moon - EMB` and `Earth - EMB`
by first forming large absolute barycentric states, then subtracting
the EMB position. This can erase small relative offsets due to floating-point
cancellation; the same issue affects velocities when scales are disparate.

Let `g` be the geocentric Moon series (position and velocity), and let
`R = mass(Earth) / mass(Moon)` be the positive Earth/Moon ratio from the
binary header:

- `Earth - EMB = -g / (1 + R)`
- `Moon - EMB = R * g / (1 + R)`
- Reversing the target and center negates those vectors.
- `Moon - Earth = g`; `Earth - Moon = -g`.

The implementation now evaluates all six relative cases directly from `g`.
It does not need the EMB absolute series for these specific differences and
does not modify general barycentric state evaluation or the meaning of the
existing public API.

## Scientific contract

- **Epoch:** caller-supplied Julian date TDB, inclusive file coverage.
- **Target and center:** only the Earth, Moon and Earth–Moon barycenter pairs
  described above.
- **Frame:** geometric equatorial ICRF axes as represented by the classic DE441
  source; no date-frame rotation, light-time or apparent corrections.
- **Units:** positions in km, velocities in km/day.
- **Coverage:** unchanged; no extrapolation or fallback.
- **Precision:** removes an avoidable loss of significance; this is not a
  quantitative accuracy guarantee against JPL. The barycentric results
  for other body combinations are unchanged.

## Synthetic regressions

`crates/ephemeris-core/tests/de441_emb_relative.rs` constructs two 32-day
classic DE441-like records with an EMB absolute position of 1e20 km and
geocentric lunar position of [82, 164, 246] km at each record midpoint.
The test checks direct and inverse states, all axes, and corresponding
velocity units. A second test omits the EMB absolute series to confirm that
the relative offsets require only the geocentric Moon series.

These deliberately unrealistic synthetic magnitudes demonstrate floating-point
cancellation, **not physical ephemeris accuracy**. They contain no external
DE441 binary, NASA/JPL dataset, Swiss Ephemeris code, or third-party numerical
test vectors.

## Verification and release gate

Required on this PR's actual head:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The repository's existing GitHub Actions CI can run these checks. This change
does not satisfy the separate independent NASA/JPL/Horizons validation gate.
Do not publish packages or merge until the PR has been reviewed and CI confirmed.

## CPAL-1.0 modification record

2026-11-10: Changed
`crates/ephemeris-core/src/de441.rs` to implement algebraic relative
Earth/Moon/EMB states directly, and added the regression file
`crates/ephemeris-core/tests/de441_emb_relative.rs`. This modification
derives from the original VirgoStars-Ephemeris work of the Initial Developer
**Cesare-ml (VirgoStars project)**. Both source files carry the CPAL-1.0
Exhibit A source notice. `LICENSE`, `NOTICE`, attribution `Powered by VirgoStars`
and https://www.virgostars.it/ remain unchanged.
