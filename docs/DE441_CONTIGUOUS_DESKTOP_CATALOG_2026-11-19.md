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

    # Create unsigned, local release-shaped metadata only after all packs verify.
    python scripts/build_de441_contiguous_packs.py release-catalog \
      --manifest /local/virgostars-desktop-packs/de441-contiguous-manifest-v2.json \
      --output-dir /local/virgostars-catalog --release-tag de441-v1-local-test

Use --packs all for all configured intervals, or --packs PACKAGE_ID
for selected IDs, with a **new output directory per manifest**. Configure
--min-year and --max-year *before* the build/plan subcommand if
the actual source covers different years. The defaults of -13000 to 16999
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

The builder and verifier now invoke the Rust calendar for the exact JD TDB
boundaries of every built artifact. Rust and its offline dependencies are
therefore required for verification too. An artifact whose logical JD
interval does not match its declared astronomical years is rejected even
when its header and hash are internally consistent. The manifest remains
unsigned and this check is not catalog authentication.

On Windows on 2026-11-19, the owner's 2,788,676,624-byte classic source
reported JD TDB [-3027215.5, 7930192.5]. Its complete Gregorian years
are -13000 through 16999; the current year-based catalog proposes 145
packs, but leaves the partial source years at both ends unrepresented.
All 145 whole-year slices were generated privately, hashed, checked against
their canonical year boundaries and compared with that same source. They
total 2,792,137,824 bytes; the 1000–2200 base is 111,662,384 bytes.
The resulting unsigned local release-shaped catalog passed the Desktop
catalog contract and its year-zero extension plan. This is not independent
astronomical validation or permission to redistribute the data.

The local file's MD5 is `4e3b924463d17b68ec9c4a18240300cd`; the
[Astrodienst download page](https://www.astro.com/swisseph/swedownload_e.htm)
lists `a7b2a5b8b2ebed52ea4da2304958053b` for its DE441 file. These
fingerprints differ. The local file's origin and any right to redistribute
its slices therefore remain unverified; MD5 alone would not authenticate
provenance even if it matched. Do not upload these artifacts or bundle them
in a public installer until the specific source and derived-file rights have
been reviewed.

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
GitHub currently allows up to 1,000 assets per release, each under 2 GiB
([GitHub release limits](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)).
The 145 proposed whole-year assets fit the count limit and the three measured
files fit the individual size limit; the other files have not been measured.
When immutable releases are enabled, assets and their tag cannot be changed
after publication ([GitHub release management](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)).
Neither immutability nor a hash supplied by an unsigned catalog authenticates
the publisher.

## Checks and remaining work

- Python synthetic catalog, mocked Rust export/parity and Mobile v1
  compatibility tests run without JPL data.
- Rust synthetic plan tests cover skipped historical bridges, required
  base, future boundary, missing/duplicate/noncontiguous installations,
  removal of interior packs and invalid dates. They need a Rust toolchain.
- Still required: support for partial source years and independent JPL
  comparisons around every boundary;
  Windows/macOS native
  download and failed-network tests; signed catalog and legal review;
  Desktop/Mobile UI integration and Android device QA.

Modification under CPAL-1.0 §3.3 (2026-11-19): original VirgoStars catalog
planning, local packaging orchestration, tests and documentation. No JPL
data, original source kernels, third-party software, app credentials,
release publication or production changes are part of this change.
