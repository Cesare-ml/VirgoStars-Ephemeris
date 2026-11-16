# MVP 1: geocentric fixed-axis six-field bridge

**2026-11-11 — original Rust, experimental geometric/one-way-light-time
intermediate. NOT apparent Swiss Ephemeris `swe_calc_ut`.** This increment
composes two existing Rust modules into one user-facing *mathematical*
calculation while the true ecliptic-of-date pipeline is incomplete:

1. `light_time::geocentric_light_time` uses an independently supplied
   **classic** JPL DE441 binary to evaluate the Earth at reception TDB and
   the target at iterated emission TDB. Its returned Cartesian velocity
   is the derivative with respect to **reception** time, including
   implicit light-time differentiation.
2. `coordinates::project_fixed_axes` rotates the Cartesian position
   and velocity by an explicitly requested **constant** X-axis tilt
   and calculates the six spherical position-and-rate fields.
3. This bridge converts **kilometres into AU** and **radians into
   degrees** without changing the underlying astronomy, and reports
   both observation/emission epochs, light-time seconds and the exact
   `Body` variant used.

The actual Rust module is
`crates/ephemeris-core/src/geocentric_six_fields.rs`. Its functions
do **not** duplicate either the light-time solver or the fixed-axis
spherical projection.

## Units, observer, provenance and strict naming

```rust
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::geocentric_six_fields::{
    geocentric_fixed_six_fields, GeocentricFixedAxes,
};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut eph = De441::open("/locally/obtained/de441.eph")?;
let result = geocentric_fixed_six_fields(
    &mut eph,
    Body::Venus,
    JulianDateTdb(2_451_545.0),
    GeocentricFixedAxes::IcrfEquatorial,
)?;
println!(
    "{} deg, {} deg/day",
    result.longitude_deg, result.longitude_speed_deg_per_tdb_day
);
# Ok(())
# }
```

The above example is an **API usage demonstration only**. It is not
executed without a legal local DE441 data file, and it is not a validated
Venus apparent position.

Input: **numeric TDB Julian date**, already computed/converted by the
caller, plus an explicitly configured classic binary DE441 reader.
It does not accept UTC/UT1/TT dates or auto-load kernels. `Body::Earth`
has no defined geocentric direction. Target names preserve exact
model semantics: Mercury and Venus are planetary-center series; the
outer bodies in classic DE441 are **Mars/Jupiter/Saturn/Uranus/Neptune/
Pluto system barycenters**, NOT the corresponding planet center, and
must not be relabeled as those planets in compatibility results.

Output: `GeocentricFixedSixFields` contains the exact selected `target`,
`center = Body::Earth`, `axes`, `observation_epoch`,
`emission_epoch`, one-way `light_time_seconds` and the signed
`light_time_rate`, plus the six scalar coordinates:

| Field | Definition / units |
|---|---|
| `longitude_deg` | Longitude, [0°, 360°), in the selected **fixed** axes |
| `latitude_deg` | Latitude, [-90°, 90°], in the same fixed axes |
| `distance_au` | Observer–target one-way distance in exactly defined astronomical units |
| `longitude_speed_deg_per_tdb_day` | Signed reception-time longitude derivative; retrograde sign preserved |
| `latitude_speed_deg_per_tdb_day` | Signed reception-time latitude derivative |
| `distance_speed_au_per_tdb_day` | Signed reception-time distance derivative |

**One AU is defined to be exactly 149,597,870,700 m**, or
149,597,870.7 km, per [IAU Resolution B2 (2012)](
https://www.iau.org/static/resolutions/IAU2012_English.pdf).

`GeocentricFixedAxes::IcrfEquatorial` uses the original fixed ICRF
equatorial coordinate axes. `TiltedIcrf { tilt_rad }` allows a
**caller-supplied constant** passive X-axis rotation. Providing an
obliquity-like tilt alone does **NOT** establish the IAU true/mean
ecliptic of date, and the arbitrary tilt is never advertised as an
official J2000/ecliptic frame. Both position and velocity are rotated
by the same constant matrix; no moving-frame `Rdot × r` term is
appropriate here.

**Scientific correction status:**
- **Applied:** Newtonian geocentric one-way reception light-time,
  including its analytic derivative; fixed passive X-axis projection;
  fixed-unit conversion.
- **Missing:** annual/steller aberration, solar gravitational light
  deflection and other relativistic corrections, Shapiro delay,
  frame bias, IAU 2006 precession, full IAU 2000A nutation,
  true ecliptic of date and rotating-frame velocity, proper outer-planet
  center corrections, UTC/UT1/TT/TDB conversion and ΔT,
  terrestrial/topocentric observers and refraction.
- **Source data:** an external *classic* DE441 `.eph`, NOT a NAIF SPK
  `.bsp` or Astrodienst `.se1` file. No ephemeris file is embedded.
- **Precision limitation:** existing light-time solver's single-f64
  observation/emission JD has a finite resolution, and real-DE441
  numerical validation has a limited set of independent Horizons
  vector fixtures, not a complete Swiss planetary parity grid.

The long-term MVP-1 product order is documented in
[three Swiss-replacement MVPs](THREE_MVP_SWISS_REPLACEMENT_2026-11-11.md)
on draft PR #69, while the current integration branch still has the
older [backend-first work packages](BACKEND_WORK_PACKAGES_2026-11-10.md).
This change is an isolated implementation increment toward **MVP-1
planet positions and speeds**; it is not a Node loader or migration.

## Test coverage and scientific validation status

`crates/ephemeris-core/tests/geocentric_six_fields.rs` adds
**9 offline Rust regression tests**:
- Hand-derived IAU AU / degree / TDB-day longitude, latitude and distance
  conversion for a six-component Cartesian state.
- Nonzero constant X tilt independently checked against
  arctan geometry and a signed latitude derivative.
- 270° longitude normalization and negative retrograde speed.
- Correct use of *already light-time-corrected* reception velocity:
  no incorrect second light-time differentiation.
- Direction/pole, nonfinite tilt and Cartesian-input failures.
- Fail-closed invalid Earth target and corrupt reception/emission metadata.
- A **single function call through a synthetic classic DE441 binary**,
  barycentric Earth reconstruction, iterated reception delay, spherical
  projection, AU conversion and all six field outputs.
- Propagation of an undefined Earth target and absent Sun series.
- Centered differences of independently produced moving circular
  Cartesian positions against the returned longitude derivative.

These tests exercise integration and arithmetic with **synthetic data**.
They are **not** independent numerical JPL/SOFA/Swiss parity tests.
A separate existing optional real-kernel probe in
`tests/light_time_horizons.rs` compares four light-time-corrected
vectors with JPL Horizons; it does not validate the new six-field
**apparent ecliptic-of-date** consumer contract.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test geocentric_six_fields
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

No local Rust compiler is presumed available. Existing read-only
GitHub Actions runs those checks on Ubuntu, native Windows and macOS
after opening a draft PR.

## Next integration gates

1. Add actual solar-system **planet-center** handling for targets where
   classic DE441 only reports the system barycenter; never remap flags
   or body names without disclosure.
2. Compose the (currently separate draft) aberration, deflection,
   IAU 2006 frame bias/precession, IAU 2000A full nutation and
   moving-date-frame state-rate transforms **with rigorously aligned
   TDB/TT/UT1 time arguments**.
3. Compare the final geocentric apparent six-field values and
   their rates against pinned, independently run Swiss `swe_calc_ut`
   reference grids with the *actual* Node 0.5.17 contract: apparent
   ecliptic of date, flags, all six fields, angle wrap and station
   behavior, Moon, inner/outer planets and historical times.
4. Implement the Node load/compatibility wrapper and the rest of
   MVP 1 (JD reversals, house calculations in `astrological-functions`,
   approved fixed-star catalog) as prioritized in the owner decision.
   Keep the production backend unchanged until all gates pass.

## CPAL-1.0 modification record — 2026-11-11

Added original `src/geocentric_six_fields.rs`,
`tests/geocentric_six_fields.rs`, a single `lib.rs` export and
this document/README link. No third-party program source, JPL binary,
official coefficient table, dependency, license change, data
download, release, deployment or production application edit.
CPAL-1.0 notices and required **Powered by VirgoStars**
(https://www.virgostars.it/) branding preserved.
