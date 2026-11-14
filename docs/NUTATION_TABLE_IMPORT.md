# Versioned nutation harmonic table import (experimental v1)

**2026-11-11 — original VirgoStars Rust parser.** This is a structural and
numerical input validation interface, **not** a source of official IAU 2000A
coefficient data, not a license-validation service, and not independent
astronomical validation.

## What has been verified and what remains blocked

The official [U.S. Naval Observatory Circular 179](
https://aa.usno.navy.mil/publications/Circular_179) describes 1,365 terms:
**678 lunar-solar** and **687 planetary**. The [IERS Conventions Centre,
chapter 5](https://iers-conventions.obspm.fr/chapter5.php) lists
standard models and supporting material.

These resources are publicly accessible, but **public access alone
does not establish a right to copy, relicense or redistribute full
coefficient tables within a CPAL-1.0 repository**. This development
increment incorporates **no** full/partial official IERS/USNO/ERFA/SOFA
coefficients or external program source. Before incorporating any real
data, identify the **specific file and version, original author/publisher,
documented permissions/terms, attribution/notice obligations, and source
digest**. Have any licensing compatibility questions reviewed.
Do not claim that a metadata line asserting `external_unverified`
constitutes approval.

## Exact UTF-8 format v1

The input is one in-memory UTF-8 string. One canonical table *family* per
input, with exactly **nine fixed metadata lines** followed by comma-separated
rows. No comments, padding, alternate header order, unknown header keys,
BOM, raw USNO ordering, or implicit unit conversions:

```text
VIRGOSTARS_NUTATION_TERMS_V1
family=lunisolar
arguments=l,lprime,F,D,Omega
amplitudes=radians
slopes=radians_per_tdb_century
source=synthetic
reference=synthetic:document-example
count=1
DATA
1,1,0,0,0,0,0.0001,0,0,0.0002,0,0
```

For the **planetary** family the two metadata fields instead are:

```text
family=planetary
arguments=l,lprime,F,D,Omega,Mercury,Venus,Earth,Mars,Jupiter,Saturn,Uranus,Neptune,pA
```

and each row contains **1 term index + 14 integer multipliers + 6
floating-point coefficients**. For lunar-solar there are
**1 index + 5 multipliers + 6 coefficients**. There is no optional
implicit count, hidden scalar factor, header normalization or auto-detect
of family. The canonical term indices must be **1,2,...,N in file order**.
The index is provenance/order metadata, not part of the trigonometric phase.

The six coefficient fields, in their fixed order, are:

| Index | Field in `NutationHarmonicTerm` | Unit |
|---|---|---|
| 1 | `longitude_sin_rad` | radians |
| 2 | `longitude_sin_slope_rad_per_century` | radians / TDB Julian century |
| 3 | `longitude_cos_rad` | radians |
| 4 | `obliquity_cos_rad` | radians |
| 5 | `obliquity_cos_slope_rad_per_century` | radians / TDB Julian century |
| 6 | `obliquity_sin_rad` | radians |

The parser requires already converted amplitude values; raw **arcseconds,
0.1-microarcsecond units or degrees must never be silently accepted**.
A deliberately wide absolute guard of **1 radian** per coefficient
rejects many unconverted inputs, but is only an engineering safety check,
**not proof** of a correct original-unit conversion.

### Explicit provenance metadata (not a license gate)

The mandatory `source` field is one of:

- `synthetic`, with `reference=synthetic:...`, used by all project tests.
- `external_unverified`, with `reference=https://...`. A URL records a
  caller's assertion, **not a verified document, hash, permission, license
  grant, or independent scientific result**.

Source strings are bounded to 300 printable ASCII bytes. The parse
result preserves the untrusted self-declared kind and reference.
**No input option means rights approved**, and no parser output is
authorized for bundling by that metadata. Full-family row count
`has_full_family_length()` is structural only: even 678+687 synthetic
zeros would pass row-count tests without representing nutation.

### Further physical and safety guards

- Both versions reject zero-count tables, counts over 678 or 687, row
  shortages/extras, non-sequential or repeated indices, malformed row widths,
  integers outside i8, nonnumeric/nonfinite amplitudes, out-of-range
  coefficients and any input over **256 KiB**.
- The **planetary** family explicitly rejects a nonzero l' (solar anomaly)
  multiplier in slot 2, as required by the historical MHB2000 planetary
  series; it also rejects both nonzero time-varying amplitude slopes.
- Every field and row must match exact versioned semantics; no automatic
  source-column reordering, graceful truncation or fallback.
- The parser is **in-memory only** and never opens files, follows URLs,
  downloads kernels, connects to an API or persists external data.
- The canonical numerical output is compatible with the separately
  proposed `nutation_harmonics::evaluate_nutation_harmonics()`, which
  still requires explicit TDB epoch, phase/rate vectors and validation.
  Parsing **never** evaluates astronomical results automatically.

## Crucial table-column transformation

The USNO Circular 179 Appendix E displays the **14** harmonic multiplier
columns in order

```text
Mercury, Venus, Earth, Mars, Jupiter, Saturn, Uranus, Neptune, pA,
l, lprime, F, D, Omega
```

whereas the VirgoStars harmonic engine takes

```text
l, lprime, F, D, Omega,
Mercury, Venus, Earth, Mars, Jupiter, Saturn, Uranus, Neptune, pA
```

A future **provenance-reviewed external importer** must explicitly
map source columns 10–14 → internal 1–5 and source 1–9 →
internal 6–14, and **must independently verify** the original source
table's 2000A convention and amplitude units (arcseconds and rates
rather than radians, with source-specific precision). The current parser
intentionally accepts **only** the canonical layout. Sending raw
USNO columns without an explicit mapping fails the metadata contract.

The IERS distribution is organized differently again. No blanket
importer for arbitrary IERS/SOFA/USNO text is currently provided.
Do not rely on the source of a coefficient record to imply a license.

## API and synthetic validation

```rust
use virgostars_ephemeris_core::nutation_table_import::parse_lunisolar_table;
use virgostars_ephemeris_core::nutation_harmonics::evaluate_nutation_harmonics;
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let fixture = "VIRGOSTARS_NUTATION_TERMS_V1\nfamily=lunisolar\narguments=l,lprime,F,D,Omega\namplitudes=radians\nslopes=radians_per_tdb_century\nsource=synthetic\nreference=synthetic:example\ncount=1\nDATA\n1,1,0,0,0,0,0.0001,0,0,0.0002,0,0\n";
let input = parse_lunisolar_table(fixture)?;
let epoch = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb)?;
let result = evaluate_nutation_harmonics(
    epoch, [0.0;5], [0.0;5], &input.terms,
)?;
assert_eq!(result.terms_evaluated, 1);
# Ok(())
# }
```

`parse_lunisolar_table(&str)` returns
`ImportedNutationTable<5>` and `parse_planetary_table(&str)` returns
`ImportedNutationTable<14>`, or a typed `TableParseError`
containing 1-based line and error kind. Neither API can claim an
independent true IAU 2000A nutation value from raw numbers.

The in-repository `tests/nutation_table_import.rs` contains **13 synthetic
checks** for canonical field mapping, an end-to-end one-term call to
the existing Rust harmonic evaluator, planetary input and source kind,
header ordering/units, self-declared provenance, skipped/duplicate term
indexes, declared counts, row widths/types, numeric limits, MHB solar/
secular restrictions, full-length synthetic sets, and oversized input.
Tests must also run under read-only CI on Linux, macOS and Windows.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test nutation_table_import
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

These tests demonstrate **parsing and synthetic arithmetic only**,
not certified numerical parity with ERFA `nut00a()`, IERS, JPL Horizons
or Swiss Ephemeris. Any use of a real table still needs provenance
verification, an independent pinned reference set (times, options,
corrections, numeric tolerances), and a consistent TT/TDB derivative
contract with precession and rotation modules in other draft PRs.

## CPAL-1.0 modification record — 2026-11-11

Original `src/nutation_table_import.rs`, `tests/nutation_table_import.rs`,
`docs/NUTATION_TABLE_IMPORT.md`, the `src/lib.rs` module export and
README link. Existing `nutation_harmonics.rs` remains unmodified.
No official coefficient table is included, no redistributable third-party
programs are copied, and no license/dependency/network/deploy/production
change occurs. Preserve CPAL-1.0 source notices and
**Powered by VirgoStars** (https://www.virgostars.it/).
