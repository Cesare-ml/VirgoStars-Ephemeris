# DE441 contiguous catalog for Desktop — 2026-11-19

**Status: source implementation and synthetic tests only. Not released,
not connected to Desktop or Mobile downloads, not cleared to redistribute
external kernels.** This change is stacked on the experimental slice-export
branch (Ephemeris PR #601); it does not supersede the existing Mobile v1
profile, its local artifact hashes, or the experimental integrated apparent
provider in Ephemeris PR #631.

## User-facing decision

- Desktop's proposed always-present base is astronomical years **1000–2200
  inclusive**, represented by a single self-contained classic .eph slice.
  This is *different from the 1890–2110 slice currently staged in Desktop*;
  no installer is changed by this PR.
- Optional packages cover consecutive **200-year** blocks outward. The edge
  block may contain fewer years depending on the requested source coverage.
- For a new installation the required base is installed before extensions.
  For an existing installation, an extension from 1000 toward year 0
  queues 800–999, 600–799, …, 0–199 in descending order; it does not allow
  an isolated 0–499 package while 500–999 is missing.
- Installed status must be based on files that the **application host has
  verified** against the approved catalog, not on filesystem existence.
  The Rust planner rejects missing required packs, holes, duplicate IDs,
  unknown IDs, and noncontiguous logical catalogs. An optional pack may be
  removed only from the earliest or latest installed edge.
- Packages are independent data assets and must live in an application-local
  data directory outside its executable, so application updates do not
  wipe them. Downloader, progress, cancellation, atomic activation, disk
  checks, and installer/first-run UI remain **application work**.

## Catalog and local build

The offline tool scripts/build_de441_contiguous_packs.py reuses the
existing Rust classic-slice exporter, real-source numerical parity
comparator, Python SHA-256 verification, and atomic manifest writer.
No Python astronomy arithmetic is added. No GitHub releases, remote
URLs, network calls, or kernel bytes are included in source control.

    # Review the proposed years/IDs before using a local file.
    python scripts/build_de441_contiguous_packs.py plan

    # Build just the Desktop base from the owner's classic little-endian DE441.
    python scripts/build_de441_contiguous_packs.py build \
      --source /local/de441.eph \
      --output-dir /local/virgostars-desktop-packs

    # Verify that local manifest and installed artifacts still match.
    python scripts/build_de441_contiguous_packs.py verify \
      --manifest /local/virgostars-desktop-packs/de441-contiguous-manifest-v2.json

Use --packs all for all configured intervals, or --packs PACKAGE_ID
for selected IDs, with a **new output directory per manifest**. Configure
--min-year and --max-year *before* the build/plan subcommand if
the actual source covers different years. The defaults of -12999 to 16999
are only a **proposed planning window** based on the classic data described
in docs/DE441.md; only an actual source export can establish coverage.
The source must remain outside the repository and may be pinned with
--expected-source-sha256. Existing destination files and manifests are
never replaced. A failed batch can leave earlier generated artifacts but
does not publish a final manifest.

The v2 **local, unsigned** manifest lists all expected package intervals,
marks unbuilt items not_built, and records real built files with byte
count, SHA-256, actual record-aligned JD boundaries and Rust source/slice
parity results. It explicitly declares
distribution_rights=review_pending and
publication_status=local_unsigned_not_authorized. Its structure is
*not* a signed download protocol: the app must not trust an arbitrary URL
or promote it to an installer resource without a separate security review.
Both the Mobile v1 and proposed Desktop v2 manifests may coexist.

## Scientific contract and invariants

- Source: classic little-endian JPL DE441 .eph, **not** NAIF .bsp or
  Swiss compressed .se1. Slices retain every coefficient in full double
  precision and adjust only the classic header's span.
- Catalog intervals: half-open **Julian Date TDB**, proleptic Gregorian
  year boundaries at January 1 00:00 TDB, astronomical numbering (0 = 1 BCE).
  Stored coefficient-record ranges overlap logical boundaries by design.
- The existing Rust evaluator returns geometric ICRF equatorial positions
  in km and velocities in km per TDB day; this planner **changes no
  interpolation, time conversion, coordinates or physical corrections**.
- De441PackManager::plan_contiguous_extension() returns a sequence of
  desired package specs, not a promise that files are present. Apps should
  use its ordered plan to download, verify SHA-256 and size, atomically
  activate one neighboring package at a time, and recompute coverage
  after a partial download. can_remove_contiguous_pack() enforces
  non-removal of required and interior packages.
- Any source/slice parity is **same-source** evidence, not independent
  astronomical validation against JPL/NAIF. Historical civil time, ΔT,
  frame accuracy and high-precision use beyond the validated models remain
  separate scientific gates.

## Planned distribution (not performed)

After explicit provenance/rights and scientific release approval, versioned
GitHub Release assets could store the packages from a single immutable
source fingerprint. An authenticated, versioned download catalog would map
each ID to an approved asset URL, SHA-256, exact byte count and TDB coverage.
The data catalog's version must change independently from the Tauri app
version; clients must not silently combine incompatible source generations.
No release may be made from the local unsigned manifest alone.

## Checks and remaining work

- Python synthetic catalog, mocked Rust export/parity and Mobile v1
  compatibility tests run without JPL data.
- Rust synthetic plan tests cover skipped historical bridges, required
  base, future boundary, missing/duplicate/noncontiguous installations,
  removal of interior packs and invalid dates. They need a Rust toolchain.
- Still required: real 1000–2200 slice with actual source, source/slice
  and independent JPL comparisons around every boundary; Cargo
  format/test/Clippy; package size measurements; Windows/macOS native
  download and failed-network tests; signed catalog and legal review;
  Desktop/Mobile UI integration and Android device QA.

Modification under CPAL-1.0 §3.3 (2026-11-19): original VirgoStars catalog
planning, local packaging orchestration, tests and documentation. No JPL
data, original source kernels, third-party software, app credentials,
release publication or production changes are part of this change.
