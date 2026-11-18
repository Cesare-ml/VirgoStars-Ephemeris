# Experimental planetary range: 13000 BCE–3999 CE

Follow-up: [direct-obliquity alternative and ancient ΔT comparisons](ANCIENT_OBLIQUITY_AND_DELTAT_2026-11-17.md).

Further follow-up: [explicit three-sample ΔT sensitivity](HISTORICAL_DELTAT_SCENARIOS_2026-11-18.md)
is available experimentally; its samples are not positional uncertainty bounds.

**Status: initial experimental implementation and sampled diagnostic; not
scientifically accepted or enabled on the main server.** The owner requested
coverage from 13000 BCE to 3999 CE while fixed-star work and production server
migration remain paused. Astronomical numbering has a year zero: **13000 BCE
is year −12999**, and 1 BCE is year 0. The tests here use proleptic Gregorian
dates; the Astro.com published tables use Julian dates for ancient epochs.

[Astrodienst's published tables](https://www.astro.com/swisseph/swepha_e.htm)
list 50-year files from August 13000 BCE to 3999 CE. [JPL describes DE441](https://ssd.jpl.nasa.gov/planets/eph_export.html)
as covering a substantially wider interval. Kernel coverage alone does not
prove the orientation, time-scale, apparent-position or consumer coverage.

## Code and guard change

The opt-in experimental long-term precession and extrapolated nutation guards
now allow Julian epochs −13000 through 5001, covering the requested calendar
endpoints with room for the internal matrix-rate stencil. The existing modern
`calcTdb` / `calcUt` guards are unchanged. This is an extension of a numerical
guard, **not** a new accuracy guarantee: the Vondrák model has a long-term
foundation, but the 77-term nutation/fundamental-argument extrapolation and
the combination of models are not certified at these dates. The sources and
corrections are described in [Vondrák et al.](https://doi.org/10.1051/0004-6361/201117274).
An independent ERFA implementation of the same Vondrák mean axes was checked
at six TT epochs from year −12999 through 3999: the largest matrix or pole
component residual was 4.45e−16. This verifies agreement between the two
implementations on those samples; it does not validate the physical model,
nutation, Swiss conventions, or the full apparent calculation.

The experimental UT1 entry point continues to require caller-supplied ΔT and
uncertainty. It reports the input and does not turn the uncertainty into an
angular bound. UT1 is not UTC, local civil time, or a historical time zone;
no ancient or future clock policy is silently selected.

## Planet comparisons

All ten targets are geocentric Sun, Moon, Mercury, Venus and Mars–Pluto system
barycenters. The caller-supplied classic DE441 has SHA-256
`476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
The positions include light-time, solar deflection and annual aberration.

| Sample | Reference | Findings and limits |
| --- | --- | --- |
| 690 body/date cases: 69 years from −12999 to 3999, mostly 250 years apart, July 1 12:00 TDB | Local Swiss 2.09.03, JPL mode, ecliptic of date; TT approximated from TDB. | Maximum absolute component differences: 8.696″ longitude (Pluto, year −10500), 45.873″ latitude (Neptune, year −12999). The maximum difference between the engine latitude rate and a centered derivative of Swiss positions was 8.922″/day (Moon, year −12750). The 0.01-day stencil and model/time conventions affect these values. These are investigation results, **not passing product tolerances**. |
| 290 body/date cases: 29 years from −9998 to 3999, mostly 500 years apart, July 1 | Horizons apparent airless ICRF geocentric direction; approximately matching TT/TDB. | Maximum angular separation: 0.000363″ Moon; 0.00000449″ other bodies. This supports the fixed-axis calculation at these samples; both use DE441 and this does not validate the ecliptic-of-date model or physical orbit accuracy. |

[Horizons currently limits its DE441 service to 9999 BCE–9999 CE](https://ssd.jpl.nasa.gov/horizons/manual.html),
so the 13000–10000 BCE part has **not** been checked against Horizons here.
These grids are sparse, not bounds between samples or for all dates. The
large ancient ecliptic differences must be investigated, including the
orientation models, before asserting a release precision. Rate differences
are not proof that one implementation or the other is incorrect.

## Time sensitivity, not a historical confidence interval

At July 1, year −12999, local Swiss `swe_deltat` gives about **197.95 hours**
of TT−UT1. Feeding that value explicitly to the engine and increasing it by
one hour changes the Moon's ecliptic longitude by about **35.42 arcminutes**
at that sample. The extra hour is an arbitrary sensitivity scenario, **not an
estimate of the uncertainty**. Swiss's historical ΔT is a reconstruction,
not a measurement of the original date or a proof that its answer is correct.
The local comparisons do not settle which historical model is better.

The sample illustrates why a reproducible 13000 BCE calculation must state
the time scale and the caller's ΔT model and uncertainty. Planetary dynamical
states can be evaluated for a TDB instant even when historical UT1 is highly
uncertain. Do not describe this experiment as validated historical civil time.

## Reproduction

After `npm run build` in `bindings/node`, use caller-owned files and new output
paths; neither kernel nor reference outputs are committed. From that folder:

```text
node scripts/compare-swiss-long-range.cjs <de441> <sha256> <swisseph> <new-report> 13000bce-3999-step-250 0.01
node scripts/compare-horizons-apparent-icrf.cjs <de441> <sha256> <new-report> 9999bce-3999-step-500
node scripts/audit-ancient-ut1-sensitivity.cjs <de441> <sha256> <swisseph> <new-report>
```

For the independent mean-axis implementation check, install pyerfa 2.0.1.5
separately and run `python scripts/generate_long_term_erfa_reference.py
<new-output.csv> extended` from the repository root. Set
`LONG_TERM_ERFA_EXTENDED_REFERENCE` to that file and run `cargo test -p
virgostars-ephemeris-core --lib external_erfa_mean_axes_extended_epochs --
--ignored --nocapture`. The reference is a numerical test of the same model.

Horizons requires network access; all three check the DE441 hash, refuse to
overwrite reports, and preserve local evidence. Retained ignored artifacts:

- `swiss-13000bce-3999-250y-20261117.json`: SHA-256 `cede753c877aa6a1cfb1bf6845bf461e8e86b12494645ae50d9215780c3a537c`.
- `horizons-9999bce-3999-500y-20261117.json`: SHA-256 `deb57b63362ce36e1e3e3af75336a5d41f93a2d4eda3bce57237f1ab858326d7`.
- `ancient-ut1-sensitivity-20261117.json`: SHA-256 `92091d30ddff051f48feb99df7aff0cfa50d8a1ed041b9810d08e85dd42dd578`.
- `erfa-extended-axes-20261117.csv`: SHA-256 `f92ee08c75968c028a156b6d9631a6c59212a6c155b2c19cf4ca780f552c1666`.

The hashes identify local bytes; they do not independently authenticate the
reference service, data rights, or chosen models. Remaining work: independent
orientation/nutation analysis, denser boundary and seasonal sampling, agreed
accuracy thresholds, a sourced time model and uncertainty, and explicit
consumer integration for the accepted interval. Lunar nodes, apogees and
rise/set have separate implementations and have not been validated for this
entire extended interval by this planetary experiment.
