# ADR-0001 — Rust astronomical core and cross-language bindings

**Status:** Accepted architectural decision. An experimental Rust DE441 reader exists; bindings, package release, independent numerical validation and production migration remain future work.
**Date:** 2026-11-09
**Scope:** `VirgoStars-Ephemeris` and its consumers, including `astrological-functions`, VirgoStars, Astrolabio and third-party applications.

## Decision

Implement the **only authoritative astronomical engine in Rust**. Do not maintain separate astronomy algorithms in TypeScript, Python, or other languages. Public language interfaces are thin wrappers over that same Rust implementation, with explicit cross-language contracts and shared reference fixtures.

- **Rust core**: input validation, time scales (UTC/TAI/TT/TDB as applicable), units, planetary SPK reading/interpolation, frame transformations, observer/geocentric/topocentric geometry and documented physical corrections; a future separately identified analytic **lite mode** for calculations without external kernel files.
- **TypeScript / JavaScript**: wrapper around WebAssembly via `wasm-bindgen` for browsers and supported Node.js use. Consider an optional Node native addon through `napi-rs` *only if measurements justify the additional maintenance*; it still calls the same Rust core.
- **Python**: a compiled extension using **PyO3**, packaged into installable wheels using **maturin**. This is a binding to the Rust core, **not** a Python rewrite. Depending on the published wheel/platform, users can `pip install` and import it without a separate Rust compiler; source builds may require Rust.
- **Rust**: expose a stable public library API for native consumers. C ABI bindings for other languages are a possible future addition, subject to demand and ABI/versioning design.

The astronomical engine is distinct from **`astrological-functions`**, which remains a framework-independent **TypeScript** package for zodiacs, house systems, aspects, chart/transit orchestration, and other astrological logic. Do not move astrological domain calculations into the Rust ephemeris crate by default.

## Proposed monorepo layout (not yet created)

```text
VirgoStars-Ephemeris/
  crates/
    ephemeris-core/           # Rust: single source of astronomical truth
    ephemeris-spk/            # Rust: optional SPK loader/interpolator
  bindings/
    wasm/                     # wasm-bindgen exports + TypeScript wrapper
    python/                   # PyO3 + maturin / Python wheels
    node-native/              # optional future napi-rs package
  tests/
    fixtures/                 # independent JPL Horizons comparisons
    cross-language/           # same result from Rust/JS/Python
  docs/
    ADR-0001-RUST-CORE-AND-BINDINGS.md
```

The core crate now exists; other directory names above are proposals, not a claim that those packages already exist. Use feature boundaries that permit smaller artifacts and optional dependencies.

## Public result contract

Define and version the public API before implementation:
- Explicit body and observer IDs, reference frame, center, coordinate kind (geometric/astrometric/apparent), units and reference epoch/time scale. Never silently mix UTC and TDB or tropical and sidereal conventions.
- Position/velocity, if applicable, returned with clear units and metadata; errors for missing kernel, insufficient date coverage, unsupported SPK type, invalid observer, and unsupported frame.
- The same input specification and scientific options must yield numerically equivalent results from Rust, TypeScript and Python within documented floating-point tolerances.
- Internals use Rust `f64` where appropriate, with numerical validation; accuracy is determined by data, models and corrections, **not guaranteed by choosing Rust**.
- File path/bytes APIs should not require a live HTTP server. For WASM, permit explicit kernel bytes/files where viable rather than assuming filesystem access.

## Python usage goal

A future distribution could support:

```bash
pip install virgostars-ephemeris
```

```python
import virgostars_ephemeris as ve

# Illustrative ONLY; module/functions/arguments are not implemented yet.
# result = ve.position(body="mars", epoch="2026-11-09T12:00:00Z",
#                      observer="earth", kernel_path="/path/to/de440.bsp")
```

Actual module names, API signatures and supported wheels will be confirmed by implementation/testing. Python users will call the compiled Rust core via PyO3, not through a subprocess or a second math implementation. Design batch APIs to avoid unnecessary per-call Python↔Rust overhead.

## Delivery order

1. Specify canonical Rust models, SPK coverage and frames; add independent astronomical reference fixtures and tolerance budgets.
2. Implement and validate the Rust kernel reader/interpolator, time scales and coordinate/correction pipeline against JPL Horizons/NAIF references.
3. Expose a minimal WASM/TypeScript adapter used by `astrological-functions`; test cross-language parity.
4. Add PyO3/maturin Python wheels for selected platforms, with an explicit support matrix and parity tests.
5. Evaluate optional native Node bindings only after benchmarks. Later implement and **separately validate** an analytic/offline lite mode with declared lower precision.

## Legal, operational and migration guardrails

- Clean-room original implementation: no Swiss Ephemeris source, copied formulas/code, wrapper or vendored SPICE Toolkit without review. Cite scientific algorithms/specifications used.
- NASA/JPL kernel availability is not permission to bundle or sublicense third-party data under the repository's CPAL-1.0 code license. Document provenance and distribution rights; default to externally supplied data where needed.
- Original VirgoStars-Ephemeris code is licensed under **CPAL-1.0**, including Exhibits A/B; see `LICENSE`, `NOTICE`, and `docs/LICENSING_AND_CREDITS.md`. A Credits/About entry is recommended, but for graphical Larger Works the license's Section 14 launch/session display requirements must also be considered. Preserve third-party license notices and review dependency compatibility.
- Do not change production VirgoStars/Astrolabio, migrate users, or publish npm/PyPI packages as a side effect of this documentation decision. The owner explicitly authorized non-deploying GitHub Actions CI on 2026-11-10; use least-privilege permissions and no unnecessary secrets.
- Record tests for Windows, macOS and Linux (and WASM browser/Node) as support targets, not existing support.

## Sintesi italiana

Il motore astronomico di VirgoStars Ephemeris è scritto **una sola volta in Rust**. TypeScript, Python e browser utilizzeranno lo stesso motore attraverso binding (rispettivamente WASM/JS e PyO3 per Python). L'API Python sarà installabile con `pip` dove esistono wheel compatibili, senza riscrivere i calcoli astronomici. `astrological-functions` resta in TypeScript per la logica astrologica. È disponibile un lettore DE441 sperimentale; i binding sono ancora da implementare.
