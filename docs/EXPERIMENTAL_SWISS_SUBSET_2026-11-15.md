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

The application's `backend/services/astrology/func.js` currently sets an
ephemeris directory globally, requests `SEFLG_SWIEPH` with speed, heliocentric
and equatorial variants, calls `swe_calc` for obliquity, `swe_azalt`, planet
names, stars and five house systems, and adds fields to house results. The
subset instead requires explicit files and epochs, accepts JPL flags only,
does not expose all those additional calls through the same interface, and returns
frozen house objects. A direct module substitution therefore fails or changes
behavior. Other consumers call `swe_rise_trans` and eclipse routines. Those
requirements need their own compatibility contracts and validation; merely
renaming a flag or claiming a Swiss source would misrepresent the model.

Fixed-star methods accept a caller-supplied catalog, and house methods require
injected providers; see the corresponding star and house documentation.

The coordinate-flag test uses a local DE441, six supplemental SPKs and all ten
targets, with same-TT Swiss 2.09.03 values as limited diagnostics. On that
single date the largest heliocentric longitude difference is about 0.000187°
(Mercury); models and correction sets differ. Selected rates are compared to
central differences of this implementation, which checks internal consistency
but does not prove independent scientific accuracy.
