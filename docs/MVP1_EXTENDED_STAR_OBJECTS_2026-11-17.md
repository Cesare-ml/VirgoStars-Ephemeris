# Extended-object leads in the fixed-star workflow — 2026-11-17

**Status: caller-asserted research, REVIEW_REQUIRED; no new astronomical
objects are implemented or approved.** The legacy requested-name list includes
names that may refer to star clusters, nebulae or groups, not individual HIP2
stars. The hypotheses below require review against the intended product
definition; a single HIP star must not silently replace an extended object.

## Research leads and unresolved meanings

| Requested name | Recorded hypothesis | Evidence and remaining uncertainty |
|---|---|---|
| Aculeus | M6 / NGC 6405, open cluster | One [historical astrological naming reference](https://www.astro.com/astrowiki/en/Aculeus) associates the name with M6; [NASA](https://apod.nasa.gov/apod/ap250719.html) and [SIMBAD](https://simbad.cds.unistra.fr/simbad/sim-id?Ident=M6) classify M6 as an open cluster. |
| Acumen | M7 / NGC 6475, open cluster | One [naming reference](https://www.astro.com/astrowiki/en/Acumen) associates the name with M7; [NASA](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-7/) describes M7 as an open cluster. |
| Facies | M22, globular cluster | One [naming reference](https://www.astro.com/astrowiki/en/Facies) associates Facies with M22; [NASA](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-22/) classifies M22 as a globular cluster. |
| Facies | NGC 6530, open cluster | A [historical discussion](https://www.astro.com/astrology/ivccn_article201127_e.htm?lang=e) describes another interpretation of the Archer's forehead as NGC 6530. It does **not** establish Facies as its direct alias; preserve the alternative for review. [NASA](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-8/) describes NGC 6530 within the Lagoon Nebula. |
| Spiculum | M8 / M20 / M21, object group | A [historical naming reference](https://www.astro.com/astrowiki/en/Sagittarius_%28Constellation%29) lists several objects under this label. [NASA](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-8/) identifies M8 as an emission nebula containing an open cluster. No centroid is selected. |
| Praesepe | M44, open cluster | [NASA](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-44/) identifies Praesepe with M44. This supplements the existing SIMBAD no-single-HIP lead. |

These sources describe naming and object types; they do not establish that a
particular backend field represents the same definition or grant redistribution
rights. Facies and Spiculum retain ambiguity explicitly. No third-party
coordinates, magnitudes or raw catalog rows were copied into this repository.

## One further stellar spelling lead

The [RASC historical-name handbook](https://www.rasc.ca/sites/default/files/World%20Asterisms%20Project%20Handbook%20V%202025.14.pdf)
associates Alsuhail with λ Velorum, standardized as Suhail, while also
describing historically related Suhail names for other stars. The local 2022
IAU snapshot supplies the **candidate** HIP 44816 with component unspecified;
the local HIP2 screen says strict and traditional eligible. This does not
settle the backend's intended identity or approve the quality policy.

## Separate, reproducible review artifacts

The previous seven stellar leads were retained and Alsuhail was added by
`attach_star_research_leads.mjs`, producing a new stellar queue. The new
`scripts/attach_star_object_leads.mjs` then appends caller-asserted extended
object hypotheses to a **separate** queue, retaining multiple alternatives,
source URLs and every null decision. It rejects HIP identifiers, coordinates
and magnitudes in these hypotheses, validates the base queue hash and refuses
to overwrite a file. Source URLs are not fetched or authenticated by the tool.

```powershell
node scripts/attach_star_research_leads.mjs <queue-v1.json> <stellar-leads-v2.json> `
  <iau-snapshot.txt> <hip2.dat> <new-stellar-queue.json>
node scripts/attach_star_object_leads.mjs <stellar-queue.json> <object-leads.json> <new-mixed-queue.json>
```

The local artifacts are outside Git under
`VirgoStarsData/star-catalog-review/2026-11-17`:

```text
research-leads-v2.json              2d0d5ce5113eede38461417797ec2073787074c4f47e8c423739e0c60f462aea
star-review-queue-stellar-v3.json   f85b22287046530657d23fac1499c42d0a5b309316a4bca31384643768f54584
object-leads-v1.json                045f8ce616b2e106ca5c439f187075575749c1e78ff4f5982b883aa78c7d1619
star-review-queue-mixed-v4.json     5f88c8a5027d1dfe51228dbb3f788b86e943b1d9e0346c703d9ca05eb309ebdd
```

The mixed queue records eight names with stellar research leads, five with
extended-object hypotheses, and **13 names without any machine-recorded lead**.
Praesepe was already among the four earlier SIMBAD exact leads, so the five
object names reduce the preceding 17 no-lead count by four, not five. Facies
has two hypotheses, and the group Spiculum is not collapsed to one member.
All decisions for all 106 names remain null. The queue is not a manifest and
the artifacts' redistribution terms are still unreviewed.

For an extended object, a future product decision must define the object,
reference coordinates and epoch, time/frame transformations, whether proper
motion is meaningful, and whether a reported magnitude is integrated,
component-specific or absent. Those calculations and scientific validation do
not exist merely because a research hypothesis is recorded here.
