# Experimental cluster magnitudes and additional designations — 2026-11-17

**Local experimental work; the Swiss inventory and stellar workflow remain incomplete.**
These caller-supplied manifests and reports are ignored by Git and are not
activated in the main server. A covered lookup is not a validated object.

## Integrated brightness

The facade can carry optional caller-asserted `magnitudeInfo` alongside a
numeric magnitude: `kind`, `band`, `quality`, `aperture`, `source` and
`status`. Metadata is validated, copied and returned only when supplied;
existing results without it retain their previous shape. The status is
`CALLER_ASSERTED`: validation does not authenticate a source or establish an
error bound. Estimates describe integrated apparent light, not a member star.

| Lookup | Estimate | Source and qualifications |
| --- | --- | --- |
| `,M44` | 3.7 | [NASA](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-44/); band and aperture unspecified. |
| `,M6` | 4.2 | [SEDS](https://messier.lamost.org/seds/seds.org/messier/en/m/m006.html); approximate total visual brightness, aperture unspecified. |
| `,M7` | 3.3 | [NASA](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-7/); band and aperture unspecified. |
| `,M34` | 5.5 | [SEDS](https://messier.lamost.org/seds/seds.org/messier/en/m/m034.html); visual, aperture unspecified; Swiss gives 5.2 locally. |
| `,M22` | 5.1 | [NASA](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-22/); band and aperture unspecified; Swiss gives 6.17 locally. |

Each is explicitly approximate; unknown apertures prevent a direct precision
claim or homogeneous comparison. The Trapezium, NGC 869 and NGC 6530 have no
reviewed integrated estimate in this manifest. The compact radio source
Sgr A* has no asserted visual magnitude. The facade returns `E_STAR_MAGNITUDE`
for these four rather than inventing a value.

`scripts/update_reviewed_magnitudes.mjs` binds a caller decision to the input
manifest's SHA-256 and fills only previously missing magnitudes, with unique
source ID and name checks. The output and report are created without
overwriting existing files. The binding checks the input bytes, not the
scientific authority of a caller assertion.

## Additional local identities

The four cluster designations use the same approximate point-center approach
and caveats as [the previous checkpoint](STAR_CLUSTER_CENTERS_2026-11-17.md):
low-precision coordinates, mixed observational sources, constant linear motion,
approximate TDB epoch and no propagation of uncertainty or cluster extent.

| Lookup | Identity and comparison |
| --- | --- |
| `,M34` | [NGC 1039](https://simbad.u-strasbg.fr/simbad/sim-id?Ident=M34); latitude differs by up to 144 arcseconds from Swiss on the nine sample dates. |
| `,M22` | [Globular cluster M22](https://simbad.cds.unistra.fr/simbad/sim-id?Ident=M22); magnitude differs from Swiss. |
| `,NGC869` | [Open cluster NGC 869](https://simbad.u-strasbg.fr/simbad/sim-basic?Ident=NGC+869); latitude differs by up to 26 arcseconds on the sample dates. |
| `,NGC6530` | [Open cluster NGC 6530](https://simbad.harvard.edu/simbad/sim-id?Ident=NGC+6530); Swiss returns the name Spiculum. |

Each lookup has an explicit compatibility warning. The historical names
`Capulus`, `Facies` and `Spiculum` are not attached by this change: possible
object/component ambiguities require separate identity review.

`Galactic Center`, `Gal. Center` and `,SgrA*` identify the compact
[Sagittarius A* radio source](https://simbad.u-strasbg.fr/simbad/sim-id?Ident=SGR+A%2A),
not the whole Galactic center. Its J2000 position is combined with
[VERA/VLBA proper motion and direct parallax](https://arxiv.org/html/2401.02312)
of 0.117 ± 0.017 mas. The RA motion is interpreted as the tangent-plane
component; this convention, the mixed epochs and the zero radial-velocity
assumption need independent review. Interstellar propagation, acceleration,
uncertainties and radio-specific effects are not modeled. The three lookups
warn that sampled differences from Swiss reach 0.058 arcseconds per component;
this is not a precision bound or validation of the underlying model.

## Coverage and reproducibility

The local manifest has **1,080 objects and 706 aliases**. Of exact Swiss lookup
strings it covers **698/729 names** and **1,088/1,113 designations**: 31 and
25 respectively remain missing. Four magnitudes are unavailable and 1,067
entries assume zero radial velocity. The manifest contains five approximate
integrated magnitude estimates listed above. These counts do not establish
physical identity or completeness of output for every recognized lookup.

The same-DE441 diagnostic sampled nine dates from 1900 to 2099 across all
lookup strings: **16,074 attempts, 16,056 numerical comparisons, 18 unavailable
Swiss references**. There were **306 cases with a component above the
one-arcsecond diagnostic threshold**; all of those and the 18 unavailable
references had declared warnings, with **zero unexpected cases**. The
threshold is not an approved accuracy requirement. The two implementations
share DE441; Swiss is supplied TT and the engine approximate TDB. This is not
an independent observational accuracy validation.

SHA-256 of local ignored artifacts:

- `manifest-final-radio-20261117.json`: `aa579421e66c9ec0aa5ac542a60fbe1eaad69e99e777b68e9277f615582edaa2`.
- `coverage-final-radio-20261117.json`: `7581f52c847f877a6d758c46c3aee77c4504ea521a66eb02b4158d6fbb9f5ae6`.
- `stars-century-final-radio-20261117.json`: `1797abd0c757b7b2f92145cc389f87e0deb7c7d1e336e22dc69e4bc504034b70`.
- `explicit-sgra-radio-warnings-20261117.json`: `baf1a6a228109e124e61b89d5d698f2f77a9264099ff19fc87f5a6c15c8cd832`.
- `explicit-additional-clusters-20261117.json`: `8c1086a54575c15205b4cdc78fa5f5a68e33588e9640a6416a22440c7de68d7a`.
- `decisions-cluster-magnitudes-20261117.json`: `104a464810dc952a81eefa6388a474c2b52739c89b4bc234454f17c6909a5dee`.

Reproduce the magnitude update and candidate attachment with new output paths,
then rerun `scripts/verify_swiss_coverage.mjs` and
`bindings/node/scripts/audit-star-century.cjs` as described at the prior
checkpoint. Continue identity work, obtain suitable photometry, review
astrometric assumptions, and define observational error budgets before any
claim of catalog completion or production activation.
