# Experimental constant-offset UT1 window

`createExplicitUt1Window` supplies UT1→TT/TDB and UT1→UTC callbacks for
multiple instants in an inclusive UTC interval. Each conversion calls the
experimental Rust CLI; this is offline and can be slow for large calculations.
The caller supplies the offset values, their source, two canonical UTC labels
with milliseconds, and both policies shown below:

```js
import {createExplicitUt1Window} from 'virgostars-ephemeris/experimental-utc-instant';
const window=createExplicitUt1Window({
  executablePath:'/absolute/path/to/true_ecliptic_at',
  startUtc:'2000-01-01T00:00:00.000Z',endUtc:'2000-01-03T00:00:00.000Z',
  ut1MinusUtcSeconds:0.355,taiMinusUtcSeconds:32,tdbMinusTtSeconds:0,
  sourceId:'example: replace with verified data and an accepted error budget',
  offsetPolicy:'CONSTANT_IN_WINDOW',leapSecondPolicy:'NO_LEAP_SECOND_IN_WINDOW',
});
// Pass window.convertUt1ToTtTdb and window.convertUt1ToUtc to the provider.
// Pass UT1 Julian dates to the provider, not unconverted UTC Julian dates.
```

This is an **approximation**, not an Earth-orientation or leap-second model.
UT1−UTC and TDB−TT vary continuously; assuming them constant introduces an
error that depends on the actual interval and chosen values. The caller must
establish that the approximation meets the intended error budget. The caller
also asserts the window has no leap second; the implementation has no leap
table and cannot verify this. Windows are limited to 31 days, 1972 through 2099,
and reject out-of-range instants. These restrictions do not establish accuracy
or make a leap-crossing interval safe. Split an interval at real leap changes
and supply independently appropriate offsets; leap-second labels themselves
cannot be represented. UTC output is rounded to the nearest millisecond and
binary64 JD resolution is about 40 microseconds near the present.

The test at 2000-01-01 12:00 UTC checks a UT1 and TT anchor independently
generated using pyerfa 2.0.1.5. Other instants in the integration test only
exercise composition of Sun and house calculations under the constant-offset assumption; they do not establish
scientific error bounds. No automatic current/historical DUT1, TAI−UTC, TDB
model, leap data, nor production-grade UTC conversion is included.
