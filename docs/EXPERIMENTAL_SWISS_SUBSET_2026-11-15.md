# Experimental Swiss-shaped Node subset

`virgostars-ephemeris/experimental-swiss-subset` exports an opt-in factory
with restricted `swe_julday`, `swe_revjul` and `swe_calc_ut`-shaped calls. It
uses the compiled Rust example and a separately supplied classic DE441 kernel;
it does not import Swiss, download data, publish a package or modify the app.
This is a **partial experimental interface, not a drop-in replacement**.

```js
import {createExperimentalSwissSubset} from 'virgostars-ephemeris/experimental-swiss-subset';

const swe = createExperimentalSwissSubset({
  executablePath, kernelPath,
  // Supply only the separate SPKs available for the desired physical centers.
  physicalCenterKernels: { Mars: marsSpkPath, Jupiter: jupiterSpkPath },
  // Caller provides a documented time model for the same instant.
  convertUt1ToTtTdb: ut1JulianDay => getCoherentTtAndTdb(ut1JulianDay),
});
const jd = swe.swe_julday(2026,11,10,0,swe.SE_GREG_CAL);
const sun = swe.swe_calc_ut(jd,swe.SE_SUN,swe.SEFLG_JPLEPH|swe.SEFLG_SPEED);
```

The converter must synchronously return `{ttJulianDay, tdbJulianDay}` for the
same instant as the supplied UT1. The sanity checks on their differences do
not calculate ΔT, UT1−UTC or TDB−TT. Calendar flags are mandatory; dates are
validated with proleptic Gregorian or Julian arithmetic, and hours must be in
[0,24). This intentionally differs from Swiss permissive normalization and
binding-specific defaults. Neither function performs timezone, UTC or leap
second conversion.

`swe_calc(ttJulianDay, SE_ECL_NUT, 0)` supplies IAU 2006/2000B obliquity
and nutation in the binding's six-field shape. `longitude` is true obliquity,
`latitude` mean obliquity, `distance` nutation in longitude, and
`longitudeSpeed` nutation in obliquity; the last two fields are zero. These
fields are angles in degrees, **not planetary positions or speed values**.
This call consumes TT directly; `swe_calc_ut(ut1JulianDay, SE_ECL_NUT, 0)`
instead requires and invokes the explicit UT1-to-TT/TDB converter. Only flags
0 are accepted for this special body. Unlike Swiss, there is no guessed
ephemeris flag. Values outside the experimental 1900–2100 TT model range are
errors. Callbacks are synchronous and receive the returned frozen object.
`swe_get_planet_name` supports numeric IDs 0–9, Sun through Pluto; all other
IDs are errors rather than guessed names.

`swe_azalt` is opt-in via `horizontalSupport: {refraction: 'GEOMETRIC'}`
or `{refraction: 'NOAA_STANDARD'}`. It accepts `SE_ECL2HOR` or `SE_EQU2HOR`,
explicit UT1 and the configured time converter. Azimuth follows Swiss's
south-through-west convention. The returned coordinate fields are mutable so
the existing app can rotate azimuth; warnings remain immutable. Directions
must already contain any intended parallax. Distance is not used; height,
pressure and temperature currently must be zero placeholders. Unsupported
weather and observer height fail instead of being ignored. Exact poles and
undefined azimuth at zenith/nadir fail explicitly.

`GEOMETRIC` returns the true altitude also as apparent altitude and labels the
absence of refraction. `NOAA_STANDARD` applies the existing fixed-atmosphere
NOAA approximation from −1° to 90°; below −1° it returns geometric altitude
with a warning. Neither mode implements pressure/temperature-aware Swiss
refraction, local weather, terrain, topocentric parallax, or Swiss parity.
The comparison to Swiss azimuth and true altitude is limited to a small Rome
sample; the standard-atmosphere model is tested against NOAA equations, not
calibrated to Swiss atmospheric output.

For legacy callers that mutate house and star results, the factory accepts
`legacyMutableResults: true`. House cusp arrays and numeric house fields are
then mutable, as are star coordinate fields and the star's `name` property.
House `trueObliquity`, `model`, and `warnings`, and star `model`, `warnings`,
and `catalogId`, remain accessible but are **non-enumerable**. This prevents
the existing app's southern-inversion `for...in` loop from treating metadata as
angles. Consequently those metadata fields are omitted by `JSON.stringify`,
object spread, and `Object.keys`: callers that persist or transmit results
must explicitly copy the model, warnings, obliquity and catalog identifier
when needed. The default remains immutable with enumerable metadata. This
switch does not supply missing house angles or certify compatibility.

`swe_rise_trans` optionally exposes solar rise, set, upper transit and lower
transit. The caller must explicitly configure, for example,
`solarEventSupport: {windowDays: 1.5, stepMinutes: 10,
timeOffsets: 'CONSTANT_IN_SEARCH'}`. The window must be in (0, 2] days and
the cadence in [1, 60] minutes. The assumption means the start-epoch TT−UT1
and TDB−UT1 differences are kept constant throughout the search; the engine
does not interpolate UT1 or handle leap seconds inside the interval. The
caller must choose an interval where this approximation is appropriate.

Only the Sun, empty star name, explicit JPL flag, the four listed event codes,
and zero height/pressure/temperature placeholders are accepted. Rise/set use
the conventional geocentric solar-center altitude of −50 arcminutes,
representing average refraction and mean solar semidiameter. Transits use the
topocentric geometric Sun at zero height, without atmospheric refraction,
observer light time or diurnal aberration. Returned `transitTime` is **UT1
Julian day**, not UTC; `bracketSeconds` is numerical convergence, not physical
accuracy. Unsupported cases, no crossing within the window and singular
locations throw; the optional callback runs synchronously only on success.
Sampling can miss grazing crossings and all results remain approximate.
At one Rome epoch the conventional rise/set differ from Swiss 2.09.03 by
about 16 seconds; this is not a general error bound or Swiss parity.

Sun, Moon, Mercury and Venus physical centers use the supplied classic DE441.
Mars through Pluto require the corresponding explicit physical-center SPK path;
missing data or coverage are errors and system barycenters are **not** silently
presented as physical planets. See [physical-center notes](PHYSICAL_CENTERS_SPK_2026-11-15.md).
Supported planetary flags require `SEFLG_JPLEPH` and may additionally include
`SEFLG_SPEED` and one of `SEFLG_EQUATORIAL` or `SEFLG_HELCTR`. The two
coordinate flags cannot be combined. Swiss data, other flags and implicit
defaults are rejected. Fixed stars still accept only JPL and optional speed.
An optional callback is invoked synchronously once on success,
and the same frozen result is returned. Failures throw; callback error behavior
and the complete Swiss callback API are not implemented. Without the speed
flag, speed fields are zero. `rflag` reports only the accepted requested JPL
and speed bits; it does not certify Swiss numerical or physical parity.

The default six fields follow the DE441 prototype's Earth-centered light-time
plus annual-aberration, IAU 2006/2000B true-ecliptic-of-date model. The
equatorial option projects the same apparent physical state to the true equator
of date and returns `rectAscension`, `declination`, and their speed fields,
matching the binding's field names. The heliocentric option calculates
Sun-centered reception light time without annual aberration and returns
ecliptic fields. For the Sun observed from its own center it returns six zero
fields as an explicitly labelled sentinel, not a physical direction; it checks
epochs but does not open an ephemeris file for that sentinel. As in the rest
of this experiment, `rflag` reports accepted input flags, not all implicit
Swiss correction flags. Speeds are per
reception **TDB day**, not exactly UT1 day. Solar deflection, full nutation,
all Swiss flag semantics, a vetted bundled fixed-star catalog, a native Node
binary loader and production use remain missing. Model and
warning fields make the incomplete correction set explicit; consumers must not
strip or ignore them when interpreting accuracy. Each call starts a process;
no throughput or concurrency guarantees are made.

The tests use Gregorian and Julian calendar anchors, error and callback cases,
and four same-TT samples recorded from Swiss 2.09.03 with the same DE441.
The latter are compatibility diagnostics for this limited sample, not an
independent validation of the complete model. A real kernel and compiled CLI
are required for the integration test; set `DE441_PATH` and
`VIRGOSTARS_EPHEMERIS_CLI`. Tests do not invoke Swiss or redistribute its data.

## Current VirgoStars integration gaps

The application's `backend/services/astrology/func.js` currently passes its
UT-based Julian day to `swe_calc` for obliquity, although `swe_calc` expects
TT; migrating the caller needs an explicit time-scale decision. It also sets an
ephemeris directory globally, requests `SEFLG_SWIEPH` with speed, heliocentric
and equatorial variants, calls `swe_calc` for obliquity, `swe_azalt`, planet
names, stars and five house systems, and adds fields to house results. The
subset instead requires explicit files and epochs, accepts JPL flags only,
does not expose all those additional calls through the same interface, and
returns frozen house objects unless legacy mutability is explicitly selected.
A direct module substitution therefore fails or changes
behavior. Other consumers call `swe_rise_trans` with Swiss flags and eclipse
routines, which this restricted interface cannot replace directly. Those
requirements need their own compatibility contracts and validation; merely
renaming a flag or claiming a Swiss source would misrepresent the model.

Fixed-star methods accept a caller-supplied catalog, and house methods require
injected providers; see the corresponding star and house documentation. The
optional `starSolarDeflection: 'POINT_MASS'` enables first-order solar
deflection for fixed stars and rejects rays occulted by the Sun; the default
`'OMIT'` preserves the earlier behavior. `planetSolarDeflection: 'POINT_MASS'`
separately enables the finite-source correction for geocentric planets, with
numerical rates and a solar-occultation error; heliocentric output is unaffected.
See [finite-source notes](FINITE_SOLAR_DEFLECTION_2026-11-15.md).

The coordinate-flag test uses a local DE441, six supplemental SPKs and all ten
targets, with same-TT Swiss 2.09.03 values as limited diagnostics. On that
single date the largest heliocentric longitude difference is about 0.000187°
(Mercury); models and correction sets differ. Selected rates are compared to
central differences of this implementation, which checks internal consistency
but does not prove independent scientific accuracy.
