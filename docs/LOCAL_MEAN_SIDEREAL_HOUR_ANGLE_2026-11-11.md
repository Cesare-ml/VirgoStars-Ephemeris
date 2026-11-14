# Local mean sidereal time and signed mean hour angle — 2026-11-11

**Status:** original, experimental Rust primitives in a draft PR, not an
integrated Astro-functions or VirgoStars production capability.

## Why and where

VirgoStars needs a reproducible astronomical meridian angle as a foundation
for rise/culmination calculations and house orientation. The active core
does not yet have a verified full IAU mean sidereal-time pipeline on
`dev/ephemeris`. Draft **PR #75** computes IAU 2000 ERA and draft
**PR #78** computes the separate IAU 2006 GMST correction. This module
does **not** duplicate either algorithm: it accepts already-calculated
**Greenwich mean sidereal time (GMST)** and east-positive longitude.

Actual house systems, ascendant/MC algorithms, chart modeling and zodiac
logic remain in `astrological-functions`, according to AGENTS.md.

## Contracts and mathematics

```text
local_mean_sidereal_angle = wrap_2pi(GMST + longitude_east)
mean_local_hour_angle    = wrap_signed_pi(local_mean_sidereal_angle - RA_mean)
```

- **GMST:** finite angle in [0, 2π) radians, based on a coherent IAU
  model and **UT1 + TT** epochs. This module receives GMST as a value
  and cannot establish its model, epoch, or accuracy.
- **Observer longitude:** finite WGS84-style geodetic angle, in **degrees
  east** within [-180°, 180°]. East is **added**; west is **subtracted**.
  +180° and -180° are the same physical meridian.
- **Output LMST:** Greenwich **mean** sidereal time rotated to the local
  meridian, normalized to [0, 2π) radians. Geographic latitude and
  height do not alter this meridian-angle arithmetic.
- **Mean right ascension:** finite angle in [0, 2π) radians, of the
  **same mean equator/equinox of date** as the supplied GMST. Mean and
  apparent/true-of-date coordinates must not be silently mixed.
- **Output H:** signed mean local hour angle in [-π, π), **positive
  toward the west**, negative toward the east; exactly 180° maps to
  -π radians. When mean RA equals LMST, H=0 (mean meridian crossing).
- **Rejections:** non-finite or nonnormalized GMST, LMST, RA and invalid
  observer longitude return typed errors rather than silently accepting
  mixed or ambiguous values.

These functions perform no civil/UT1/TT/TDB conversion, EOP/ΔT model,
polar motion, nutation, equation of the origins, refraction, parallax,
light-time or physical geocentric/topocentric correction. **LMST is not
local apparent sidereal time or GAST** and H is not by itself an observed
transit instant. Do not pass apparent-of-date RA to the mean formula
without an explicitly reviewed reference-frame conversion.

## API example

```rust
use virgostars_ephemeris_core::local_sidereal::{
    local_mean_sidereal_time, local_mean_hour_angle
};

let gmst_rad = 1.0; // Example value, NOT computed from a real UT1/TT epoch.
let east_longitude_deg = 12.5;
let ra_mean_of_date_rad = 1.4;

let lmst = local_mean_sidereal_time(gmst_rad, east_longitude_deg)?;
let h = local_mean_hour_angle(lmst, ra_mean_of_date_rad)?;
println!("LMST {lmst} rad; mean local hour angle {h} rad");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Verification and references

This PR adds six **deterministic geometry** tests:
1. The Greenwich identity and 15° east/west = one sidereal hour in
   angular units (15° = π/12).
2. ±180° date-line meridian identity and angle normalization.
3. Mean transit H=0 and positive-west/negative-east hour angles.
4. 0°/360° wrapping and signed 180° opposition behavior.
5. Round-trip LMST reconstruction from mean RA + hour angle.
6. Explicit errors for NaN, infinite and out-of-domain angles.

These are mathematical invariants, not numerical agreement of computed
GMST, UT1, planetary states or Swiss `swe_rise_trans`, `swe_houses`
and `swe_houses_pos` with an independent oracle. Standalone GMST
numerical validation is in PR #78 and requires separate review.

References used for convention only:
- [USNO — Computing Approximate Sidereal Time](https://aa.usno.navy.mil/faq/GAST)
- [IERS Conventions (2010), Chapter 5](https://iers-conventions.obspm.fr/conventions_material.php)

CI gate:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Original CPAL-1.0 source and test notices are retained. No third-party
implementations, ephemeris kernels, catalogues, new dependencies, releases
or production migrations are introduced. This draft PR is intended for
`dev/ephemeris`, not `main`. After its review, integrate with PRs
#75/#78, establish coherent UT1/TT inputs, and independently validate the
combined orientation/house-axis pipeline before use.
