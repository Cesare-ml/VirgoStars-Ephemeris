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

## Independent astrometric-quality audit (new)

The optional `scripts/audit-hipparcos-2007.mjs` performs a *read-only analysis* of a **locally supplied genuine** CDS I/311 `hip2.dat` without requiring any stellar V magnitudes or radial velocities. It verifies the full **117,955 rows at 276 characters per row**, a caller-provided source SHA-256, unique HIP identifiers and the 2007 quality columns:

| Catalogue columns (1-based) | Units | Interpretation |
| --- | --- | --- |
| 70–75, 77–82 | milliarcseconds | Errors on RA and Dec |
| 84–89 | milliarcseconds | Parallax formal error |
| 91–96, 98–103 | mas/year | Proper-motion formal errors |
| 8–10, 14 | codes | Solution type `Sn` and component count `Nc` |

The quality check classifies each of the ten HIP targets as `ASTROMETRY_PRECHECK_PASSED`, `LOW_PARALLAX_SIGNAL_TO_NOISE`, `NONPOSITIVE_PARALLAX`, `PARALLAX_UNCERTAINTY_UNAVAILABLE`, `COMPLEX_ASTROMETRY_REVIEW`, or `HIP_NOT_FOUND`. The default threshold is parallax divided by its formal error >= 5, and the standard single-star solution requires `Sn=5, Nc=1`. **A precheck pass is not scientific certification or permission to distribute the data.** The audit always records `catalogReady:false` until independent photometry, RV, rights and epoch-based positional comparisons are reviewed.

### Genuine dataset source

The original CDS I/311 site is https://cdsarc.cds.unistra.fr/ftp/I/311/ . A mirror with the exact 2008-09-15 catalog release is publicly listed by Harvard at https://vizier.cfa.harvard.edu/ftp/cats/I/311/ (`hip2.dat.gz`, with a separate `ReadMe`). See the mirror `ReadMe` for corrected editions: the original June–September 2008 reduction files had errors. Verify the downloaded archive's source identity and current permissions before use. The importer itself does **not** fetch or publish data.

On Windows PowerShell, to prepare a local file (replace paths if needed):

```powershell
Invoke-WebRequest "https://vizier.cfa.harvard.edu/ftp/cats/I/311/hip2.dat.gz" -OutFile "hip2.dat.gz"
$source = [System.IO.File]::OpenRead((Resolve-Path "hip2.dat.gz"))
$gz = [System.IO.Compression.GZipStream]::new($source,[System.IO.Compression.CompressionMode]::Decompress)
$target = [System.IO.File]::Create((Join-Path (Get-Location) "hip2.dat"))
try { $gz.CopyTo($target) } finally { $target.Dispose(); $gz.Dispose(); $source.Dispose() }
$hash = (Get-FileHash -Algorithm SHA256 "hip2.dat").Hash.ToLowerInvariant()
node scripts/audit-hipparcos-2007.mjs "--hip2=$(Resolve-Path 'hip2.dat')" "--hip2-sha256=$hash" "--targets=$(Resolve-Path 'docs/data/fixed-stars-target-crossmatches.json')" "--out=$(Join-Path (Get-Location) 'hip2-audit.json')"
```

The computed hash pins the checked local input; it does **not** by itself authenticate the original publisher or confer redistribution rights. The audit writes an external JSON file and refuses to overwrite it. Keep the audit outside the source tree or under an ignored local path; do **not** commit external catalog measurements until licensing review.

For an **opt-in authentic-data regression** after manually sourcing the complete file:

```powershell
$env:VIRGOSTARS_HIP2_PATH = (Resolve-Path "hip2.dat").Path
$env:VIRGOSTARS_HIP2_SHA256 = $hash
node --test test/audit-hipparcos-2007.test.mjs
```

The genuine-data test is intentionally **skipped** in ordinary CI unless these two environment variables are present. Synthetic/structural tests are *not* a substitute for real-data tests or independent numerical validation.

