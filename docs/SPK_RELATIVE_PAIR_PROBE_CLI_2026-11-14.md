# Offline geometric SPK pair diagnostics CSV

**Experimental, read-only CLI · 2026-11-14 · Not independently NASA/JPL validated**

## Why this example exists

The canonical Rust batch example, spk_batch_at, is intentionally **atomic**: one failed body-pair query makes the entire output fail without partial results. The already implemented spk_pair_diagnostics::probe_relative_states_at is deliberately **diagnostic** and returns each pair's success or typed error.

This new spk_pair_probe_at example exposes that existing diagnostic method as machine-readable CSV, useful when reviewing gaps, unsupported SPK segments and broken center chains in *caller-supplied* kernels. It does not reimplement orbital computation, segment priority or center-chain resolution.

## Run

~~~sh
cargo run --offline --locked --example spk_pair_probe_at -- \
  --et 0 \
  --pair 301:399 \
  --pair 399:0 \
  --pair 301:0 \
  /path/to/older.bsp \
  /path/to/newer-overlay.bsp
~~~

- The required --et argument is a finite numeric **TDB second offset from J2000**, not a Julian Day, UTC or TT. Explicitly converting source timestamps into TDB is the caller's responsibility.
- One or more --pair TARGET:CENTER arguments supply signed decimal i32 NAIF IDs. Target and center must differ; a same-body mathematical zero is not validation of ephemeris coverage. Repeat pairs to investigate duplicate or order-dependent results.
- One or more existing **local** SPK files must be supplied in **load priority order**, last loaded wins. Use -- before dash-prefixed filenames.
- No kernel search, network request, automated download, automatic time-scale conversion, global path or secret is involved.

## CSV schema

~~~text
et_tdb_seconds,request_index,target_id,center_id,status,common_ancestor_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s,error_code,error_message
~~~

Exactly one row per supplied pair, **in input order including duplicates**. Request indices are **zero-based**.

Successful rows use status=ok, a real common ancestor and all six components in scientific notation with 17 fractional digits. Coordinates are **geometric target-minus-center position in kilometres** and **velocity in kilometres per TDB second**, in fixed **NAIF frame 1 J2000/ICRF**. No light time, aberration, topocentric correction, zodiac conversion or apparent state is included.

Failed rows use status=error and leave **all seven scientific fields (ancestor and 6 components) blank**, never fabricated zero vectors. The error_code is a stable category independent of OS-specific message formatting. The error_message is a quoted RFC-style CSV field; embedded quotes are escaped. Codes currently include:

- invalid_epoch, cycle, disconnected, numerical_overflow
- segment_io, segment_invalid_epoch, segment_missing
- unsupported_frame, unsupported_type, invalid_segment, interpolation

The code identifies an evaluation failure only; it **does not** establish that the supplied kernel was authentic or quantitatively accurate. Detailed error text may vary across platforms.

## Exit codes and fail-closed behavior

- **0:** all requested pair rows succeeded.
- **2:** valid CLI/kernel preflight, but one or more individual pair evaluations failed. **All rows remain in stdout** for diagnosis. Scripts must treat this as an unsuccessful validation gate, not silently ignore the process status.
- **1:** invalid CLI options, nonfinite epoch, missing/invalid local kernel or another preflight failure. **No CSV header or rows are printed before preflight completes**.

Once output begins, an ordinary stdout write error can still truncate CSV; callers should use atomic redirection to a temporary file if durable output is needed.

For **all-or-error production calculations**, use spk_batch::relative_states_at or spk_batch_at; never treat partial diagnostic results as a successful full batch.

## Regression verification

Five synthetic Rust example tests, using entirely original in-memory little- and big-endian DAF/SPK buffers and no NASA/JPL data, cover:

1. Good/bad/good four-row ordering, duplicate pair preservation, a disconnected error, fixed output shape, successful km component values and velocity fields.
2. A newer unsupported SPK frame that must block earlier valid data for affected pairs while leaving unrelated pairs evaluable.
3. An intentionally invalid higher-priority coefficient-directory record that must fail rather than silently use an older source.
4. Strict explicit CLI options, signed IDs, nonfinite epoch rejection and dash-prefixed local paths.
5. Preflight-error stdout cleanliness and escaping CSV messages.

Use the repository's read-only CI on the exact PR commit:

~~~sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
~~~

Native Linux, macOS and Windows tests are required. This establishes deterministic synthetic routing behavior, **not independent numerical parity against real NAIF/SPICE/Horizons or a real JPL kernel**. Such evidence remains a separate scientific acceptance gate.

## Licensing and coordination

Original source retains **CPAL-1.0** Exhibit A attribution to the VirgoStars initial developer, alongside the repository NOTICE and Powered by VirgoStars credits. Modification date: **2026-11-14**. No third-party binaries, external kernels, new dependencies, CI permissions, package publication, protected-branch changes or production deployment.
