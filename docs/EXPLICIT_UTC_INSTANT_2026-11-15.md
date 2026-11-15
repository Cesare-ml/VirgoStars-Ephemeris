# Explicit one-instant UTC conversion

The opt-in `virgostars-ephemeris/experimental-utc-instant` helper accepts a
normal UTC timestamp and **caller-supplied offsets for that exact instant**.
It invokes the Rust `explicit_utc_instant` implementation and returns the UT1,
TT and TDB Julian dates plus two single-instant callbacks usable by the
experimental planetary and house APIs. No leap-second table, DUT1 forecast,
historical ΔT, TDB model, network lookup or default offset is invented.

```js
import {createExplicitUtcInstant} from 'virgostars-ephemeris/experimental-utc-instant';
import {createExperimentalSwissSubset} from 'virgostars-ephemeris/experimental-swiss-subset';

const instant = createExplicitUtcInstant({
  executablePath, datetimeUtc: '2026-11-10T00:00:00.000Z',
  ut1MinusUtcSeconds, taiMinusUtcSeconds, tdbMinusTtSeconds,
  sourceId: 'caller-verified-offset-source',
});
const swe = createExperimentalSwissSubset({
  executablePath, kernelPath, convertUt1ToTtTdb: instant.convertUt1ToTtTdb,
  // If using houses, also supply a houseSupport object with the house provider
  // and convertUt1ToUtc: instant.convertUt1ToUtc, plus source identifiers.
});
const sun = swe.swe_calc_ut(instant.ut1JulianDay, swe.SE_SUN,
  swe.SEFLG_JPLEPH | swe.SEFLG_SPEED);
```

Input is a canonical `YYYY-MM-DDTHH:mm:ss.sssZ` label between 1972-01-01
inclusive and 2100-01-01 exclusive, without a leap-second label (`:60`). The
caller provides DUT1 = UT1−UTC in seconds within ±0.9, an integral TAI−UTC
between 10 and 100 seconds, TDB−TT in seconds within ±0.01, and a nonempty
source identifier. These are **sanity bounds only**; the code does not verify
the values or their source, including whether the supplied leap-second offset
is correct for the date. Obtain them from an appropriate authoritative source.
The source ID is a caller assertion and is not a verification or data hash.

Rust performs the one-instant arithmetic as UT1 = UTC + DUT1,
TT = UTC + (TAI−UTC) + 32.184 s, and TDB = TT + (TDB−TT), avoiding a large
single-JD addition until the final result. POSIX milliseconds represent normal
UTC labels but cannot represent the leap second itself; the caller must update
the applicable offsets on either side. Single binary64 Julian dates have
roughly 40-microsecond granularity near the present. The callbacks reject any
other UT1 instant, preventing silent reuse of stale offsets. A separate time
model and explicit assumptions are needed for interval searches and dates
outside this range. The existing experimental astronomy models may impose
additional narrower limits.

Four independent numeric fixtures were produced with pyerfa 2.0.1.5
`dtf2d`, `utctai`, `taitt` and `utcut1`, including normal labels immediately
before and after the 2016 leap second. The test deliberately supplies the
respective offsets and checks the two-second TT interval. These fixtures do
not validate actual source data, future leap-second schedules, a UTC `:60`
label or a TDB model. ERFA is not a runtime dependency, and the example's
local files and datasets are not bundled.
