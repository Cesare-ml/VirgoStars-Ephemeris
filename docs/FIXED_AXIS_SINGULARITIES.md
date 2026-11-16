# Fixed-axis projection: singularities and longitude wrapping

Modification record: 2026-11-11 — add deterministic Rust regressions for the existing `project_fixed_axes` API.

The integration test `crates/ephemeris-core/tests/fixed_axis_singularities.rs` checks three separate contracts:

1. At the origin and exactly on either pole (in the unrotated frame), longitude and angular rates are undefined. The API must return `ProjectionError::UndefinedDirection` rather than manufacturing a longitude.
2. At cardinal directions and in the fourth quadrant, longitude is normalized to `[0, 2π)` using the existing `atan2` convention.
3. Non-finite tilt, epoch, or velocity must return `ProjectionError::NonFinite`.

The test uses synthetic geometric state vectors, with position in km and velocity in km/day. It tests **fixed axes**, not ecliptic-of-date or apparent coordinates. It is not an independent JPL Horizons accuracy test. Near-pole conditioning (rather than exact poles) requires separate numerical study before a precision guarantee can be made.

Validation requested on the feature PR: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`. These checks must be reported from actual CI results; no local Rust toolchain was available during authoring.

Source license: CPAL-1.0, with VirgoStars notices and Exhibit B attribution retained. No external ephemeris data or third-party source is included.
