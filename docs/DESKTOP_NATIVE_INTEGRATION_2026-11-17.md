# VirgoStars Desktop: native Rust integration handoff

**Status: integration candidate, locally tested core; not a released or
scientifically accepted provider.** Desktop may start integrating planets
without waiting for stellar catalog completion. The separate restriction on
using the new engine on the main VirgoStars server remains in force.

## Consume the Rust core

Desktop is Tauri/Rust. Depend directly on package `virgostars-ephemeris-core`
from this repository, at an explicitly pinned reviewed Git commit after it
reaches the integration branch. The crate is version `0.1.0`, `publish =
false`; no package registry release or stable API is implied. A local sibling
checkout can be used to develop with `path = "../../VirgoStars-Ephemeris/crates/ephemeris-core"`
if that layout actually exists. Commit the consumer's lockfile and pin the
revision for reproducibility; do not use a floating branch for builds.

The crate needs an explicitly supplied **classic binary DE441** file: it is
not a SPK `.bsp`, is not bundled, and no network download or fallback occurs.
Configure it in the native host, not via a WebView-supplied path; verify the
source, digest and coverage before use. Distribution of external datasets is
separate from this source package. Preserve the CPAL license, notice and
applicable VirgoStars attribution in the consuming application.

The runnable example performs both desktop coordinate calculations:

```text
cargo run --example desktop_planets -- <path-to-classic-de441> 2451545.0
```

The repository-root `npm test` runs the standalone JavaScript research and
script tests. The separate `bindings/node` suite requires its native addon to
be built first; neither is needed to call the Rust crate from Tauri.

The argument is **TDB Julian date**, not UTC, UT1 or TT. Output contains
true ecliptic/equinox-of-date longitude, latitude, distance and rates, plus
true equator/equinox-of-date right ascension and declination. Angles are
degrees, distance AU, angular rates degrees/TDB day and distance rate
AU/TDB day. Results are apparent geocentric: light-time, solar deflection,
annual aberration, IAU 2006 precession and IAU 2000B nutation are applied.
The targets for Mars through Pluto are **system barycenters**. See the
individual core model docs for limits and missing corrections.

## Mapping to the desktop provider

The desktop's `EphemerisProvider` expects `ChartSnapshotV1` including the
Sun and Moon, body speeds, both frames, house systems and a UT Julian date.
The example computes only the ten DE441 planetary entries and does not
implement that provider. Map the barycenter identifiers explicitly and keep
their provenance; never label an absent value as calculated.

- Civil UTC must be converted under an explicit, documented time policy;
  do not pass it to a TDB API unchanged or conflate UTC and UT1. The standard
  apparent orientation model is limited to 1900–2100; the separate long-term
  path remains experimental and does not certify years 0–5000.
- Houses are owned by `astrological-functions`, not the ephemeris crate.
  Desktop needs an explicit compatible house adapter, including polar errors;
  the existing Node house bridge cannot simply be called as Rust.
- Lunar nodes/apogees have separate experimental APIs and model assumptions.
  Chiron requires an independently supplied compatible type-21 SPK. Report
  missing data and unsupported bodies using `unavailableBodies` and
  `unavailableHouseSystems`; do not silently fall back to Swiss/Moshier.
- Return search requires the provider's `luminary_longitude` and a consistent
  UT-to-TDB policy. Stars are out of scope for this desktop integration step.
- Keep provider selection opt-in until desktop comparison tests, approved time
  data, error contracts and reproducible Windows builds are complete. Public
  API versioning, independent scientific review, macOS and installers remain
  separate work. The current desktop code is not changed by this handoff.

## Evidence and acceptance

See [MVP reconciliation](MVP1_RECONCILIATION_2026-11-16.md) and
[long-range grid](SWISS_ALIASES_AND_PLANET_GRID_2026-11-17.md) for the
actual local checks and caveats; software tests are not a scientific accuracy
guarantee. Before making this the default desktop provider, compare matched
epochs, frames, centers, units and correction settings across representative
dates, locations, missing data and polar cases. Confirm the chosen range,
time policy, house results, provenance and rollback behavior in a native build.
No production server or desktop deployment is performed by this package.
