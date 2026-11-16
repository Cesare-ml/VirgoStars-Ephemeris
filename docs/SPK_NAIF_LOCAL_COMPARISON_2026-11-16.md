# Local SPK/NAIF comparison — 2026-11-16

**Status: comparator pass, provenance and scientific review pending.** This
is a limited local experiment, not acceptance of E1 or a production release.

## Inputs and contract

- Kernel: `de440s.bsp`, downloaded for this run from the [official NAIF
  planetary-kernel directory](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/de440s.bsp),
  32,726,016 bytes, SHA-256
  `c1c7feeab882263fc493a9d5a5b2ddd71b54826cdf65d8d17a76126b260a49f2`.
  It remains external to Git; redistribution rights have not been reviewed.
- Independent reference: SpiceyPy 8.4.0 with CSPICE N0067 `spkezr`, using
  the same one-file kernel load, J2000 frame, geometric `NONE` correction,
  TDB seconds past J2000, km and km/s. Rust 1.99.1 on Windows evaluated the
  vectors separately from the same file.
- Cases: NAIF target:observer pairs `10:399`, `301:399`, `399:0`, `4:399`,
  and `5:399`, at ET `-3155760000`, `0`, and `3155760000` seconds. IDs 4
  and 5 are planetary-system barycenters, not Mars/Jupiter centers. The
  local SPK inspector listed all required segments as frame 1, type 2, with
  declared coverage from ET -4734072000 through 4735368000 seconds.
- Diagnostic component limits were selected **before** the run: 1e-5 km
  position and 1e-8 km/s velocity. These are numerical-comparison bounds,
  not approved physical accuracy budgets or measurements of DE440 accuracy.

## Result

At 2026-11-16 18:28 UTC, the existing `scripts/spk_naif_reference.py run`
generated 15 reference rows, ran the Rust comparator offline, rechecked the
input hashes and wrote an unsigned review-pending receipt. All 15 comparisons
passed; worst absolute component residuals were **1.192092896e-7 km** and
**3.552713679e-15 km/s**. The E1 matrix auditor passed the minimum breadth
and checked receipt/reference consistency. This is a direct comparison to
CSPICE, not a comparison to the repository's JavaScript prototype.

The reference CSV SHA-256 was
`731c0b091443fb76a37432f199210867b8719869395cab6122bd5170859bae00`;
the generator SHA-256 was
`54f017b20222a9733a7123b8e7e2eb83783c3ce915d0018e6030d38a3b76e80f`.
The reference CSV, full per-case residuals, command and receipt are retained
locally under ignored `target/naif-validation/`; they are not committed and
may be removed by a clean build. Retain an independently reviewed copy before
using this as durable scientific evidence. The source script and kernel are
checked before and after comparison, not locked against all concurrent changes.

Reproduce with the documented `run` command in [the validation procedure](RUST_SPK_NAIF_OFFLINE_VALIDATION_2026-11-13.md),
passing the 15 combinations above, `--max-position-km 0.00001` and
`--max-velocity-km-s 0.00000001`, and run `audit_spk_e1_matrix.py` against
the resulting reference and receipt. Install CSPICE/SpiceyPy independently;
the repository does not bundle either dependency or kernel.

## Remaining gates

Review the exact downloaded file and independent SPICE provenance, retained
raw output and diagnostic thresholds. Add further independently identified SPK
segment boundary probes, broader epochs and targets, and matching
Horizons evidence. Type 3, multiple-kernel priority, non-J2000 frames and
apparent corrections are not established by this single type-2 kernel run.
No CI, staging or production execution is implied.

## Internal record boundary comparison — 2026-11-16

NAIF/CSPICE DAF summary and array reads of the same kernel identified type-2
record grids with `INIT=-4734072000` ET and intervals of 345600 seconds
(Moon and Earth), 1382400 seconds (Sun and Earth-Moon barycenter), and
2764800 seconds (Mars and Jupiter system barycenters). The inspected
internal boundaries nearest J2000 were respectively `-43200`, `648000`,
and `-734400` ET seconds. These are record boundaries, not segment-coverage
endpoints; all selected comparison cases lie inside the required chains.

For the same five target/observer pairs and predeclared diagnostic limits,
the comparator checked each boundary at one second before, exactly at, and
one second after it: **45 cases passed**. Worst component residuals were
`1.192092896e-7 km` and `1.065814104e-14 km/s`. The receipt/reference
consistency and matrix audit also passed. The reference CSV SHA-256 is
`f242f48f43f3eedd72b98459bca771b8d62eca3ffff17d78a4267f3566ecd7de`;
the unsigned receipt SHA-256 is
`e43dff48994a5e88e6b03f76d2990649be18b9fe81e1eba3f5df44977aac41ce`.
Both are retained locally under `target/naif-validation/`, subject to clean
build removal. This comparison does not test f64-adjacent epochs, segment
overlays, type 3 or independent kernel provenance and remains review-pending.

## Declared coverage endpoints — 2026-11-16

The same five pairs were also compared exactly at the common declared start
and end ET (`-4734072000` and `4735368000` seconds) and one second inside
each endpoint. All **20 comparisons passed** with the same predeclared
diagnostic thresholds; the maximum component residuals were
`1.192092896e-7 km` and `7.105427358e-15 km/s`. Matrix breadth and receipt
consistency checks passed. The reference CSV SHA-256 is
`eeda45f85951d6ad8dcab4a82452116a416fa5a30945e0c94a7db39c89ef1105`;
the unsigned receipt SHA-256 is
`dd0f0e730e810f25090f12370f0a593d7769991fef0a5640c4661c0ebc4f7a1e`.
The local output files are under ignored `target/naif-validation/`. These
tests do not establish behavior outside coverage, validate all segment
transitions, or remove the provenance and scientific review requirements.

**CPAL-1.0 modification record (2026-11-16):** recorded a local comparison;
no third-party code or kernel is redistributed in the repository.
