# Real classic-DE441 gate for light-time + annual aberration

**Date: 2026-11-12. Status: test harness implemented, real-kernel execution NOT yet performed for this new test.**

## Why this exists

The synthetic regression tests in the draft Rust light-time / observer-aberration / civil-time stack prove algorithm composition, derivative consistency and error handling, but they are not numerical evidence on a **real** ephemeris kernel. The original repo already has four independently recorded **JPL Horizons VEC_CORR=LT** positions and one-way light times in `crates/ephemeris-core/tests/light_time_horizons.rs`, queried on 2026-11-10. This PR adds an **optional ignored regression** for the **composed aberrated** fixed-axis pipeline on that same official reference date, without shipping a kernel or duplicating the numerical algorithms.

New file: `crates/ephemeris-core/tests/aberrated_real_de441.rs`. Existing `lib.rs`, source astronomy functions, ordinary synthetic tests and previously pinned JPL reference values remain unchanged.

## Independent reference provenance and exact conventions

The four literal XYZ vectors and one-way light times in the test are directly copied from the repository's existing, independently recorded Horizons LT numerical reference (`tests/light_time_horizons.rs`) to ensure the same scientific benchmark and body semantics:

- **Provider:** NASA/JPL Horizons (queried 2026-11-10; output reported DE441 for both target and Earth).
- **Epoch:** `2461354.5` **TDB Julian date**. The test deliberately does **not** pass a UTC timestamp here: adding a guessed UTC→TDB offset would compare different instants.
- **Ephemeris:** `EPHEM_TYPE=VECTORS`, `MAKE_EPHEM=YES`, `CENTER='500@399'`, `TIME_TYPE=TDB`, `TLIST='2461354.5'`, `TLIST_TYPE=JD`, `REF_SYSTEM=ICRF`, `REF_PLANE=FRAME`, `OUT_UNITS=KM-S`, `VEC_TABLE=3`, **`VEC_CORR=LT`**.
- **Targets:** Moon `301`, Venus `299` / `2` as resolved by Horizons, Mars **system barycenter** `4`, Jupiter **system barycenter** `5`. No mistaken physical planet-center reassignment.
- **Units:** XYZ in km, one-way light time in seconds; the test uses the original Rust `JulianDateTdb(2_461_354.5)`.

Reference documentation: [NASA Horizons HTTP API](https://ssd-api.jpl.nasa.gov/doc/horizons.html) and [Horizons vector table manual](https://ssd.jpl.nasa.gov/horizons/manual.html).

**Essential distinction:** this is a reference for **light-time only (LT)**, *not* a direct Horizons `LT+S` numerical fixture. The Rust pipeline applies a **special-relativistic** Lorentz aberration, while common Horizons/NAIF `LT+S` options are documented as a Newtonian stellar-aberration correction. Do not compare these as exactly identical at machine precision. The new regression checks the aberrated pipeline **after removing the same Rust aberration boost** by applying the inverse Earth reception velocity, then compares that recovered light-time direction, range and light-time to **independently recorded Horizons LT** components. Inverse boosting using the same Rust implementation does **NOT independently validate** the boost formula or full apparent coordinates; the independent numeric gate applies to the recovered LT geometry only.

## Two new opt-in tests

1. `real_de441_aberrated_pipeline_recovers_horizons_light_time_geometry`
   - Opens the **separately supplied classic** DE441 `.eph` via `DE441_PATH`.
   - Gets Earth's actual barycentric reception velocity from the same real file.
   - Calls `geocentric_aberrated_fixed_spherical` for each of the four reference targets, reconstructs the full corrected ICRF Cartesian vector from its spherical fields, inverse boosts its direction with **negative** Earth barycentric velocity and retains the corrected range.
   - Compares every recovered LT XYZ vector to Horizons with the existing **0.01 km** maximum-component tolerance, range residual under **0.01 km**, and light-time residual under **1e-6 s**. These are targeted validation limits for the specified reference, not full accuracy guarantees.
   - Verifies exact `Body` and Earth-center metadata and distinct emission epoch.

2. `real_de441_aberrated_batch_rates_follow_neighboring_full_pipeline_states`
   - Uses the existing same-epoch multi-body pipeline on a real classic DE441 reader for the same four bodies at **TDB ±600 s**, plus the center.
   - Independently computes central differences of **all three** fixed-axis outputs (including circular longitude wrap, latitude and range), preserving reception TDB day units and system-barycenter targets.
   - Compares these *numerical derivative estimates* to the returned analytically propagated rates. The initial **2e-4 degree/TDB-day** angular and **1e-7 AU/TDB-day** radial limits are deliberately **provisional** until locally executed, reviewed and calibrated against real independent rate references. They are **not** an assertion of achieved scientific precision or externally verified velocity parity.
   - Does not compare the output speeds to Horizons `VEC_CORR=LT` instantaneous velocity columns, which have a different derivative convention from the current analytic reception-time state.

Both tests are **`#[ignore]`** so CI does **not** require a 2.8 GB binary or external network calls. They are compiled on Linux/Windows/macOS and await an explicitly configured local real-kernel run.

## Run on a machine with an independently obtained classic DE441 binary

From repository root, with a supported Rust toolchain:

```powershell
$env:DE441_PATH = "D:\Astronomy\de441.eph" # change to your actual local file
cargo test --workspace --test aberrated_real_de441 -- --ignored --nocapture
```

For ordinary offline CI with only built-in synthetic mathematical fixtures:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**Do not run `cargo test --workspace -- --ignored` on a CI runner without a real file**: these two tests require `DE441_PATH` and intentionally fail when a required binary was not supplied.

The local test output prints per-target absolute error magnitudes; record the actual file identity/size, SHA256, exact Rust platform/commands, reference query and results before claiming evidence. A passing result does not establish independent Horizons `LT+S` accuracy, true ecliptic-of-date speeds, Swiss `swe_calc_ut` parity, moving-frame transforms, solar deflection or SPK kernel support. Those are separate scientific gates.

## License, provenance and integration

**CPAL-1.0 modification record (2026-11-12):** new original Rust integration-test source `tests/aberrated_real_de441.rs` with complete existing Exhibit A notice; this English documentation. No NASA/NAIF/Swiss source, SPK toolkit, external kernels, JPL binaries or additional third-party dependencies are redistributed or committed. Original CPAL `LICENSE` and `NOTICE` remain intact. Applicable attribution: **Powered by VirgoStars** — https://www.virgostars.it/.

This test-only PR is stacked on **draft #118** (which integrates the civil-time adapter and shared-Earth optimization) and depends on its upstream **#114 / #107 / #77 / #96** work. Keep it **draft**, do not merge it into `dev/ephemeris` or `main` before explicit review/owner approval, and do not deploy, release or publish packages.
