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

The provider only exposes an experimental ecliptic apparent path. Existing
Desktop also uses true-equatorial coordinates, solar events, lunar points,
stellar deflection, small-body SPK and historical snapshots through APIs that
take a single `De441`. None is automatically multipack by this change.
Unsafe sub-ULP emission refinement crossings still produce an explicit error.
Do not switch Desktop's provider or present optional packs as usable until
these paths have compatible, tested provider-backed implementations.

No kernel, catalog authentication key, release, installer, download service
or data redistribution authorization is included. The provenance and rights
gate documented by #650 remains open.
