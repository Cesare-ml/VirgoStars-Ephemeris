# Experimental cluster centers — 2026-11-17

**Local experimental candidates, not full catalog completion.** These four
objects are open clusters, not individual stars. The owner chose the correct
physical identity and explicit notice of differences from Swiss. The main
server remains unchanged.

## Caller-supplied center models

The existing explicit-astrometry path is used as a **point approximation for
the cataloged cluster center**, not a model of cluster extent, member stars or
integrated light. Each center has an ICRS J2000 position; 2451545.0 TDB is
used as an approximation to the catalog epoch. Proper motions, parallaxes and
radial velocities below come from distinct published sources as transcribed by
SIMBAD. Coordinates are low-precision (SIMBAD quality E); combining sources,
assuming constant linear motion and extrapolating the center need independent
review. These uncertainties are not automatically propagated to the result.

| Local lookups | Object and primary source | Approximation and local Swiss comparison |
| --- | --- | --- |
| `Trapezium`, `,th-1Ori` | [Orion Trapezium open cluster](https://simbad.harvard.edu/simbad/sim-id?Ident=%2A+tet01+Ori) | Center from SIMBAD, parallax 2.557 mas. Swiss returned a direction nearly opposite in longitude on the nine selected epochs; cause not established. |
| `Praesepe Cluster`, `,M44` | [Praesepe / NGC 2632](https://simbad.harvard.edu/simbad/sim-id?Ident=M+44) | Parallax 5.371 mas. Center differs from the local Swiss position by up to 1,238″ in latitude on the nine selected epochs. |
| `Aculeus`, `,M6` | [Messier 6 / NGC 6405](https://simbad.cds.unistra.fr/simbad/sim-id?Ident=M+6); local historical name [Aculeus](https://astrologyking.com/aculeus-star/) | Parallax 2.1626 mas. Center differs from the local Swiss position by up to 45.2″ in longitude on the nine selected epochs. |
| `Acumen`, `,M7` | [Messier 7 / NGC 6475](https://simbad.u-strasbg.fr/simbad/sim-id?Ident=M7); local historical name [Acumen](https://astronectar.com/fixed-stars/) | Parallax 3.5704 mas. Center differs from the local Swiss position by up to 173″ in latitude on the nine selected epochs. |

SIMBAD describes the Trapezium as a group of multiple stars. We do not map
the cluster name to any one member. Cataloged cluster centers can differ by
definition, and a numerical match is not an identity or error guarantee.
Each lookup has an explicit `SWISS_POSITION_MISMATCH` warning and a display
name identifying it as a center. The results are not shapes, member-star
positions or full cluster dynamics.

**Integrated visual magnitudes remain unavailable for all four candidates.**
The local facade reports `E_STAR_MAGNITUDE` for their magnitude lookups; it
does not substitute a component's magnitude or zero. Consequently these are
position-only candidates and do not complete the stellar workflow.

## Reproduction and validation

The new `virgostars-explicit-star-candidates/2` input binds a local explicit
candidate document to the exact input manifest digest and its asserted Swiss
source digest. The existing `/1` input remains supported. These checks do
not authenticate external source data or independently verify the caller's
identity, values, epoch or units. Replay with new output filenames:

```text
node scripts/attach_explicit_star_candidates.mjs <prior-manifest> <trapezium-candidates> <new-manifest>
node scripts/attach_explicit_star_candidates.mjs <intermediate-manifest> <three-cluster-candidates> <new-manifest>
node scripts/verify_swiss_coverage.mjs <new-manifest> <sefstars> <new-report>
node bindings/node/scripts/audit-star-century.cjs <new-manifest> <hip2> <de441> <de441-sha256> <swisseph> <sefstars> <new-report> all-names
```

The local final manifest has **1,075 objects and 704 aliases**. It covers
**696/729 names and 1,083/1,113 designations**, leaving **33 names and 30
designations**. These are exact lookup strings, not distinct stars. There
are four missing magnitudes, precisely these centers, and 1,066 objects with
assumed-zero radial velocity. A string match does not prove numerical or
physical correctness.

The refreshed local nine-epoch same-DE441 comparison attempted **16,011**
lookup/epoch cases, with **15,993** numerical comparisons and **18** Swiss
lookup failures. Of the comparisons, **288** exceeded the one-arcsecond
diagnostic threshold; all those and the 18 unavailable references had
declared warnings. The eight new cluster lookups account for 72 of the
position mismatches. Swiss inputs use TT and the engine approximate TDB;
this is neither independent orbital truth nor a precision guarantee. A
local real-engine facade check verified all eight lookups return center
names, warnings and explicit missing-magnitude errors. Synthetic tests
also check candidate hash binding, legacy `/1` compatibility and missing
magnitude behavior.

SHA-256 of local ignored artifacts:

- `manifest-cluster-centers-20261117.json`: `f9e2d6f5ab97ed0e33e0ec6273679c3360be76b22e7ee2805bfa25f6ba7c0a3e`.
- `coverage-cluster-centers-20261117.json`: `21556ba6f6cba05b2e205876a29060de63540bfe29f3ec2d129284552746acc7`.
- `stars-century-cluster-centers-20261117.json`: `bea9155f09a14c65a3a71732e611d26e9f8f552d54b4d716187a12dae7f8f846`.
- Trapezium candidate: `e5f72bb067734f941fd23023c84ac9027677aa14cb2f25b8adb849b31cfe70e5`.
- Three-cluster candidate: `e8d5b6542aeced9fd30573f236bbeee21571ba70ec2ea06222de9134d49a6a3b`.

Remain: identify and model the other names and objects, separately resolve
the missing integrated magnitudes, review component and center definitions,
and establish observational error budgets. Do not count a position-only entry
as a complete or production-approved catalog entry.
