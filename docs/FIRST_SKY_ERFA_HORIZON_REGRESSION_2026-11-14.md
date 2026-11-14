# Independent ERFA geometric horizon regression

**Modification date:** 2026-11-14. Original VirgoStars Rust code remains under CPAL-1.0; see [LICENSE](../LICENSE), [NOTICE](../NOTICE), and [LICENSING_AND_CREDITS.md](LICENSING_AND_CREDITS.md). ERFA numerical fixtures are separately credited in [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) and [third_party/ERFA_LICENSE.txt](../third_party/ERFA_LICENSE.txt).

## Why

The experimental `first_sky::calculate` already uses `horizontal_direction` for geometric, topocentric altitude and azimuth, but the existing standalone unit regressions covered only cardinal directions and zenith. These new tests anchor this **currently used Rust subcomponent** to an independently supplied, non-cardinal external reference.

## Reference and units

External oracle: [ERFA `t_hd2ae` conformance fixture](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c). Its inputs are:
- Local hour angle: **1.1 radians**
- Equatorial declination: **1.2 radians**
- Site latitude: **0.3 radians**

Its output values, in radians, are:
- Azimuth **5.916889243730066194**, measured eastward from north, modulo 2π
- Altitude **0.4472186304990486228**, positive above the geometric horizon

VirgoStars `horizontal_direction` instead takes a **Cartesian direction vector** in km, site latitude (radians), and local sidereal angle (radians), returning altitude and azimuth **in degrees**. Construct an independent equatorial position vector from declination and right ascension `RA = local sidereal angle - local hour angle`. The test sets local angle **0.7 radians** and distance **42,000 km**; converts returned degrees to radians; and requires **1e-13 radians** absolute agreement with each published ERFA output. Literal precision in Rust is truncated only to what `f64` can represent.

Additional geometric invariants confirm that a positive scaling of the XYZ distance and a full-turn sidereal-angle shift do not change directional output, within `1e-11 degrees`. At exact zenith and nadir, altitude is respectively +90° and -90° but azimuth is undefined (`None`); a detectable lateral perturbation makes azimuth defined.

## Scientific scope and limitations

This checks the *local geometric equatorial-to-horizon transform*, including hour-angle and azimuth sign conventions, units and singularities. It does **not** independently verify the source equatorial vector, DE441/Horizons parity, sidereal time model, UTC/UT1/EOP, atmospheric refraction, parallax in complete calculations, annual aberration or apparent position. In the current experimental first-sky pipeline, UT1 is approximated by UTC and the reference frame remains mean of date with documented limitations.

No production algorithms, API signatures, third-party code, kernel assets, new dependencies, external downloads or network operations are added.

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace first_sky::tests::geometric_horizontal_coordinates_match_independent_erfa_hd2ae
cargo test --workspace first_sky::tests::geometric_zenith_and_nadir_have_no_defined_azimuth
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

A successful exact-head GitHub Actions run (Linux, Windows and macOS) must be inspected before marking the PR ready. This is independent ERFA **subcomponent** conformance, not independent JPL validation of the complete engine.
