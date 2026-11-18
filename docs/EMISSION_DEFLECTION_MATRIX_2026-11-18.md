# Separating emission refinement and solar-deflection rate

**Status: exploratory, isolated.** The four paths below are experiments in
geocentric apparent ICRF at TDB reception, using light-time, solar deflection
and annual aberration. They are not a new production default. This continues
the [analytic-rate experiment](EXPERIMENTAL_ANALYTIC_DEFLECTION_2026-11-18.md)
and uses the [existing Horizons fit sample](HORIZONS_DENSE_RATE_HOLDOUT_2026-11-18.md).

## Method

For the same body and epoch calculate the existing numerical and experimental
analytic deflection rates, each with and without the existing first-order
sub-ULP refinement of the light-time vector. The newly added Rust function
`experimental_analytic_apparent_icrf_refined` selects that combination
explicitly; the existing unrefined function retains its behavior.

This is a comparison of the implemented paths, not a fully precise four-way
split-JD calculation. In the refined paths, the target and Sun used by the
deflection geometry are still sampled at the rounded emission JD. The old
numerical deflection derivative also samples its surrounding light-time
solutions without the first-order refinement. No accuracy improvement is
claimed at kernel boundaries, near occultations or at the denominator floor;
the analytic branch still rejects the floor guard without fallback.

The 14 existing cases cover seven bodies and astronomical years -8750 and
2000, at 0.42°–2.26° solar elongation. They were selected near the Sun, not
as a representative survey. All paths use the caller's classic DE441 file.
Four previously fitted Horizons position series provide rate references;
Horizons uses DE441 as well, and the reference fits are correlated and
method-sensitive. The variation between fits is not a physical error bound.

## Local results

Tangent-vector differences and residuals are in arcseconds per TDB day.

| Comparison | Largest difference across 14 cases |
| --- | ---: |
| Refinement, numerical deflection | 0.0000000113 |
| Refinement, analytic deflection | 0.0000000113 |
| Analytic versus numerical, unrefined | 0.00010065 |
| Analytic versus numerical, refined | 0.00010065 |
| Interaction between the two changes | 0.0000000000023 |

The largest examples for each of the first four rows occur for Mercury at
JD TDB 2451674, approximately 0.416° from the Sun. The largest sampled
position difference due to refinement is about 0.00000094″; the two
deflection-rate methods use the same position for each emission choice to
the printed precision. The interaction is obtained by subtracting the two
rate-change vectors, and is near the numerical reporting floor.

For context, RMS rate residuals against the four correlated Horizons fits:

| Fit | Numerical, unrefined | Analytic, unrefined | Numerical, refined | Analytic, refined |
| --- | ---: | ---: | ---: | ---: |
| Wide degree 5 | 0.00002403 | 0.00004185 | 0.00002403 | 0.00004185 |
| Wide degree 7 | 0.00003522 | 0.00001870 | 0.00003522 | 0.00001870 |
| Narrow degree 5 | 0.00004353 | 0.00003182 | 0.00004353 | 0.00003182 |
| Narrow degree 7 | 0.00005732 | 0.00005051 | 0.00005732 | 0.00005051 |

For this sample, emission refinement is too small to explain the
method-dependent ordering of those residuals. The wide degree-five fit favors
the numerical rate, while the other three favor the analytic rate by this
sample RMS measure. This does not establish either method's physical accuracy
or generalize to other dates, elongations or planetary bodies.

## Reproduction

Build the `compare_emission_deflection_matrix` Rust example. Then run from
the repository root, substituting local paths and a new output filename:

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example compare_emission_deflection_matrix
node bindings/node/scripts/audit-emission-deflection-matrix.cjs KERNEL KERNEL_SHA256 DENSE_JSON DENSE_SHA256 EXAMPLE_EXE NEW_REPORT
```

The audit validates the reference schema, epochs and prototype rates, checks
the kernel and input SHA-256 before and after, and writes a new local report
without overwriting an existing one. Data and kernels are not distributed.
Run the real-DE441 test with `DE441_PATH` set and `cargo test --offline --locked
-p virgostars-ephemeris-core --test analytic_emission_refinement -- --ignored`.

- DE441 SHA-256: `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
- Dense Horizons input SHA-256: `62abfbe1f7d3284f4fff3ce131c2c477ab93836ff584c308223b8ea14b48cf37`.
- Local matrix report SHA-256: `1e48b86bc7f4b242c570a503a41b8372e2f9d8d580a4b02d3925def6f41b2b50`.

Further work should compare against independent implementations and more
geometries, investigate split-JD evaluation consistently through the full
model, and determine handling of the solar floor before considering use in
ordinary calculations. Historical-time uncertainty remains a separate issue.
