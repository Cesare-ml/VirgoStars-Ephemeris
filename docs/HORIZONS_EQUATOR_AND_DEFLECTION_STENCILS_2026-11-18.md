# Horizons equator/ecliptic and solar-deflection stencil diagnostics

**Status: exploratory.** This continues the [normalized deflection and
Horizons frame audit](NORMALIZED_DEFLECTION_AND_HORIZONS_FRAME_2026-11-18.md).
No production calculation, API, star catalog or server activation changes.

## Distinguishing equator and ecliptic

For seven fixed TT dates from astronomical year −9998 through 2000, obtain
[Horizons](https://ssd.jpl.nasa.gov/horizons/manual.html) airless geocentric
apparent Sun, Moon and Venus positions in apparent ICRF (quantity 45),
apparent equator/equinox of date (2), and ecliptic of date (31). Horizons'
returned apparent ICRF directions are directly projected into the local
experimental axes; this avoids attributing differences in the input apparent
direction to the coordinate transformation. Horizons identifies DE441 for
the bodies and Earth. The 21 directional samples do not validate an orbital
model or represent continuous time coverage.

The diagnostic constructs the local true equator from the Vondrák pole-derived
mean equator and the extrapolated nutation angles; its nutation rotation uses
the obliquity implied by the two pole fits. It also projects onto both the
existing pole-derived ecliptic and the mixed direct-obliquity alternative.
It does **not** change or expose a new production equatorial API. The geometric
relation of this true equator and the pole-derived ecliptic is regression
tested at ancient and modern epochs.

For the Sun at year −9998, local minus Horizons is about **−213.21″ in
RA × cos(declination)** and **−22.95″ in declination** in the true equator
of date. The corresponding pole ecliptic longitude difference is −210.94″,
reproducing the prior diagnostic with the *same Horizons ICRF direction*.
Hence a substantial difference already exists in the equatorial-of-date
transformation. This does not separate mean precession, nutation, frame bias
or all reference conventions, and does not decide which model is correct.

An effective Horizons rotation between its equatorial and ecliptic directions
is inferred per body, assuming a common equinox and a pure rotation about
that axis. Across the three bodies at each sampled epoch, inferred values
agree to within **0.0014″**; the largest mismatch in the supposedly shared
x component is **0.00018″**. These are internal checks of the rotation
assumption, limited by Horizons output rounding, not independent physical
uncertainties. For the oldest Sun sample, Horizons' effective obliquity is
about **24.1614475°**. Local *true* obliquity relative to this value differs
by **−35.36″** for the pole-fit angle and **−3.99″** for the direct fit.
The pole-based Sun ecliptic latitude is closer in that sample because all
orientation components combine; a single obliquity difference does not
predict the full latitude residual in isolation. Horizons and local models
use distinct long-term precession and nutation conventions.

## Crossing the solar deflection floor

Reprocess the previously recorded 257-point DE441 Uranus sweep. The existing
centered derivative has half-step 1/256 day; two finer fourth-order reference
stencils use 1/2048 and 1/4096 day. Each stencil is classified independently
by comparing the guard value at its sampled epochs with its center. A lack
of a sampled sign change does not prove no crossing between samples.

Of the 32 previously flagged crossings, **28** have a sampled crossing in
the old stencil but not the two reference stencils: 14 centers lie on each
side of the floor. Among these 28 samples the old maximum residual is
29.6512″/day, the analytic candidate's maximum is 0.0000041″/day, and the
two reference stencils disagree by at most 0.0000096″/day. These numbers
concern the normalized ICRF angular correction against a numerical derivative
of the *same model*; maxima may occur in different rows and near-limb cases
are occulted, not physical accuracy measurements.

For the remaining **4** samples at least one finer reference stencil also
crosses the floor. Its two reference estimates disagree by as much as
13.78″/day, and cannot be treated as a smooth derivative oracle. The exact
boundary is nondifferentiable and the candidate rejects a small neighborhood
of it; adopting a production boundary/occultation policy remains open.

## Reproduction and local evidence

Supply the separately obtained DE441 and prior reports with their SHA-256s;
the reports remain ignored and are not distributed. The Horizons diagnostic
uses network access and validates target/center DE441, airless state, TT
epochs and coordinate columns; it reports common-axis residuals. The floor analysis
is offline and checks input hashes before and after. Source details and raw
Horizons replies are in the ignored report.

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example diagnose_horizons_orientation_components
cargo test --offline --locked -p virgostars-ephemeris-core --example diagnose_horizons_orientation_components
node bindings/node/scripts/compare-horizons-orientation-components.cjs <prior-ecliptic-report> <sha256> <orientation-executable> <new-report>
cargo build --offline --locked -p virgostars-ephemeris-core --example compare_deflection_rate_candidates
node bindings/node/scripts/analyze-deflection-floor-stencils.cjs <de441> <kernel-sha256> <prior-floor-report> <floor-sha256> <deflection-executable> <new-report>
```

- Horizons components local report SHA-256: `935d335d3e33875486363fc0e821150b7512189c672bd4c3a8b2f77381d4a62d`.
- Floor stencil classification SHA-256: `5ea908d701e6363cf095afc34dde3c694ee2c425a46f59014c31bade61146faa`.
