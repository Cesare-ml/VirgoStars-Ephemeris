# DE441 streaming I/O regression — 2026-11-10

## Change
Added a deterministic integration test in `crates/ephemeris-core/tests/de441.rs` for a seekable source whose reads fail **after** the header and file-length checks succeed. It asserts that `De441::state` returns `De441Error::Io` rather than silently returning an incomplete vector. Once the injected read failure is removed, the same reader can return finite geometric position and velocity values.

## Scientific contract
No astronomical algorithm, input format, time scale, frame, center, units or physical corrections change. The synthetic fixture uses Julian Date **TDB**, geometric equatorial ICRF coordinates, positions in **km** and velocities in **km/day**. This test exercises I/O propagation and recovery, **not** numerical agreement with NASA/JPL or Horizons.

## Verification
The test is committed for GitHub Actions and local execution via:
```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```
Do not claim these checks passed until the actual CI result has been inspected. The initiating environment has no Cargo toolchain.

## Provenance / license
Original test code only, under the repository's CPAL-1.0 license; no third-party code or ephemeris data included. Modification date: 2026-11-10. Initial Developer: Cesare-ml (VirgoStars project). The existing repository `LICENSE`, Exhibit A and `NOTICE` apply. No changes to attribution obligations.

## Risk
Small, test-only change; no public API changes. It is independent of the header validation and pointer-overlap PRs. This fixture does not validate external file I/O across operating systems.
