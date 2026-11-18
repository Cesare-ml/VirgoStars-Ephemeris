# Horizons near-solar rates and equatorial frame fit

**Status: experimental validation.** This continues the [apparent ICRF and
nutation audit](APPARENT_SOLAR_RATE_AND_LOCAL_NUTATION_2026-11-18.md). No
production algorithm, public API, star catalog or server activation changes.

Continuation: [dense Horizons rate fits and interleaved holdout](HORIZONS_DENSE_RATE_HOLDOUT_2026-11-18.md).

## Apparent speeds from Horizons positions

Fourteen geocentric, airless apparent ICRF cases were selected before querying
Horizons, for Mercury, Venus and the Mars–Neptune system barycenters, at
epochs near astronomical years −8750 and 2000. Their sample-center solar
elongations are 0.42°–2.26°; these are sparse examples, not a continuous
coverage grid or an occultation/visibility certification. Horizons identifies
DE441 for target and Earth. Local states use the unrefined single-f64
light-time JD; the TT↔TDB conversion uses one periodic term.

Query Horizons quantity 45 at seven neighboring fixed TT instants per case,
convert its ICRF angles to unit vectors, and numerically differentiate those
positions using fourth-order stencils. Project derivatives onto the central
right-ascension and declination tangent directions. The local original and
analytic-candidate rates use the same apparent ICRF position; velocity
residuals are tangent-vector norms in arcseconds per TDB day. This compares
independent implementations using the **same DE441**, not independent orbits.

At sample centers the largest local/Horizons positional angular separation
is **0.0000057″**. With reference half-steps 1/64 and 1/128 day, maximum
speed residuals versus the 1/64-day derivative are **0.00011234″/day** for
the current method and **0.00011229″/day** for the candidate; reference-step disagreement reaches
**0.0003201″/day**. The candidate's residual is smaller in 11 of 14 rows,
but this count is not conclusive at the reference's precision. With half-steps
1/16 and 1/32 day, the reference-step disagreement reaches **0.003906″/day**;
the larger step can add curvature/truncation effects. Horizons output rounding,
the time conversion, and sensitivity near the Sun limit the comparison.
These two step pairs do not establish an accuracy bound or a reliable winner.
The diagnostic validates the central denominator but does not prove that no
guard crossing occurs between every sampled instant.

## Fitted equatorial rotation against Horizons

Reuse the seven TT epochs and three apparent ICRF directions (Sun, Moon,
Venus) from the previous Horizons study. Project the same ICRF vectors into
the local mean and true equators and fit a small three-axis rotation from each
local set to Horizons' apparent true-equatorial directions. The fit solves
the first-order direction equations and reports exact-rotation RMS residuals
and elimination pivots; these are diagnostics for a **three-direction fit**,
not an independent reconstruction of the physical Earth pole.

At year −9998, the fitted rotation magnitude is about **218.57″** for local
mean versus Horizons true equator and **214.47″** for local true versus
Horizons true; the fitted rotation vectors differ by **8.76″**. Their exact
rotation RMS residuals on the three directions are 0.021″ or less. Across
the seven samples the smallest elimination pivot is about 0.045, occurring
near 2000; pivots alone do not establish a full condition-number bound.

The rotation shared by the three directions supports an orientation-frame
interpretation of the ancient discrepancy. It does **not** isolate the
local precession error: the comparison against Horizons true coordinates
also contains Horizons nutation and differences in conventions. It does not
prove which long-term model is more physically accurate, and seven dates do
not establish the entire 13000 BCE–3999 CE interval.

## Reproduction and evidence

The scripts check caller-provided input hashes before and after and require
fresh report paths. Horizons queries require network access; the frame fit is
offline. DE441, raw Horizons results and reports remain local and ignored.
The archived fine-scale report was made before the optional `scale` field was
added, so its hash identifies that file rather than byte-identical new output.

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example compare_deflection_rate_candidates
node bindings/node/scripts/compare-horizons-near-sun-rates.cjs <de441> <sha256> <case-report> <case-sha256> <deflection-executable> <new-report> fine
node bindings/node/scripts/compare-horizons-near-sun-rates.cjs <de441> <sha256> <case-report> <case-sha256> <deflection-executable> <new-report> coarse
node bindings/node/scripts/analyze-horizons-frame-rotation.cjs <orientation-report> <sha256> <projection-executable> <new-report>
```

Local report SHA-256s:

- Fine Horizons rate sample: `712ddd301f50d952b5873f41ad43932837014d6945a6022a3e26539c529be298`.
- Coarse Horizons rate sample: `0d0aa99e9c82264a85ebe20e4ddf95121a2b357b2ee58902265fb21ce978f243`.
- Equatorial frame fit: `f311f5a56cce739f7394b41f8946b84d734994c3e657c88702c32fe06e81d77b`.
