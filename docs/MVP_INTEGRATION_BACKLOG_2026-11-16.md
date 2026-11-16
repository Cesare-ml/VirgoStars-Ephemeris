# MVP integration and verification backlog

**Status: experimental local working tree, 2026-11-16.** This is a prioritized
work list, not a release or production-readiness declaration. Current code,
actual executed tests and reviewed evidence take precedence over this list.

## Baseline and existing work

- `dev/ephemeris` at `af2f6e8` contains classic DE441, experimental SPK
  type 2/3 and offline reference tooling. One local classic-DE441 light-time
  comparison covers four targets at one epoch; provenance review remains open.
- The working tree carries forward IAU 2000B orientation from
  `codex/iau2000b-earth-orientation` (`96208d7`), IAU 2000 ERA from
  `origin/feat/iau2000-earth-rotation-angle-20261111` (`9fc954c`),
  IAU 2006 GMST from `origin/feat/iau2006-gmst-correction-20261111` (`928dae7`), and
  local mean sidereal geometry from
  `origin/feat/local-mean-sidereal-hour-angle-20261111` (`3fb5470`).
  These are working-tree integrations, not merged PRs or production features.
- `integration/mvp1-apparent-pipeline-20261116` at `01ac627` contains a much
  larger apparent-position, Node and MVP2 stack. Its 100+ changed files have
  not been reviewed or merged here; reconcile it incrementally rather than
  assuming its features, tests or external evidence are already accepted.
- Owner-supplied Claude Code handoff and a read-only Git inspection identify
  existing work on that integration branch: apparent-of-date positions and
  rates, a Node facade, fixed-star/Hipparcos processing, Chiron/type-21 SPK
  (`f4762df`), horizon and rise/transit operations (`be40608`), house facade
  (`8b93135`), and solar/lunar eclipse operations (`01ac627`). The branch
  contains corresponding tests and aggregate local-reference receipts;
  their recorded results have **not** been independently rerun in this audit.
  Earlier phrasing that these features still need to be built from scratch
  would be incorrect. Preserve and review the owner's documented choices,
  including polar-house behavior and the Regiomontanus polar difference.
- The standalone annual-aberration direction (`c916f594`) and the separately
  developed light-time plus aberration state/rate stage (`7ae358b0`) have been integrated
  locally. Direction normalization and centered finite-difference sample
  precision were hardened during integration; neither is a full apparent model.
- The fixed-axis spherical projection and shared degree/AU conversion are
  reconciled with that finite-difference stage, including signed rates and
  acceleration provenance; see [the contract](MVP1_FIXED_AXIS_ABERRATED_SIX_FIELDS_2026-11-16.md).
  Analytic acceleration and its scalar spherical adapter are now integrated;
  the optimized batch path remains unintegrated.

## Priority and acceptance

1. **Orientation and time policy (in progress):** finish review of imported
   2000B, ERA, mean GMST and local sidereal modules; preserve separate UT1 and
   TT and their rates. Add combined references across epochs, UT1/TT offsets,
   discontinuities and explicit EOP policy. The existing approximate equation
   of equinoxes omits complementary terms: do not call it full GAST. Provide
   full apparent sidereal time only with a specified, validated model.
2. **SPK/NAIF E1 (in progress):** use a separately acquired official DE440s
   SPK and independently installed NAIF/SPICE to generate a multi-body,
   multi-epoch J2000 geometric `NONE` matrix in km and km/s. Capture kernel
   origin, ordered hashes, toolkit version, command, predefined thresholds,
   residuals and a review-pending receipt. The first 15-case comparison and
   breadth audit passed locally; see [the evidence](SPK_NAIF_LOCAL_COMPARISON_2026-11-16.md).
   Further local comparisons cover three independently inspected internal
   record boundaries (45 cases) and declared coverage endpoints (20 cases).
   Audit provenance, thresholds, outside-coverage failures and segment
   transitions; never commit the kernel or claim validation solely from
   synthetic tests.
3. **Reconcile existing MVP1 apparent stack:** inspect its dependency graph,
   source and license provenance, recent changes to shared modules, relevant
   branch and PR status, and local evidence. Bring forward small cohesive
   pieces with regression tests. Validate geometric/light-time, aberration,
   frame/nutation and derivatives independently before combining them. The
   DE441 analytic-acceleration path has been reconciled with the hardened
   reader and checked on synthetic and local-file cases; broader independent
   numerical validation remains open. Next review the date-frame and rate
   transformations and their dependencies before importing them.
   Prefer evaluating the existing integrated branch as a whole, comparing it
   with the hardened current reader and uncommitted local work, over recreating
   its already implemented features one at a time. Keep existing worktrees and
   branches intact; resolve discrepancies deliberately.
4. **Independent apparent-position and rate matrix:** multiple targets and
   epochs, stations and wrap, matching body centers, time scales, reference
   frames and physical corrections; justify per-field error budgets before
   evaluating results. Review original reference responses and provenance.
5. **Civil-time and Earth orientation:** explicit leap-second expiration,
   caller-supplied UT1/EOP or documented bounded approximation, historical
   Delta-T policy, unsupported eras and errors; no silent UTC=UT1 fallback.
6. **Consumer boundary:** evaluate the existing Node bridge and
   `astrological-functions` integration only after core contracts are stable;
   add cross-language parity and error tests. Houses remain in the functions
   layer. No production migration without its separate gates.
7. **Release gate:** reproducible offline tests, platform/CI results,
   dependency and dataset license audit, reproducible packaging and rollback,
   reviewed supported ranges and uncertainties. Publishing and deployment
   require separate authorization.

## Evidence and limitations

Tests of ERFA numerical anchors do not validate the DE441 or SPK readers.
Synthetic kernels prove software behavior only. An individual successful
real-kernel comparison cannot validate all dates, bodies, frames or modes.
`first_sky` remains a separately documented approximate geometric pipeline;
no orientation import activates a new production path.

**CPAL-1.0 modification record (2026-11-16):** added this backlog and
reconciled existing experimental orientation primitives, tests and the
required ERFA attribution. No kernel or production integration is bundled.
