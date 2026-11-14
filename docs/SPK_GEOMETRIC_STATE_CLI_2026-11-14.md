# Offline geometric SPK state query — 2026-11-14

## Scope and source-code record

New Rust CLI example: `crates/ephemeris-core/examples/spk_at.rs`. Original VirgoStars project code is covered by CPAL-1.0, retaining its attribution and notices. This addition does not introduce dependencies, download external data, modify the Rust state solver or redistribute a SPK kernel.

This command evaluates a **single geometric relative position and velocity** from local, caller-supplied NAIF DAF/SPK files using the existing `SpkKernelSet` Rust core. It is not an independent ephemeris calculator or a numerical validation against JPL Horizons/NAIF SPICE.

## Usage

```sh
cargo run --offline --locked --example spk_at -- \
  --target 301 --center 399 --at-et 0 \
  /local/path/to/first.bsp /local/path/to/second.bsp
```

- `--target` and `--center` are explicit signed NAIF integer body IDs. Example: 301 for the Moon and 399 for Earth.
- `--at-et` is a **finite TDB/ET number of seconds since J2000**, not UTC, Julian days or a civil calendar date. Negative ET values are allowed.
- Local SPK files are given in their intended **load order**. The last loaded file with a covering target wins; the last covering segment within a file wins.
- One or more kernel files are mandatory. The command rejects missing/duplicate required flags, unknown switches, nonfinite ET, unreadable or malformed kernels, disconnected center chains, cycles, unsupported selected data types or frames, and numeric errors; it does not fall back to another model.
- No network call or automatic kernel download occurs. An independent rights/provenance review is required for external kernel data.

The command writes one two-line CSV (header + result) to stdout **only after successfully evaluating** the state. The numeric fields use 17-digit scientific notation suitable for binary64 round-trip. The columns are:

```text
et_tdb_seconds,target_id,center_id,common_ancestor_id,frame_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s
```

`common_ancestor_id` identifies the shared node used to compose the requested target-minus-center state; it is not always the solar system barycenter.

## Scientific contract

- Reference frame: only **NAIF frame 1, fixed J2000/ICRF**.
- Positions: km. Velocities: km per **TDB second**. ET is TDB seconds past J2000.
- Supported segment models: direct SPK types **2 and 3** along an available target/center chain, using the existing Rust Chebyshev implementation.
- Not provided: other frames/types, light-time, aberration, gravitational deflection, atmospheric refraction, date-frame rotation, UTC/TT conversion, topocentric astronomy or a full SPICE API.
- Synthetic kernel tests exercise CLI parsing, mixed IEEE byte order and multi-file center-chain composition. **They are not evidence of real numerical parity or data origin.** The independent reference comparison described in `docs/RUST_SPK_NAIF_OFFLINE_VALIDATION_2026-11-13.md` remains a separate blocking gate.

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace --example spk_at --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

For a real SPK file, run the command only with an externally obtained, locally stored kernel that has been reviewed for provenance and licensing. Before any production integration, retain a SHA-pinned NAIF/JPL reference comparison with explicit numerical tolerances, time scales, and body/center/frame metadata.
