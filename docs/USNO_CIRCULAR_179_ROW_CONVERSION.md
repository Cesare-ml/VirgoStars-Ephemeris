# Source-specific USNO Circular 179 numerical row conversion

**2026-11-11 — Original Rust, source-defined column mapping. NOT a bundled
IAU 2000A coefficient table, source license grant, or completed nutation
model.** This addition extends the **still-draft** v1 canonical importer
proposed in PR #68, itself dependent on draft PR #54.

## Identified primary publication and licensing research

G. H. Kaplan, *The IAU Resolutions on Astronomical Reference Systems,
Time Scales, and Earth Rotation Models: Explanation and Implementation*,
U.S. Naval Observatory **Circular No. 179 (20 October 2005)**:
<https://aa.usno.navy.mil/publications/Circular_179>.

- Its **Appendix E, printed pages 88–103**, publishes all **1,365**
  terms of the IAU 2000A nutation series: **1–678 lunar-solar** and
  **679–1365 planetary**, with **14 multiplier columns followed by
  six angular coefficients**. The original term numbers are global.
- The official [USNO site privacy/use notice](
  https://aa.usno.navy.mil/about/PrivacyNotice) says information from
  the site is generally distributable or copyable **unless otherwise
  specified** and requests source attribution. The USNO publisher and
  US-government authorship may be relevant under
  [17 U.S.C. §105](https://uscode.house.gov/us/usc/119-118/t17/s105/).
  **Neither statement by itself resolves any foreign/joint-authorship
  rights, scientific database rights, or third-party contributions
  underlying the numerical series.**
- In particular, the [official IAU SOFA software license](
  https://www.iausofa.org/terms-and-conditions) has distinct conditions
  for copied/adapted SOFA source and derived work; it is **not**
  interchangeable with VirgoStars CPAL-1.0. We did **not** copy, adapt
  or translate SOFA/ERFA program source or coefficient arrays.
- **Rights decision: not cleared for full table bundling** pending
  identification of the precise data owners and applicable terms,
  including territory and attribution, then a documented compatibility
  decision. Publicly accessible material is not an automatic grant
  to relicense third-party datasets as VirgoStars original code.

The Rust module uses **the published scientific description of row
layout and angular units**, and one independent numeric reference row
in the tests. It includes **no bulk coefficient database**. Do not
infer redistributable status, scientific approval or legal clearance
from its API, test success or CPAL notices.

## Verified source-to-canonical mapping (no guessing)

Circular 179's printed multiplier columns **j=1 through j=14** are:

```text
Source:     Mercury Venus Earth Mars Jupiter Saturn Uranus Neptune pA  l l' F D Omega
Zero-based:    0      1     2     3     4      5      6      7    8  9 10 11 12 13
```

The already proposed VirgoStars harmonic kernel and v1 canonical
importer require the **opposite family grouping**:

```text
Canonical:  l l' F D Omega Mercury Venus Earth Mars Jupiter Saturn Uranus Neptune pA
Zero-based: 0  1 2 3     4      5     6     7    8       9     10     11      12 13
```

The converter implements the exact permutation:

```text
canonical[0..5]  = printed[9..14]
canonical[5..14] = printed[0..9]
```

It **does not** attempt to infer printed versus canonical ordering
from multiplier contents, download files or repackage raw USNO tables.

For **lunar-solar** rows, printed terms **1..678**, the first nine
planetary multipliers must be zero, and only printed `[9..14]`
becomes the canonical five-vector `[l,l',F,D,Omega]`.

For **planetary** rows, printed terms **679..1365**, the historical
MHB2000 planetary model has *no* solar-anomaly `l'` contribution
(source `j=11`/zero-based index 10), and **no secular coefficient
slopes**. The converter rejects nonzero values in those fields.
Its 14-multiplier output has the exact canonical order.

### Units, not automatic time conversion

The six printed coefficients follow the source table's order

```text
S, Sdot, Cprime, C, Cdot, Sprime
```

where the amplitudes are **arcseconds** and the slopes are
**arcseconds per Julian century**. The converter accepts them only
through an explicitly named `coefficients_arcseconds` input field.
It applies `radians = arcseconds × PI / (180 × 3600)` to each
field, including time-amplitude slopes. The six canonical Rust fields
in `NutationHarmonicTerm` are radians and radians per
**TDB Julian century**, consistent with the experimental TDB
harmonic evaluator's declared units. This conversion does **not**
silently transform TT↔TDB or validate the full IAU/SOFA time argument.

A deliberately wide |coefficient| ≤1 **radian** bound rejects some
unconverted values but is not a substitute for checking the source
units. The crate's existing harmonic sum remains responsible for
multiplying slope by centuries and computing derivatives per TDB day.

## API and one independently published numeric check

```rust
use virgostars_ephemeris_core::nutation_usno_adapter::{
    convert_usno_lunisolar_row, UsnoPrintedHarmonicRow
};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut source_multipliers = [0_i8; 14];
source_multipliers[13] = 1; // printed j=14, Omega
let source_row = UsnoPrintedHarmonicRow {
    printed_term_number: 1,
    multipliers: source_multipliers,
    coefficients_arcseconds: [
        -17.2064161, -0.0174666, 0.0033386,
        9.2052331, 0.0009086, 0.0015377
    ]
};
let term = convert_usno_lunisolar_row(source_row)?;
assert_eq!(term.multipliers, [0, 0, 0, 0, 1]);
# Ok(())
# }
```

The six scalar values and the `Omega` coefficient above are an
**independent published numerical fixture from Circular 179, Appendix E,
printed page 88, term 1**, not a copy of the complete published
series or of any external program. The tests independently check their
radian values and a one-term synthetic-phase result against the
existing harmonic engine. They **do not** establish full-series
Δψ/Δε accuracy, or true/mean-of-date frame parity.

### Regression coverage

`crates/ephemeris-core/tests/nutation_usno_adapter.rs` adds
**10** deterministic tests:

1. First independently published USNO row, all six angular fields
   and the printed lunar-node position.
2. All five lunisolar multiplier basis vectors.
3. All **13 permitted** planetary multiplier basis vectors (l'
   deliberately excluded).
4. Source global-index boundaries and family mismatches.
5. Reject any of the nine planetary multipliers in a lunar row.
6. Reject nonzero planetary solar anomaly or secular slopes.
7. Reject nonfinite coefficients and values above the wide radian guard.
8. Roundtrip USNO lunar-row conversion into **existing v1 importer**
   and the **existing harmonic summation** using a synthetic phase.
9. Roundtrip planetary row and verify nontrivial source permutation.
10. Explicit arcsecond and arcsecond-per-century conversion.

The tests validate this **small mathematical mapping**, with a
single externally sourced scalar fixture. They cannot validate all
678+687 terms, legal redistribution, time-scale normalization, true
planetary coordinates or agreement with the IAU/SOFA `Nut00a` API.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test nutation_usno_adapter
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## Data integration gate before the full 1,365 rows

1. Identify an authorized **specific** source version (publication,
   IERS dataset edition or equivalent) and independently verify who
   owns/permits reproduction of all 1,365 coefficients, including use
   internationally and any required acknowledgement.
2. Store source URL, exact version/date, explicit reuse authority,
   and SHA-256 digest of the independently acquired source **outside**
   this crate's original-source header/license; preserve attribution
   and avoid representing external coefficients as CPAL-original work.
3. Independently compare source-row mapping and coefficient scalings
   across *both* source representations and ensure no rows lost,
   duplicated or reordered.
4. Test independently pinned Δψ/Δε across multiple epochs against
   SOFA/ERFA numerical oracle results, then reconcile the full
   TT↔TDB argument and derivative conventions and integration with
   the still-draft IAU 2006 precession and nutation matrix.

## CPAL modification record — 2026-11-11

Added independently authored `src/nutation_usno_adapter.rs`,
`tests/nutation_usno_adapter.rs`, its module export, this document,
and README link to the ongoing draft importer PR #68. CPAL-1.0
source notices and mandatory **Powered by VirgoStars**
(https://www.virgostars.it/) retained. No third-party software code,
bulk coefficient tables, packages, NASA/JPL files, deployment or
license modification was introduced.


## Added complete-series sequencing gate — 2026-11-11

The v1 source-specific adapter now additionally accepts *slices of*
caller-supplied `UsnoPrintedHarmonicRow` records and enforces strict
original printed numbering **before** emitting any converted terms:

- `convert_usno_lunisolar_batch(rows, UsnoBatchMode)`: restricts
  source indices to 1..=678, a maximum of 678 rows.
- `convert_usno_planetary_batch(rows, UsnoBatchMode)`: restricts
  source indices to 679..=1365, a maximum of 687 rows.
- `UsnoBatchMode::ContiguousSubset` explicitly permits a **nonempty,
  consecutive** fragment in the original global numbering. This does
  not present a fragment as a complete model.
- `UsnoBatchMode::CompleteFamily` requires exactly **678 or 687 rows**,
  starting at original printed term 1 or 679 respectively, and
  proceeding with no skips, duplicates, reversals or inserted rows.
  No implicit sorting, truncating or fixing is performed.
- `convert_usno_complete_series(lunar, planetary)` requires **both**
  entire families and returns `StructurallyCompleteUsnoSeries`. The
  `UsnoFullSeriesError` identifies which family is invalid.
- All batch APIs **fail atomically**. A corrupt coefficient, wrong
  family, nonzero planetary solar-anomaly coefficient, missing row or
  repeated global term number returns a typed error identifying the
  zero-based input row position; no partially converted output is
  returned. Oversized/empty submissions are rejected before allocation.

There are **nine additional synthetic batch regressions** covering
single-family contiguous fragments, gaps/duplicates/reordering, incomplete
or shifted full families, **all 1,365 synthetic rows**, corrupted entries,
cross-family boundaries, empty/oversized sequences and correct failing
family. Taken together with the ten earlier scalar converter tests this
file now contains **19 tests**.

**This is an integrity-of-order check only, not cryptographic source
authenticity.** Any sequence of 1,365 synthetic zeros can pass these
structural checks; the caller must still audit **each coefficient against
a legally obtainable, exact-version source**, independently record
cryptographic content hashes and licensing permissions, and compare the
computed full Δψ/Δε with independent IAU reference values. No SHA-256
digest is invented or hardcoded before acquiring an authoritative file.
Neither a passed parser nor a passed test changes the TT-versus-TDB
model convention.

2026-11-11 CPAL modification record: updated the *original*
`src/nutation_usno_adapter.rs`, its
`tests/nutation_usno_adapter.rs` and this document to add the
non-destructive, fail-closed sequence validation. No original external
1,365-term table, third-party software or new dependency added.
