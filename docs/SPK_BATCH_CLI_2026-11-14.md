# Offline geometric SPK batch CLI — 2026-11-14

**Status: experimental; no independent real NASA/JPL/NAIF validation.**

The `spk_batch_at` Rust example exposes the existing canonical
`spk_batch::relative_states_at` API through a read-only command line.
It processes explicitly supplied, local SPK Type-2/Type-3 files in load order,
and queries several NAIF target/center pairs at **one shared epoch**.
There is no file download, default epoch, hidden body substitution, frame
rotation, apparent correction or second implementation of interpolation.

## Run

From the repository root, after installing Rust and supplying your own legally
obtained SPK files whose provenance and SHA-256 digests you have verified:

```sh
cargo run --offline --locked --example spk_batch_at -- \
  --et 0 \
  --pair 301:399 --pair 399:0 --pair 301:0 \
  /absolute/path/to/first-kernel.bsp /absolute/path/to/second-kernel.bsp
```

The example paths are placeholders. **No binary kernels, numerical results or
SPICE toolkit are bundled.** Use `--` immediately before the paths if any
kernel filename starts with `-` (e.g. `-- -test.bsp`).

- `--et` is mandatory, exactly once, and a finite signed floating-point **TDB
  seconds past J2000**. Not UTC, TT, Julian Date or a local civil time.
- `--pair TARGET_ID:CENTER_ID` is mandatory at least once and may be repeated.
  IDs are signed NAIF `i32` numbers. Target and center must differ, because
  same-body zeros are mathematical identities, not evidence of kernel coverage.
- The relative state resolves using the order in which SPK paths are given:
  later loaded covering segments mask older ones. All required SPK links must
  use supported frame **J2000/ICRF (NAIF frame 1)** and SPK Type 2/3.
- Duplicate request pairs are retained with their original order and values;
  no deduplication, sorting or caching is performed.
- The example does not silently skip unsupported segments, missing center
  chains, invalid coefficients, coverage gaps, or incomplete kernel loads.

## CSV format and error behaviour

On success, writes one header plus one row per requested pair:

```text
et_tdb_seconds,target_id,center_id,common_ancestor_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s
```

Rows contain signed geometric positions in **km** and velocities in **km/s**;
the common ancestor is diagnostic. Floating-point quantities use 17-digit
scientific formatting. The command also prints a distinct experimental-status
warning to **stderr**.

Each request is calculated before the first CSV byte is emitted. If any kernel
fails to load or any pair fails, the process exits nonzero, reports the error on
stderr, and produces no apparently successful partial result CSV. Filesystem
write failures (e.g. broken stdout pipe) can, naturally, truncate output;
this is not a transactional file writer.

**This is a diagnostic CLI, not a NAIF/SPICE reference file producer.** Its
10-column CSV includes `common_ancestor_id` and lacks independent NAIF
provenance declarations, so it **must not** be passed directly as the
9-column `validate_spk_reference` independent-reference CSV. For scientific
comparison use the separately documented reviewed
[NAIF/SPICE reference generator and Rust comparator](RUST_SPK_NAIF_OFFLINE_VALIDATION_2026-11-13.md)
with independent `spkezr` geometric `NONE` vectors, kernel hashes, declared
tolerances and audited evidence.

## Validation and limitations

The example includes synthetic tests for explicit CLI arguments and malformed
flags/IDs/epochs; duplicate and negative NAIF IDs; mixed-endian, two-kernel
Moon-Earth-SSB relative states, correct order, units and rate values; all-or-error
CSV behaviour, corrupted kernels and no source files. The existing read-only
CI runs these tests on Linux, native Windows and native macOS:

```sh
cargo test --workspace --example spk_batch_at --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The tests contain **only artificially generated DAF/SPK binary layouts**.
They are not independent NASA/JPL kernel interpolation validation, and this
tool is **not approved for production astronomical calculations**.

## License and modification

Created 2026-11-14: added an original Rust CLI example, tests and nonprivileged
CI coverage, based only on the existing core and standard library. CPAL-1.0,
the original developer notice and required VirgoStars attribution remain
unchanged. No third-party ephemeris data, Swiss Ephemeris code, new dependency,
package publication, production migration or GitHub deployment was introduced.
