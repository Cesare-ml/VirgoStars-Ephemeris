# First-sky WGS84 observer geometry — ERFA cross-check

**Modification date:** 2026-11-14. Original Rust source is covered by CPAL-1.0; see [LICENSE](../LICENSE), [NOTICE](../NOTICE), and [LICENSING_AND_CREDITS.md](LICENSING_AND_CREDITS.md). The separately licensed ERFA reference values are attributed in [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).

## Scope

The experimental `first_sky::observer_vector` turns a WGS84 geodetic position into a 3D observer vector in **kilometers**. Its inputs are latitude in degrees, ellipsoidal height in **meters**, and a supplied local rotation angle in **radians**. It produces Cartesian coordinates in the rotating mean-date frame used by the experimental topocentric sky. These tests do not change the algorithm or public interface.

The added tests provide:

1. **Independent external oracle:** ERFA `t_gd2gc` / `eraGd2gc(ERFA_WGS84, ...)`, with geodetic longitude `3.1 rad`, latitude `−0.5 rad`, and ellipsoidal height `2500 m`. Official expected ECEF XYZ in meters:
   - X: `-5599000.5577049947`
   - Y: `233011.67223479203`
   - Z: `-3040909.4706983363`

   For this isolated geometry test, use `3.1 rad` as the observer's supplied local rotation angle (no additional Earth rotation). Convert the ERFA meters to kilometers. Require **absolute error at most `1e-9 km`** per Cartesian axis (1 micrometer). The reference tests use a standard WGS84 ellipsoid, not a spherical Earth.

2. **Height and angular symmetries:** adding exactly `1000 m` to ellipsoidal height moves the result by `1 km` along the outward geodetic surface normal `(cos φ cos θ, cos φ sin θ, sin φ)`. A 180° increase in local rotation reverses the X and Y components while preserving Z. Both checks use the same non-cardinal latitude/angle and `1e-9 km` absolute component tolerance.

## Reference and licensing

Independent numeric source: [ERFA t_erfa_c.c, function `t_gd2gc`](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c) and its WGS84 configuration (equatorial radius `6378137 m`, inverse flattening `298.257223563`). ERFA is separately licensed; its implementation is **not** translated, copied or incorporated. Numerical test values are attributed in the repository's third-party notices. No new dependency is required.

## Reproduction

```sh
cargo fmt --all -- --check
cargo test --workspace first_sky::tests::wgs84_observer_xyz_matches_independent_erfa_gd2gc_reference
cargo test --workspace first_sky::tests::wgs84_height_and_half_turn_preserve_geodetic_geometry
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The external ERFA data test validates the **WGS84 geodetic-to-Cartesian subcomponent** only. It does not validate actual planetary vectors from JPL DE441, full UTC/UT1 Earth orientation, topocentric apparent positions, sidereal time, or Swiss Ephemeris parity. The broader experimental limitations remain documented in [FIRST_PLANET_POSITIONS_2026-11-10.md](FIRST_PLANET_POSITIONS_2026-11-10.md). Exact-head GitHub CI should be confirmed before review or integration.
