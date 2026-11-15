# Fixed-axis radial-motion regression tests

**Added:** 2026-11-11. **Status:** synthetic mathematical checks; not independent JPL/Horizons validation.

The integration test `crates/ephemeris-core/tests/fixed_axis_radial_invariants.rs` exercises `coordinates::project_fixed_axes` using the Cartesian vector (3, 4, 12) km and its parallel velocity (6, 8, 24)/13 km/day. The expected distance is 13 km and its rate is 2 km/day. Both longitude and latitude rates must vanish because the motion is purely radial.

The test also applies several fixed passive X-axis rotations (radians), including positive and negative angles. An orthogonal rotation preserves distance, radial velocity and the zero angular-rate property. This checks projection invariants without requiring external ephemeris data.

Run locally:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

**Limits:** These tests do not validate ICRF frame bias, precession, nutation, apparent positions, time scales or a genuine DE441 file. No kernel data is distributed. The existing CPAL-1.0 LICENSE, NOTICE and VirgoStars attribution apply.
