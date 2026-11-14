# Offline effective SPK source inventory at one TDB epoch

**2026-11-14 · Read-only Rust CLI · Synthetic structural tests only**

## Goal

The optional example `crates/ephemeris-core/examples/spk_effective_at.rs` exports a machine-readable CSV table for **every NAIF target declared across the caller-loaded SPK kernels**, including targets with **no segment coverage at the requested TDB epoch**.

It consumes the existing canonical `spk_effective_index::effective_segments_at`, `SpkKernelSet::declared_target_ids`, and the loaded kernel set's validated DAF metadata. This is not a new priority algorithm, SPK reader or orbital calculator. It is complementary to the metadata-only coverage union and boundary-probe examples.

## Command (offline)

```sh
cargo run --offline --locked --example spk_effective_at -- \
  --et 0 \
  /path/to/older-kernel.bsp \
  /path/to/newer-overlay.bsp
```

Input files must be **explicit** local SPK paths in declared load order. A later-loaded file has higher priority; the last covering descriptor in a file wins. For filenames beginning with a dash, put `--` before the filenames. No file discovery, default kernel, internet access, time conversion or data download is performed.

## CSV fields and meaning

```text
et_tdb_seconds,target_id,status,kernel_load_index,center_id,frame_id,spk_type,candidate_j2000_type23,start_et_tdb_seconds,end_et_tdb_seconds,initial_word,final_word
```

Exactly **one row per declared NAIF target**, sorted by signed integer ID. A target absent at the requested time has `status=missing`, with **nine empty source/descriptor fields**, not invented zero values. An actual winner has `status=covered`; the `kernel_load_index` is **zero-based** (0 is the oldest loaded file), matching the Rust metadata API. Timestamps are written in scientific notation using sufficient precision for an `f64` round trip.

The boolean `candidate_j2000_type23` means only that the **selected descriptor** declares NAIF frame **1 (J2000/ICRF)** and SPK **Type 2 or 3**. `true` does *not* prove that its coefficient arrays are readable, the record time grid is valid, the center chain resolves, the numerical state is physically accurate, or NASA/JPL provenance is authentic. A newer unsupported frame/type remains visible with `false` instead of being silently replaced by an older, otherwise compatible record.

The first and last DAF addresses are **inclusive, 1-based, 8-byte words**, reported directly from the selected descriptor; they are **not** offsets in a downloaded or trusted dataset.

## Precision, validation and fail-closed handling

- **Epoch:** `--et` is required and must be a **finite binary64 number of TDB seconds past J2000**; it is not a Julian date, a UTC timestamp or terrestrial time. No TT/UTC/TDB conversion is inferred. Negative ET is allowed.
- **Coverage:** declared segment start/end ET boundaries are inclusive; a one-ULP increase can change the selected segment. Targets without coverage still appear with `status=missing`.
- **Error handling:** all arguments and **every kernel's index** are validated and the full effective snapshot computed before any CSV header/row is emitted. Malformed flags, missing paths, invalid DAF records and NaN/infinite ET fail with an error instead of producing a plausible partial CSV. Ordinary downstream write errors may still interrupt output.
- **No coefficients:** the inventory is **metadata-only**; the CLI does not evaluate geometric km/km-per-second state vectors, apparent corrections, non-J2000 frames or independent NAIF/SPICE references.

## Deterministic regressions

Five offline Rust example tests use synthetic in-memory DAF/SPK buffers in **both endian formats**, no JPL files:

1. Includes both covered and missing targets in strict signed-ID order, preserves exact selected source, frame/type, word addresses and 12-column CSV shape.
2. Exercises inclusive overlay boundary, the next representable ET, a negative ET and restoration of older file priority after unload.
3. Preserves explicit all-missing rows rather than inventing coverage or zero-valued descriptors.
4. Rejects missing, duplicate and invalid `--et`, nonfinite epochs, implicit paths and unknown flags; handles dash-prefixed local filenames explicitly.
5. Checks that malformed ET and missing local files produce **no CSV output**, and that an empty in-memory set has an explicit header-only result.

These tests do not independently validate real JPL/NAIF numerical positions or velocities.

## Review gates and licensing

Expected read-only checks on the PR's exact head:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

Modification dated **2026-11-14**. Original source and attribution remain **CPAL-1.0** with Exhibit A, `NOTICE` and **Powered by VirgoStars** unchanged. No bundled kernel, third-party implementation, dependency, privileged workflow, production migration, package release or merge.
