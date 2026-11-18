# Experimental analytic solar-deflection rate

**Status: EXPERIMENTAL, isolated, opt-in.** This implements the derivative
previously investigated in the [dense Horizons holdout](HORIZONS_DENSE_RATE_HOLDOUT_2026-11-18.md).
It does not select the candidate in ordinary calculations, change the main
server, establish physical accuracy, or complete MVP acceptance.

## Contract

Rust callers can use `experimental_analytic_apparent::experimental_analytic_apparent_icrf`.
Node callers with a locally supplied classic DE441 can call
`engine.experimentalAnalyticDeflectionIcrfTdb(jdTdb, body)`. Bodies use the
existing numeric IDs 0–9. The return is an Earth-centered ICRF Cartesian state
in km and km per TDB day. It includes geocentric light-time, solar deflection
and annual aberration, without atmospheric refraction. It does not provide
ecliptic-of-date coordinates, UT1/UTC conversion or historical Delta T.
Outer-planet IDs represent the corresponding system barycenters.

The emission epoch here is the unrefined single-f64 Julian date used by the
offline prototype. It differs from the separate existing experimental ICRF
path, which uses a first-order sub-ULP emission refinement. The two paths
must not be treated as a controlled change in deflection rate alone.

The lower-level `experimental_analytic_deflect_sun_with` accepts an
Earth-observed light-time state and a barycentric state provider. The caller
is responsible for the Earth-observer interpretation. Provider target,
center, epoch and finite components are checked. The derivative propagates
the light-time rate and differentiates both the solar correction and the
normalization of the corrected position. The Sun is returned unchanged.

The existing model clamps the dimensionless deflection denominator to
`1e-6`; the resulting function is not differentiable at that boundary. The
candidate returns `NonDifferentiableFloor` within `1e-12` of it, exposed by
Node as `E_DEFLECTION_BOUNDARY`, with no automatic fallback. On either side,
the selected branch is differentiated as written. This is a numerical model
guard, not a solar-occultation or visibility test. In particular, output on
either side does not imply an observable or physically validated position.

## Evidence and reproducibility

Local synthetic Rust tests compare the normalized rate against a fourth-order
position difference, preserve range, exercise invalid geometry and check both
sides of the floor and the rejected boundary. Node tests with the local DE441
sample all ten bodies at eight dates from astronomical year -12999 through
3999, check standard output is unchanged, and verify the boundary error using
Uranus near JD TDB -1474169.0782648325. One close-solar Jupiter case compares
against a fourth-order position difference. These are sampled and
same-model checks, not coverage or physical-accuracy guarantees.

The local 121-row previously reconstructed solar-deflection grid was compared
against the separate offline candidate example using the supplied classic
DE441 (SHA-256 `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`).
The largest differences were approximately `2.4e-9` arcsec in position and
`4.3e-11` arcsec/day in angular rate. Both computations share the same model
and ephemeris; this checks implementation consistency, not independent
physical validity. The grid and kernel are local inputs, not distributed.

After building the existing example and Node addon, reproduce with:

```text
node scripts/check-analytic-deflection-against-prototype.mjs KERNEL GRID_JSON EXAMPLE_EXE
```

Run from `bindings/node`; `GRID_JSON` is the previously reconstructed
`solar-deflection-baseline-reconstructed-20261118.json`, and `EXAMPLE_EXE`
is the compiled `compare_deflection_rate_candidates` example. The script
reads inputs and reports aggregate residuals; it does not generate the grid.

The earlier Horizons comparison used DE441 as well, and its fitted rates have
method-dependent uncertainty. Further independent comparisons and a decision
on emission-time precision and behavior across the clamp are required before
considering any change to the ordinary path.

The subsequent [controlled emission/deflection matrix](EMISSION_DEFLECTION_MATRIX_2026-11-18.md)
adds a separate refined-emission Rust experiment and quantifies both changes
on the existing Horizons sample. The original unrefined function is unchanged.
