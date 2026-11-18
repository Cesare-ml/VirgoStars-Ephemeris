# VirgoStars Ephemeris

**A free and open-source astronomical ephemeris project for astrology applications — licensed under CPAL-1.0.**

**Status: EXPERIMENTAL — the Rust core, a local Node binding and an explicit backend-shaped facade are implemented and locally tested. Limited SPK type-2 and type-3 geometric comparisons against CSPICE passed with real kernels; scientific and provenance review, broader validation and production integration remain open.** This is an original implementation, not a fork, translation, wrapper or redistribution of Swiss Ephemeris. See the [current MVP integration checkpoint](docs/MVP1_RECONCILIATION_2026-11-16.md).


## Purpose

Provide verifiable astronomical quantities to [Astrological Functions](https://github.com/Cesare-ml/astrological-functions) and other clients, using NASA/JPL ephemerides with documented algorithms. The current experimental state reader supports the classic DE441 binary. Rust DAF/SPK **metadata indexing, type-2/3 J2000 direct states and ordered multi-kernel target–center chain composition** are experimental; non-J2000 frames, other SPK types and broader type-3/multi-kernel validation remain open. No login, database, Angular, artwork, interpretation texts, or VirgoStars application data belong here.

**Language decision:** The canonical astronomical core is written in **Rust**. An experimental native Node binding exists; WebAssembly and PyO3/maturin Python bindings are planned. See [ADR-0001](docs/ADR-0001-RUST-CORE-AND-BINDINGS.md).

**Separation of responsibilities:** `virgostars-ephemeris` handles time scales, kernel access, coordinates, physical/geometric corrections and eventually astronomical event primitives. `astrological-functions` handles zodiac/house methods, aspects, synodic phases, astrological events and application-facing chart models. VirgoStars production keeps using its current implementation until tested migration gates pass.

## Read first

- [Independent DE441 and apparent-pipeline validation, including the failing lunar baseline gate](docs/DE441_INDEPENDENT_VALIDATION_2026-11-18.md)
- [MVP readiness and remaining acceptance work](docs/MVP_READINESS_2026-11-17.md)
- [MVP 1 proposed priorities with stellar work paused](docs/MVP1_NEXT_PRIORITIES_2026-11-17.md)
- [Experimental planetary range 13000 BCE–3999 CE and historical-time diagnostic](docs/PLANET_VALIDATION_13000BCE_3999_2026-11-17.md)
- [Experimental direct-obliquity comparison and ancient ΔT models](docs/ANCIENT_OBLIQUITY_AND_DELTAT_2026-11-17.md)
- [Opt-in analytic solar-deflection rate and its validation limits](docs/EXPERIMENTAL_ANALYTIC_DEFLECTION_2026-11-18.md)
- [Emission refinement versus deflection-rate matrix](docs/EMISSION_DEFLECTION_MATRIX_2026-11-18.md)
- [Explicit historical ΔT sensitivity scenarios](docs/HISTORICAL_DELTAT_SCENARIOS_2026-11-18.md)
- [Ancient frame comparison and correction to rotation-fit residuals](docs/HORIZONS_FRAME_EPOCH_HOLDOUT_2026-11-18.md)
- [CPAL-1.0 license, with VirgoStars attribution in Exhibit B](LICENSE)
- [NOTICE of copyright and third-party exclusions](NOTICE)
- [Licensing, required attribution and credits](docs/LICENSING_AND_CREDITS.md)
- [Architecture and boundaries](docs/ARCHITECTURE.md)
- [Accepted Rust architecture decision (not an exhaustive architecture map)](docs/ADR-0001-RUST-CORE-AND-BINDINGS.md)
- [Native desktop integration handoff](docs/DESKTOP_NATIVE_INTEGRATION_2026-11-17.md)
- [Italian overview](docs/README.it.md)
- [Historical branch integration status](docs/BRANCH_INTEGRATION_STATUS_2026-11-10.md)
- [Engineering roadmap](docs/ROADMAP.md)
- [Astronomical accuracy and validation](docs/VALIDATION.md)
- [E1 SPK reference-matrix breadth audit (case coverage only)](docs/SPK_E1_REFERENCE_MATRIX_AUDIT_2026-11-13.md)
- [Kernel provenance and distribution preflight](docs/KERNEL_DATA_DISTRIBUTION_PREFLIGHT_2026-11-14.md)
- [Backend compatibility planning inventory (not a shipping API contract)](docs/BACKEND_SWISS_COMPATIBILITY_2026-11-10.md)
- [Contributor and AI-agent rules](AGENTS.md)
- [Sintesi italiana della decisione architetturale](docs/ADR-0001-RUST-CORE-AND-BINDINGS.md#sintesi-italiana)
- [Experimental DE441 reader](docs/DE441.md)
- [Experimental Rust DAF/SPK metadata index](docs/RUST_SPK_DAF_INDEX_2026-11-12.md)
- [Experimental read-only SPK descriptor coverage inspector (not JPL-validated)](docs/RUST_SPK_LOCAL_COVERAGE_INSPECTOR_2026-11-13.md)
- [Experimental offline SPK geometric target/center state query CLI (not independent JPL validation)](docs/SPK_OFFLINE_STATE_QUERY_2026-11-14.md)
- [Offline SPK direct-state diagnostic CSV report at an explicit TDB epoch (synthetic only)](docs/SPK_DIRECT_PROBE_CLI_2026-11-14.md)
- [Experimental SPK type-2/type-3 direct-segment states (not JPL-validated)](docs/RUST_SPK_TYPE2_TYPE3_DIRECT_STATE_2026-11-12.md)
- [Experimental SPK fixed-axis spherical longitude, latitude and per-second rates](docs/SPK_FIXED_AXES_SPHERICAL_2026-11-14.md)
- [Experimental single-kernel SPK center-chain states (not JPL-validated)](docs/RUST_SPK_CENTER_CHAINS_2026-11-12.md)
- [Experimental loaded-kernel precedence and cross-file states (not JPL-validated)](docs/RUST_SPK_KERNEL_SET_PRIORITY_2026-11-13.md)
- [Experimental effective SPK segment snapshot at one TDB epoch (metadata only)](docs/SPK_EFFECTIVE_SEGMENT_SNAPSHOT_2026-11-14.md)
- [Offline effective SPK source inventory CSV CLI (including uncovered targets; metadata only)](docs/SPK_EFFECTIVE_EPOCH_INVENTORY_CLI_2026-11-14.md)
- [Experimental same-TDB-epoch batch SPK target/center states (not JPL-validated)](docs/SPK_BATCH_RELATIVE_STATES_2026-11-14.md)
- [Offline SPK relative target/center pair diagnostics CSV CLI (partial failures retained)](docs/SPK_RELATIVE_PAIR_PROBE_CLI_2026-11-14.md)
- [Experimental SPK per-pair geometric diagnostic results (not JPL-validated)](docs/SPK_PER_PAIR_DIAGNOSTICS_2026-11-14.md)
- [Offline NAIF/SPICE SPK validation procedure](docs/RUST_SPK_NAIF_OFFLINE_VALIDATION_2026-11-13.md) and [local DE440s comparison](docs/SPK_NAIF_LOCAL_COMPARISON_2026-11-16.md)
- [Independent SPICE reference generator and ordered kernel SHA256 verification](scripts/spk_naif_reference.py)
- [First geometric planet positions for a time and place](docs/FIRST_PLANET_POSITIONS_2026-11-10.md)
- [Experimental geocentric light-time state](docs/GEOCENTRIC_LIGHT_TIME_2026-11-10.md)
- [Experimental annual-aberration direction in fixed ICRF axes](docs/ANNUAL_ABERRATION.md)
- [MVP-1 annual-aberration state with analytical reception-time velocities (not yet full apparent position)](docs/MVP1_ABERRATION_WITH_RATES_2026-11-11.md)
- [MVP-1 analytic DE441 Chebyshev acceleration for aberration rates (no finite-difference step)](docs/MVP1_DE441_ANALYTIC_ACCELERATION_2026-11-11.md)
- [MVP-1 complete light-time + aberration six fixed spherical fields/rates and shared degrees/AU export (not apparent of date)](docs/MVP1_ABERRATED_FIXED_SPHERICAL_2026-11-11.md)
- [MVP-1 geocentric apparent ecliptic of date with rates, locally tested on a real DE441 (solar deflection, civil time 1900–2027, evidence receipts; not integrated)](docs/MVP1_APPARENT_ECLIPTIC_OF_DATE_2026-11-16.md)
- [MVP-1 fixed-star calculation engine, locally compared; no catalog bundled or approved](docs/MVP1_FIXED_STARS_2026-11-16.md)
- [Fixed-star catalog: paused local checkpoint and resumption instructions](docs/STAR_CATALOG_PAUSED_2026-11-17.md)
- [MVP-1 Node binding scaffold (N-API), declared time policy, Swiss-shaped facade; not a drop-in replacement](docs/MVP1_NODE_BINDING_2026-11-16.md)
- [MVP-1 comparison with JPL Horizons: separations to 8 µas, frame difference understood](docs/MVP1_HORIZONS_COMPARISON_2026-11-16.md)
- [MVP-1 Swiss-style facade shape verified against swisseph 0.5.17 (listed calls)](docs/MVP1_SWISSEPH_SHAPE_2026-11-16.md)
- [MVP-1 measured ΔT (USNO files supplied by the caller) for UT inputs; Moon within 0.02″ of the reference 1972–2018](docs/MVP1_DELTA_T_MEASURED_2026-11-16.md)
- [Heliocentric and equatorial output compared with swisseph 0.5.17](docs/MVP1_HELIOCENTRIC_EQUATORIAL_2026-11-16.md)
- [Lunar nodes and apogees (bodies 10-13) compared with swisseph 0.5.17](docs/MVP2_LUNAR_POINTS_2026-11-16.md)
- [Chiron from a JPL small-body SPK (new type 21 reader), compared with Horizons and swisseph](docs/MVP2_CHIRON_2026-11-16.md)
- [Horizon coordinates, refraction, rise/set and transits (azalt, refrac, rise_trans), compared with swisseph 0.5.17](docs/MVP2_HORIZON_EVENTS_2026-11-16.md)
- [`swe_houses` facade: vertex, East Point, co-ascendants, cusps via astrological-functions, compared with swisseph 0.5.17](docs/MVP2_HOUSES_FACADE_2026-11-16.md)
- [Global solar and lunar eclipses (`swe_sol_eclipse_when_glob`, `swe_lun_eclipse_when`), compared with swisseph 0.5.17](docs/MVP2_ECLIPSES_2026-11-16.md)
- [Functional coverage target](docs/FEATURE_COVERAGE.md)
- [Backend-first Swiss-style compatibility inventory](docs/BACKEND_SWISS_COMPATIBILITY_2026-11-10.md)
- [Work packages for contributors and AI agents](docs/BACKEND_WORK_PACKAGES_2026-11-10.md)

## Scientific sources

- [NASA/JPL planetary ephemerides](https://ssd.jpl.nasa.gov/planets/eph_export.html)
- [NAIF generic SPK kernels](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/)
- [NAIF SPK required reading](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html)
- [NAIF rules governing kernels and toolkit](https://naif.jpl.nasa.gov/naif/rules.html)
- [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/)

**Important:** NASA/JPL public data and separately obtained ephemeris files are **not** licensed under this repository's CPAL-1.0 license. Do not commit .bsp/.eph files or copy NAIF toolkit / Swiss Ephemeris source code without a documented rights review. The SPK kernels' redistribution terms differ from those of the SPICE Toolkit.

## Project status

- Experimental Rust workspace: `Cargo.toml`, `crates/ephemeris-core/`, and the classic binary DE441 reader documented in [docs/DE441.md](docs/DE441.md).
- Experimental time-and-place CLI computes topocentric geometric positions using an externally supplied DE441 binary; see its documented time range and approximations.
- The experimental Rust `spk_index::inspect_spk` indexes NAIF DAF/SPK headers and segments in both IEEE byte orders. The `spk_direct_state::direct_spk_state` module evaluates primitive J2000/ICRF type-2/3 states. `spk_relative_state::relative_spk_state` combines states within **one kernel**, while the experimental `spk_kernel_set::SpkKernelSet` applies **later-loaded SPK precedence** and composes cross-file center chains. Non-J2000 frames and broader independent validation remain open.
- An experimental **offline** Rust SPK comparison example accepts externally obtained NAIF/SPICE geometric state vectors, ordered SPK files and explicit error tolerances; the separate Python 3.10+ standard-library `scripts/verify_spk_kernel_hashes.py` checks declared ordered SHA-256 digests against actual kernel bytes before comparison. The Rust example does not invoke that preflight automatically. Limited [real-kernel comparisons](docs/SPK_NAIF_LOCAL_COMPARISON_2026-11-16.md) passed locally and remain subject to review.
- The validation-only Python generator `scripts/spk_naif_reference.py` can use an independently installed NAIF/SPICE Toolkit via SpiceyPy to produce geometric reference vectors and verify ordered kernel/source hashes; it is **not** a second production ephemeris engine.
- The optional `python scripts/spk_naif_reference.py run` mode combines local SPICE vector generation, offline Rust comparison, pre/post kernel checks and a **review-pending, unsigned JSON audit receipt** written only after success; no astronomical accuracy certification is implied.
- The optional `python scripts/spk_naif_reference.py compare` command checks ordered SPK kernel bytes and reference fingerprints, runs the canonical Rust NAIF comparator with Cargo offline, then rechecks the files even on failure.
- The historical JavaScript prototypes and synthetic tests are research artifacts, not a second production astronomical engine.
- Real NASA/JPL reference-vector validation and broadly validated SPK support remain open gates; passing synthetic tests does not establish scientific accuracy.
- Rust-backed WebAssembly/TypeScript and Python bindings, stable public releases and production integration are not implemented.
- GitHub Actions `Rust development CI` is present on `dev/ephemeris` and checks Rust formatting, debug/release tests, strict Clippy, Rustdoc, Windows GNU cross-compilation, and native Windows/macOS tests. Passing CI validates these code checks, **not** astronomical accuracy against independent NASA/JPL data.

**License:** Original VirgoStars-Ephemeris code is provided under [CPAL-1.0](LICENSE), including its Exhibits A/B, with required attribution and network-use provisions. See [NOTICE](NOTICE) and [Licensing and visible credits](docs/LICENSING_AND_CREDITS.md). A Credits/About link is encouraged but does not necessarily satisfy the license's launch/session prominence requirement.

This project is not affiliated with or endorsed by NASA, JPL, NAIF or Astrodienst. External datasets and dependencies retain their own terms.

The historical standalone JavaScript SPK, precession, nutation and light-time
prototypes and their tests remain research fixtures. Rust is the canonical
astronomical core; none of the prototypes is a production provider.
