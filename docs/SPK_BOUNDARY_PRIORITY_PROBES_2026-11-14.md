# SPK boundary priority probes (metadata-only)

Modified 2026-11-14. Experimental **read-only** Rust SPK diagnostic.

## Motivation

The earlier Rust APIs `declared_segments_for_target` and
`declared_coverage_boundaries_for_target` identify declared metadata
without assuming continuous target-to-center coverage. The new example
`spk_boundary_probes.rs` bridges those APIs with the existing canonical
`SpkKernelSet::selected_segment` priority resolver.

It reports which SPK descriptor wins **immediately before**, **at** and
**immediately after** every declared coverage endpoint of one NAIF target.
This provides targeted candidates for subsequent validation against independent
NAIF/SPICE results; it is **not** such a scientific validation.

## Offline usage

With one or more legally obtained local SPK files:

```bash
cargo run --offline --locked --example spk_boundary_probes -- \
  --target 399 /local/de440s.bsp > target-399-boundary-probes.csv
```

For multiple files, list them in **exact kernel load order**: the last loaded
file takes precedence when both cover an epoch. No network access or
environment-supplied kernel path is used. A target ID and at least one file
are mandatory.

## CSV output

The CSV columns are:

```text
boundary_et_tdb_seconds,probe,probe_et_tdb_seconds,load_index,center_id,frame_id,spk_type,candidate_j2000_type2_or3
```

- Times are **TDB/ET seconds past J2000**, never UTC, TT or Julian days.
- Each boundary has `before`, `at` and `after` observations where
  the adjacent representable finite binary64 epoch exists.
- The neighboring ETs use the **immediate f64 predecessor/successor**, not
  an arbitrary one-second margin. At extreme finite times the outside
  neighbor may be infinite and is omitted; `-0.0` is treated as zero.
- `load_index` is **1-based**, in the original ordered file list.
  `center_id`, `frame_id` and `spk_type` are raw NAIF metadata.
- When no descriptor covers a probe ET, the five metadata columns are
  **empty**, never misleading zero values. Coverage endpoints are inclusive.
- `candidate_j2000_type2_or3=true` only means declared frame 1 (J2000)
  and type 2 or 3. It does **not** prove coefficient validity, state-vector
  accuracy or center-chain reachability.

All input files are indexed successfully before a CSV header is emitted.
Missing/corrupt kernels or a target with no declared segments cause an error
and produce no CSV output. The tool never reads Chebyshev payloads for
interpolation, downloads datasets or compares an astronomical position.

## Boundary science warning

Adjacent f64 instants may be separated by much less than an integration
record's physical or numerical resolution. **Do not** use the immediate
neighbors as physical validation epochs without inspecting the actual
Chebyshev record interval, error budget and independent NAIF reference.
For a real JPL comparison, select scientifically meaningful offsets inside
the intersection of **all required center-chain links**. One target's
descriptor alone does not establish relative-state coverage.

The deterministic tests use synthetic DAF metadata for little- and
big-endian files, a higher-priority overlay, inclusive endpoints,
signed-zero and finite-neighbor limits, unknown targets, malformed CLI
inputs, and output atomicity on file errors.

## Provenance and status

No official NASA/JPL kernel is included or fetched. No real-kernel
validation has been performed by this example. Source files remain
licensed under CPAL-1.0 with the existing `Powered by VirgoStars`
attribution requirements; external ephemeris datasets remain separately
licensed. See `LICENSE`, `NOTICE` and `docs/VALIDATION.md`.
