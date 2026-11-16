# MVP 1 stellar catalog inventory — 2026-11-16

**Status: source inventory and caller-reviewed manifest interface; no real
catalog is reviewed, approved or bundled.** The local backend's
`STARS_ASTRODIENST` contained 106 distinct requested strings when inspected.
They are application inputs, not proof that each designates one HIP2 star.
Named clusters, asterisms, blended sources, aliases and individual components
must be resolved individually; an uncertain identity is not a valid HIP match.

## Independent source candidates

- [Hipparcos new reduction I/311](https://cdsarc.cds.unistra.fr/viz-bin/cat/I/311)
  supplies the independently obtained astrometry file already used by the
  engine. It does not supply the Johnson V magnitude, radial velocity or the
  product's aliases. Its existing local file fingerprint is recorded in
  [the source receipt](evidence/hip2-source-provenance-2026-11-16.json).
- ESA's Hipparcos main catalog I/239 contains a Johnson V field H5;
  [NASA HEASARC's catalog description](https://heasarc.gsfc.nasa.gov/W3Browse/star-catalog/hipparcos.html)
  identifies that field and notes provenance. Magnitudes of variable or blended
  systems require special handling; a single static number is not universal.
- The [Hipparcos Input Catalogue](https://heasarc.gsfc.nasa.gov/W3Browse/star-catalog/hic.html)
  describes radial velocities where available, positive for recession, with
  quality information. HIC/HIP identity and component mapping still require
  verification; a missing or unsuitable measurement must not become an
  unreported zero. The source describes heterogeneous historical estimates.
- The [IAU WGSN announcement](https://www.iau.org/IAU/Iau/News/Ann2026/New-Star-Names-2026.aspx)
  are identity leads, not automatic proof of every legacy alias or combined
  object. Resolve each name and component against the cited source.

[CDS terms](https://cds.unistra.fr/legals/) require the license of each
dataset to be respected and the dataset/service to be cited. General site
access is not evidence that a specific derived catalog can be redistributed.
The sources above are candidates only; their specific data-use and distribution
terms, citations, accuracy, cross-identifications and quality decisions have
not been approved by this software. No values were copied from Swiss Ephemeris.

## First identity leads

The [All Skies Encyclopaedia's Leo page](https://ase.exopla.net/index.php/UR.GU.LA)
lists four names also present in the local backend: Regulus → HIP 49669,
Algieba → HIP 50583, Denebola → HIP 57632 and Zosma → HIP 54872. These are
**candidate cross-identifications**, not an approved catalog. In particular,
the page also distinguishes a component of Algieba, illustrating why a label
alone does not settle component or photometric identity. No magnitude or radial
velocity from that page is imported. The existing genuine-HIP2 strict-policy
test covers HIP 49669 only; it does not establish quality for the other three.
The remaining backend names have not yet been individually reviewed.

## Reproducible candidate and quality screening

The historical 2022-04-04 IAU-CSN snapshot obtained from a
[public mirror](https://github.com/mirandadam/iau-starnames/blob/master/catalog_data/IAU-CSN.txt)
contains 451 records and has SHA-256
`84fac0c90f1b19abc491c2793469e0caa7b003ad1bc93a790ca41147010d0eb0`.
The IAU WGSN [currently describes its catalog](https://exopla.net/iau-wgsn-catalogs/)
as a living document: this mirror is a **historical snapshot**, not evidence
of the current catalog. Its header asserts Creative Commons attribution for IAU
products, but the mirror's exact provenance, applicable license version,
component identities and distribution conditions still require review.

On the local backend's 106 distinct input names, an exact case-insensitive
match to the snapshot produced **77 provisional matches with HIP numbers** and
29 unmatched names. All 77 snapshot entries list band V; that label alone does
not establish suitable magnitude or identity, especially for blended or
variable objects. No fuzzy matching, spelling correction or cluster-to-star
guess was applied. `inspectHipparcosQuality` screened the 77 HIP2 records
using the Rust core's existing filters: **40 pass strict, 63 pass traditional**.
These are overlapping counts, not 103 accepted objects; 14 fail both filters.
Passing the traditional filter does not extend the owner's earlier decision
for 16 named stars to other objects. Matching strings and numerical eligibility
do not approve any record, alias, magnitude, radial velocity or data rights.

The caller's source snapshot, requested-name input and generated reports are
kept outside Git and build output under `VirgoStarsData/star-catalog-review/2026-11-16`;
the HIP2 file remains separate. Four copied files were checked by SHA-256.
The reports can contain application identifiers and derived third-party data;
their distribution has not been approved.

Reproduce using a caller-supplied UTF-8 IAU-CSN file and a JSON array of names.
Both commands are offline and refuse to overwrite their output. The first
reports names with no match and flags names sharing an HIP among the requested
set; it does not validate the catalog source or automatically resolve aliases.
The second calls the Rust HIP2 reader and checks input hashes before and after.
The file paths below are illustrative and must be replaced:

```powershell
node scripts/iau_star_candidates.mjs <iau-csn.txt> <requested-names.json> <https-source-url> <new-candidates.json>
node bindings/node/scripts/audit-star-quality.mjs <new-candidates.json> <hip2.dat> <new-quality.json>
```

The quality audit leaves ambiguous or missing-HIP entries for review; it
requires unique selected HIPs and fails rather than silently choosing between
duplicates. File hashes detect changed bytes but do not prove provenance or
provide an atomic snapshot. The candidate parser's fictional tests and the
real-file HIP2 binding check are software checks, not scientific validation.

## Caller-reviewed manifest

`prepareStarCatalog(manifest)` validates the *presence* and consistency of
explicit reviewer, astrometry source, data-use decision, quality policy, and
per-object identity source and quality decision. Finite magnitudes and radial
velocities each require their own source; a null requires an explicit missing
reason. Duplicate HIPs and case-insensitive names/aliases are rejected. It
returns `hips`, `qualityPolicy` and `stars` ready for the existing engine and
facade. **It does not authenticate any source, establish data rights, inspect
the HIP2 quality flags, or turn a review assertion into a scientific approval.**
The manifest remains the caller's record and is not included in output results.
The engine still validates the selected HIP2 records when loading.

Illustration only; the name, number, sources and values below are fictional:

```js
const { Engine, prepareStarCatalog, createSwissShapedFacade } = require('./index.js');
const manifest = {
  schema: 'virgostars-star-manifest/1',
  reviewer: 'Example reviewer', astrometrySource: 'Example astrometry',
  dataUseDecision: 'Fictional example; no redistribution decision',
  qualityPolicy: 'strict',
  stars: [{
    name: 'Example star', aliases: ['Example alias'], hip: 123,
    identitySource: 'Example cross-identification', qualityDecision: 'Example decision',
    magnitude: 1.2, magnitudeSource: 'Example V magnitude',
    radialVelocityKmS: null, radialVelocityMissingReason: 'No example measurement',
  }],
};
const catalog = prepareStarCatalog(manifest);
const engine = new Engine('/path/to/caller-owned/de441.eph');
engine.loadHipparcos('/path/to/caller-owned/hip2.dat', catalog.hips, catalog.qualityPolicy);
const facade = createSwissShapedFacade(engine, { starCatalog: catalog.stars, provenance: true });
```

Null RV is the explicit 0 km/s assumption and appears in provenance notes;
null magnitude makes magnitude lookup fail. `traditional` is an available
numeric quality filter, not authorization to extend the maintainer's earlier
decision for 16 traditional stars to all 106 inputs. Each additional star and
each blended/extended object requires a separate decision and cited evidence.

## Work still required

1. Resolve every requested object and component; record unsupported and
   ambiguous names explicitly. Verify aliases and HIP cross-identifications
   against independent sources, including objects with no single HIP entry.
2. Record V magnitudes, radial velocities, variability, source quality and
   uncertainty where available; review whether a zero-RV assumption is
   acceptable for each missing value and requested date range.
3. Review data-specific rights and provenance; decide storage, attribution,
   licensing and update policy. Only then assemble and test an actual manifest.
4. Run the full backend star workflow and independent numerical comparisons on
   representative reviewed data. A one-star fictional smoke test is not parity.
