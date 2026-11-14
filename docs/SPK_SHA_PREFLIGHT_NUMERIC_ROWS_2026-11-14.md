# SPK SHA-256 preflight: distinguish numeric rows from arbitrary CSV body text

**Date:** 2026-11-14  
**Status:** experimental validation-tool hardening; **not** NASA/JPL numerical validation.

## Failure mode

The offline `scripts/verify_spk_kernel_hashes.py` byte-identity preflight previously
set its nonempty-vector flag for **any** nonblank text after the expected CSV
header. As a result, a header followed only by a repeated header, a comment-like
free-text fragment or a nonfinite placeholder could pass the row-presence gate
if all declared kernel hashes matched. That green SHA result did **not** mean
the file had a valid geometric state; the Rust comparator would still reject
the malformed scientific reference.

## Change

A candidate row must now have **nine nonempty comma-separated fields**:
an ET scalar, two integer NAIF ID fields and six finite numeric vector
components. ET must also be finite. Text-only rows and nonfinite placeholders
no longer satisfy the minimum row-presence requirement. All identity checks
(ordered SHA-256, optional names and sizes, observable file swap detection)
stay unchanged.

This is intentionally **not** a replacement for the canonical Rust
`validate_spk_reference` example: only Rust validates the entire CSV,
required metadata, target/center semantics, time scale, ICRF/J2000 frame,
geometric corrections, load-order interpretation and residual tolerances.
In particular, a file containing one plausible numeric row and additional
invalid rows may still pass Python's limited SHA preflight; Rust must reject
the invalid reference. Passing both tools cannot authenticate the source
of the claimed NAIF/SPICE vectors.

## Reproduction

```bash
python -m unittest discover -s scripts -p 'test_verify_spk_kernel_hashes.py'
```

The test uses **synthetic bytes only** and covers repeated headers,
incomplete fields, noninteger IDs, nonfinite ET/velocities, valid rows
and existing ordered-file identity checks. No external kernel is bundled,
downloaded or executed.

## Scientific and distribution boundaries

Time-scale contract for eventual comparisons remains **TDB seconds past
J2000**, frame **J2000/ICRF**, target and observer **explicit NAIF IDs**,
corrections **NONE (geometric)**, units **km and km/s**. These values are
the reference format contract, **not values computed or verified by this
Python utility**. Real-kernel NAIF/SPICE comparisons, provenance review
and independently justified numerical tolerances remain outstanding.

Original VirgoStars source uses **CPAL-1.0** with the required VirgoStars
notices and attribution. No third-party source, kernels or dependencies
are added by this change.
