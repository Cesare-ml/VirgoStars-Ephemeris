# DE441 package orientation extension assessment — 2026-11-18

**Recommendation: retain the ordinary guard and continue with an explicitly selected long-term experiment; do not simply widen the guard or silently switch models.** The existing experimental Rust path computes across sampled dates in all ten local packages, with useful agreement against ERFA and JPL. That does not establish physical Earth-orientation accuracy for the full interval or authorize enabling it in Mobile. The [plan and diagnostic thresholds](DE441_ORIENTATION_EXTENSION_PLAN_2026-11-18.md) were recorded before reference queries.

## What the boundary means

The ordinary IAU 2000B nutation implementation deliberately checks TT JD **[2415020.0, 2488070.0)**: 1899-12-31 12:00 TT inclusive to 2100-01-01 12:00 TT exclusive. Six-field rates need the orientation at TT ±0.01 day, further narrowing usable centers. This is a **numerical/model policy guard**, not a DE441 file limit. It is motivated by the model's modern-epoch approximation: [ERFA's 2000B documentation](https://github.com/liberfa/erfa/blob/master/src/nut00b.c) gives approximately milliarcsecond performance around 1900–2100, not a guarantee throughout years 0–2300. The exact endpoint and half-step are implementation choices; the guard is not itself a physical-accuracy proof.

The local manifest's base package logically requests years 1600–2100 inclusive (TDB `[2305447.5, 2488434.5)`); eight optional historical slices cover years 0–1599 and one future slice covers 2101–2300, with year 0 = 1 BCE. Stored records extend beyond logical boundaries. Therefore the ordinary apparent orientation covers only part of the base and does not cover the optional historical/future ranges; most of calendar 2100 is outside its orientation guard. The package range is a **TDB data** property, the orientation guard is in **TT**. Having coefficients for a date does not prove that an apparent frame or civil time is supported.

## Existing alternative: opt-in experiment

`experimental_long_term_apparent_ecliptic` already combines the Vondrák long-term pole-derived mean equator/ecliptic, IAU 2006 J2000 bias, and the 77-term nutation series evaluated with full IERS polynomial arguments. This extrapolated series is **not standard IAU 2000B**. It uses light time, solar deflection and annual aberration, includes an experimental first-order emission-epoch refinement and a numerical moving-frame derivative, and accepts explicit TDB rather than resolving historical civil time. The separately named direct-obliquity alternative is a mixed frame and was not selected in this audit. Neither experiment changes the ordinary path.

On **372 TT epochs** (four seasons every 25 years from astronomical 0 through 2300), Rust and independent ERFA `ltp`/`ltpecl` implementations of the same Vondrák model agreed within 4.45e−16 per mean-equator matrix element and 5.56e−17 per ecliptic-pole element, below the predeclared 1e−12 diagnostic limit. Comparing the experimental 77-term nutation with ERFA full IAU 2000A gives maximum component difference **0.002097″** (year 1000), below the predeclared 0.05″ diagnostic. Unguarded ERFA standard 2000B compared with 2000A differs by up to **0.228591″** at year 25, 0.009261″ within sampled years 1600–2100 and 0.005057″ within sampled years 2101–2300. These are **model differences**, not measured physical errors, continuous bounds or an acceptance budget. The full [ERFA 2000A implementation](https://github.com/liberfa/erfa/blob/master/src/nut00a.c) also omits free-core nutation; neither model substitutes for observed Earth-orientation corrections.

## Full file, local slices and Horizons

The source and all ten local package files were read-only checked against manifest sizes and SHA-256 both before and after. On each package, experimental six-field outputs were compared with the source at logical start+1 day and end-exclusive−1 day for ten targets: **200/200** checks passed. A separately planned direct-reader check at each logical start, start+1 second, and end−1 second gave **300/300** checks within the same thresholds. All six output fields were equal at the examples' printed precision; this is not a claim of bitwise equality. This tests reading each supplied slice directly, **not** `De441PackManager` selection, mobile installation, or a missing-package flow. `distribution_rights` remains `review_pending`.

For 26 epochs (the 20 one-day-inside slice samples, four guard-neighbor samples and two modern controls) and ten geocentric targets, JPL Horizons Q45 supplies apparent airless ICRF directions and Q20 supplies apparent distances. ERFA rotates each Q45 direction to the pole-based long-term ecliptic using Vondrák `ltp`/`ltpecl`, J2000 bias and **full 2000A** longitude nutation. Thus the frame precession implementation is independently coded and the nutation is a different theoretical model; both sides use DE441 and the comparison is not independent orbital truth. Positions and two fourth-order rate stencils were compared using the actual requested TT epochs; TT/TDB conversion is approximate.

| Comparison | Maximum absolute component residual | Predeclared diagnostic limit | Failures |
| --- | ---: | ---: | ---: |
| Longitude | 0.001162″ | 1″ | 0/260 |
| Latitude | 0.00000678″ | 1″ | 0/260 |
| Distance | 1.184e−11 AU | 1e−9 AU | 0/260 |
| Longitude rate | 0.003323″/day | 0.1″/day | 0/520 |
| Latitude rate | 0.0001365″/day | 0.1″/day | 0/520 |
| Distance rate | 1.243e−10 AU/day | 5e−10 AU/day | 0/520 |

These are sparse sampled diagnostics, not accepted scientific accuracy limits. The minimum non-Sun elongation was Venus at 0.523°, a near-solar stress case; the model is not validated at the solar limb or in occultation. Direct Q20 `deldot` differed from the derivative of Q20 distance beyond the separate 0.001 km/s limit in **60/520** stencil checks (30/260 target/epoch cases), with maximum **0.008782 km/s**. As in the [prior Q20 study](DE441_ECLIPTIC_RADIAL_RATE_VALIDATION_2026-11-18.md), these failures remain explicit and the Q20 convention has not been confirmed by JPL.

Horizons Q31 uses a different IAU76/80/Owen ecliptic-of-date convention and, in its applicable era, EOP corrections. The maximum candidate-minus-Q31 component differences are **3.032″ longitude** (Venus, year 0 sample) and **0.05259″ latitude** (Saturn barycenter, year 2300 sample). This is a cross-convention comparison, not an equal-frame JPL error, an adopted tolerance or proof which model is physically better. See the [Horizons reference-frame description](https://ssd.jpl.nasa.gov/horizons/manual.html).

## Transition considerations and proposed path

All **60/60** standard-path guard expectations passed across four guard-neighbor dates plus two modern controls and ten targets: the standard path remained fail-closed outside the guard and succeeded at the selected inside/control dates. On the upper inside date, the largest whole-pipeline experimental-minus-standard longitude difference was **0.000930″** (Moon) and the largest longitude-rate difference was **0.00005724″/day** (Moon). This comparison includes the experimental emission refinement and model/frame differences; it is not a pure frame discontinuity.

A separately planned post-hoc ERFA-only model comparison used full 2000A nutation on both sides. The relative Vondrák versus IAU 2006 ecliptic-frame rotation was about **0.000223″** at the upper guard and **0.000280″** at the lower guard. This isolates a theoretical frame contribution, not the full Rust pipeline transition, a measured physical error, or a complete acceptance criterion. No automatic switching was introduced.

For an eventual extension:

1. Keep the guarded standard result as the default and expose any long-term calculation only under an **explicit model/policy selection** with model identity, TDB/TT scale, documented limitations and typed failure; do not silently substitute it when the standard call rejects a date.
2. Define stakeholder-approved angular and rate budgets and investigate sensitivity to nutation, precession/ecliptic convention, free-core nutation/EOP and historical/future uncertainty. Sample intermediate dates, exact package handoffs, stations and near-Sun exclusion zones; verify single-model continuity and any intentional model-switch offsets on matching physical inputs. The present tests are not sufficient to approve automatic activation.
3. Integrate with the actual multi-pack provider and test coverage, missing/corrupt files, the reception/emission and derivative stencils across handoffs, including exclusive end behavior. The current checks open one slice directly. Preserve typed errors and no fallback.
4. Treat civil-time conversion and ΔT/UT1 as separate gates. The inspected Mobile diagnostic civil conversion is limited to 1972–2027 and is not a historical-time solution. Review DE441 distribution rights, Android operation, product semantics and the remaining Q20/Q46 convention questions independently.

## Reproduction and retained evidence

With the caller-owned classic DE441, all ten local manifest v1 slices, Python 3.12, pyerfa 2.0.1.5, NumPy 2.5.5 and network access for Horizons, from the repository root:

```text
cargo build --release --offline -p virgostars-ephemeris-core --example audit_long_orientation_models --example audit_pack_orientation_candidate
python scripts/audit_package_orientation_erfa.py <model-exe> <new-erfa-report.json>
python scripts/audit_package_orientation_horizons.py <kernel> <manifest> <pack-dir> <candidate-exe> <new-horizons-report.json>
python scripts/audit_orientation_pack_edges.py <kernel> <manifest> <pack-dir> <candidate-exe> <new-edges-report.json>
python scripts/audit_package_orientation_frame_delta.py <horizons-report.json> <horizons-report-sha256> <new-frame-report.json>
```

The Horizons command writes its full report and exits **1** because the separate direct Q20 threshold fails; position/rate and pack checks above pass. Results are local; no JPL response or DE441 artifact is committed. The source SHA-256 is `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`; the local manifest SHA-256 is `a4a204169555ee4ec01d5b3f25ce331a44ff6c9fd273d7f812cfa245e6ef6b7a`. Evidence SHA-256:

- `package-orientation-erfa.json`: `cf351779a5c4341fd5d2390a1f7a22a3c36e7acf90eb696237ea3332abe7ff54`.
- `package-orientation-horizons.json`: `89227c6342d5c00781e32c99ab31e7b2ffefb5d5ca5f6f0d819ea29c0254c808`.
- `package-orientation-frame-delta.json`: `02b43efd0a3948d79cebad79e5f09b4bb633c2e0f44c2bbdccb2340d6fc0a0b0`.
- `package-orientation-exact-edges.json`: `8bc045a3c64db06776c77b9cbff7d557dd7549a201ae6e6ff5cfde56824945cd`.

Local Windows checks: 510 Rust workspace tests passed with 31 ignored; four ignored real-DE441 apparent tests passed separately. Rust fmt, Clippy, the independent ERFA fixture test, existing Python numerical tests and Python syntax checks passed. No app engine, ordinary guard, release or merge was changed.
