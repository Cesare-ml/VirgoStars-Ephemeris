# Fixed-star catalog: independent Hipparcos 2007 import (experimental)

**Status (2026-11-15): importer, synthetic parser tests, 81 named HIP identity candidates, five extended objects and ten pending historical-name reviews only. NO vetted real star dataset is committed, no observed-star validation has passed, no Swiss parity and no production migration.**

## Scientific data boundary

- Astrometry: CDS **I/311/hip2**, F. van Leeuwen, *Hipparcos, the New Reduction of the Raw Data* (2007), epoch **ICRS J1991.25**. The published raw `hip2.dat` has **117,955 records**. The CDS fixed-width ReadMe is authoritative for the 1-based columns: https://cdsarc.cds.unistra.fr/viz-bin/ReadMe/I/311 . Values `RArad` and `DErad` are **radians**; `pmRA` is **mu_alpha*cos(delta)** in mas per Julian year. `Plx` is milliarcseconds.
- The bridge expects an epoch as a Julian Date in the TDB convention. J1991.25 = JD **2448349.0625** by the Julian-epoch definition; this does **not** convert observational UTC timestamps to TDB or prove any additional timescale accuracy.
- Hipparcos `Hp` photometry **is not** interchangeable with Johnson **V** magnitude; the importer uses only an explicit external V-band measurement. `hip2.dat` does not provide barycentric radial velocities, which must likewise be sourced independently. The importer does not assume RV=0, use Hp as V, or fabricate parallax.
- Names and aliases must be cross-identified by **HIP ID**, using independently checked IAU/SIMBAD/original-source associations; do not parse or copy Swiss `sefstars.txt`, Astrodienst's list, or proprietary Swiss material as source data.

## Reproducible local import

Obtain `hip2.dat` separately from the CDS I/311 archive. Review its licensing and redistribution terms before using or publishing transformed catalog data. The importer **does not download files** and commits **no third-party catalog**.

A first curated target list of **81 named-star / Hipparcos crossmatches** is in [docs/data/fixed-stars-target-crossmatches.json](data/fixed-stars-target-crossmatches.json). These are **identifications, not scientific position fixtures**: no usable V magnitude or barycentric radial velocity measurement is included there. A source citation for an object's identity does not certify its distance or velocity. This target list does not reproduce Astrodienst data.

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

Use fully specified arguments `--key=value`. The importer checks both digests, the exact 117,955-row Hipparcos file size in records, 276 printable ASCII characters in **every** record, global HIP uniqueness, fixed-column angular units, positive parallax and unique star names/aliases. The shared parser preserves trailing fixed-width spaces rather than calling `trimEnd()` on the entire file. It rejects missing source measurements instead of guessing. It reads the Hipparcos `Sn` solution type (code `10*d+s`) and number of components (`Nc`). By default only a standard 5-parameter single-star solution (`Sn=5, Nc=1`) is accepted. Complex solutions including binaries, variables, photocenter measurements and 7/9-parameter solutions require separate scientific review. The explicit `--allow-complex-solutions=true` override permits an experimental import but records `EXPERIMENTAL_COMPLEX_SOLUTION_NOT_CERTIFIED` in the audit; this is **not** a validation of a linear motion model. The outputs are written with `wx` to avoid silently replacing an earlier audit. It creates the JSON object `{id,records}` accepted by the experimental `loadExperimentalStarCatalogFile` PR #429, and a separate audit manifest with per-star supplemental source URLs and both source hashes. The audit is **not automatically attached to each `swe_fixstar_ut` response**.

Run `node --test test/import-hipparcos-2007.test.mjs` and `node --test test/fixed-star-crossmatches.test.mjs` for synthetic parser and crossmatch-manifest regression; neither test validates observed stellar positions. Once the #429 loader is integrated, use it with the JSON, pinned output SHA-256, explicit source and verified rights labels. Keep the generated raw/derived files out of Git until data-rights approval. No runtime network calls, new third-party library, or changes to astronomical Rust algorithms are introduced here.

## Formal-uncertainty export gate (2026-11-15)

The CDS I/311 official [Hipparcos 2007 ReadMe](https://vizier.cfa.harvard.edu/ftp/cats/I/311/ReadMe) was reviewed against the importer. Its fixed-width columns `HIP` (1–6), `Sn` (8–10), `Nc` (14), `RArad` (16–28), `DErad` (30–42), `Plx` (44–50), `pmRA` (52–59), `pmDE` (61–68), and the five *formal astrometric uncertainties* (70–103) match the currently implemented byte offsets. The actual original 117,955-row gzip cannot currently be downloaded from this agent's environment due to outbound network/DNS restrictions; verification of a real file's individual values **has not been performed**.

**New importer safety rule:** An accepted stellar row must have all five published formal uncertainties (`e_RArad`, `e_DErad`, `e_Plx`, `e_pmRA`, `e_pmDE`); `e_Plx` must be strictly positive. In addition to a positive parallax, the signal-to-noise ratio `Plx/e_Plx` must be at least **5 by default**. This `5` threshold is a VirgoStars selection policy, not an official Hipparcos catalogue rule and **not a scientific certification**. Do not infer missing errors as zero or claim that high SNR excludes orbital motion, astrometric systematic errors, spurious associations or variability.

For experiments only, the operator can supply `--min-parallax-snr=NUMBER` to the importer, with a finite positive value at most 1000. The separate audit records that threshold, each star's actual five formal errors and its parallax SNR. Reducing the threshold may admit more sources, but it is **not a substitute for independent numerical comparison**. The observational audit uses the same shared CDS uncertainty parser and now marks stars with missing RA, Dec or proper-motion errors `OTHER_FORMAL_UNCERTAINTIES_UNAVAILABLE`.

Tests: `test/import-hipparcos-2007.test.mjs` (missing/zero/low-SNR rejection), `test/audit-hipparcos-2007.test.mjs` (quality flags), and `test/hipparcos-catalog-loader-integration.test.mjs` (quality-checked synthetic import → loader). All are synthetic and network-free; genuine-source execution is still a separate release gate.

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

The quality check classifies each of the 81 HIP targets as `ASTROMETRY_PRECHECK_PASSED`, `LOW_PARALLAX_SIGNAL_TO_NOISE`, `NONPOSITIVE_PARALLAX`, `PARALLAX_UNCERTAINTY_UNAVAILABLE`, `COMPLEX_ASTROMETRY_REVIEW`, or `HIP_NOT_FOUND`. The default threshold is parallax divided by its formal error >= 5, and the standard single-star solution requires `Sn=5, Nc=1`. **A precheck pass is not scientific certification or permission to distribute the data.** The audit always records `catalogReady:false` until independent photometry, RV, rights and epoch-based positional comparisons are reviewed.

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


## One-command explicit acquisition (optional; never during runtime)

For reproducible work without a manual gzip extraction step, `scripts/fetch-hipparcos-2007.mjs` is a separately invoked **operator-only** downloader. It requests exclusively the fixed Harvard CDS mirror URL using HTTPS, rejects redirects and unexpected content, bounds compressed/decompressed sizes, validates **all 117,955 records and HIP identifiers**, computes both SHA-256 hashes and writes an uncompressed `hip2.dat` plus `hip2.dat.source.json` provenance outside Git. It never silently overwrites existing files and is **not invoked** when the astronomical package is imported or called.

Example on Windows PowerShell, from the repository root:

```powershell
$dataDir = Join-Path $HOME 'VirgoStarsData'
New-Item -ItemType Directory -Force -Path $dataDir | Out-Null
node scripts/fetch-hipparcos-2007.mjs "--out=$(Join-Path $dataDir 'hip2.dat')"
```

If the HTTPS mirror is blocked but you already obtained the archive through another approved channel, use the **fully offline** mode; no network request is attempted:

```powershell
node scripts/fetch-hipparcos-2007.mjs "--archive=$(Join-Path $dataDir 'hip2.dat.gz')" "--out=$(Join-Path $dataDir 'hip2.dat')"
```

The local archive must be an absolute path and must contain the authentic, uncompressed-equivalent 117,955×276-character table. The generated `.source.json` labels `acquisitionMode: OPERATOR_SUPPLIED_LOCAL_ARCHIVE` and leaves `sourceUrl` null: it deliberately does **not** assert that arbitrary local bytes came from the CDS mirror. An independently verified expected archive SHA-256 may be added as `--archive-sha256=<digest>`. The command rejects an existing output; place source and output in a dedicated user data folder, **outside Git**.

This requires normal HTTPS access from the machine executing the command only when no `--archive` argument is supplied. Source archive SHA-256 may optionally be independently pinned via `--archive-sha256=<64-lowercase-hex>`, obtained from a **trusted external reference**; it is **not** known/pinned by this repository. Without it, the local hashes record byte identity, **not verified publisher authenticity**. Never paste an invented digest. Do not assume possession of the scientific data grants commercial redistribution rights.

The downloaded `hip2.dat` can be passed to `scripts/audit-hipparcos-2007.mjs` with its `rawSha256` from the manifest. The complete, genuine CDS integration check remains opt-in and excluded from network-free CI. The importer and auditor now share `scripts/hipparcos-source-rows.mjs` for exact fixed-width integrity checks.


## Backend fixed-star coverage audit (read-only, no app runtime dependency)

The **backend's actual usage**, not the Astrodienst wiki or the original Swiss data file, sets the MVP coverage target. A read-only review of `Cesare-ml/virgo-stars` at [`backend/helpers/stars.helper.js`](https://github.com/Cesare-ml/virgo-stars/blob/1248260317ddf78546c458fc8fa80152897f541b/backend/helpers/stars.helper.js) (blob SHA `e14f9bf4258297e786311ab8f7e2f5e5a379dda3`) observed:

- **106** names declared in `STARS_ASTRODIENST`, of which **96** have IDs in `STARS_ASTRODIENST_TO_COW_ID` and are considered by current `getStelleTema()`.
- **10** legacy list names have no mapping and are not currently used by that mapped-star path: Ahadi, Birdun, Girtab, Kakkab, Koo She, Manus, Muhlifain, Peregrini, Regor and Wei.
- The experimental curated HIP target list currently identifies **81 of those 96** mapped names now have preliminary name/HIP candidates; five other labels represent extended objects and ten remain explicitly unresolved historical aliases. Every candidate still requires source-reviewed measurements and scientific validation.
- **Zero** stars are independently certified for production calculation by these inventories. A name/ID match is not a scientific numerical validation.

The backend `getStella()` call site also uses `swe_fixstar_ut`, `swe_fixstar_mag`, `swe_azalt`, mutates the returned `latlng` object and applies an azimuth orientation/ southern-hemisphere longitude transformation. The app-specific behavior belongs to integration compatibility tests, not the standalone astronomical library.

`scripts/audit-fixed-star-coverage.mjs` can reproduce the mapping audit from a **locally supplied backend helper file** and this repo's target manifest. It parses only the simple literal list and ID mapping (no `require`, `eval`, database access, or app data copying into the ephemeris runtime), and fails if the helper syntax changes. Example, using absolute paths to already checked-out repositories:

```powershell
node scripts/audit-fixed-star-coverage.mjs "--backend-helper=C:\\repos\\virgo-stars\\backend\\helpers\\stars.helper.js" "--candidates=C:\\repos\\VirgoStars-Ephemeris\\docs\\data\\fixed-stars-target-crossmatches.json"
```

The result reports mapped/unmapped names, identified candidates and remaining names, **not** the positions or release readiness of stars. The comparison script is an operator-only audit and does not introduce a runtime dependency on VirgoStars or Astrolabio. **Do not copy** the older `database/effemeridi/astrodienst/sefstars.txt` into this source or importer.

### Optional two-repository regression

With both repositories cloned locally, check that the inventory and the curated names remain aligned:

```powershell
$env:VIRGOSTARS_BACKEND_STARS_HELPER = "C:\\repos\\virgo-stars\\backend\\helpers\\stars.helper.js"
node --test test/audit-fixed-star-coverage.test.mjs
```

The test requires the current known source contract: 106 listed names, 96 mapped backend objects, 81 preliminary named-star HIP candidates, five extended-object labels, ten identity reviews and zero untriaged names. It intentionally skips in isolated CI when the backend path is absent. These are naming/cross-identification counts, **not** verified positions, brightness measurements, production readiness or a substitute for `swe_fixstar_ut` numerical parity.

## Previous checkpoint: 59 HIP identities and five extended objects (2026-11-15)

An additional 29 **backend-active** star names have been cross-identified to HIP IDs using the IAU Catalog of Star Names (including the [IAU historical catalog](https://iauarchive.eso.org/public/themes/naming_stars/)) and specific SIMBAD lookups. Each entry retains its HIP number and a SIMBAD query link, while its source type explicitly indicates that no observed Hipparcos astrometry has been imported or certified. The 59 entries are **identity candidates, not certified positions**.

Four legacy VirgoStars spellings need explicit name-alias review before integration: `Adara` → official `Adhara` (HIP 33579), `Agena` → `Hadar` (HIP 68702), `Etamin` → `Eltanin` (HIP 87833) and `Hasseleh` → `Hassaleh` (HIP 23015). These mappings must not silently change public display names or duplicate lookup aliases. A large number of selected targets are visual, spectroscopic or eclipsing binaries, or variable/supergiant stars requiring additional scientific treatment.

**Five additional active backend labels are not individual Hipparcos stars:** `Aculeus` → M6 / NGC 6405 (open cluster), `Acumen` → M7 / NGC 6475 (open cluster), `Facies` → M22 / NGC 6656 (globular cluster), `Praesepe` → M44 / NGC 2632 (open cluster), and `Spiculum` → a historically associated *nebular region* near M8 / NGC 6523 (not an unambiguous unique astronomical point). Their source-backed classifications live in [`docs/data/extended-fixed-star-targets.json`](data/extended-fixed-star-targets.json). NASA's [M7](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-7/), [M22](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-22/), [M44](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-44/) and [M8](https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-8/) entries independently corroborate those object classes.

**Do not generate a fake HIP ID or six-dimensional stellar state for any of the five.** Extended objects require a distinct catalog, explicit adopted center and epoch, and an application-level policy for point-vs-extended geometry and angular extent. Even a measured cluster proper motion is not an individual stellar proper motion.

The **previous checkpoint** classified 59 preliminary HIP-based identities, five extended objects and 32 outstanding labels. The subsequent 81/5/10 triage below supersedes these counts. Scientific acceptance remains **0**. The first two manifests are inputs to the read-only naming audit:

```powershell
node scripts/audit-fixed-star-coverage.mjs "--backend-helper=C:\\repos\\virgo-stars\\backend\\helpers\\stars.helper.js" "--candidates=C:\\repos\\VirgoStars-Ephemeris\\docs\\data\\fixed-stars-target-crossmatches.json" "--extended=C:\\repos\\VirgoStars-Ephemeris\\docs\\data\\extended-fixed-star-targets.json"
```

Catalog data remains caller-supplied and separately rights-reviewed; the PR does not import third-party datasets into the shipping engine.

## Latest triage: 81 stellar HIP identities, five extended objects and ten unresolved historical labels

A further 22 **exact-string** IAU name-to-HIP candidate pairs, all corresponding to mapped names actually used by VirgoStars, were added to [fixed-stars-target-crossmatches.json](data/fixed-stars-target-crossmatches.json). For these 22, a pinned source snapshot is recorded on every entry: [the IAU-CSN derivative CSV](https://github.com/cyschneck/iau-star-names/blob/main/data/1_iau_stars.csv), Git blob `c3d9289db7dbf585bf5fcb51980c008cb4f3d38d`. Cross-reference with the [IAU Catalog of Star Names](https://exopla.net/star-names/modern-iau-star-names/) and SIMBAD before promoting any identity to science-ready. The CSV is an external compiled naming reference; its fields do **not** provide approved 6D Hipparcos inputs and must not be treated as an authoritative star-position table. Example checks: Nashira is **HIP 106985**, Tania Australis **HIP 50801** (not Tania Borealis, HIP 50372), and Aljanah/Epsilon Cygni **HIP 102488** differs from Gienah/Gamma Corvi **HIP 59803**.

The remaining ten backend names are explicitly recorded in [fixed-star-identity-review.json](data/fixed-star-identity-review.json): `Alsuhail`, `Foramen`, `Gienah cygni`, `Gienah Ghurab`, `Han`, `Metallach`, `Ras Alhague`, `Rigil`, `Shedir` and `Tureis`. Each review row contains source URLs and **non-binding candidate alternatives** where supported. No reviewer has approved a unique HIP alias; the values in the review queue are not imported or passed to any numerical engine. In particular:

- `Tureis` can mean IAU Tureis/Rho Puppis (HIP 39757) **or** an older Iota Carinae/Aspidiske association (HIP 45556). These are different stars, so a name-only automatic mapping would be unsafe.
- `Rigil` can refer to the Alpha Centauri system or its A component (Rigil Kentaurus, HIP 71683), whereas the B component is Toliman (HIP 71681). Binary propagation requires component-specific treatment.
- `Gienah cygni` is Epsilon Cygni/Aljanah and `Gienah Ghurab` is Gamma Corvi/Gienah. The formal [IAU WGSN report](https://www.iau.org/static/science/scientific_bodies/working_groups/280/wg-starnames-triennial-report-2015-2018.pdf) explicitly records the historical duplicated name.
- `Foramen` historically denotes Eta Carinae, an interacting, strongly variable system. Do **not** invent a Hipparcos parallax or pretend a normal 5-parameter source solution exists for it.

This yields **96/96 backend labels classified or explicitly triaged**: 81 named individual stellar HIP candidates, five nonstellar/extended targets, ten unresolved historical-name cases. That does **not** imply 96 valid astronomical coordinates: 10 labels remain unresolved and **0** sources have independently validated numeric parity, photometric/RV provenance and usage rights.

Run the read-only local backend coverage audit with all three manifests:

```powershell
node scripts/audit-fixed-star-coverage.mjs "--backend-helper=C:\\repos\\virgo-stars\\backend\\helpers\\stars.helper.js" "--candidates=C:\\repos\\VirgoStars-Ephemeris\\docs\\data\\fixed-stars-target-crossmatches.json" "--extended=C:\\repos\\VirgoStars-Ephemeris\\docs\\data\\extended-fixed-star-targets.json" "--review=C:\\repos\\VirgoStars-Ephemeris\\docs\\data\\fixed-star-identity-review.json"
```

The audit reports `candidatesIdentified=81`, `nonstellarIdentified=5`, `pendingIdentityReview=10`, `unclassifiedCount=0` against the reviewed VirgoStars helper version. Its `missingCandidates=10` count remains **nonzero by design**: pending review is never counted as a successfully resolved HIP star. The opt-in test requires `VIRGOSTARS_BACKEND_STARS_HELPER` pointing to the local helper. Normal CI tests only synthetic files/manifests, not the authentic Hipparcos catalog.

Release gates unchanged: authentic I/311 input, external V-band photometry, separately documented barycentric RV (or independently validated proper fallback), data rights, epoch-by-epoch astrometric reference checks and backend behavior parity.

## Reproducible IAU name-to-HIP audit (does not download or calculate positions)

`scripts/audit-iau-name-hip.mjs` can compare the 81 proposed backend name/HIP pairs with an external locally supplied IAU naming CSV, without importing the CSV into this repository. It uses the **explicit historical alias → official IAU name** field for the five legacy names: El Nath/Elnath, Adara/Adhara, Agena/Hadar, Etamin/Eltanin, Hasseleh/Hassaleh.

A read-only comparison on 2026-11-15 between the candidate JSON and [this IAU-CSN-derived snapshot](https://github.com/cyschneck/iau-star-names/blob/main/data/1_iau_stars.csv), Git blob SHA `c3d9289db7dbf585bf5fcb51980c008cb4f3d38d`, found **76 exact names with matching HIP IDs plus five documented historical aliases pointing to matching IAU/HIP IDs, and zero mismatches**. This comparison is an **identity mapping check only**. The CSV is a third-party derived index: confirm with the [IAU catalog](https://exopla.net/star-names/modern-iau-star-names/) and object records in SIMBAD, especially for multiple components, before scientific release.

An opt-in test can reproduce this check with the verified local version:

```powershell
$env:VIRGOSTARS_IAU_NAMES_CSV = (Resolve-Path "C:\\VirgoStarsData\\1_iau_stars.csv").Path
node --test test/audit-iau-name-hip.test.mjs
```

Or call the CLI directly:

```powershell
node scripts/audit-iau-name-hip.mjs "--iau-csv=C:\\VirgoStarsData\\1_iau_stars.csv" "--stars=C:\\repos\\VirgoStars-Ephemeris\\docs\\data\\fixed-stars-target-crossmatches.json"
```

The script checks the Git blob SHA-1 of the external source file by default, fails closed on mismatched IDs and deliberately reports `scientificallyValidated=0`. The named CSV is NOT Hipparcos I/311/hip2 and is not suitable for numeric position propagation or substitute photometry. If the source snapshot is updated, review the new catalog, explicitly revise the expected digest and compare changes before trusting it. No network access occurs in ordinary tests or runtime.
