# Inspect real local SPK segment coverage (Rust metadata only)

**2026-11-13 — experimental read-only diagnostic.** Depends on the unmerged Rust SPK indexing and ordered-kernel branch stack through draft [#164](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/164). **No real NASA/JPL kernel has been inspected in CI**. The tool displays segment descriptors declared by local kernel files; it does **not** validate numerical states or complete E1.

## Motivation and scope

E1 scientific issue [#3](https://github.com/Cesare-ml/VirgoStars-Ephemeris/issues/3) needs carefully selected multi-body, multi-epoch and boundary cases for independent NAIF/SPICE versus VirgoStars Rust comparisons. Selecting times without checking real kernel descriptors can produce invalid or unsupported samples.

The original Rust example `crates/ephemeris-core/examples/inspect_spk_coverage.rs` calls the existing **canonical Rust** `spk_index::inspect_spk` parser. It does not implement another DAF/SPK reader, duplicate astronomy in Python, import SpiceyPy, download any kernel or calculate ephemeris vectors. It can inspect any locally readable DAF/SPK file supported by the current Rust metadata index, even when a segment's frame or SPK type is not yet supported for state evaluation.

## Offline usage

Given a legally obtained, local SPK file, from the repository root:

```bash
cargo run --offline --locked --example inspect_spk_coverage -- /local/path/de440s.bsp
```

To limit the listing to NAIF body ID `399` (Earth center):

```bash
cargo run --offline --locked --example inspect_spk_coverage -- \
  --target 399 /local/path/de440s.bsp > earth-coverage.csv
```

Multiple file arguments are accepted in **exact load order** (later files and later covering segments may override earlier ones under NAIF priority). Do not reorder them when subsequently generating an independent reference. Output is to stdout as CSV, and a non-claiming summary goes to stderr. A missing/corrupt SPK file causes a nonzero exit before *any* CSV is emitted; a target filter matching zero descriptors also returns a nonzero exit.

## CSV contract

```text
load_index,segment_index,target_id,center_id,frame_id,spk_type,coverage_start_et_tdb_seconds,coverage_end_et_tdb_seconds,candidate_j2000_type2_or3
```

All indices are **1-based**. `load_index` references the original CLI kernel path order. `segment_index` preserves a kernel's original DAF summary order; this matters for segment selection precedence. Segment start and end times are reported in **TDB/ET seconds past J2000**, using Rust's `f64` scientific representation. Frame ID `1` denotes NAIF J2000. `candidate_j2000_type2_or3=true` means only that the descriptor declares frame `1` and SPK type `2` or `3` — *not* that the data payload is valid or an independent target/observer chain can be evaluated.

This file is intentionally **not** a final validation receipt: it includes no positions, velocities, external JPL source URL, hash signature or NAIF/SPICE residuals. File names are not serialized into the CSV; keep the original ordered file-argument list alongside any local output. The user remains responsible for independently recording kernel source, byte count, SHA-256 and redistribution rights.

## Selecting safe candidate test epochs

1. Identify a *specific* target, observer and load order. Examine the actual source and supported SPK descriptors for the complete center chain (e.g. Earth–Moon). Do not infer a complete relative state merely from one target's declared coverage.
2. Prefer at least three common ET epochs **strictly within** the effective coverage of every required link, with adequate margin for numeric evaluation.
3. Identify genuinely **internal** record or segment boundaries. For segment boundaries, the listed `coverage_start`/`coverage_end` are *declared* endpoints; verify the other adjacent segment(s) and NAIF priority before treating them as a continuous boundary. Internal Chebyshev **record** boundaries are not exposed by this segment metadata tool.
4. The independent E1 matrix planner in draft [#165](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/165) can prepare before/at/after *candidate* `--case TARGET:CENTER:ET` values using `--boundary-et` and `--boundary-offset-seconds`. Do not use those samples if they lie outside actual effective coverage.
5. Run the independent NAIF/SPICE comparison tool from draft #164 only when external locally obtained SPK data, SpiceyPy/NAIF Toolkit, Rust/Cargo, matching ordered kernels, scientifically approved thresholds, and exact frame/time/unit metadata are available. Compare signed position/velocity residuals, record the hashes and review the evidence manually.

**Scientific status:** synthetic Rust index tests validate parsing and CSV formatting on little- and big-endian DAF fixtures. They are *not* proof of NASA/JPL accuracy, real kernel coverage, general type support, or production readiness. E1 issue #3 remains **open** until genuine independent reference comparison and review.

## Modifications and attribution

2026-11-13: Added original Rust metadata-only CLI `inspect_spk_coverage.rs`, deterministic parser-backed and CLI unit tests, and three-platform read-only Rust CI checks. No third-party kernel files or SPICE toolkit code are included. Original modifications are CPAL-1.0 covered with VirgoStars notices; see [LICENSE](../LICENSE) and [NOTICE](../NOTICE).
