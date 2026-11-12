# Independent classic DE441 numerical validation protocol

Status: **unexecuted scientific acceptance procedure**, 2026-11-12. This document does not claim that the classic DE441 reader agrees with JPL Horizons. No kernel or proprietary reference vectors are committed.

## Preconditions

1. Obtain a classic binary DE441 file from an authorized distributor, record its exact source URL, retrieval date, file size and SHA-256, and check the distributor's terms. Never commit the binary.
2. Use the same **DE441 solution** for the reference as for the Rust calculation. A default Horizons solution may differ; explicitly record the selected solution and stop if it cannot be matched. Do not silently compare DE441 with DE440.
3. Record the Git commit, Rust toolchain, host OS and exact command lines. Keep downloaded datasets outside the repository.

## Matching a state vector

For each sample, record the following fields **before** calculating a residual:

| Field | Required specification |
|---|---|
| Epoch | Julian Date on TDB scale; do not pass a UTC JD as TDB |
| Target | Exact numeric JPL body ID and whether it is a planet center or system barycenter |
| Center | Exact numeric JPL observer/center ID |
| Reference frame | ICRF/equatorial inertial; no ecliptic-of-date transformation |
| Quantity | Geometric position and geometric velocity; no light-time or aberration |
| Units | km for position, km/day for velocity; convert reference km/s to km/day by multiplying by 86400 |
| Ephemeris | Explicit DE441 reference solution and exact classic binary provenance |

For Horizons VECTORS output, preserve the full query parameters and response header. Confirm that its center, correction mode, reference plane and time format match the Rust request. A Horizons API response or NAIF state computed from a different kernel, center or frame is **not** a valid independent reference for a strict parity claim.

## Coverage matrix

Choose at least three well-separated TDB epochs within the actual file header coverage (including J2000), plus points immediately inside each file boundary. Include Sun versus solar-system barycenter, Earth versus Earth-Moon barycenter, Moon versus Earth, Mars **system barycenter** versus solar-system barycenter, and an outer-planet barycenter. Do not equate Mars center with Mars barycenter. Add an out-of-coverage request that must fail deterministically.

For every sample, compare each Cartesian component and the Euclidean norm of position and velocity differences. Report maxima, median, number of samples and units separately for position and velocity. Record a justified tolerance **before** inspecting residuals; do not choose a threshold after seeing the results. Do not use the earlier JavaScript implementation as an independent oracle.

## Reproducible evidence

Store a compact manifest with the query settings, reference provenance, data hash, software commit, exact commands, tolerance rationale, sample count, residual statistics and any failed cases. Small independently redistributable reference values may be included only after reviewing their source terms. A CSV parser test or synthetic Chebyshev fixture establishes software behavior, not astronomical accuracy.

## Acceptance gate

Mark the independent validation gate **passed** only after executing the real-file comparison and reviewing the provenance, query settings and residuals. If Horizons cannot be configured for the same DE441 solution, mark the gate **blocked**, document why, and seek an independently generated DE441 reference from a suitable NAIF workflow. Keep production migration, release and numerical-parity claims blocked until this gate is complete.

See [DE441.md](DE441.md), [VALIDATION.md](VALIDATION.md), [DATA_AND_LICENSING.md](DATA_AND_LICENSING.md), and [AGENTS.md](../AGENTS.md).

## Machine-checkable metadata preflight

Before running the independent vector comparison, prepare a JSON manifest with
`kernel_url`, `kernel_sha256` (64 hexadecimal digits), `kernel_bytes`,
`retrieved_utc`, `engine_commit`, `reference_source`,
`reference_solution` (exactly `DE441`), `reference_query` (preserve
the complete query/settings), positive finite `tolerance_position_km` and
`tolerance_velocity_km_day`, and a nonempty `samples` array.

Each sample must contain numeric `jd_tdb`, integer `target_id` and
`center_id`, `frame: "ICRF"`, `correction: "NONE"`,
`reference_plane: "FRAME"`, `position_unit: "km"`, and
`velocity_unit: "km/day"`. Distinct target/center IDs are required.

Run the offline standard-library-only preflight:

```bash
python3 tools/check_de441_validation_manifest.py /path/to/validation-manifest.json
python3 -m unittest discover -s tests -p 'test_de441_validation_manifest.py'
```

The preflight rejects missing or incompatible metadata; it does **not** verify
that the URL/hash are authentic, that the Horizons query actually selected
DE441, that the declared tolerance is scientifically justified, or that any
numerical vector agrees. Those checks require human review and independent
real-kernel computation. Keep the manifest and reference evidence with the
validation report, not with bundled ephemeris binaries.
