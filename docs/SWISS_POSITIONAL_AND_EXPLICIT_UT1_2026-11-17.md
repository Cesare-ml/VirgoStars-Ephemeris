# Positional star candidates and explicit long-range UT1 — 2026-11-17

**Status: experimental, local and unapproved; production gate remains open.**
This advances [the previous checkpoint](LONG_RANGE_AND_SWISS_ALIASES_2026-11-17.md).

## Positional candidate catalog

`scripts/extend_star_manifest_positions.mjs` compares the locally supplied
Swiss file with the locally supplied IAU WGSN NEC snapshot, binding both and
HIP2 to the existing manifest hashes. It considers ICRS records only, requires
consistent duplicate rows per designation, a **single** NEC row within one
arcsecond and 0.1 magnitude, and a one-to-one HIP mapping. Other nearby NEC
rows are considered even when their HIP is already present, to avoid hiding
ambiguity. Only HIP2's existing **strict** quality policy is accepted.

Each result remains a positional **candidate**, with NEC magnitude and HIP2
astrometry; no Swiss position or proper motion is imported. Missing radial
velocities explicitly assume zero. A positional match, especially for a
multiple system, does not prove the identity or justify a release. The NEC
snapshot's file provenance remains unverified. The algorithm uses caller-
provided source bytes and does not bundle either external catalog.

The local files yielded 822 one-to-one matches; 520 passed strict HIP2 quality
and were appended to the 196-star candidate manifest. An explicit comparison
using the local `swisseph` package, the same DE441 file, and UT dates in 1950,
2000 and 2024 produced 1,560 position comparisons. With a predeclared diagnostic
limit of 1 arcsecond per longitude and latitude component, 1,559 passed; no
reference or engine errors occurred. The one exception was `,omePhe` (HIP 4829)
in 1950, longitude difference 1.3124 arcseconds. It is withheld for review.
The max latitude difference in the comparison was 0.3312 arcseconds.

`scripts/filter_positional_swiss_diagnostic.mjs` verifies the manifest digest,
source hashes, dates, comparison counts and outlier records before producing a
candidate manifest without the outlier. The resulting local manifest has
**715 candidate stars and 411 candidate aliases**. Case-insensitive exact
lookup covers 418 of the local Swiss snapshot's 729 distinct traditional
names and 708 of its 1,113 distinct comma-prefixed designations. These counts
are not Swiss lookup-parity, individual identity approval or complete catalog
coverage. Only the **519 newly retained** positional stars have this specific
three-date comparison; earlier entries have different evidence.

The initial 715-star manifest was then compared as a whole against Swiss at
the same three epochs. Of 2,145 comparisons, 2,120 were within the diagnostic
bound and 25 were outside it; all returned finite results. Nine distinct
candidate stars were excluded from a second, more conservative local manifest:
Algedi, Proxima Centauri, Fafnir, Ran, Ukdah, Barnard's star, Sadalbari,
Suhail and Heze. Some discrepancies are very large; they may reflect different
lookup names, components, data or other causes and must not be silently
assigned one explanation. They remain open for investigation, not deleted from
the source inventory. The `,omePhe` outlier was already withheld in the
previous filtering step.

The resulting **706-star, 404-alias candidate manifest** has all its entries
within one arcsecond of the Swiss reference on these three samples. It covers
408/729 exact local Swiss traditional names and 702/1113 comma-prefixed
designations by case-folded exact lookup. These are candidate and sampled
compatibility counts, not proof of stellar identity, all-date accuracy or
complete Swiss lookup behavior. Full diagnostic and prior manifests are kept
under `local-data` for review.

The experimental Virgo Stars bridge now resolves the legacy star list through
explicit catalog aliases. Its local 706-star manifest supports 48 legacy lookup
names representing 47 selected entries, and the existing conjunction flow was
tested with the explicitly supplied Chiron SPK. This does not switch production,
prove real PostgreSQL behavior or validate the entire legacy catalog.

```text
node scripts/extend_star_manifest_positions.mjs <manifest> <sefstars> <nec.csv> <hip2> <new-manifest> <new-report>
node bindings/node/scripts/audit-positional-swiss.cjs <manifest> <hip2> <de441> <sha256> <swisseph-package> <sefstars> <new-report> [positional|all]
node scripts/filter_positional_swiss_diagnostic.mjs <manifest> <diagnostic> <new-manifest>
```

The actual reports, source hashes, outlier and manifests are in ignored
`local-data/swiss-census-2026-11-17/`. Swiss comparisons use its own local
catalog and JPL flag; a flag and filename do not independently authenticate
its kernel or establish absolute scientific truth.

## Explicit UT1 and Delta T

`experimentalLongTermApparentUt1(jdUt1, body, deltaTSeconds,
uncertaintySeconds, source)` adds the supplied `TT−UT1` to UT1, approximates
TDB from TT using the existing leading periodic term and invokes the isolated
long-range apparent experiment. It returns the three epochs, the exact
caller-supplied Delta T, uncertainty and source label. Input must be finite;
uncertainty must be nonnegative and the source label nonempty. Output rates
remain per TDB day, and the uncertainty is **not** turned into an angular
error. The epoch and long-range apparent model both retain their own guards.

UT1 is not UTC or a local civil time. This method does not resolve historical
calendar conventions, time zones, leap seconds or the uncertainty of Earth
rotation. It does not silently choose a model and is not used by `calcUt`.

`experimentalEspenakMeeus2006DeltaT(decimalYear)` separately exposes the
uncorrected [NASA/Espenak–Meeus 2006 polynomial](https://eclipse.gsfc.nasa.gov/LEcat5/deltatpoly.html)
for astronomical decimal years 0 through 3000. Input below 0 or from 3001
onward fails. This is an opt-in historical model: its post-2005 values are
old projections, **not current Earth-orientation observations**, and it does
not supply uncertainty. NASA's eclipse-canon lunar secular-acceleration
correction is not applied. The caller must decide whether an estimate and its
uncertainty are suitable; for years 3001–5000 a caller must supply another
explicit Delta T value and uncertainty. Selected published historical table
points are checked within five seconds; this checks source consistency, not
physical truth or predictive accuracy.

Real DE441 Node tests exercise years 0, 1000, 2000, 3000, 4000 and 5000 with
caller-provided Delta T and verify equivalence to the explicit TDB path.
They also check input failures and agreement with the existing modern path
under the stated approximation UT1≈UTC. This verifies conversion and wiring,
not the factual Delta T at those dates or accuracy of resulting long-range
positions.

Remaining: complete Swiss names, designations, components and nonstellar
objects; investigate the withheld stars, review positional identities and source
provenance, validate apparent long-range positions/rates more broadly, choose
and validate historical/future Earth-rotation policies, implement civil time
and calendar semantics, and handle small bodies and houses. No production
activation until the owner's full-catalog condition is met.
