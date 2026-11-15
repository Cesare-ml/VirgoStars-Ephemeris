# Independent Hipparcos ICRS epoch comparison against ERFA (experimental)

**Status: benchmark tools only. No authentic Hipparcos full catalog has been imported here, no observed-star ERFA/Rust comparison has run, and no numerical scientific certification is claimed.**

## Coordinate reference and units

The existing VirgoStars experimental `swe_fixstar_ut` returns a geocentric, apparent **ecliptic** result. It must NOT be compared to the **SSB-centered ICRS astrometric** coordinates from SOFA/ERFA `pmsafe`. The dedicated script `scripts/compare-star-epoch-icrs.mjs` requires a separate barycentric ICRS stellar-motion output. The candidate exporter below uses the existing Rust classical geometric propagation; it does not relabel the ecliptic bridge result.

The independent reference uses the ERFA `pmsafe` algorithm (https://github.com/liberfa/erfa/blob/master/src/pmsafe.c). Input epochs are **TDB Julian Dates**. Input RA, Dec and proper motion are in radians and radians per Julian year respectively; parallax is in arcseconds and RV in km/s (+ for recession). By contrast, Hipparcos I/311/hip2 provides `pmRA = (dRA/dt) * cos(Dec)` in **mas per Julian year**. The reference generator explicitly divides by `cos(Dec)`, converts mas to radians and converts parallax mas to arcsec. It uses split two-part TDB epochs to limit rounding error.

The default three epochs are **J1991.25** (JD 2448349.0625), **J2000** (JD 2451545.0) and **J2050** (JD 2469807.5), so each test includes a genuine multi-decade motion baseline.

## Step 1: obtain real local Hipparcos data and a full audit

See [HIPPARCOS_FIXED_STAR_IMPORT.md](HIPPARCOS_FIXED_STAR_IMPORT.md). Use the authentic complete 117,955-row CDS I/311 data, trusted source hashes, reviewed Johnson V magnitudes, *barycentric* RV sources and a provenance audit. Keep all third-party data and generated outputs **outside Git**. The optional generator rejects complex Hipparcos solutions, missing formal astrometric errors, low parallax SNR, non-barycentric RV and mismatched audit/catalog identifiers.

Install and verify `pyerfa` in an operator-controlled Python environment (not automatically installed by the repository). This is the optional independent reference engine. Run with independently computed SHA-256 values:

```powershell
python scripts/generate-hipparcos-erfa-reference.py `
  --catalog="C:\VirgoStarsData\hipparcos-curated.json" `
  --catalog-sha256="<trusted catalog SHA-256>" `
  --audit="C:\VirgoStarsData\hipparcos-curated.audit.json" `
  --audit-sha256="<trusted audit SHA-256>" `
  --out="C:\VirgoStarsData\erfa-icrs-reference.json"
```

The report uses `referenceModel:"ERFA_PMSAFE"`, `coordinateFrame:"ICRS_ASTROMETRIC"`, `epochConvention:"TDB_JULIAN_DAY"`, and `scientificallyValidated:false`, with source file hashes and the ERFA library version. It aborts on ERFA warnings instead of silently accepting numerical overrides.

## Step 2: export separate VirgoStars ICRS astrometric positions

Build the experimental Rust example and run the candidate exporter with pinned input files and an output outside the repository:

```powershell
cargo build --example true_ecliptic_at --locked
node scripts/export-hipparcos-icrs-candidate.mjs `
  --catalog="C:\VirgoStarsData\hipparcos-curated.json" `
  --catalog-sha256="<trusted catalog SHA-256>" `
  --audit="C:\VirgoStarsData\hipparcos-curated.audit.json" `
  --audit-sha256="<trusted audit SHA-256>" `
  --executable="C:\VirgoStarsData\true_ecliptic_at.exe" `
  --out="C:\VirgoStarsData\virgostars-icrs-candidate.json"
```

The executable path must point to the actual compiled example. The exporter
checks the catalog and audit hashes and their identity mapping, but **does not
independently certify the audit's scientific or licensing assertions**. The
separate reference generator applies its own more restrictive science gates.
It accepts at most 1000 records, launches one Rust process per star/epoch, and
uses the three fixed epochs above. Output is exclusively created outside Git.

The candidate is a **classical geometric approximation** to the ICRS catalog
motion and deliberately omits stellar Roemer/light-time and relativity,
unlike ERFA `pmsafe`. Its `ICRS_ASTROMETRIC` frame label makes the directions
comparable in the comparator's common frame; it does not assert identical
physical models. Differences from ERFA are expected, can grow with stellar
motion or elapsed time, and must not be hidden. No geocentric parallax,
aberration, deflection, observed-star certification or Swiss parity follows.
The supplied catalog epoch is interpreted as TDB under the importer contract.

The candidate JSON contract includes:

- `schemaVersion:1`, `coordinateFrame:"ICRS_ASTROMETRIC"`, `epochConvention:"TDB_JULIAN_DAY"`.
- `catalogId`, `inputCatalogSha256` and `epochsTdbJulianDay` exactly matching the reference.
- `candidateModel:"VIRGOSTARS_CLASSICAL_BARYCENTRIC_ICRS_NO_LIGHT_TIME"`.
- `stars`, one per HIP identifier, each containing `name`, `hip`, and three `samples` with `tdbJulianDay`, `rightAscensionDeg`, and `declinationDeg`.

Input coordinates must be astrometric ICRS at the source epoch, with the same radial-velocity, proper-motion and TDB conventions. The comparator refuses mismatched frames, epochs, source hashes, duplicate HIP IDs or missing samples.

## Step 3: compare the two pinned datasets

```powershell
node scripts/compare-star-epoch-icrs.mjs `
  --reference="C:\VirgoStarsData\erfa-icrs-reference.json" `
  --reference-sha256="<reference SHA-256>" `
  --candidate="C:\VirgoStarsData\virgostars-icrs-candidate.json" `
  --candidate-sha256="<candidate SHA-256>" `
  --max-arcsec=0.1 `
  --out="C:\VirgoStarsData\icrs-parity-report.json"
```

The `0.1` arcsec limit is only an **illustrative explicit operator tolerance**, not a validated release criterion. The comparator measures great-circle angular separation (including RA wraparound), computes maximum and RMS residuals, and returns process exit code **2** if any sample exceeds the tolerance. Even a pass retains `scientificallyValidated:false`, as no complete end-to-end astrophysical correctness or rights assessment follows from this comparison alone.

## Offline synthetic regression

```bash
node --test test/compare-star-epoch-icrs.test.mjs
python -m unittest discover -s test -p 'test_hipparcos_erfa_reference.py'
```

The Node tests exercise angular wraparound, bad frames, missing epochs and file hashes. The Python tests inject a **mock** in place of ERFA to check the correct proper-motion/parallax unit conversion and source gates; they do not run authentic ERFA, and real observed-star numerical parity remains untested.

The candidate exporter also has a fictional-star regression against three
recorded pyerfa 2.0.1.5 `pmsafe` values. A local operator-only smoke test ran
the actual reference generator with installed pyerfa, the Rust candidate
exporter and this comparator at those three epochs; the synthetic example
passed an illustrative 0.01 arcsecond threshold. The test source and audit
are fabricated, and this single result is neither a general error bound nor
validation of any real star, rights, or the apparent/ecliptic pipeline.

### Remaining work

1. Acquire authentic full Hipparcos I/311 source and separately vet catalog rights, Johnson V, barycentric RV and HIP component identity.
2. Run genuine multi-epoch comparisons with independently vetted positions and acceptable tolerances, accounting for the different light-time models.
3. Subsequently validate the geocentric apparent, ecliptic, topocentric, magnitude and legacy Swiss API compatibility paths separately.
