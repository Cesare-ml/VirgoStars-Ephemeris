# Local stellar catalog recovery — 2026-11-17

**Follow-up:** six additional locally sourced candidates and their magnitude
warnings are recorded in [the gap review](STAR_CATALOG_GAP_REVIEW_2026-11-17.md).

**Experimental, local only; catalog coverage remains incomplete.** No
production provider is changed. The owner chose the astronomically correct
identity when Swiss selects another object, with an explicit warning.

## Recovery

The original 1,062-object manifest, reports and inputs were found in the old
`target/mvp1-reconcile` worktree and copied into this checkout's ignored
`local-data/swiss-census-2026-11-17/` directory. All 157 original files were
compared by SHA-256 after copying: no mismatch. The recovered
`manifest-iau-near-aliases.json` has SHA-256
`5dc46732ab8638b25c927c9430ec9499ac53e9f1469b36e2ab2c7eff7c65f731`,
matching its saved coverage report. The source files and manifests are local,
not committed or bundled. The prior readiness document's missing-file
observation was accurate for that earlier checkout; it is now superseded.

The newly repeated gate reproduced **687/729 names and 1,065/1,113
designations**, leaving 42 names and 48 designations. It recorded no null
magnitudes and 1,057 catalog entries with radial velocity assumed zero. These
are lookup-string counts, not numbers of distinct physical objects.

## Source-backed local additions

- `Rigel Kentaurus`, `Bungula` and `,alCen` now point to the existing
  `Rigil Kentaurus` entry, HIP 71683, **Alpha Centauri A**. The historical
  system names and designation can refer to multiple components; this local
  decision selects A explicitly and does not claim a modeled system centroid.
  [IAU WGSN bulletin 2](https://www.iau.org/static/science/scientific_bodies/working_groups/280/WGSN_bulletin2.pdf)
  identifies Rigil Kentaurus as Alpha Cen A; the [University of Colorado ASTRAL
  project](https://casa.colorado.edu/~ayres/ASTRAL/) uses the Rigel spelling,
  and the [RASC asterisms handbook](https://www.rasc.ca/sites/default/files/World%20Asterisms%20Project%20Handbook%20V%202025.14.pdf)
  records the historical Bungula name. Local Swiss comparisons at May 15 of
  1950, 2000 and 2024 returned up to 17.202 arcseconds difference in latitude;
  each added lookup carries an explicit `SWISS_POSITION_MISMATCH` warning.
  HIP2's linear motion does not model the binary orbit.
- `Oculus` and `,piCap` now resolve to HIP 100881, π Capricorni. The
  caller-supplied IAU-WGSN NEC snapshot cross-identifies HIP 100881 with
  π Cap; the historical Oculus name is also documented in this
  [star-identification reference](https://astrologyking.com/oculus-star/).
  V magnitude 5.08 is taken from the separately supplied Hipparcos I/239
  record, while its HIP2 solution type 15, two components, is explicitly
  **experimental**. Only five-parameter linear motion is used. A missing
  radial velocity is explicitly assumed zero. Component identity and binary
  dynamics still need review.

The new local manifest is `manifest-recovered-oculus-20261117.json`,
SHA-256 `17418de24bbafed8e55b5d224df92548595592440f2d62a754942232a1bb912a`.
Its gate reports **1,063 candidates**, 694 aliases, **690/729 names** and
**1,067/1,113 designations**: **39 names and 46 designations remain missing**.
Of 1,063 candidates, none has a null magnitude and 1,058 assume zero radial
velocity. A covered lookup or caller assertion is not scientific approval.

The refreshed same-DE441 Swiss JPL-mode diagnostic compared all 1,757 names
at nine TT epochs (May 15, 1900–2099) against approximately corresponding
TDB evaluations. All **15,813** comparisons completed. **198** exceeded one
arcsecond in at least one component; all 198 had predeclared lookup warnings,
none was unmarked. The three-epoch alias-only diagnostic on the intermediate
manifest had 42 cases above its 1″ threshold, all 15 affected aliases marked.
These are compatibility diagnostics, not evidence of absolute accuracy,
complete identity review, or performance between epochs. Report hashes:

- `coverage-recovered-oculus-20261117.json`: `52e1655c879cd1f07c6fbb8b90db1537e76384df99d8e70f7d772140e6bb320f`.
- `stars-century-recovered-oculus-20261117.json`: `aa3a7306c0dff8534c0ea80df3907c50999b2c8e78bf74e39a526b83531bc8d6`.

## Repeatable local workflow

The new alias and star tools require separate caller-supplied decision files,
exact manifest and source hashes and new output paths. They validate catalog
shape, prevent duplicate lookups and check HIP2 quality for new stars; URLs,
identities, magnitudes and decisions remain caller assertions, not verified
by the programs. The HIP tool's selection requires the existing explicit
experimental policy. The alias tool preserves lookup-specific warnings.

```text
node scripts/attach_reviewed_star_aliases.mjs <manifest> <sefstars> <alias-decisions> <new-manifest> <new-report>
node scripts/add_reviewed_hip_stars.mjs <manifest> <sefstars> <hip2> <star-decisions> <new-manifest> <new-report>
node scripts/verify_swiss_coverage.mjs <manifest> <sefstars> <new-coverage-report>
node bindings/node/scripts/audit-star-century.cjs <manifest> <hip2> <de441> <de441-sha256> <swisseph> <sefstars> <new-report> all-names
```

The local Alpha Centauri decision file hash is
`d8d3186ad375995d76fa16ea76bcb709e2df7cb063c0e3386b9b1fb63c69cdbb`;
the Oculus decision file hash is
`176b734b118adb251acc5a96a502134a26115278a2faddc0bac05aa6d298b924`.
Keep inputs distinct and do not overwrite prior evidence.

## Remaining

The remaining list includes ambiguous shared names such as Kuma and Jabhat al
Akrab, stars absent from HIP2 such as Alula Australis, extended objects,
reference directions and test records. A direction-only or extended-object
model must not be fabricated from a stellar parallax. Establish each object
and component from independent sources, model it explicitly where supported,
and retain an unresolved result when evidence or a suitable model is absent.
The complete catalog and main-server gate remain open.
