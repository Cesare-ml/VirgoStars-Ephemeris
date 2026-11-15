# Explicit experimental legacy bridge

## CommonJS initialization

Node CommonJS code can asynchronously initialize the same ESM bridge:

```js
const {loadExperimentalLegacyBridge} = require('virgostars-ephemeris/experimental-legacy-bridge');
async function start(options) {
  const swe = await loadExperimentalLegacyBridge(options);
  // After initialization, bridge methods themselves remain synchronous.
  return swe.swe_julday(2000, 1, 1, 12);
}
```

All explicit configuration and scientific limitations described below still
apply. `require()` returns a loader object, not a `swisseph` replacement: the
application must await initialization before injecting the resulting instance
into code that uses it. This does not modify the backend, add missing APIs,
provide time data or remove the need for a reviewed integration and validation.
The loader was exercised locally with Node 22.18.0; Node 20 was not tested here.

The bridge exposes `SE_PLUTO_PICKERING = 54` because legacy callers may use it
as an inclusive loop bound. This is only an identifier, not support for that
body or for the other IDs between Pluto and 54. Attempting to calculate an
unsupported body still fails explicitly. Without the bound, JavaScript loops
can silently skip every planet when comparing against `undefined`.

A local, non-production diagnostic intercepted the backend's `swisseph`
module in a separate process and exercised its chart function with the bridge,
local DE441 and six planetary SPKs, the main house providers, and synthetic
constant time offsets. At one date it produced all ten supported planets and
the requested five house systems, and exercised southern inversion. Its default
request failed explicitly on an unsupported body. This was a shape/composition
probe, not a numerical comparison, production migration, or reproducible CI
test; it did not include a real star catalog. The backend currently feeds a
civil UTC-derived Julian date to UT1-facing calls and passes that value to
`swe_calc`, whose contract is TT. A production integration must handle those
time-scale differences explicitly rather than relying on the diagnostic's
zero-DUT1 assumption. The existing Placidus position context is also tied to
the caller-supplied epoch and must be configured appropriately for each chart.

## ESM initialization

`virgostars-ephemeris/experimental-legacy-bridge` exports an opt-in ESM
factory that wraps the experimental Rust-backed Node subset with a small
portion of the calling shapes used by VirgoStars. It is not a native binding,
an automatic `require('swisseph')` replacement, or a production migration.
The executable, classic DE441 kernel, optional satellite SPKs, time converter,
house providers and star catalog must be explicitly supplied as described in
the neighboring experimental notes. Each astronomy call launches a process.

```js
import {createExperimentalLegacyBridge} from 'virgostars-ephemeris/experimental-legacy-bridge';

const bridge = createExperimentalLegacyBridge({
  executablePath, kernelPath, ephemerisDirectory, convertUt1ToTtTdb,
  defaultCalendar: 'GREGORIAN',
  flagPolicy: 'MAP_SWISS_AND_DEFAULT_TO_JPL',
  mutableResults: 'LEGACY',
  // Supply physicalCenterKernels, catalog, houseSupport, etc. as needed.
});
bridge.swe_set_ephe_path(ephemerisDirectory);
const jd = bridge.swe_julday(2026, 11, 10, 0);
const sun = bridge.swe_calc_ut(jd, bridge.SE_SUN,
  bridge.SEFLG_SWIEPH | bridge.SEFLG_SPEED);
// sun.rflag reports JPL + speed and sun.warnings reports the source change.
```

The named flag policy is mandatory. An explicit `SEFLG_SWIEPH` request or an
omitted source flag is intentionally mapped to the separately configured JPL
source. It never reads Swiss files or reports a Swiss source. Position results
return the actual JPL bit in `rflag`; warnings name the substitution and the
original `requestedFlags` is available as a non-enumerable field. Solar event
results expose non-enumerable `effectiveFlags` as well. Mixed JPL/Swiss,
Moshier, invalid or unsupported flags fail; the underlying implementation
continues to reject modes outside its documented subset. The `SE_ECL_NUT`
special case with flags 0 is passed through without changing its flags.

`swe_set_ephe_path` is only a compatibility **identity check**, not a global
setter or a search-path resolver: its absolute directory must exist and
resolve to the explicit configured directory. The actual JPL and SPK paths
are separately configured, and are not inferred from that directory. The
method returns the canonical path as a string (not necessarily the exact
return type of the Swiss binding). A different directory is rejected.
`defaultCalendar` must explicitly select Gregorian or Julian for calls that
omit the calendar flag. Supplied calendar flags still take precedence.

`mutableResults` must explicitly be `LEGACY` or `FROZEN`; the former allows
the application's house and star mutations and makes some metadata
non-enumerable. `requestedFlags`, event `effectiveFlags`, and non-enumerable
house/star provenance are not preserved by JSON or object spread; callers
that persist results must explicitly copy these fields. Warnings should not be
discarded. Synchronous callbacks run once on successful calls and receive the
same object returned; errors throw. Not all Swiss callback/error variants are
implemented.

The bridge does not provide a bundled catalog, historical ΔT or automatic
UTC/UT1 data, all required body IDs, all flags, arbitrary path switching,
complete house angles, eclipse APIs, or Swiss numerical parity. It does not
install a binary or alter the app. A separate integration and scientific
review is necessary before substituting it in any existing workflow.
