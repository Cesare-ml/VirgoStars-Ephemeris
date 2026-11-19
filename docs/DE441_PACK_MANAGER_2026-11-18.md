# DE441 classic mobile pack selection — experimental Rust API

**Status:** Original VirgoStars Rust pack-selection code; synthetic tests only
for this new module. A separate owner-run real-file slice report has passed
source/slice parity, but this manager has not been validated on Android ARM64
or independently against JPL/Horizons. It does not authenticate distribution
rights, download, remove, publish or bundle external JPL data.

The single astronomical evaluator remains [`De441`](../crates/ephemeris-core/src/de441.rs).
[`De441PackManager`](../crates/ephemeris-core/src/de441_pack_manager.rs)
provides only selection, lazy I/O and typed diagnostics. The implementation
does not depend on an app, database, HTTP client or JSON parser.

## Mobile profile (v1)

```rust
use virgostars_ephemeris_core::de441::{Body, JulianDateTdb};
use virgostars_ephemeris_core::de441_pack_manager::De441PackManager;

let mut packs = De441PackManager::from_mobile_directory("/data/virgostars/ephemeris")?;
let epoch = JulianDateTdb(2451545.0); // already TDB, never UTC
let selected_id = packs.select_pack(epoch)?.id.clone(); // no file opened
let mars = packs.state(Body::MarsBarycenter, Body::Earth, epoch)?;
println!("{}: {:?} km", selected_id, mars.position_km);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`from_mobile_directory` registers ten **logical, half-open** intervals. The
filename/ID and boundaries match the v1 Python-generated JSON manifest:

| ID | File | Astronomical years (inclusive) | Required |
| --- | --- | --- | --- |
| `core` | `de441-core.eph` | 1600–2100 | Yes |
| `history-0000-0199` … `history-1400-1599` | `de441-history-XXXX-YYYY.eph` | 0–1599, eight 200-year blocks | No |
| `future-2101-2300` | `de441-future-2101-2300.eph` | 2101–2300 | No |

The mobile profile uses **proleptic Gregorian** dates interpreted directly
as TDB at Jan 1 00:00. Year 0 is 1 BCE. No conversion from local civil dates
or UTC is done. For example, JD TDB `2488434.5` selects the **future** pack,
not the core, even though the core file's 32-day record-aligned header may
still cover that JD. At the exclusive end of 2300, the manager reports no
registered pack; the underlying classic reader's own header still reports
an inclusive stored endpoint.

The caller can instead register custom intervals and paths with
`De441PackManager::from_specs(Vec<De441PackSpec>)`. Invalid, duplicate or
overlapping intervals are rejected. For one **complete** classic DE441 file,
`De441PackManager::from_full_file(path)` preserves the existing reader's
inclusive final endpoint.

## Host responsibilities and lifecycle

1. The Android/desktop app securely obtains an approved package catalog and
   checks expected SHA-256 and byte length **before** placing a download in the
   active data directory. The local unsigned v1 exporter manifest is not a
   signed, authenticated publication catalog; do not deploy it as such.
2. The app installs downloads atomically under the documented file names,
   outside the application source repository.
3. The app calls `inventory()` for current file existence, or `select_pack()`
   to learn the required ID/path for a selected TDB epoch. `inventory()` does
   **not** compute hashes or prove scientific accuracy.
4. On first use or after a pack change, the manager opens only the selected
   file with the existing classic-DE441 binary header/length validation and
   ensures its stored coverage contains the registered logical interval.
5. If an optional file is missing, `state()`/`states_at()`/
   `state_with_acceleration()` return `De441PackError::MissingPack { id, path }`
   for the app to present a download prompt. **No fallback to other packs or
   less accurate calculations occurs.** Bad files, unsupported dates and
   invalid epochs receive distinct error variants.
6. **Before uninstall or replacement**, the host calls `packs.unload()` to
   drop the active file handle (necessary for Windows and recommended on all
   platforms). The host then deletes/replaces the file. A newly installed
   optional pack can be opened by the same manager instance at the next query.

The manager keeps at most **one active file handle** and the bounded cache
already implemented by `De441`. Batch requests use the existing 1 MiB cap.
All returned states remain **geometric ICRF equatorial positions (km),
velocities (km/TDB day)** and, when explicitly requested, Chebyshev analytic
accelerations (km/TDB day²). Outer planets are system barycenters; no
astrological chart, house calculation, light-time or apparent correction is
performed by this module.

## Contiguous installation planning (2026-11-19)

The additional `plan_contiguous_extension(installed_ids, desired_start,
desired_end_exclusive)` method supports a Desktop-style non-removable base
with optional extensions on either side. It accepts **only a fully verified
installed-ID set supplied by the host**; `inventory()` alone cannot prove
installation. It rejects missing required files, interior gaps, duplicate
IDs, unknown IDs, inconsistent required-base positions, nonadjacent logical
intervals, inclusive-end catalogs and invalid requested ranges.

An installed core covering 1600–2100 followed by a request for 0–599 will
queue every missing history package in descending order beginning with
1400–1599. It cannot produce an isolated 0–599 range. On initial install,
required packs come first. An optional installed pack is removable only if
`can_remove_contiguous_pack()` says it is the earliest or latest edge;
required and interior packs remain protected. Neither method downloads,
hashes, activates, removes or changes astronomical results. Existing custom
catalogs and the Mobile v1 profile remain supported; full original files
with inclusive final endpoints are intentionally not treated as sliced
catalogs by this planner.

The proposed 1000–2200 Desktop profile and offline release-inventory builder
are described in
[DE441_CONTIGUOUS_DESKTOP_CATALOG_2026-11-19.md](DE441_CONTIGUOUS_DESKTOP_CATALOG_2026-11-19.md).

## Verification, caveats, scope

- Synthetic integration tests: known mobile JD boundaries, deterministic
  selection across overlapping physical records, equality with the unsliced
  evaluator, batch/acceleration delegation, optional install/remove, invalid
  catalogs/headers/coverage, no-package errors and full-file endpoint.
- This module **does not** parse or validate the v1 JSON manifest itself:
  canonical mobile intervals are built from the same calendar algorithm and
  fixed filename profile; host applications may parse and verify an approved
  manifest, then pass custom specs where appropriate.
- External file replacement while a reader is open requires explicit
  `unload()`; file existence does not imply a correct checksum.
- Actual mobile storage budgets, performance, ARM64 integration, external
  JPL/NAIF reference parity and redistribution permission remain future gates.

## Modification record (2026-11-18)

New CPAL-1.0 VirgoStars Rust package selection layer and synthetic tests,
developed as a modification of the original VirgoStars-Ephemeris source code
by the Initial Developer (Cesare-ml / VirgoStars project). No external JPL
files or independently licensed software were copied or redistributed.
