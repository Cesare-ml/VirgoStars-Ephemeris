# Geocentric light-time state — 2026-11-10

The experimental Rust `light_time::geocentric_light_time` is one stage toward
the backend's apparent geocentric position and speed. It is **not** an
apparent position, `swe_calc_ut`, a Node binding, or Swiss parity. It reads an
external classic DE441 file; no kernel or Swiss code is included.

## Contract

- Input: `&mut De441`, a `Body`, and a **TDB** observation Julian date; Earth
  is the geocentric observer. The Earth state is sampled at observation and
  the target state at emission. Output: target, both TDB epochs, light time in
  seconds, its dimensionless rate and the relative position/velocity in ICRF
  axes, kilometres and kilometres per TDB day. Mars–Pluto targets are system
  barycenters. Earth as target and zero range are undefined and rejected.
- Solve `tau = |target(t - tau) - earth(t)| / c` by fixed-point iteration,
  using the exact SI speed of light 299792.458 km/s. Convergence means the
  next emission epoch is identical at the reader's **single-f64 JD**
  resolution; do not interpret this as arbitrary physical accuracy. A limit
  of 32 iterations, non-finite geometry, missing data and out-of-range
  emission epochs produce explicit errors. No extrapolation or fallback.
- The returned velocity is the derivative of the returned position vector
  with respect to **observation** time, not simply the subtraction of
  target and Earth velocities evaluated at their two different epochs. With
  unit line-of-sight `n`, target velocity `vt` at emission, observer velocity
  `vo` at reception, and units consistent with `c`, the derivative of the
  delay is `d_tau/d_t = n·(vt - vo) / (c + n·vt)` and the vector derivative
  is `vt * (1 - d_tau/d_t) - vo`. This matters for angular speeds and
  retrograde/station calculations. Derivation follows directly by
  differentiating the light-time equation.
- No stellar/annual aberration, relativistic light deflection, Shapiro delay,
  precession, nutation, time conversion, geodetic observer or refraction is
  applied. No equatorial/ecliptic-of-date speeds are supplied. A later
  apparent pipeline must explicitly add and validate those stages, including
  rotating-frame derivatives. `first_sky` remains unchanged.

## Independent reference and tests

NASA/JPL [Horizons](https://ssd.jpl.nasa.gov/horizons/manual.html) API 1.2,
queried 2026-11-10: `EPHEM_TYPE=VECTORS`, `CENTER=500@399`,
`START_TIME=2026-11-10`, `STOP_TIME=2026-11-11`, `STEP_SIZE=1d`,
`TIME_TYPE=TDB`, `OUT_UNITS=KM-S`, `REF_PLANE=FRAME`, `VEC_CORR=LT`,
`VEC_TABLE=3`, at JD TDB **2461354.5**. Four targets were queried separately:
301 Moon, 2 Venus (Horizons resolves 299 Venus), 4 Mars barycenter, and 5
Jupiter barycenter. Horizons reported **DE441** for all targets and Earth,
ICRF, and Newtonian receive light-time correction. Reference numbers are in
`tests/light_time_horizons.rs`. The sample is small and covers one epoch;
it is not comprehensive validation of the reader or the apparent pipeline.
[Horizons API documentation](https://ssd-api.jpl.nasa.gov/doc/horizons.html)
describes the parameters. The query is for reference data only; no NAIF or
Swiss source is incorporated. See also
[NAIF's correction description](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/abcorr.html).

For these four specific Horizons vector queries, the reported `VX/VY/VZ`
numerically match `vt(t-tau) - vo(t)`; they **do not** match the derivative
of the light-time-corrected position. The independent position comparison and
the comparison with those instantaneous velocities use the Horizons values;
the derivative itself is separately tested against both an analytical linear
motion example and centered position differences over ±600 s on the real
file. Do not compare these two different velocity conventions as if they
were interchangeable. This observation is limited to the queried mode and is
not a general statement about every Horizons output option.

On Windows with the local DE441 file, the maximum component residuals for the
four target samples were 0.000353 km position, 1.1e-10 km/s instantaneous
velocity versus Horizons, 4.1e-7 km/s corrected velocity versus centered
finite differences, and 8e-10 s light-time. Predefined test limits are
0.01 km, 1e-6 km/s, 3e-6 km/s and 1e-6 s respectively. The position limit
accounts conservatively for single-f64 emission JD quantization (about 40 µs
per representable step at this epoch) and planetary motion; the finite
comparison also allows differencing and curvature. These limits are testing
bounds for this sample, not a general accuracy guarantee.

```powershell
$env:DE441_PATH = 'D:\GitHub\virgo-stars\database\effemeridi\astrodienst\de441.eph'
cargo test --locked --test light_time_horizons -- --ignored --nocapture
```

The regular offline suite tests an analytical moving-target case, derivative,
invalid epoch, invalid velocity, no direction, error propagation and bounded
iteration. Optional real-data tests are ignored by default and never download
a kernel. 2026-11-10: added the independent Rust light-time module and tests;
all existing CPAL notices and third-party attribution remain unchanged.

## Additional angular-rate cross-stage regression — 2026-11-10

A new Rust unit regression in `crates/ephemeris-core/src/light_time.rs`,
`transverse_light_time_spherical_rates_match_reception_finite_difference`,
links two existing stages of the **Rust** core: the geocentric reception
light-time solution and the **fixed ICRF-axis geometric** projection
`coordinates::project_fixed_axes`. It uses a deliberately constructed linear
target/observer motion in three dimensions (not a Swiss or JPL fixture):
both bodies have nonzero transverse velocities, the target is retarded to its
emission TDB epoch, and the observer remains at the reception TDB epoch.

For a reception-centered ±600-second step, the regression compares:
the signed shortest-angle longitude rate (radians/TDB day), latitude rate
(radians/TDB day), radial distance rate (km/TDB day), and light-time rate
(seconds per reception second) against independent finite differences
of newly solved reception positions. It checks that the example exhibits a
negative/retrograde longitude rate. Numerical tolerances are respectively
`3e-7` rad/day, `3e-7` rad/day, `0.1` km/day and `3e-8` s/s;
these are conservative regression thresholds for this fixed synthetic sample
and single-`f64` Julian-date resolution, **not** astronomical error budgets
or Swiss Ephemeris parity. The same increment tests that zero Earth–target
range and a non-finite observer velocity are rejected, never silently
converted to a valid direction.

Run the new regression and all existing tests with:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked transverse_light_time
cargo test --workspace --locked zero_relative_range
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The core's `geocentric_light_time` and `project_fixed_axes` algorithms,
public APIs, time-scale rules, ephemeris file formats, dependencies and
CPAL-1.0 attribution remain unchanged. These tests do not add aberration,
nutation, deflection or date-frame derivative handling. An independent
NASA/JPL comparison and an **apparent-of-date** output are still required
before treating any result as a Swiss-style `swe_calc_ut` replacement.

**CPAL-1.0 modification record (2026-11-10):** expanded synthetic
numerical/error-case tests in `crates/ephemeris-core/src/light_time.rs`
and documented their limits here; initial developer and VirgoStars copyright
notices preserved, no external source/data incorporated.

### Retrograde zero-meridian continuity regression

A second synthetic reception-time test,
`retrograde_longitude_rate_is_continuous_across_zero_meridian`,
places a target just above the positive ICRF X axis at observation and gives
it a negative transverse Y velocity. Across a ±600-second reception window,
its longitude crosses the 0/2π meridian in the retrograde direction. The
test explicitly checks that the before/after longitudes lie on opposite
sides of the discontinuity, that the analytically projected angular speed
is negative, and that a shortest-arc unwrapped finite-difference speed
matches within `3e-7` rad/TDB day. This protects downstream station and
retrograde calculations from accidentally treating a coordinate wrap as
a genuine large prograde displacement. The input remains a synthetic
linear-motion model; no independent JPL or Swiss accuracy claim is implied.

CPAL-1.0 modification record (2026-11-10): added this deterministic
zero-meridian regression to the existing Rust light-time test module;
preserved original Exhibit A, license and VirgoStars attribution.
