# Offline SPK geometric state query (experimental)

**2026-11-14.** This is a read-only convenience CLI around the **existing canonical Rust** `SpkKernelSet::relative_state` implementation, not a second ephemeris engine or a scientific-reference validator.

## Run one explicitly specified state

Supply one or more **locally and independently obtained** DAF/SPK files in NAIF load order (later files have higher priority). Neither Cargo nor the CLI downloads kernels, chooses a default kernel, looks up a civil date or calls a web service.

```bash
cargo run --offline --locked --example spk_state_at -- \
  --target 301 --center 399 --et 0 \
  /local/path/to/first.bsp
```

On Windows PowerShell, the same command can be entered on one line:

```powershell
cargo run --offline --locked --example spk_state_at -- --target 301 --center 399 --et 0 "C:\kernels\first.bsp"
```

For multiple files, append paths in **actual SPICE load order**, e.g. `first.bsp second.bsp`. Filenames beginning with a hyphen must be preceded by a separate `--` after the options.

- `--target`: explicit signed 32-bit NAIF **body ID** for the desired target. `301` is the Moon body center; it is **not** the Earth-Moon barycenter.
- `--center`: explicit signed 32-bit NAIF body ID for the reference center. `399` is the Earth body center; it is **not** the Earth-Moon barycenter `3`.
- `--et`: explicitly supplied **TDB/ET seconds past J2000**, as a finite binary64 number. `0` means J2000 ET zero, **not** the current time, UTC midnight or a calendar date.
- At least one local SPK path is mandatory. No default observation site, frame, correction mode or converted epoch is provided.
- Same-body target/center requests are refused by this diagnostic CLI to prevent recording a meaningless zero as a successful nontrivial astronomical test. The underlying Rust API separately supports the mathematically valid same-body zero.

## Output contract

The command writes exactly one CSV header and one result row on stdout:

```text
et_tdb_seconds,target_id,center_id,common_ancestor_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s
```

The numeric fields are emitted with 17 digits after the decimal in scientific notation. The **common ancestor** is a diagnostic center-chain connection and need not be the solar-system barycenter.

All results are geometric **target minus center**, with no light-time, stellar aberration, gravitational deflection or topocentric transformation. Only fixed NAIF **frame ID 1 (J2000/ICRF)** and **SPK Type 2/3** polynomial segments are supported in the underlying experimental resolver. The positions are Cartesian **kilometres**, velocities **kilometres per TDB second** (km/s). The tool makes **no** UTC/TT conversion, precession, nutation, ecliptic projection or astrological interpretation.

Selected-segment priority follows the canonical Rust source: later-loaded kernels override earlier covering ones, and the last covering segment in a kernel wins. A covered unsupported/corrupt high-priority segment **fails closed**, not by silently falling back. A missing required center path, invalid metadata/coefficient grid, corrupt/truncated kernel, invalid ET or nonfinite output exits nonzero and never writes a plausible partial CSV result. An SPK descriptor listing alone is not evidence that the required chain is evaluable.

Use the separate [descriptor coverage inspector](RUST_SPK_LOCAL_COVERAGE_INSPECTOR_2026-11-13.md) to inspect file-level time ranges before choosing ET cases.

## Scientific validation and limitations

**Do not use this CLI's output as independent NAIF/JPL proof.** It is computed by the same Rust implementation being evaluated. For meaningful E1 acceptance, obtain a reviewed external NAIF/SPICE `spkezr` or JPL reference for the **identical kernel bytes, ordered load set, target, center, TDB ET, frame `J2000`, corrections `NONE`, and units km/km/s**. Use the [offline NAIF comparison and SHA-256 preflight](RUST_SPK_NAIF_OFFLINE_VALIDATION_2026-11-13.md), an [E1 case-breadth audit](SPK_E1_REFERENCE_MATRIX_AUDIT_2026-11-13.md), explicit reviewed numerical tolerances and a retained evidence receipt. Neither synthetic tests nor a successful CLI invocation completes E1 scientific validation.

There are no JPL/NAIF `.bsp` or classic DE441 kernel binaries in the repository; do not commit downloaded files or vendor SPICE/Swiss code. Check independent data rights. The original VirgoStars code remains **CPAL-1.0**, with required [Powered by VirgoStars attribution](LICENSING_AND_CREDITS.md) and [NOTICE](../NOTICE).

## Regression checks

```bash
cargo fmt --all -- --check
cargo test --workspace --example spk_state_at --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The new example tests reject missing/duplicate/nonfinite CLI options and test a mixed-endian, mixed-Type-2/3 synthetic two-file chain from Moon to Earth to SSB, including Cartesian velocities and CSV units. They **do not** use actual JPL data. Read-only Linux, macOS and Windows checks run this exact example in the repository CI.

**Modification record (2026-11-14):** Added the explicit offline SPK Rust CLI and synthetic example tests, plus read-only CI coverage and this user guide; no Rust astronomy engine changes, public API changes, new dependencies, network requests, external kernels, package publishing, production integration or scientific accuracy certification.
