# SPK target segment inventory (experimental)

Modified 2026-11-14. Scope: the canonical Rust `SpkKernelSet` in
`crates/ephemeris-core/src/spk_kernel_set.rs`.

## Purpose

`SpkKernelSet::selected_segment(target_id, et)` reports only the effective
descriptor at a particular finite TDB/ET epoch. The diagnostic method
`declared_segments_for_target(target_id)` instead returns **all** descriptors
for one NAIF target ID from already loaded kernels, including descriptors
that are out of coverage, overlapped by later descriptors, or unsupported by
the type-2/type-3 frame-1 state evaluator.

This allows a caller to explain why a particular kernel or segment won
without conflating metadata presence with a computable astronomical state.

## Contract

- Input: a NAIF target ID (`i32`); no timestamp or time-scale conversion.
- Output: `Vec<(usize, SpkSegmentDescriptor)>`; the index is zero-based
  kernel load order, oldest first.
- Within each kernel, results preserve the original DAF summary order;
  overlapping segments and duplicates are **not** collapsed.
- Descriptors carry declared start/end coverage in **TDB/ET seconds past
  J2000**, plus NAIF target/center/frame/type and 1-based DAF word addresses.
- Unknown target or empty kernel set returns an empty vector.
- The method performs no interpolation, file I/O, online lookup or state
  validation. It does not filter for coverage or supported types/frames.
- Use `selected_segment(target_id, et)` to find the effective descriptor
  at a particular TDB epoch; use `direct_state` / `relative_state` to
  attempt computation. Unsupported or corrupt winning segments must still
  fail closed, without falling back to older files.

Example (using an already loaded `SpkKernelSet`):

```rust
let inventory = kernels.declared_segments_for_target(399);
for (load_index, descriptor) in inventory {
    println!(
        "kernel #{load_index}: center={} frame={} type={} ET=[{}, {}]",
        descriptor.center_id,
        descriptor.frame_id,
        descriptor.spk_type,
        descriptor.start_tdb_seconds,
        descriptor.end_tdb_seconds
    );
}
```

## Regression and limitations

The inline Rust synthetic-kernel regression checks mixed endian loaders,
duplicates within an older kernel, an unsupported newer overlay, an
out-of-coverage winner change, and restoration of the earlier inventory
after unloading. This is a metadata/order contract only, not an
independent JPL/NAIF state-vector comparison.

No third-party ephemeris bytes are included or redistributed. The license
and required VirgoStars attribution remain unchanged (CPAL-1.0;
see `LICENSE` and `NOTICE`).
