# Polar Ascendant: select the rising horizon branch (18 November 2026)

**Status: experimental Rust correction with exact-head Rust/Node CI passed;
independent polar numerical reference review remains open.** This file documents a
modification derived from the original VirgoStars-Ephemeris code by
Cesare-ml (VirgoStars project), originally released under CPAL-1.0.
Original CPAL source notices, `LICENSE`, `NOTICE` and the required
**Powered by VirgoStars** attribution are unchanged.

## Defect and owner-supplied synthetic evidence

The original `house_angles::angles_from_armc()` computed one of the two
antipodal ecliptic/horizon intersections with a traditional `atan2` expression,
but did not check which one was **rising**. At polar latitudes, when the
Midheaven can be below the horizon, that expression sometimes selects the
*setting* point. The public contract defines `ascendant_deg` as the rising
point. This affects real downstream callers of the experimental Node binding.

Owner-supplied J2000 synthetic receipt, with explicitly distinct UT1 JD
`2451545` and TT JD `2451545.0008`:

| Field | Value |
| --- | ---: |
| Geodetic latitude | +80° |
| Longitude | +12.4964° east |
| ARMC | 292.95347186375847° |
| True obliquity | 23.437676823739082° |
| Original `ascendant_deg` | 164.5489205114233° (setting) |
| Correct rising intersection from same supplied angles | 344.5489205114233° |
| Midheaven (unchanged) | 291.23482098822° |

The receipt's installed binary hash and source commit were asserted by the
caller; their build linkage is **not independently established**. Other six
latitude observers in the receipt already selected the rising intersection.

## Geometry and implementation

For a *candidate* ecliptic longitude `λ`, true obliquity `ε`, local meridian
right ascension `θ` and geodetic latitude `φ`, an equatorial unit direction is

`x = cos λ`, `y = sin λ cos ε`, `z = sin λ sin ε`.

Its local horizon altitude is determined by

`sin h = sin φ·z + cos φ·(x cos θ + y sin θ)`.

With increasing local sidereal angle (Earth rotation), the altitude derivative
is `d(sin h)/dθ = cos φ·(−x sin θ + y cos θ)`.

For `|φ| < 90°`, `cos φ > 0`, so the expression in parentheses distinguishes
the **rising** branch (positive) from the **setting** branch (negative). The
correction selects the original candidate when that expression is nonnegative,
otherwise its 180° antipode. Exactly zero means a tangential crossing with no
unambiguous instantaneous rising/setting derivative; the existing deterministic
candidate is retained. No other house-system or cusp formula is introduced.

The model and input contract remain unchanged: ARMC in apparent right ascension
degrees from IAU 2006/2000B Earth orientation using separate UT1 and TT; true
obliquity of date in degrees; longitude east-positive; latitude used as given.
The correction does not read DE441 kernels or alter time-scale conversion.

This branch selection is consistent with the independent geometric discussion
in [Astrodienst, *Astrology in polar regions and on the southern hemisphere*](https://www.astro.com/astrologie/in_polar_asc_e.htm),
which explains why a traditional Ascendant may require a 180° adjustment when
the MC falls below the horizon. No Swiss Ephemeris source was copied or used.

## Preserving the five extra-angle contracts

The public extra points (`vertex`, `equatorial_ascendant`,
`koch_co_ascendant`, `munkasey_co_ascendant`,
`munkasey_polar_ascendant`) were independently fitted to the reference using
the old *unflipped* `atan2` construction with various **auxiliary latitudes**.
An auxiliary latitude may be polar when the actual observer is not (for
example Munkasey at 10°N uses `90°−10° = 80°`).

Blindly replacing all such auxiliary formula calls with the newly corrected
`angles_from_armc()` would change the existing extra-point API by 180° in
some cases. This patch therefore extracts the historical formula as a
**private helper**, uses it unchanged for those five extra-angle definitions,
and applies the rising-branch selection **only to the public true
`ascendant_deg`**. Unit tests lock representative auxiliary cases. A separate
scientific review may later propose deliberate extra-angle changes.

## Validation and integration gates

- New Rust tests independently evaluate the horizon altitude and positive
  crossing derivative over both hemispheres, 13 latitudes and 72 ARMC values,
  at two true obliquities; they also lock the owner's polar fixture,
  the full Rust house-angle entry point and representative original
  extra-angle values.
- The existing GitHub CI run **#302** on correction commit
  `8f43e4d1629341175bcc2a7d0d81383d6cf853d3` completed successfully:
  Rust workspace tests, strict Clippy, formatting, native Node binding tests,
  and JavaScript prototype tests all passed. This is **code/geometry**
  validation, not independent scientific polar-house parity.
- Additional Rust-backed Node tests on this follow-up patch call both the
  explicit UT1/TT provider and the UT convenience binding at polar latitudes;
  they verify the seven original synthetic latitude points and rising horizon
  derivatives. These **must run** against a freshly compiled native addon;
  do not count a JavaScript-only check as proof the tests passed.
- **Required before integration:** run `cargo fmt --all -- --check`,
  `cargo test --workspace` and
  `cargo clippy --workspace --all-targets -- -D warnings` on the exact PR
  commit. On the development host where this patch was prepared, Rust/Cargo
  were unavailable; this document does not claim these checks passed.
- Compare corrected angles with independently specified polar reference
  cases and re-run the downstream owner-provided 7×23 native method grid.
  The downstream methods that reconstruct Ascendants require their own
  method-specific review; this fix cannot make all polar house systems valid.
- No native binary/release, Node package, Desktop/Mobile app or production
  backend has been updated. No GitHub Actions workflow or license changes.

Tracking: [upstream issue #594](https://github.com/Cesare-ml/VirgoStars-Ephemeris/issues/594),
[downstream analytic audit PR #222](https://github.com/Cesare-ml/astrological-functions/pull/222).
