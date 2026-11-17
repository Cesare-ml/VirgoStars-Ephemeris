# MVP 2 — horizon coordinates, refraction, rise/set and transits (2026-11-16)

**Status: Locally tested against the installed `swisseph` 0.5.17. Bodies 0–9 and Chiron
only; no fixed stars. Not integrated.** `swe_azalt`, `swe_azalt_rev`, `swe_refrac`,
`swe_refrac_extended` and `swe_rise_trans` (sunrise, sunset, meridian transits, twilight).
These are the calls behind the backend's horizon coordinates (`func.js`) and planetary
hours (`func-ore-planetarie.js`).

## What exists

| Piece | Where |
|---|---|
| Refraction (two published models), horizon dip, pressure estimate, altitude/azimuth, inverse, ecliptic→equator | `ephemeris-core::horizon` |
| Rise/set/transit search with diurnal parallax, any body through a closure | `ephemeris-core::rise_transit` |
| `Engine.azaltUt`, `azaltRevUt`, `riseTransUt`, `refrac`, `refracExtended` | `bindings/node` |
| `swe_azalt`, `swe_azalt_rev`, `swe_refrac`, `swe_refrac_extended`, `swe_rise_trans` in the Swiss-shaped facade | `bindings/node/index.js` |

### Refraction

Two models, both from the published literature, with pressure `P` (hPa) and temperature
`T` (°C):

- **Plain** (`swe_refrac`): Sæmundsson (1986) for true altitudes up to 15°, the
  `58.276″ tan z − 0.0824″ tan³z` series above, scaled by `(P/1010)(283/(273+T))`.
- **Extended** (what `swe_azalt` and `swe_rise_trans` use): the *Astronomical Almanac*
  formula as a function of the apparent altitude, `(34.46 + 4.23a + 0.004a²)/(1 + 0.505a +
  0.0845a²)′` up to 17.904°, `0.97′/tan a` above, scaled by `((P−80)/930)/(1 + 8·10⁻⁵(R+39)(T−10))`.
  True→apparent is solved by iteration, apparent→true is direct. The result is applied only
  if the refracted altitude is above the **horizon dip** of the observer's height
  (Schaefer & Liller), so a body just below the horizon jumps up across it.

It took a measurement to learn that `swe_azalt` uses the second model: assuming the
Sæmundsson series gave 2–3 % errors in the refraction. Pressure `0` means "unknown" and is
estimated as `1013.25 (1 − 0.0065 H / 288)^5.255`; a temperature of `0` is taken literally.

### Horizon coordinates

`swe_azalt` takes **true ecliptic** (`SE_ECL2HOR`) or **true equatorial** (`SE_EQU2HOR`) of
date coordinates, the geodetic latitude as given, and the apparent sidereal time (IAU 2006
/ IAU 2000B). Azimuth is returned **from the south toward the west**, as the reference does
(the backend adds 180° itself). `swe_azalt_rev` returns the ecliptic or equatorial
coordinates of a *true* altitude. Output keys: `{azimuth, trueAltitude, apparentAltitude}`
and `{azimuth, trueAltitude}` (the reference reuses these two names for longitude/latitude
or RA/Dec).

### Rise, set, transit

The search is a scan of the event function in half-hour steps with a bracketed refinement
to below a millisecond:

- the **centre** of the body must cross `h* − s` (upper limb), `h*` (centre,
  `SE_BIT_DISC_CENTER`) or `h* + s` (lower limb, `SE_BIT_DISC_BOTTOM`), with `h*` the true
  altitude whose apparent altitude is zero (−0.56° at 15 °C; 0 with `SE_BIT_NO_REFRACTION`)
  and `s` the topocentric semi-diameter from the radius table the reference also uses (Sun
  696,000 km, Moon 1,737.5 km, planets from their IAU radii). `SE_BIT_FIXED_DISC_SIZE`
  freezes the Sun's and Moon's disc at their mean distance. Civil, nautical and
  astronomical twilight put the centre at −6°, −12°, −18° with no refraction;
- the position is **topocentric**: the WGS84 observer vector, rotated by the sidereal
  time, is subtracted from the apparent geocentric vector (diurnal parallax, 1° for the
  Moon). Diurnal aberration and polar motion are not modelled;
- transits are the hour angle of that direction being 0° (`SE_CALC_MTRANSIT`) or 180°
  (`SE_CALC_ITRANSIT`);
- a body that only *grazes* the horizon for less than a scan step (the Moon at high
  latitudes can clear it for eleven minutes) is found by locating the local extremum of the
  event function; a plain scan would miss it;
- no event in the window is `null` in the engine and `{name: '', transitTime: -2}` in the
  facade, never an error.

**Search window.** The reference reports `-2` when the next event is more than about 26
hours after the start (measured: found at 1.0811 days, missed at 1.0855). The facade
reproduces this so callers that treat `-2` as "none today" behave the same; the engine's
`riseTransUt` takes a window of its own (default 2 days, at most 31), so a caller who wants
the next sunset of the midnight-sun period can have it.

## Evidence

Receipt: `docs/evidence/mvp2-horizon-events-local-reference-receipt-2026-11-16.json`
(aggregates only).

- **azalt / azalt_rev** (3000 random sites, 1950–2026, ecliptic and equatorial, pressures
  0/850/1013.25/1030, temperatures 0/15/−20/35, heights up to 4000 m): azimuth and true
  altitude agree to **3 milliarcseconds**, the apparent altitude to 0.04″.
- **Extended refraction** over 0–90° at five atmospheres and two heights: true→apparent to
  0.03″, apparent→true exactly, dip to 0.06″ (the reference's dip differs by a constant at
  the 1e-5° level).
- **Plain `swe_refrac`** true→apparent is exact. The apparent→true direction does **not**
  reproduce the reference's own inverse (it differs by up to 21″ at 700 hPa, 3″ rms) because
  the reference uses another approximate formula that the plain model does not call for; the
  engine returns the exact inverse. Nothing in the backend uses it.
- **Rise/set/transit** (about 43,000 random cases: Sun, Moon, Mercury–Saturn, Pluto, ten sites
  including Reykjavik, Tromsø and Murmansk, all option bits, three atmospheres):
  - events present or absent **coincide in every case**, polar ones included;
  - transits agree to 0.04 s (planets) and 0.9 s (Moon); rise and set to hundredths of a
    second, rms 0.05 s. The largest residuals are near-grazing polar passages, where the time
    is very sensitive to altitude (Moon 5.6 s for an eleven-minute passage; the Sun at
    Reykjavik next to the midnight-sun period 1.4 s).
  - The Moon's 0.2 s rms is most likely the reference's own ΔT and UT1 handling (not isolated).
- Core: 18 offline tests for refraction, geometry and the search (stars rising at zero
  altitude, hour angle zero at transit, circumpolar bodies, parallax, limb and refraction
  ordering, grazing passages from sixty start offsets, refusals). Node: 10 tests (3 without
  data, 6 with the kernel, 1 against the real `swisseph`); the whole Node suite is 56 tests with
  all data (14 pass and 42 skip without). All pass.

## Not done

- **Fixed stars** in `swe_rise_trans` (a star name returns `E_UNSUPPORTED_BODY`).
- The bits `SE_BIT_GEOCTR_NO_ECL_LAT` (128) and `SE_BIT_HINDU_RISING`, which need a
  geocentric-ecliptic-latitude convention; they return `E_UNSUPPORTED_FLAG`.
- Lunar points and Earth have no rise or set (`E_UNSUPPORTED_BODY`).
- The lapse rate of `swe_refrac_extended` enters only the horizon dip, as in the reference;
  the dip formula differs from the reference by up to 0.06″.
- Dates outside 1900-01-01 … 2027-06-28 UT without a loaded ΔT table (same limit as `swe_calc_ut`).
- Diurnal aberration, polar motion, a custom horizon height (`swe_rise_trans_true_hor`) and
  sidereal modes.

## A finding about the backend

`backend/services/astrology/mezzogiorno-astronomico.js` calls `swe_rise_trans` with the
shape `(jd, body, [lon, lat, alt], 2, SE_CALC_MTRANSIT, null, callback)`. That is not the
signature of the installed `swisseph` package, which reads the third argument as a **star
name**: it returns `{name: "Virgo Cluster,VC", transitTime: null}` (observed), i.e. the
helper does not compute a solar noon at all. The facade refuses that shape with an explicit
`E_RANGE` error instead of repeating the garbage. The correct call is the ten-argument
form used in `func-ore-planetarie.js`. This was observed, not changed: the backend was
only read.
