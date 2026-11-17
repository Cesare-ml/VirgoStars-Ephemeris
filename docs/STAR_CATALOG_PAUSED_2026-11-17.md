# Fixed-star catalog — paused checkpoint, 2026-11-17

**Status: PAUSED by owner; local experimental catalog, not production.** The
owner has decided that the current breadth is sufficient for now and that
other work takes priority. Do not keep researching missing stars as a default
follow-up; resume this track when explicitly requested. This is a scheduling
decision, not a declaration that the catalog is complete or scientifically
approved. The user's existing condition remains: do not replace Swiss on the
main VirgoStars server until the full Swiss stellar inventory is covered and
the integration and validation gates are met.

## Where work stopped

The latest [two-candidate checkpoint](STAR_CATALOG_K01PUP_ETACAR_2026-11-17.md)
describes k¹ Puppis and Eta Carinae and records the source assertions, model
assumptions, hashes and same-DE441 diagnostic. It follows the earlier
[pulsar and names checkpoint](STAR_CATALOG_PULSAR_AND_NAMES_2026-11-17.md) and
the linked sequence of catalog reviews.

The local coverage report for that checkpoint records:

- 1,083 objects and 710 aliases in the experimental manifest.
- 702 of 729 Swiss name strings and 1,091 of 1,113 designation strings
  recognized; 27 names and 22 designations remain missing. These sets can
  overlap and include nonstellar objects and reference directions.
- Six objects without a usable magnitude, 1,068 with an assumed zero radial
  velocity, and 64 lookup-specific caller-asserted warnings. A recognized name
  can still have missing data, an approximate system model or a known difference
  from Swiss; these counts are not a measure of scientific accuracy.
- The nine-epoch 1900–2099 same-DE441 diagnostic attempted 16,137 cases,
  compared 16,119 and had 18 unavailable Swiss references. Of the comparisons,
  325 cases exceeded the diagnostic one-arcsecond threshold in a component;
  these and the 18 unavailable references were all declared. Shared DE441,
  time-scale approximations and sparse sampling prevent treating this as an
  independent accuracy validation or a global error bound.

The caller-supplied manifest and decisions are **ignored local files, not
bundled or versioned catalog data**. The cited SHA-256 values authenticate
neither the astronomical provenance nor the choices in those files. In this
workspace the latest manifest and reports are under
`local-data/swiss-census-2026-11-17/`:

- `manifest-etacar-k01pup-20261117.json` — SHA-256
  `908da6937d255c5b5dc6c23395823953d528c9ff5752ad966a9d6d93c2115065`.
- `coverage-etacar-k01pup-20261117.json` — SHA-256
  `ad31070fbf97d2cda7c3158d41164d3a98a660a853fe4c6bc81cda5d8f8b2b36`.
- `stars-century-etacar-k01pup-20261117.json` — SHA-256
  `fdf603b1321642cb9ec55d867ee5c4b10166f40357574add7c65f7c1c9b6e5a7`.

These local files are not protected by Git and will not be present in a fresh
clone unless separately preserved. Do not claim the catalog is distributed
with the library or rebuild it from undocumented assumptions if they are lost.

## When the owner resumes this track

1. Verify the local files and hashes above, available source-data permissions,
   current code and whether another checkpoint has superseded this one. If the
   local files are missing, recover them or explicitly reconstruct and review
   the evidence; do not silently infer their contents from the counts here.
2. Read the coverage report's missing lists. Resolve identities and components
   using traceable sources; distinguish multiple systems, extended sources and
   reference directions. The owner chose the astronomically appropriate
   identity and an explicit warning when Swiss returns a different object.
3. Review missing and variable magnitudes, radial-velocity assumptions,
   astrometric epochs, physical models and uncertainty. Repeat the exact-string
   coverage and same-DE441 checks, then obtain independent validation with
   appropriate reference data and tolerances before claiming precision.
4. Prepare a reviewed integration plan and meet the owner's full catalog
   condition before any production server switch.
