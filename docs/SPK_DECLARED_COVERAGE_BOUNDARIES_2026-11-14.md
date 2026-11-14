# Declared SPK target coverage boundaries

Modified 2026-11-14. Experimental canonical Rust core: `SpkKernelSet`.

## Purpose

When a target is declared in several loaded DAF/SPK files, a caller may
need to inspect every place where the declared coverage begins or ends
before choosing numerical verification epochs. The additive
`SpkKernelSet::declared_coverage_boundaries_for_target(target_id)` method
returns that finite set of candidate boundaries without evaluating
astronomical states or silently selecting a lower-priority kernel.

## API contract

- **Input:** signed NAIF target ID (`i32`), with no implicit target alias.
- **Output:** sorted, deduplicated `Vec<f64>` of **TDB/ET seconds past J2000**.
  These are **not** JD values and **not** UTC/TT/UT1 instants.
- Endpoints are gathered from *every loaded kernel*, including overlapping
  descriptors, unselected older kernels, and unsupported NAIF frames/types.
  Both start and end are kept; `-0.0` and `+0.0` normalize to `+0.0`.
- No matching target or an empty kernel set produces an empty vector.
  `load` / `unload_last` update the result automatically.
- This is a snapshot of previously inspected DAF metadata, with no
  interpolation, disk I/O beyond prior loading, external data retrieval,
  corrections, or new dependencies.

Example:

```rust
let boundaries = kernels.declared_coverage_boundaries_for_target(301);
for et in boundaries {
    // ET is TDB seconds past J2000, not a date string.
    let winner = kernels.selected_segment(301, et)?;
    println!("at ET={et}, winner={winner:?}");
}
```

**Interpretation:** boundaries are *candidates*, not a proof of continuous
coverage, valid SPK coefficients or physically correct positions. For
example, a later type-9 descriptor can mask an older type-2 record and
cause `direct_state` to fail explicitly, even though the target has a
declared interval. Both interval endpoints are inclusive in segment
selection; test the actual ET and both representable sides if checking
priority transitions. Distinct boundaries may represent the same winner.
Do not infer a winning segment solely from this list.

## Verification and provenance

Synthetic Rust regression exercises mixed IEEE endianness, overlapping
segments, duplicate endpoints, signed-zero normalization, unsupported
overlays, inclusive boundary priority, unrelated target isolation and
recovery after unload. No independent NAIF/JPL vector benchmark or
real-kernel precision result is claimed.

All kernels remain user supplied and unbundled. No changes to source
license (CPAL-1.0), NOTICE, third-party data rights or required
Powered by VirgoStars attribution.
