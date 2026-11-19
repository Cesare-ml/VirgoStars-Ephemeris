# Desktop DE441 provider reconciliation — 2026-11-19

Status: **local Rust integration candidate; not connected to Desktop**.
This branch combines the Desktop-pinned core at `0535e9d` with the
experimental apparent pack provider from #631. It ports the contiguous
planning and edge-removal methods from #649 without replacing the provider.
The underlying `De441` remains the sole evaluator.

On Windows the combined workspace test run passed. The 10 package manager
tests and three provider tests also passed after the planner port. An ignored
real-data test was run explicitly using three privately generated,
SHA-256-verified Desktop slices and the matching local source. At each of
two boundaries it compared ten bodies at seven reception offsets (140
cases), all six apparent ecliptic fields, with the same-source full file.
It passed the pre-existing tolerance limits. This is **not** an independent
JPL/NAIF validation and does not validate civil time, UI or installed Desktop.

The provider now also exposes an experimental standard geocentric path for
both true-ecliptic and true-equatorial frames with solar deflection, without
the long-term experiment's emission refinement. Against the same local full
file, the standard path matched at a modern epoch in both frames and rejected
the Desktop 1000 and 2201 boundaries with the same unsupported-orientation
error as the single-file path. The available coefficients therefore do not
by themselves establish usable Desktop calculations across the base range.

Desktop also uses solar events, lunar points, stellar deflection, small-body
SPK and historical snapshots through APIs that take a single `De441`.
None is automatically multipack by this change.
Unsafe sub-ULP emission refinement crossings still produce an explicit error.
Do not switch Desktop's provider or present optional packs as usable until
these paths have compatible, tested provider-backed implementations.

No kernel, catalog authentication key, release, installer, download service
or data redistribution authorization is included. The provenance and rights
gate documented by #650 remains open.
