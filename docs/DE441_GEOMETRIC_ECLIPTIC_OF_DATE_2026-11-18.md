# Experimental DE441 geometric true ecliptic of date (2026-11-18)

**Status:** experimental, locally compiled on Windows on 2026-11-18. The
orientation was compared on three synthetic-vector cases with ERFA, and locally
held packs were compared with the same source. These checks do not independently
validate the DE441 planetary states or establish production readiness.

## Scientific contract

`crates/ephemeris-core/src/de441_geometric_date.rs` joins two *existing* canonical Rust components:

1. `De441PackManager::state(target, Earth, epoch_tdb)` evaluates an explicitly installed **classic** DE441 mobile pack, producing a **geometric geocentric ICRF/J2000** vector in km and km per TDB day.
2. `earth_orientation::true_ecliptic_of_date_matrix(orientation_epoch_tt_jd)` returns the existing **IAU 2006 bias-precession plus IAU 2000B nutation** orientation for a separately supplied **TT** epoch. Its implementation is bounded to JD TT **[2,415,020, 2,488,070)** (approximately 1900–2100); the upper endpoint is excluded.
3. The position is matrix-rotated and converted into ecliptic longitude `[0°,360°)`, latitude `[-90°,90°]`, radial distance in km and IAU 2012 astronomical units.

`project_geocentric_geometric_state` is a pure, file-free adapter. `geocentric_geometric_true_ecliptic_of_date` calls the manager first, with the same pack coverage/reader errors. Both return `GeometricTrueEclipticOfDate` with explicit target, Earth center, epochs and orientation-model label. Planetary-system barycenters remain explicitly named as such.

## **What these degrees are not**

These are **geometric** coordinates in the *true ecliptic/equinox axes of date*. They are **not** apparent, astrometric, topocentric, sidereal zodiac, or Swiss `swe_calc_ut` output. In particular, they do **not** apply:

- reception light-time iteration;
- annual aberration or gravitational light deflection;
- topocentric parallax, Earth rotation/EOP, refraction;
- **any** conversion between UTC, TAI, TT, TDB or historical Delta-T;
- angular speeds or retrograde rates. A moving-date-frame derivative requires the additional `Ṙ × r` term; rotating the original ICRF velocity alone would be physically incomplete.

**TT and TDB must represent the same physical instant and be provided separately by the caller.** The adapter cannot prove this relationship from two floating-point Julian dates. It does not fabricate TT from a TDB label. The mobile diagnostic's existing civil UTC→TDB approximation is valid only for the core's bounded 1972–2027 coverage and is not by itself a validated historical TT/TDB pipeline.

Outside the orientation range (including most historical DE441 packs), the function rejects the request rather than assuming a date frame. A missing/corrupt pack never falls back to another ephemeris method.

## API (illustrative)

```rust
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb};
use virgostars_ephemeris_core::de441_pack_manager::De441PackManager;
use virgostars_ephemeris_core::de441_geometric_date::geocentric_geometric_true_ecliptic_of_date;

let mut manager = De441PackManager::from_mobile_directory("/private/app/de441-packs")?;
let direction = geocentric_geometric_true_ecliptic_of_date(
    &mut manager,
    Body::Sun,
    JulianDateTdb(2_451_545.0), // TDB epoch, caller supplied
    2_451_545.0,                // TT epoch, caller supplied
)?;
println!("{} deg", direction.longitude_deg);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The epochs in this example share a numeric value **only as a compact demonstration of the typed API**; an actual time conversion must supply independently computed TT and TDB values for the same instant.

## Tests and validation gates

Synthetic Rust tests cover:

- known equatorial-axis projection near the J2000 obliquity, finite positive distance, degree range and preservation of center, body and epoch tags;
- orientation-dependent changes, while distance is preserved by rotation;
- no undefined target=Earth direction, no non-Earth-center state, no zero-range/pole longitude, no NaN or overflow;
- failure for TT outside the supported orientation window;
- error propagation for a deliberately absent local pack, without fallback;
- invalid input being rejected *before* pack I/O.

Run locally once Rust is available:

```powershell
cargo fmt --all -- --check
cargo test -p virgostars-ephemeris-core --test de441_geometric_date
cargo clippy -p virgostars-ephemeris-core --all-targets -- -D warnings
```

**This still does not validate NASA/JPL positions.** Independent reference comparisons must specify exact target and center, TT/TDB epochs, frame convention, correction set and predeclared per-field tolerances. Before integrating mobile charts, evaluate the existing unmerged MVP1 apparent-of-date work rather than rewriting that pipeline or labeling rotation-only values "apparent."

### Windows audit, 2026-11-18

The test `synthetic_geometric_direction_matches_independent_erfa_orientation`
uses pyerfa 2.0.1.5 `pmat06`, `nut00b`, `obl06`, `numat`, and `rx` to
transform the same synthetic ICRF vector with IAU 2006 bias-precession and IAU
2000B nutation. At TT JD 2415020.0, 2451545.0 and 2488069.0, maximum observed
absolute longitude residual was 5.6844e-14 degree and latitude residual was
zero at displayed precision; the predeclared per-coordinate tolerance was
1e-8 degree. This tests the orientation implementation, not the astronomical
state, time conversion, or physical validity at every date.

An explicitly ignored, opt-in test also compared Sun/Earth geometric states at
the starts and just before the ends of all ten logical pack intervals with the
same DE441 source. Its environment variables are `DE441_PATH` and
`DE441_PACKS_DIR`; no ephemeris data are included or redistributed. The source
and slices share coefficients and this is **not** independent JPL validation.

Modification 2026-11-18: corrected a synthetic overflow fixture, formatted the
adapter/tests, clarified the exclusive TT limit and added the above checks.

No ephemeris kernel, external third-party algorithm source, extra dependency, download URL, database or data distribution rights are introduced. Original source remains CPAL-1.0, with required notices/attribution.
