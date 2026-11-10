# Backend-first work packages for contributors and AI agents

**Status:** proposed work, 2026-11-10. Read [AGENTS.md](../AGENTS.md),
[ADR-0001](ADR-0001-RUST-CORE-AND-BINDINGS.md), and the
[backend compatibility inventory](BACKEND_SWISS_COMPATIBILITY_2026-11-10.md)
first. Inspect `dev/ephemeris`, open PRs, active work and any nested agent
instructions before claiming a package: concurrent work can change this plan.
A task below is an independently reviewable PR or a sequence of small PRs, not
an instruction to modify several repositories in a single branch.

## Concurrency and handoff

1. Pick an unclaimed package, inspect its dependencies and announce ownership
   through the team's existing coordination mechanism. Never assume a checklist
   here is a live reservation system. Avoid overlapping files and check the
   current branch before editing. Link the upstream issue/PR if one exists.
2. Use a new branch and PR to `dev/ephemeris` for this repository; never commit
   to `main`. Work for `astrological-functions` belongs in that repository's
   own branch and PR. Do not alter VirgoStars production as a side effect.
3. State input/output units, time scale, center, frame, model, dependencies,
   unsupported inputs and errors. Implement independently from cited scientific
   specifications; do not copy or translate Swiss source. Do not add kernels,
   proprietary catalogs or unreviewed third-party datasets.
4. Report actual tests, independent references, fixture provenance, tolerances,
   licensing and remaining gaps in the PR. A synthetic fixture is not numerical
   validation. Keep generated files and external multi-GB data out of commits.
5. When done, update the inventory/status in the same focused PR where
   possible. Mark work complete only after its acceptance cases pass on the
   merged head. Do not silently change another agent's in-flight PR.

## Dependency graph

```text
A: calendar and timescale policy ----\
B: reference/compatibility harness ---+--> C: apparent positions + rates --> E: Node facade
D: Earth orientation + nutation -----/                  |                      |
                                                        +--> F: nodes/Chiron   +--> I: chart integration proof
D + A -------------------------------------> G: houses (functions repo) ------+
C + D -------------------------------------> H: altitude / rise / transit -----+
C + A -------------------------------------> J: stars and eclipses (later)
```

A/B/D can begin independently, provided shared interfaces are agreed in their
PRs. C may be split into reviewed correction stages; E can start with
capability-gated contract tests and typed errors, without claiming a working
planet facade. G can design and test its contracts independently, using
explicit inputs and fixtures, until orientation is available. J need not wait
for every other capability if a self-contained reference/model is selected.

## A — Calendar, UTC, UT1, TT and historical ΔT (P0)

**Owner:** ephemeris core. **Start:** `calendar.rs`, `time.rs`,
`first_sky.rs`, [calendar](CALENDAR_JULIAN_DATES_2026-11-10.md) and
[TAI/TT](TAI_TT_SPLIT_JULIAN_DATES_2026-11-10.md) docs. Inspect draft PRs for
existing leap-schedule work before starting.

- Separate civil UTC, UT1, TT and TDB; document policies and leap data expiry.
  Design historical and future ΔT/EOP inputs, source provenance and explicit
  unsupported periods. Respect date-boundary and fractional-second behavior.
- Define which version and coverage the first release supports; historical
  natal dates and eclipse range 0100–3000 need separate scoped acceptance.
- Acceptance: independent Gregorian/Julian JD fixtures and reverse conversion;
  midnight, fractional hours, leap boundaries, expired tables and invalid inputs;
  documented independent time-scale references and tolerance. `swe_julday`
  arithmetic must not pretend it converts UTC to UT1. Unsupported eras error
  with actionable diagnostics. No implicit UTC=UT1 without explicit metadata.

## B — Independent references and binding contract tests (P0)

**Owner:** ephemeris core/test tooling. **Start:** `examples/validate_horizons.rs`,
[offline comparator](DE441_HORIZONS_OFFLINE_COMPARISON_2026-11-10.md).

- Specify per-field comparison metrics before implementation: circular angle
  difference at 0/360, degrees and degrees/day, AU and AU/day, source, dates,
  bodies, reference frame, center, model settings, permitted deviations and
  rationale. Capture actual tolerances alongside reviewed reference provenance;
  never choose tolerances simply to make an implementation pass.
- Create separate, reproducible optional probes for an independently obtained
  JPL Horizons reference and separately installed/appropriately licensed
  Swiss Node 0.5.17. Do not depend on the user's private checkout for normal CI.
  Review rights before checking in any external fixture; sanitized structural
  mocks do not establish astronomical parity.
- Acceptance: deterministic tests run offline without external kernels;
  optional reference runs report missing dependencies, actual source and
  unexecuted comparisons clearly. Include retrograde/station, Moon, wraparound,
  historical and modern dates, supported locations and error cases. Compare
  exact JS property names/flags and sync versus callback separately from numeric
  differences. Do not claim results for runs not actually performed.

## C — Apparent geocentric Sun–Saturn and rates (P0)

**Owner:** ephemeris core. **Depends on:** A, B and D for complete parity.
Start with `de441.rs`, `state.rs`, `coordinates.rs`, `first_sky.rs`. The
existing topocentric geometric output is useful foundation, not a compatible
answer to `swe_calc_ut`.

Progress: an independently testable geocentric reception light-time stage and
velocity derivative is documented in [GEOCENTRIC_LIGHT_TIME_2026-11-10.md](GEOCENTRIC_LIGHT_TIME_2026-11-10.md).
It does not yet supply apparent coordinates, rotating-frame speeds or a Node
facade; the package remains open.

- Build an explicit, modular position pipeline: observer and body center,
  light-time, frame bias/precession/nutation, aberration and deflection as
  required by the selected reference contract; derive position and velocity
  in the final rotating-of-date frame. State model/accuracy choices explicitly.
  Split physical corrections into separate PRs if useful.
- Implement Sun, Moon, Mercury, Venus, Mars, Jupiter, Saturn geocentric
  apparent ecliptic coordinates and all six ordinary polar output fields.
  Distinguish system barycenters from planet centers in data and output.
- Acceptance: B's independent referenced modern-date grid and tolerances,
  rate/sign checks across stations, angular wrap and Moon parallax, invalid
  data/source errors, no location-induced topocentric correction. Explain any
  dataset/model limitations and no-speed behavior. Do not expose unvalidated
  results through the facade as exact Swiss parity.

## D — Earth orientation, nutation and ECL_NUT (P0/P1)

**Owner:** ephemeris core. Implemented in `crates/ephemeris-core/src/earth_orientation.rs`
and documented in [IAU2000B_EARTH_ORIENTATION_2026-11-10.md](IAU2000B_EARTH_ORIENTATION_2026-11-10.md).

- Evaluates the 77-term IAU 2000B nutation series, mean and true obliquity of date,
  rotation matrices from ICRF to True Equator of Date and True Ecliptic of Date,
  sidereal time (ERA, GMST, GAST) with explicit UT1 vs TT separation, and `EclipticNutation`
  with Swiss-compatible `to_swiss_ecl_nut()` representation.
- Acceptance: independent numerical verification against ERFA `nut00b` and `num00b`
  at JD 2453736.5 TT (< 1e-13 rad / element), matrix orthogonality, vector length
  preservation, and explicit domain bounds (1900–2100 TT).

## E — Rust-backed Node compatibility facade (P0)

**Owner:** ephemeris bindings; compose astrology-functions for houses. No
existing Node/WASM binding is available on the inspected branch. Evaluate the
ADR's WASM-first approach against the backend's synchronous initialization,
file size/IO and callback needs; record the choice and measured constraints.

- Design a versioned, per-instance modern API and a thin Swiss-shaped layer.
  Inventory actual 0.5.17 constants, signatures, object properties, defaults,
  errors, return flags and sync/callback behavior from separately installed
  references. Explicitly configure a DE441 **file**, not just a `.se1` directory.
  `SEFLG_SWIEPH` must not silently assert Swiss data when DE441 is used.
- Acceptance: offline B contract tests for `swe_julday`, `swe_calc_ut`, error
  cases, concurrency/isolation and callback timing; true reported provenance;
  unsupported flags/bodies return explicit errors; deterministic no-network
  operation. Then run parity cases backed by C. Compatibility is incremental,
  never a promise of every upstream `swe_*` function.

## F — Additional default chart bodies (P0)

**Owner:** ephemeris core; new data policy where needed. Start with actual
backend default IDs in the inventory. Split nodes/apogees and Chiron into
separate PRs; data and models are different.

- Cover outer planet systems, mean/true lunar node, mean/osculating apogee,
  and Chiron with named, documented algorithms and source/data provenance.
  Distinguish mathematical points from physical apparent bodies.
- Acceptance: body IDs and names, independent fixtures in-range and at
  boundaries, correct rates and error behavior; no fabricated positions if
  the classic DE441 file lacks a required model or target. Include reference
  mode/flag differences and legally reviewed data acquisition.

## G — Houses and house positions (P0; separate repository)

**Owner:** `astrological-functions`, with ephemeris time/orientation inputs.

- Independently implement `V`, `P`, `K`, `R`, `O`, axes and fractional house
  position needed by the backend. Define a versioned contract for ARMC, true
  obliquity, latitude, east longitude, input coordinates and returned axes.
- Acceptance: independent comparisons for each system, equator/north/south,
  longitude wrap, polar latitudes and singularities; define which errors and
  fallback policies are permitted instead of silent algorithm substitution.
  Binding arrays, axis fields and fractional `housePosition` must match the
  tested JS contract. Backend southern inversion belongs to chart assembly.

## H — Horizon coordinates and solar rise/set/transit (P1)

**Owner:** core + facade. **Depends on:** A, C, D as appropriate.

- Add the explicit `swe_azalt` variants used by the backend, true/apparent
  altitude, refraction and atmospheric defaults; implement solar rise, set,
  upper culmination and `swe_revjul`. Handle no-event/polar behavior.
- Acceptance: cardinal azimuth fixtures proving Swiss south/west versus modern
  north/east conventions; refraction, pressure zero and boundary cases; known
  rise/set/transit events with source, conventions and tolerance; field and
  callback tests. Investigate the noon helper's inconsistent signature/result
  rather than canonizing it without a runtime check.

## I — Backend integration proof (P0/P1, separate repository)

**Owner:** VirgoStars/Astrolabio plus functions and facade maintainers.

- Start with the smallest actual current-sky sample and a natal-chart fixture;
  test representative data and the application's expected response structure.
  Keep version/fingerprint/provenance semantics explicit when replacing the
  existing pinned Swiss adapter. Preserve geocentric semantics and computed
  rates; do not inject topocentric `first_sky` values into that flow.
- Acceptance: parallel non-production execution, reviewed tolerances and
  diffs, handled missing bodies/data, dates, house systems, errors and
  sync/callback paths; reproducible rollback and a separately authorized
  deployment. No external service call or production mutation in tests.

## J — Fixed stars and eclipses (P2)

**Owner:** core/events + bindings, with catalog provenance review.

- Fixed stars: lookup/aliases, proper motion and magnitude from an approved
  catalog; reproduce the consumed `swe_fixstar_ut`/`swe_fixstar_mag` shape.
- Eclipse routines: global solar/lunar search, type flags, contact times,
  forward search and calendar conversion. The current backend requests years
  0100–3000, requiring an explicit date-dependent ΔT policy. Compare actual
  Node response fields with routine assumptions before migrating.
- Acceptance: catalog license and source, independent star/event fixtures,
  boundary/absence errors, documented event classification and tolerances;
  distinguish partial/total/penumbral contacts and handle no-event cases.

## Long-term coverage after the backend slice

Use the full Swiss programming manual to enumerate remaining APIs, flags,
object catalogs, sidereal modes, heliacal/occultation events and supported
calendar ranges. Add individual acceptance cases as work is scheduled. The
backend inventory is a priority filter, not a limitation on the owner's full
coverage goal.
