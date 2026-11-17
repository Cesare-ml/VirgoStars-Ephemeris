# Swiss apparent-rate investigation — 2026-11-17

**Status: exploratory, sampled numerical diagnostics; not a production fix or
release certification.** This follows [the seasonal comparison](PLANETARY_SPEED_VALIDATION_SEASONAL_2026-11-17.md).
The normal engine calculation and consumer APIs are unchanged.

The [subsequent station matrix](SWISS_STATION_MATRIX_2026-11-17.md) expands
this investigation to 944 selected events and examines correction modes.

## Which corrections affect the discrepancy?

For 5,400 new body/epoch pairs (Moon through Pluto system barycenter; years
10–2990 every 20 years, 1 January/April/July/October at 12:00 TT), the local
Swiss Ephemeris 2.09.03 was called in JPL mode with the same external DE441.
Each comparison is **Swiss's own reported speed minus a fourth-order centered
derivative of Swiss's own positions**, not a comparison with another frame.
For each flag mode, the finite difference uses positions from that same mode.
The half-step is 0.02 TT day, with samples at ±0.02 and ±0.04 day, and actual
rounded JD spans. The requested JPL, speed and correction flags were verified
in every returned flag word. This grid is new relative to the earlier
20-year seasonal grid, although the models share DE441 and observations.

| Swiss mode | Longitude RMS ″/day | Latitude RMS ″/day | Radial RMS AU/day | Radial maximum AU/day |
| --- | ---: | ---: | ---: | ---: |
| Default apparent | 0.04111 | 0.02458 | 2.216e-5 | 8.076e-5 |
| Annual aberration disabled | 0.04112 | 0.02458 | 1.428e-8 | 9.874e-8 |
| Gravitational deflection disabled | 0.04111 | 0.02458 | 2.216e-5 | 8.077e-5 |
| Nutation disabled, mean ecliptic of date | 0.04104 | 0.00322 | 2.216e-5 | 8.076e-5 |
| Fixed J2000 ecliptic | 0.04104 | 0.00311 | 2.216e-5 | 8.076e-5 |
| True position | 0.00227 | 0.02441 | 2.083e-11 | 1.694e-10 |

The [Swiss programming documentation](https://www.astro.com/swisseph/swephprg.htm)
describes these flags. J2000 also implies no nutation; true-position mode
changes multiple apparent corrections, so its outcome does not isolate just
one effect. The angular columns in different frames describe different
coordinates and cannot be directly interpreted as positional errors. The
results identify an association with aberration for the radial discrepancy
and with the nutation option for much of the latitude discrepancy. They do
not prove the implementation's internal cause. In particular, longitude RMS
remains substantial for the Moon when aberration alone is disabled; the much
smaller value in true-position mode changes multiple factors at once.

## Approximate radial scaling hypothesis

The radial discrepancy's distance-dependent behavior suggested a simple
diagnostic prediction, selected from a handful of earlier cases and then
evaluated on the new 5,400-case grid:

```text
prediction ≈ (Earth acceleration · apparent line of sight) × (range / c)
```

Earth acceleration is approximated by the negative derivative of the
geometric Sun-relative-to-Earth velocity (solar barycentric acceleration is
neglected); the line of sight and range are from the engine's apparent ICRF
state. Swiss receives TT and the engine receives the same numeric JD as an
approximate TDB epoch. This is a diagnostic approximation, **not** an
established description of Swiss internals or an adjustment to either engine.

The observed radial residual RMS was **2.216e-5 AU/day**; the RMS of
`observed − prediction` was **4.258e-8 AU/day**. A zero-intercept least-squares
slope of observed versus prediction was **0.999995**. Distant bodies dominate
the aggregate fit; individual-body results are in the local report. This
strong numerical association and the flag experiment motivate further
independent review. They do not make Swiss's published speed automatically
wrong under every definition, establish its internal algorithm, or justify
altering the VirgoStars apparent derivative to reproduce it.

## Eight sampled stations

For one previously identified candidate per Mercury through Pluto, the
engine's apparent longitude rate and a finite derivative of Swiss's own
apparent longitudes were separately bracketed and bisected around the same
station. Swiss reference JPL/speed flags were checked; its TT epoch is matched
approximately to engine TDB. The samples use the same DE441 but different
frame/orientation models. The finite-difference half-step was tested at 0.01,
0.02 and 0.04 day. Solar separations ranged from 15.25° to 129.69°.

For each of these **eight selected cases**, the numerical brackets were about
0.66 second wide. At all three steps the engine and Swiss-position-derivative
bracket centers were equal at that grid resolution, except Pluto, where they
were about 0.66 second apart. This is **numerical bracket resolution**, not
physical sub-second accuracy: model differences, finite differences and time
conversion remain. It does not cover all stations or establish historical
civil time.

For comparison, bisecting Swiss's *reported* longitude speed in the same eight
intervals gave a difference from the engine station of approximately 0 to
**4.78 minutes**, the latter at the sampled Pluto station near January 2820.
That is a single selected example and a separate Swiss speed convention; it
does not establish an upper bound for other dates. The two Swiss root methods
therefore cannot be treated as interchangeable without specifying the desired
quantity and acceptable error.

## Reproduction and retained evidence

From `bindings/node`, after building the local native binding, supply the
caller-owned classic DE441, its SHA-256 and a separately installed Swiss
module. Each command is offline and requires a new output file:

```text
node scripts/audit-swiss-apparent-rate-flags.cjs <de441> <sha256> <swisseph> <new-report>
node scripts/compare-swiss-station-roots.cjs <de441> <sha256> <swisseph> <new-report>
```

Kernel SHA-256 is
`476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
Flag-grid report SHA-256 is
`f802d4c14d7c9a453b5949bfef701eae93fff1036d82f393653147690319610a`;
station report SHA-256 is
`7ce5c6ce14083f3ce2dad77b5efe3e4cd333c059c288ae0a3d46fc694eb78f46`.
Hashes identify local bytes, not publisher authenticity. No third-party data
or implementation is committed.
