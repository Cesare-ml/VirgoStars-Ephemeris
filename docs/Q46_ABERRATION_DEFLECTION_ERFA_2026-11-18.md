# Q46: isolating aberration and solar deflection with ERFA — 2026-11-18

**Status: exploratory independent implementation comparison.** In the sampled cases, changes in the apparent direction due separately to aberration and solar deflection have the magnitudes needed to explain major Q45/Q46 discrepancies. This does not establish Horizons' internal Q46 algorithm. A [draft inquiry to JPL](JPL_HORIZONS_Q46_CLARIFICATION_DRAFT_2026-11-18.md) is prepared and has **not** been sent. The experiment follows the [previous Q46 study](HORIZONS_Q46_DISCREPANCY_2026-11-18.md); its [grid and thresholds](Q46_ERFA_SEPARATION_PLAN_2026-11-18.md) were recorded before the new queries.

## Method and conditions

For Moon 301, Mercury 199, Venus 299 and Jupiter system barycenter 5 observed from Earth center 399, query Horizons Q1 astrometric, Q45 apparent airless and Q46 rates, ICRF and DE441, at five TDB dates `2451580, 2451620, 2451674, 2451740, 2451820`. Each uses JD TT converted by the existing Rust approximation for Horizons, with samples ±0.005 and ±0.01 TDB day. All outputs are ″/hour in the local ICRF RA/declination tangent basis; the component difference Q45−Q1 is projected on the Q1 center basis, while Q45's own rate uses the Q45 center basis. Q46 is never treated as an independent apparent-position derivative by definition.

The Rust example exports the existing classic-DE441 light-time geometry and correction-stage positions; it does not create a second core. Independent [ERFA `eraLd`](https://github.com/liberfa/erfa/blob/master/src/ld.c) computes finite-distance solar deflection, and [ERFA `eraAb`](https://github.com/liberfa/erfa/blob/master/src/ab.c) computes aberration applied after deflection. Both receive the same target, observer and Sun geometry as Rust, including target/Sun at the emission epoch. The model and ephemeris are shared: independent implementation does not mean independent physical orbit or a model-free measurement. ERFA includes a small solar-potential aberration term absent from the Rust pure Lorentz boost; its effect, printed-angle precision, approximate TT/TDB conversion, input epoch quantization and finite-difference truncation limit interpretation.

Before querying, component-wise diagnostic limits were set at 0.0005″/hour for ERFA versus Rust isolated correction rates, 0.005″/hour for the sum of ERFA correction rates versus the Horizons Q45−Q1 derivative, and 0.001″/hour for sensitivity to the two half-steps. These are test limits, not physical uncertainties or release acceptance budgets.

## Results

For **20 cases at each of two half-steps**:

| Comparison | Maximum absolute component difference | Above limit |
| --- | ---: | ---: |
| ERFA versus Rust, deflection rate | 7.93e−11″/h | 0/40 |
| ERFA versus Rust, aberration rate | 3.70e−9″/h | 0/40 |
| ERFA deflection + aberration versus Horizons Q45−Q1 rate | 0.0000201″/h | 0/40 |
| Difference between half-steps, Horizons Q45−Q1 | 0.00002994″/h | 0/20 |

The measured step sensitivity of the ERFA deflection and aberration components also stayed below 0.001″/hour. ERFA and Rust share the same published deflection formulation and supplied geometry, so their agreement is a code-level comparison, not proof that the model is complete. The data and fixed-stencil calculations are available for review in the local report identified below.

At JD TDB **2451674.0**, the same samples illustrate why isolating the stages matters. Values in the Q1 tangent basis are ″/hour:

| Target | Solar elongation | ERFA deflection (RA, Dec) | ERFA aberration (RA, Dec) | Horizons derivative of Q45−Q1 (RA, Dec) |
| --- | ---: | ---: | ---: | ---: |
| Moon | 72.590° | (+0.000000016, −0.000000002) | (+0.186212, −0.024405) | (+0.186221, −0.024403) |
| Mercury | 0.416° | (−0.028359, −0.015111) | (+0.000601, +0.000205) | (−0.027763, −0.014890) |
| Jupiter barycenter | 1.295° | (−0.001141, +0.006871) | (+0.000244, +0.000296) | (−0.000899, +0.007182) |

The Moon's observed Q45−Q46 difference is about (+0.18566, −0.02442)″/hour in the apparent tangent basis, comparable to the independently calculated aberration term. For Mercury, Q45−Q46 is about (−0.02242, −0.01289)″/hour, with a large deflection-rate component. These are **not** exact equalities: light-time derivative, cross terms, differing tangent bases, precision and models also contribute. Mercury was intentionally selected as a known near-solar stress case; at 0.416° it lies within a few solar radii in apparent separation, outside the Rust model's well-supported geometry. These numerical outcomes are not claims of physical accuracy or visibility near the Sun. The smallest sampled deflection denominator is 0.000369, above the model floor 1e−6 at the central epochs; that does not establish behavior at the floor.

## Reproduction and checks

The local classic kernel SHA-256 is `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`. With a caller-supplied pyerfa and NumPy and network access:

```text
cargo build --release --offline -p virgostars-ephemeris-core --example audit_correction_geometry
python scripts/audit_q46_corrections_erfa.py <kernel> <kernel-sha256> <example-executable> <new-report.json>
```

This run used Python 3.12.14, pyerfa 2.0.1.5 and NumPy 2.5.5 on Windows. The script checks kernel and executable hashes before and after and refuses an existing output. It retains the exact four Horizons URLs, responses and all per-case results locally, writes the complete report even if a diagnostic limit fails, and exits nonzero on failures. `q46-erfa-isolation-final.json` SHA-256 is `831c98b9d4cecc99a15d38d5dca3952fc813f453bcc4e69e7851c32c2d1ea1d5`; the kernel and raw responses are not committed. A new run can legitimately differ as Horizons' service and timestamps change.

Windows checks: `cargo test --workspace --offline` passed 508 tests, with 31 ignored; the four ignored real-DE441 apparent tests were run separately and passed 4/4. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --offline -- -D warnings`, the release build of the diagnostic example and Python syntax validation passed. The ERFA/Horizons diagnostic ran twice, including the final automated threshold checks above.

These checks do not validate ecliptic-of-date speeds, radial rates, the whole historical interval or the DE441 orbit against a separate ephemeris. Q46's actual convention remains an external question for JPL. No Mobile engine is activated by this work.
