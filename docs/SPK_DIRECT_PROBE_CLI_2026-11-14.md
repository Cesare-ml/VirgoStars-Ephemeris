# Offline SPK direct-state diagnostic report CLI — 2026-11-14

**Experimental, synthetic validation only.** New CPAL-1.0 source example: `crates/ephemeris-core/examples/spk_probe_at.rs`; original developer notice, LICENSE/NOTICE and **Powered by VirgoStars** attribution remain applicable.

## Why and limits

A declared SPK target can have a selected segment that is unsupported or corrupt. The existing Rust `SpkKernelSet::probe_direct_states_at(et)` is the authoritative diagnostic for this: it preserves the winning SPK descriptor and the real Type-2/3 direct-evaluation result **per target**. This CLI only parses explicit local file paths, calls the Rust API, and renders CSV; it does not implement a second ephemeris algorithm, silently skip the highest-priority segment, acquire external kernels or claim NASA/JPL numerical parity.

The diagnostic is **not** an atomic astronomical batch. Some target rows can succeed while other rows contain an explicit error. Successful rows are single-epoch geometric **target minus selected segment center** values in fixed NAIF frame 1 (J2000/ICRF), with positions in **km**, velocities in **km/s**, and time as **TDB/ET seconds past J2000**, never UTC or Julian day. It does not validate multi-link center chains, non-J2000 frames, apparent/light-time corrections, completeness across a date interval, or real-kernel accuracy.

## Usage

Run from the repository root with an existing Rust toolchain and caller-acquired, legally reviewed local SPK files:

```sh
cargo run --offline --locked --example spk_probe_at -- --et 0 /path/to/older.bsp /path/to/newer.bsp
cargo run --offline --locked --example spk_probe_at -- --et -43200 --target 301 /path/to/file.bsp
```

Later files have higher priority; use `--` before a kernel path beginning with `-` (after all options). Required `--et` must be finite and explicitly interpreted as TDB seconds past J2000; optional `--target` is signed NAIF `i32`. Without a filter the report covers every **declared target whose selected descriptor covers that ET**, sorted by signed target ID. `--target` absent from the entire loaded inventory is a preflight error; one declared but not covered at this epoch yields header-only CSV.

Stdout is machine-readable CSV with columns:

```text
et_tdb_seconds,target_id,kernel_load_index,segment_center_id,frame_id,spk_type,coverage_start_et_tdb_seconds,coverage_end_et_tdb_seconds,status,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s,error
```

`status=ok`: six numeric values, empty error field. `status=error`: six **empty**, not fabricated-zero, numeric fields followed by a quoted and escaped error string. All floating-point values use 17-digit scientific notation; kernel load index is zero-based and tracks actual priority. The diagnostic footer is sent to stderr, never embedded in CSV.

**Exit codes:** 0 = all reported target evaluations succeed (including no covered target); 1 = invalid arguments / missing kernel / load or output failure (preflight does not write a CSV); 2 = report successfully written but at least one target had an evaluation error. This allows CI and operators to detect partial failures without losing the report.

## Offline tests

Synthetic mixed-endian single-record SPKs test successful and failed rows in the same report, sorted signed IDs, shadowing without fallback, corrupt winning record RSIZE, strict numeric/option validation, target filters, missing coverage and no partial CSV on invalid input, plus error CSV quoting. All example tests run in read-only GitHub CI on Linux, macOS and Windows using:

```sh
cargo test --workspace --example spk_probe_at --locked
```

The tests validate CLI contracts and controlled synthetic data only. Full independent NAIF/JPL/Horizons numerical and provenance validation remains outstanding. This feature does not change `main` or the VirgoStars production integrations.
