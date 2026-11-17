# Local SPK declared-coverage union CLI — 2026-11-14

**CPAL-1.0 §3.3 modification record.** Added `crates/ephemeris-core/examples/spk_coverage_union.rs`. Existing CPAL Exhibit A source notice, LICENSE, NOTICE and **Powered by VirgoStars** attribution are preserved.

## Why

Before comparing local NASA/JPL SPK results with an independent NAIF/Horizons reference, investigators need to know **which NAIF target IDs** and **which TDB/ET intervals** their explicit, ordered kernel set declares. Individual descriptor inventories list overlapping sources; this CLI exposes a compact, sorted union, preserving true gaps so unsupported epochs do not silently slip into reference batches.

The example uses only the canonical Rust `SpkKernelSet::load`, `declared_target_ids` and `declared_coverage_intervals_for_target` methods. It implements **no new ephemeris mathematics**, parser, timestamp conversion or segment-precedence rule. It reads only user-selected local files, never downloads/rehosts third-party kernels.

## Usage

From a clone with a compatible Rust toolchain:

```sh
cargo run --offline --locked --example spk_coverage_union -- /path/to/one.bsp /path/to/two.bsp
cargo run --offline --locked --example spk_coverage_union -- --target 301 /path/to/one.bsp /path/to/two.bsp
```

The paths are in **NAIF load order** (later files have higher priority for covering target/ET combinations). To pass a filename beginning with a hyphen, separate options with `--`.

Stdout is machine-readable CSV:

```csv
target_id,coverage_start_et_tdb_seconds,coverage_end_et_tdb_seconds
301,0.00000000000000000e0,1.00000000000000000e1
301,2.00000000000000000e1,3.00000000000000000e1
```

*Illustrative numbers only; these are not measured real NASA/JPL intervals.*

- Identifiers are signed 32-bit NAIF **target IDs**, sorted ascending; center-only bodies are not reported unless they also appear as targets.
- Spans are sorted, inclusive, merged on actual overlap or shared endpoints, and never bridged across even one `f64` ULP gap. Opposite-endian, overlapping and fully shadowed declarations are included.
- Times are **TDB/ET seconds past J2000**, not Julian dates or UTC. Numeric output uses 17-digit scientific precision.
- A target filter which is not present fails **before any CSV is printed**. A missing or unreadable kernel likewise aborts before output; no implicit search path or default kernel exists.
- A note on **stderr** explains that all intervals are *declared-only*. CSV stdout contains no footnotes.

## What this does NOT prove

The summary **does not test the coefficient bytes**, effective frame/type compatibility, center-chain connectivity, the currently winning segment, real-kernel provenance, numerical accuracy or agreement with NAIF/Horizons. A declared target may not be computable at any epoch. For source selection use `inspect_spk_coverage` or `spk_boundary_probes`; for actual Type-2/3 J2000 state computation use `spk_state_at` or `spk_batch_at`, and validate against independently sourced real NAIF vectors with approved tolerances. No changes to production or to `main`.

## Synthetic tests and CI

Five unit regressions use only fabricated DAF/SPK headers (no JPL data), including:

1. Overlap union across mixed IEEE byte orders, separate gaps and signed target ordering.
2. A one-binary64-ULP gap remaining unmerged and negative zero becoming +0.
3. Missing filtered NAIF target fails closed without partial CSV.
4. Missing file, duplicate/invalid CLI options, signed negative target IDs and dash-prefixed filenames.
5. Absent source path fails without output.

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Use exact-head read-only GitHub CI across Linux, Windows, and macOS. Passing this CLI/test gate is a **preflight utility**, not a certificate of scientific numerical parity.
