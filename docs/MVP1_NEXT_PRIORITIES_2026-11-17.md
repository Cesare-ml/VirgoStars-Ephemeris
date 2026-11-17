# MVP 1 — status and proposed priorities, 2026-11-17

**Owner update:** the requested planetary range is now 13000 BCE through
3999 CE. An [initial extended-range experiment and diagnostics](PLANET_VALIDATION_13000BCE_3999_2026-11-17.md)
supersede the narrower year 0–3000 goal in the historical proposal below. The
extension is experimental and has not passed scientific acceptance.

**Status: experimental library and locally integrated Desktop provider; no
scientific acceptance or main-server replacement.** This is a work order and
acceptance proposal based on the current source and existing local evidence,
not a claim that tolerances, dates or release scope have been approved. The
owner paused stellar work; [its checkpoint](STAR_CATALOG_PAUSED_2026-11-17.md)
remains the authority for that track.

## What exists

- The Rust core reads a caller-supplied classic DE441 and provides experimental
  apparent geocentric positions and rates. Mars through Pluto in that path are
  **system barycenters**, not distinct planet centers. The standard civil-time
  path has narrower limits than the separate long-term experimental API.
- A sampled year 0–3000 TDB exercise compared 3,010 directions with Horizons
  and 12,040 seasonal positions with Swiss; both references use the same DE441
  solution. The recorded maxima and important qualifications are in
  [planet validation](PLANET_VALIDATION_0_3000_2026-11-17.md). Year 0 uses
  astronomical numbering. This is not a continuous accuracy bound, and it
  does not establish historical local civil-time accuracy.
- Velocity and retrograde-station comparisons exist, but published reference
  speeds and derivatives of reference positions can differ. Sampled station
  directions were checked with Horizons, not certified to an exact time; see
  [station analysis](STATION_JPL_AND_HISTORICAL_UT1_2026-11-17.md).
- SPK/NAIF type-2 and type-3 geometric states and selected multi-kernel paths
  have limited real-kernel CSPICE comparisons. This is not broad SPICE
  compatibility or validation of apparent and topocentric calculations.
- Desktop main `15d7a4a` has a native provider pinned to ephemeris core
  `6835e00`; its implementation accepts years 1900–2033, subject to ΔT data
  ending before the end of 2033. Its documents record local Windows tests and
  an unsigned, unpublished installer. This is not evidence that newer main
  commits, a published build, other platforms or years 0–3000 were tested.
- The Node compatibility facade and backend integration laboratory exist.
  The main server has not been switched. Full stellar inventory coverage remains
  the owner's condition for that switch; the catalog track is paused.

## Proposed work order

1. **Define the MVP acceptance contract.** Identify the intended consumer and
   version, supported bodies and barycenter conventions, corrections, units,
   flags, errors and tolerances for positions, speed and station events. Keep
   dynamic time (TDB/TT), UT1, UTC and historical local civil time distinct.
   Preserve the year 0–3000 validation goal; an initial Windows/Desktop
   delivery limited to the currently supported modern interval is a possible
   intermediate milestone, not an assumption that the owner has reduced the
   long-term requirement.
2. **Close the planetary and time acceptance gaps.** Validate apparent
   positions, velocities and direction changes against documented independent
   calculations with matched conventions and agreed tolerances; separately
   document the shared-orbit limitation of same-DE441 comparisons. Establish
   how a caller supplies historical/future ΔT, its source and uncertainty, and
   how uncertainty affects results. Test range endpoints and failure paths.
   The experimental explicit-ΔT path exists; it is not an approved model or a
   guarantee for ancient and future UT1. Years 3001–5000 need separate coverage.
3. **Make the Desktop consumption reproducible.** Reconcile its pinned core
   with reviewed library main, run native end-to-end checks on the agreed
   bodies, speeds, houses, polar/missing-data cases and date boundaries, and
   validate the data bundle and error messages. Preserve the currently limited
   date contract until any extension is reviewed. Installer packaging, data
   provenance/redistribution terms and signing are delivery decisions; a
   locally tested unsigned installer is not a published release.
4. **Review SPK only to the scope actually required.** Verify existing
   type-2/type-3 and Chiron paths, source files, coverage, tolerances and error
   handling for the chosen consumer. Expand other segment types or frames only
   when a required MVP body or use case needs them.
5. **Keep production integration separate.** Later, exercise actual backend
   calculations against its current provider across required bodies, flags,
   locations, polar cases and error paths; define data provisioning and a
   verified rollback. The stellar pause means the full-catalog prerequisite
   is intentionally still unmet; do not activate the new engine on the main
   VirgoStars server in this phase.

## Done criteria to agree before calling MVP 1 complete

- A versioned library/consumer contract with explicit range, conventions and
  failure behavior, with chosen tolerances and reproducible evidence.
- Passing relevant local and CI tests plus reviewed reference comparisons;
  clearly distinguish independent calculations from an independent orbit model.
- A documented civil-time/ΔT policy and uncertainty treatment appropriate to
  each accepted range; no claim that a single historical time is exact.
- A reproducible native integration with its external data provisioning and
  missing-data behavior; separately report local testing, packaging and release.
- Explicit separation of the paused stellar/server gate from the planet-only
  library or Desktop milestone. No single percentage is meaningful until the
  owner chooses the exact release scope and tolerances.
