# Exhaustive classic DE441 target–center matrix regression

Created 2026-11-14. The new Rust integration file
`crates/ephemeris-core/tests/de441_all_body_pairs.rs` verifies all 13
publicly supported `Body` variants in a **synthetic** classic JPL DE441
binary fixture. No changes to production logic, APIs, dependencies, or
GitHub Actions.

## Independent arithmetic oracle

The fixture models *two* adjacent 32-TDB-day coefficient records with
**11 active three-axis T0/T1 Chebyshev series** and a mass ratio
`Earth/Moon = 81`. Each series/axis/record has a distinct analytically
chosen constant and linear coefficient, so accidentally swapping series,
axes or data records changes both the expected position and derivative.

At normalized `u = 2 * (JD - record_start) / 32 - 1`, the expected
geometric Cartesian state is:

`position_km = c0 + c1*u`, `velocity_km_per_day = c1/16`.

The reader is queried for **all 13×13 = 169 ordered target–center pairs** at
eight exact epochs, including J2000 **JD 2451545.0 TDB**, both sides of the
record boundary at JD 2451577.0, and the inclusive end at JD 2451609.0.
That is **1,352** independent XYZ/rate expected-value comparisons (three
axes for each position and velocity), not just internal invariants.

The reference oracle explicitly computes Earth and Moon from the
Earth–Moon barycenter (series 2) and geocentric lunar vector (series 9):
`Earth = EMB - MoonGeocentric/82` and
`Moon = EMB + 81*MoonGeocentric/82`. The other planetary series represent
**system barycenters where named**, not the physical outer planet centers.
The Sun is series 10, solar system barycenter is the zero origin.

A second test computes complete pair-state matrices at four epochs, checking
reflexivity, antisymmetry and `A−C = (A−B)+(B−C)` for **every ordered body
triple** in both position (km) and velocity (km per TDB day). Finite arithmetic
is checked with conservative absolute budgets (position 1e-9 km, velocity
1e-12 km/day) on deliberately small integer fixtures. No corrections,
time-scale conversion, light-time or frame rotation occurs.

## Verification

```sh
cargo test -p virgostars-ephemeris-core --locked --test de441_all_body_pairs
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The existing authorized, read-only CI exercises the new integration test on
the PR head. A passing synthetic fixture cannot establish **real NASA/JPL
DE441 accuracy**, NAIF SPK interoperability or independent Horizons
positional/velocity parity. Those require separately obtained official data,
matched kernel provenance and independent reference comparisons.

All new code preserves **CPAL-1.0 Exhibit A** and VirgoStars's required
**Powered by VirgoStars** attribution. No JPL kernel, Swiss code, binary
dataset or third-party package is committed; no release or deployment.
