# Fixed-star catalog: independent Hipparcos 2007 import (experimental)

**Status (2026-11-15): importer, synthetic parser tests and ten named HIP crossmatch targets only. NO vetted real star dataset is committed, no observed-star validation has passed, no Swiss parity and no production migration.**

## Scientific data boundary

- Astrometry: CDS **I/311/hip2**, F. van Leeuwen, *Hipparcos, the New Reduction of the Raw Data* (2007), epoch **ICRS J1991.25**. The published raw `hip2.dat` has **117,955 records**. The CDS fixed-width ReadMe is authoritative for the 1-based columns: https://cdsarc.cds.unistra.fr/viz-bin/ReadMe/I/311 . Values `RArad` and `DErad` are **radians**; `pmRA` is **mu_alpha*cos(delta)** in mas per Julian year. `Plx` is milliarcseconds.
- The bridge expects an epoch as a Julian Date in the TDB convention. J1991.25 = JD **2448349.0625** by the Julian-epoch definition; this does **not** convert observational UTC timestamps to TDB or prove any additional timescale accuracy.
- Hipparcos `Hp` photometry **is not** interchangeable with Johnson **V** magnitude; the importer uses only an explicit external V-band measurement. `hip2.dat` does not provide barycentric radial velocities, which must likewise be sourced independently. The importer does not assume RV=0, use Hp as V, or fabricate parallax.
- Names and aliases must be cross-identified by **HIP ID**, using independently checked IAU/SIMBAD/original-source associations; do not parse or copy Swiss `sefstars.txt`, Astrodienst's list, or proprietary Swiss material as source data.

## Reproducible local import

Obtain `hip2.dat` separately from the CDS I/311 archive. Review its licensing and redistribution terms before using or publishing transformed catalog data. The importer **does not download files** and commits **no third-party catalog**.

A first curated target list of **ten named-star / Hipparcos crossmatches** is in [docs/data/fixed-stars-target-crossmatches.json](data/fixed-stars-target-crossmatches.json). These are **identifications, not scientific position fixtures**: no usable V magnitude or barycentric radial velocity measurement is included there. A source citation for an object's identity does not certify its distance or velocity. This target list does not reproduce Astrodienst data.

Prepare a local UTF-8 **TSV** with this exact header (tabs between fields):

```text
HIP	name	aliases	visualMagnitude	visualMagnitudeSource	radialVelocityKmPerSecond	radialVelocitySource	radialVelocityReferenceFrame	nameSource
```

One line per HIP identifier. `aliases` are optional and use `|` between names; other fields are mandatory. The `radialVelocityReferenceFrame` must be exactly `BARYCENTRIC`. SIMBAD radial velocities may be shown in a **heliocentric** reference frame; do not silently copy them as barycentric without review of measurement and adopted reference frame.

Every source column must be a direct **HTTPS URL** to the source or crossmatch reference actually checked by the curator. A source URL is metadata, **not proof** of measurement quality or data-reuse rights. RV values must be **barycentric km/s** with the standard positive-receding convention, V magnitude is **Johnson V**, and names must correspond to that precise HIP source. Do not populate with made-up example measurements. Stars without dependable V, RV, or positive parallax are excluded from this strict MVP input and tracked separately for future handling.

Compute checksums of *exact raw bytes* using `sha256sum` (Linux/macOS) or `Get-FileHash -Algorithm SHA256` (PowerShell), then run:

```bash
node scripts/import-hipparcos-2007.mjs \
  --hip2=/abs/path/hip2.dat \
  --hip2-sha256=<64-lowercase-hex> \
  --supplements=/abs/path/stars-curated.tsv \
  --supplements-sha256=<64-lowercase-hex> \
  --out=/abs/path/hipparcos-curated.json \
  --audit=/abs/path/hipparcos-curated.audit.json
```

Use fully specified arguments `--key=value`. The importer checks both digests, the exact 117,955-row Hipparcos file size in records, fixed-column angular units, positive parallax and unique HIP/name/alias mappings. It rejects missing source measurements instead of guessing. It reads the Hipparcos `Sn` solution type (code `10*d+s`) and number of components (`Nc`). By default only a standard 5-parameter single-star solution (`Sn=5, Nc=1`) is accepted. Complex solutions including binaries, variables, photocenter measurements and 7/9-parameter solutions require separate scientific review. The explicit `--allow-complex-solutions=true` override permits an experimental import but records `EXPERIMENTAL_COMPLEX_SOLUTION_NOT_CERTIFIED` in the audit; this is **not** a validation of a linear motion model. The outputs are written with `wx` to avoid silently replacing an earlier audit. It creates the JSON object `{id,records}` accepted by the experimental `loadExperimentalStarCatalogFile` PR #429, and a separate audit manifest with per-star supplemental source URLs and both source hashes. The audit is **not automatically attached to each `swe_fixstar_ut` response**.

Run `node --test test/import-hipparcos-2007.test.mjs` and `node --test test/fixed-star-crossmatches.test.mjs` for synthetic parser and crossmatch-manifest regression; neither test validates observed stellar positions. Once the #429 loader is integrated, use it with the JSON, pinned output SHA-256, explicit source and verified rights labels. Keep the generated raw/derived files out of Git until data-rights approval. No runtime network calls, new third-party library, or changes to astronomical Rust algorithms are introduced here.

## Open scientific/release gates

1. Independently check real HIP cross-identification, V magnitude, radial velocity **and uncertainty** for representative stars such as Sirius, Regulus, Spica, Aldebaran, and Antares. Bright binaries and variable stars need special care: HIP component identity, variability, RV epoch/method, and proper-motion solution class matter.
2. Establish catalog data licensing/redistribution and required third-party attribution. Keep datasets outside CPAL-1.0 source licensing.
3. Compare observed-source results at multiple epochs against independent **ERFA/SOFA or NOVAS** references with clearly declared frames, time scales, correction choices, tolerances and documented failures. Synthetic fixtures only establish parsing/contract behavior.
4. Integrate reviewed PRs in dependency order, complete `swe_fixstar_ut`/`swe_fixstar_mag` contracts and the Node CommonJS bridge; then verify real backend call sites without switching production until scientific and regression gates pass.
5. Keep astrological attributions and interpretations in `astrological-functions`, not the astronomical ephemeris engine. The Astro.com list is a **coverage reference**, not a source of redistributable catalog bytes.
