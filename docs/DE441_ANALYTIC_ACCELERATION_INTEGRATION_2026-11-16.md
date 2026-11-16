# DE441 analytic acceleration integration — 2026-11-16

**Status: experimental local working tree; independent physical validation
pending.** This reconciles the existing work from
`origin/feat/mvp1-de441-analytic-acceleration-20261111` (`bcb12f5`) with
the current hardened classic-DE441 reader, rather than replacing that reader.

`De441::state_with_acceleration` evaluates position, velocity and the second
time derivative of the **selected Chebyshev polynomial** in fixed ICRF axes,
km, km/TDB-day and km/TDB-day². It retains the existing record/granule
selection, cache, bounds, missing-series errors and direct lunar relative
handling. `De441::state` does not evaluate second derivatives. At a boundary
the acceleration belongs to the selected one-sided polynomial; continuity is
not guaranteed. Finite inputs that cannot be evaluated safely fail explicitly.

The recurrence follows `T''_k = 4T'_(k-1) + 2xT''_(k-1) - T''_(k-2)` and
scales the second derivative by `(2/width_days)^2`. Scaling is applied in
stages rather than squaring a factor that can overflow or underflow before
the final result; zero second derivatives stay zero at very small widths.
This still rejects nonfinite recurrence/interpolation intermediates and a
nonzero acceleration contribution or final result rounded to zero, even if
a different summation method might recover a finite mathematical result.

`geocentric_light_time_annual_aberration_analytic` and the analytic
fixed-axis spherical/six-field adapters use Earth's reception-time polynomial
acceleration, recording `AnalyticDe441Chebyshev` provenance. The existing
centered-velocity-difference APIs remain available and report their actual
representable step. Neither variant supplies a full apparent-of-date result.

## Verification performed locally

- Eleven offline synthetic tests cover quadratic and cubic closed-form second
  derivatives, position/velocity parity with `state`, target-center reversal,
  Earth/Moon mass weights, record boundaries, unavailable data, and extreme
  scaling including genuine overflow. They also compose the analytic
  acceleration into light-time, aberration and the fixed-axis six fields.
- With the local DE441 file identified in the [earlier receipt](GEOCENTRIC_LIGHT_TIME_2026-11-10.md#local-rerun-on-the-integration-branch--2026-11-16),
  four epochs and four target/center pairs compared analytic accelerations
  against centered differences of **this reader's own** velocities. The worst
  axis discrepancy was `0.001099989633075893 km/TDB-day²`, below the
  predeclared test bound of `0.2 km/TDB-day²` with a requested ±0.001-day
  interval. This is internal numerical consistency, not an external reference.
- For Mercury, Moon and Mars-system barycenter at three epochs, analytic and
  centered-step aberration paths agreed within `0.000011276919394731522
  km/TDB-day` worst axis in velocity; the test bound was 1 km/TDB-day and
  the requested centered Earth acceleration step was 0.02 TDB day. Position
  components agreed within the test's 1e-6 km bound. Again this is internal
  consistency and does not prove apparent planetary accuracy.

```powershell
cargo test --locked --test de441_analytic_acceleration
$env:DE441_PATH = '<path-to-classic-DE441>'
cargo test --locked --test de441_analytic_acceleration -- --ignored --nocapture
```

Further work: independent reference acceleration/rate comparison, extended
epochs and granule boundaries, date-frame derivatives, apparent corrections,
and integration of the existing optimized batch only after separate review.
No CI, publication, deployment or production activation is implied.

**CPAL-1.0 modification record (2026-11-16):** integrated and hardened the
analytic second-derivative path and scalar aberration adapters, added synthetic
and optional real-file regression checks. No third-party code or data added.
