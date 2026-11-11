# Three MVPs for replacing Swiss Ephemeris in VirgoStars

**Product priority decision — 2026-11-11.**
**Status: agreed scope and sequencing; NOT an implementation or a release.**

This document is the **authoritative product-priority order** for
VirgoStars-Ephemeris and its cooperating
[Astrological Functions](https://github.com/Cesare-ml/astrological-functions)
project. It supersedes the *priority labels* P0/P1/P2 in the
[2026-11-10 backend Swiss inventory](BACKEND_SWISS_COMPATIBILITY_2026-11-10.md)
and [work packages](BACKEND_WORK_PACKAGES_2026-11-10.md) wherever they
differ, **not** their detailed observed call-site evidence or scientific
contracts. The E0–E6 **engineering phases** in
[ROADMAP](ROADMAP.md) remain prerequisites and validation gates; the
three MVPs below are distinct **product deliveries**, not renamings of E1,
E2, or E3.

**Source of actual usage:** `Cesare-ml/virgo-stars`,
`backend/services/astrology/func.js` and the additional backend
locations recorded in the inventory. The 2026-11-10 inventory used a
specific backend revision and the Node `swisseph` binding 0.5.17;
**reinspect actual deployed and source behavior before migration.**
The word `swe_*` below describes **compatibility targets only**,
not code or algorithms to copy from Swiss.

## MVP 1 — core calculations needed first

Implement these features in the **following product order**. Lower rows
may be researched/developed in parallel where technically necessary,
but they do not take release priority over the first geocentric
planet-position pipeline.

| Product order | Required functionality | Existing Swiss reference / API contract | Project owner / scope |
|---|---|---|---|
| **1** | **Sun, Moon and planets, positions and velocities** | `swe_calc_ut`, especially `SEFLG_SPEED` with the existing geocentric apparent ecliptic-of-date behavior. Start with Sun–Saturn; extend to Uranus, Neptune, Pluto, and every **physical planetary body** used by the backend. Preserve the binding's six polar position/rate fields, units, effective source/flags and errors. A DE441 planetary-system barycenter must not be silently presented as the planetary center. | **Rust core**: ephemeris interpolation, correct frames/times and apparent corrections; **thin Node adapter**: observed JS contract |
| **2** | **Julian-day roundtrips** | `swe_julday`, `swe_revjul`, Gregorian/Julian variants and observed return-field behavior. Separate calendar arithmetic from UTC, UT1, TT, TDB and historical ΔT; include accurate fractional times, leap boundaries and explicitly supported birth-date ranges. | **Rust core** + Node adapter |
| **3** | **Houses and house positions** | `swe_houses`, `swe_houses_pos`; first cover exactly the backend's `V`, `P`, `K`, `R`, `O` codes (currently called Egyptian, Placidus, Koch, Regiomontanus and Porphyry in the app), with independently verified codes/definitions, angles, axes, cusps and fractional house positions. No silent fallback near polar singularities. | **Astrological Functions** owns astrological house algorithms; Rust provides orientation/sidereal/obliquity/time primitives; **Node facade** composes both |
| **4** | **Fixed stars** | `swe_fixstar_ut`, `swe_fixstar_mag`: observed names/aliases, catalog identifiers, apparent coordinates, angular motion, magnitude and missing-star errors. Select and license-audit an independent stellar catalog and its reference frame, proper motion and epoch conventions. | Rust stellar/catalog calculations, thin facade and approved data source |
| **5** | **Load/configuration and the replacement interface** | Install/load a VirgoStars-native Node package in place of the `swisseph` binding; provide an explicit `load`/initialization contract and relevant `swe_set_ephe_path`-shaped configuration, stable constants/body IDs, synchronous and callback variants actually used, errors and object keys. A directory of `.se1` files must never masquerade as a DE441 classic binary file or a NAIF SPK. | Rust-backed Node native or WASM **thin binding**, compatibility facade, and backend integration harness |

**Important implementation nuance:** the loading/binding *scaffold* and its
offline contract tests should start in parallel with row 1, so the
first planetary computations can be exercised from Node.js. The
**full replacement/load interface** is the last **MVP-1 acceptance
gate**, not a reason to postpone Node work until everything else is
ready. The architectural choice between Node-native and Node-WASM
requires an explicit review of synchronous file I/O, initialization and
performance; **neither is yet a shipped drop-in replacement**.

**Body-scope distinction:** MVP 1 means Sun, Moon and the **physical
planets** (including the outer planets used by the app) and their rates.
The existing default chart array also includes **non-planetary
astrological points and Chiron**. Those remain explicitly in MVP 2
below unless they are required to complete a specific MVP-1
acceptance fixture; never claim MVP 1 reproduces the *entire* default
VirgoStars chart when those bodies are absent.

### MVP-1 exit gates — ALL mandatory

1. **Scientific truth:** a legally obtained and versioned real JPL
   ephemeris file, real independent JPL Horizons/NAIF vector comparisons
   and defensible per-field tolerance; no synthetic-only "accuracy"
   claim. File formats `de441.eph`, `.bsp` and `.se1` are explicitly
   distinguished. Unsupported source/ranges fail closed.
2. **Correct observed result:** geocentric **apparent**, ecliptic of
   **date**, with specified center/frame/corrections and all consumed
   velocities; compare an independently installed Swiss reference
   using the same date, physical body, flags, units and behavior.
   Test Sun, Moon, inner and outer planets, 0°/360° crossing,
   retrograde/stations and rates against centered differences.
3. **Date contract:** Julian-day forward/reverse fixtures including
   calendar transitions (by declared proleptic rules), midnight and
   fractional-day boundaries, UTC/UT1/TT/TDB policy, historical natal
   dates, leap seconds and unsupported intervals. No silent extrapolation.
4. **Houses and stars:** five observed house modes, axes and
   `swe_houses_pos`, northern/southern/polar cases, and star names,
   missing aliases, proper motions, magnitude and reference-catalog
   licensing. Verify values and the actual JS return shapes.
5. **Load and consumer proof:** Node package can be loaded in a clean
   offline test application and used for one representative current-sky
   sample and one **in-range natal-chart scenario**; synchronous and
   callback behavior, source flags, errors, isolation and source-data
   configuration are independently checked. No direct production
   substitution. A complete existing VirgoStars default chart needs
   the additional MVP-2 bodies/operations as applicable.
6. **Integration:** review and integrate the upstream draft PRs with
   passing Rust CI; run the new engine and Swiss in parallel against
   pinned fixtures, with a documented rollback and explicit production
   approval before any switch. Passing compilation or unit tests alone
   is not proof of numerical parity.

## MVP 2 — all other Swiss functions actually used by VirgoStars

After the MVP-1 *feature* set, implement **every other Swiss call, body
type, flag and behavior used by the existing software**, even if it is
not needed for the first vertical slice. This is the gate for **removing
the Swiss Ephemeris dependency from the existing VirgoStars backend**.

The observed backend inventory currently includes these families:

| Functional group | Existing reference examples | Required result |
|---|---|---|
| Remaining chart bodies and mathematical points | Mean/true lunar nodes, mean/osculating lunar apogees, Chiron; default backend body IDs `[0..13,15]` | Explicit source/models, correct dates, rates, IDs and errors rather than fabricated DE441 values |
| Alternate planet coordinates and modes | `swe_calc_ut` with `SEFLG_HELCTR`, `SEFLG_EQUATORIAL`; additional options/flags present in the real callers | Observed heliocentric/equatorial output objects, flags, frames, centers and rates only when requested |
| Nutation and orientation output | `swe_calc` with `SE_ECL_NUT` | Correct TT-vs-UT interpretation, fields and model; comparison with an independent reference |
| Horizon and atmospheric corrections | `swe_azalt` | Swiss azimuth convention, true/apparent altitude, pressure/refraction behavior and observed result fields |
| Solar day and planetary hours | `swe_rise_trans`, `SE_CALC_RISE`, `SE_CALC_SET`, `SE_CALC_MTRANSIT` | Sunrise/sunset/transit, missing polar events, correct timestamps/fields and error policy |
| Eclipse lookup routines | `swe_sol_eclipse_when_glob`, `swe_lun_eclipse_when`, relevant `SE_ECL_*` and `SEFLG_*` | Event search and classifications, times/contacts/flags and historical ΔT policy; distinguish scheduled batch jobs from interactive use |
| Miscellaneous observed facade behavior | `swe_get_planet_name`, overloaded `swe_calc`/callback shapes, `swe_revjul` where reused | Match existing observed JS object fields, constants, errors, callbacks and alias behavior |

Reinspect the source and actual Node binding before accepting the
exhaustiveness of this list: include any other **reachable
`swe_*`** use in the current VirgoStars and Astrolabio software,
not just the 2026-11-10 snapshot. Avoid treating dormant commented
code and offline jobs as the same operational priority; they are still
within the MVP-2 **coverage target** if the existing software uses
them. Keep the separate
[VirgoStars Event Ephemeris](https://github.com/Cesare-ml/VirgoStars-Event-Ephemeris)
and this engine's event functionality coordinated without duplicating
astronomical numerical source-of-truth.

### MVP-2 exit gate

**Every observed Swiss-dependent pathway**, including chart assembly,
stars, planetary hours, fixed-star lookup, eclipse batch routines and
required Astrolabio workflows, passes explicit behavioral and scientific
parity/regression tests **without importing or invoking Swiss at
runtime**. Backend deployment still requires a separate authorized
migration/rollback process. A pure-math implementation of one function
or an output merely shaped like Swiss is not sufficient.

## MVP 3 — all remaining Swiss Ephemeris capabilities

**Full documented Swiss Ephemeris functional coverage is the long-term
target regardless of whether VirgoStars itself calls a function.**
After MVP 2, build a **versioned function-by-function and flag-by-flag
inventory of the upstream Swiss Ephemeris API**, beyond the used
backend subset. Include the applicable variations, body catalogues,
error conditions and operating ranges.

The later inventory should cover at least these categories:
- Additional planetary/small-body families and ephemeris modes:
  heliocentric, barycentric, topocentric, astrometric, sidereal,
  equatorial, rectilinear, speed and speed3 variants, full
  planet/satellite/asteroid coverage, lunar nodes/apsides and
  orbital-element/phenomenon calls where documented.
- All remaining house methods, house positions and Gauquelin sectors,
  sidereal modes and ayanamshas, named/extended time and calendar APIs,
  sidereal time and ΔT calculation variants.
- Fixed-star and catalog variants beyond the MVP-1 subset; name
  resolution, magnitudes, proper motion, epochs and supported catalogues.
- Other events: eclipses and occultations with local/global search
  variants, rise/set/transit options, heliacal phenomena, visible
  magnitude and optical/atmospheric methods where applicable.
- Remaining upstream utility, configuration, data provenance, naming,
  constants, result variants and language API compatibility surfaces.

Do **not** claim all functions/flags are known from the categories
above; first create an audited exact inventory pinned to a specific
Swiss manual/binding version, then close each row only after
independent clean-room implementation, provenance/rights checks,
and reference benchmarks. A different numerical model may require
declared tolerances and cannot always be bit-for-bit identical.
Unsupported or data-dependent features must return typed errors rather
than silently mimicking Swiss success.

### MVP-3 exit gate

Every agreed publicly documented Swiss API/flag combination is
either **implemented and independently validated**, or its omission
is explicitly disclosed and **prevents the literal "100% coverage"
claim**. The core remains an independent Rust implementation; JS/TS
and Python consumers use thin bindings, not duplicated astronomy.
Rights to third-party ephemeris files, star catalogs and harmonic
coefficient tables are evaluated separately from the project's
**CPAL-1.0** source-code license.

## Cross-repository responsibility map

| Work | Source of truth | Consumers |
|---|---|---|
| JPL data interpolation, physical planetary/stars models, observer/time/frame transformations, low-level events | `VirgoStars-Ephemeris` **Rust** | All language wrappers and apps |
| Houses (MVP-1 codes first; other systems MVP 3), zodiac methods, aspects, charts and other astrological rules | `astrological-functions` (TypeScript), using Rust outputs | VirgoStars, Astrolabio and Desktop |
| Thin Node load/config adapter and Swiss-shaped compatibility layer | Dedicated binding/facade over the Rust core; compose `astrological-functions` for houses | VirgoStars backend and future clients |
| Current application call-site truth and integration/rollback | `Cesare-ml/virgo-stars` and relevant Astrolabio code | Safe staged deployment |
| Visualization/assets | `astrological-charts`, `astrological-assets`, `virgostars-desktop` | Not part of core astronomical parity |

The user requirement **"free/open source with VirgoStars credit"**
continues to apply. Original code is licensed under CPAL-1.0 and
must preserve the applicable **Powered by VirgoStars**
<https://www.virgostars.it/> credits. Do not copy Swiss/ERFA/SOFA
program code or include unreviewed NASA/JPL, catalog or nutation
datasets.

## Operational planning rule for contributors and scheduled agents

- At each development cycle, prefer unfinished **MVP-1** work in its
  stated product order, then MVP 2, then MVP 3. Do not open repeated
  low-impact test/boilerplate PRs while the first real planetary
  input→apparent-six-field→Swiss-comparison vertical slice is blocked.
- Check `AGENTS.md`, this document, existing open PRs and their
  dependencies, and the detailed work-package checklist before editing.
  Existing draft PRs for corrections and nutation are *partial
  components*, not proof of a full apparent-position pipeline.
- Parallel scaffolding for MVP-1 dependencies (date models, houses,
  catalog procurement, Node binding) is welcome if it unblocks the
  first integrated result; priority does not mandate idle waiting.
- No guessed deadline, percent complete, full parity claim or
  production migration from green synthetic tests alone. Record
  evidence, exact scope and independently validated tolerances in
  each PR; keep merges/release/deploy behind review and user approval.

**2026-11-11 documentation change record:** wrote this MVP ordering
to align product priorities with the actual Swiss calls used in
VirgoStars; no code, dataset, dependency, CI behavior, license or
production changes.
