# MVP 2 — global solar and lunar eclipses (2026-11-16)

**Status: Locally tested against the installed `swisseph` 0.5.17. Global circumstances
only (no local circumstances, no `where`). Not integrated.** `swe_sol_eclipse_when_glob`
and `swe_lun_eclipse_when`: the two eclipse calls of the backend routines
`calcolo-eclissi-solari-per-db.js` and `calcolo-eclissi-lunari-per-db.js`.

## What exists

| Piece | Where |
|---|---|
| Lunar eclipses: opposition search, kind, penumbral/partial/total contacts, magnitudes | `ephemeris-core::eclipse` |
| Solar eclipses: conjunction search, kind (total, annular, hybrid, partial, central or not), contacts of the penumbra, umbra and axis with the WGS84 globe, greatest eclipse, local noon of the ground point | `ephemeris-core::solar_eclipse` |
| `Engine.lunarEclipseUt(jd, backward)`, `solarEclipseUt(jd, backward)` | `bindings/node` |
| `swe_sol_eclipse_when_glob`, `swe_lun_eclipse_when` in the Swiss-shaped facade (result keys, `ifltype` filter, `backward`, trailing callback) | `bindings/node/index.js` |

Both core modules take a closure with the apparent geocentric positions of the Sun and the
Moon (as `rise_transit` does), so they are tested with synthetic bodies and know nothing
about DE441.

## Method

**Search.** A syzygy is found from the right-ascension difference of Sun and Moon (Newton
with the mean elongation rate), then the instant of closest approach to the shadow axis is
found by golden section within ±1 day. A candidate is an eclipse when the Moon (lunar) or
the penumbra (solar) touches the Earth's shadow / globe. All 450 lunations of 1900–2026 are
scanned in a few seconds.

**Lunar.** The Earth's shadow is the pair of cones tangent to the Sun and to the Earth; the
Moon is a sphere of 1,738.15 km. Penumbral, partial and total contacts are `ρ = r_p + R☾`,
`ρ = r_u + R☾` and `ρ = r_u − R☾`; the greatest eclipse is the minimum of `ρ`. **The Earth is
enlarged by 1 % for the atmosphere** (`R⊕/0.99`): this is what the reference does. It was
measured, not assumed: without it the contact times were about 77 s early and 5 of 83
eclipses were classified a step too weak; the fitted bias in the shadow radii was 64.6 km for
the penumbra, the umbra and the total contact alike (scatter 1 km), and `R⊕/0.99` removes it
(bias 0.0 km).

**Solar.** The axis goes from the Moon to the Sun. The penumbral and umbral cones have
half-angles `sin f = (R☉ ± R☾)/D`; beyond the umbra apex the same cone is the antumbra of an
annular eclipse. The Earth is the WGS84 ellipsoid. A point `P` is shadowed when its distance
from the axis `ρ(P)` is below the cone radius at its coordinate `z` along the axis; contacts
are the zeros of `min_P [ρ(P) − r(z(P))]`, the minimum found by iterating the tangency
condition (the gradient of `ρ + z tan f` is antiparallel to the outward normal); an axis that
pierces the globe is handled analytically. `begin`/`end` are the first and last contact of
the penumbra, `totalBegin`/`totalEnd` of the umbra or antumbra, `centerBegin`/`centerEnd` of
the axis. `maximum` is the instant the axis is closest to the geocentre. `noon` is the
instant, near the maximum, at which the point of the globe under the axis has the Sun on its
meridian: this reproduced the reference's `noon` to 0.05 s in the first comparison, but the
reference does not document the field, so the definition is inferred.

Kind bits are the reference's: 4 total, 8 annular, 16 partial, 32 hybrid, plus 1 central or 2
non-central; lunar 4, 16, 64. Hybrid means total in the middle and annular at the ends of the
central line.

## Evidence

Receipt: `docs/evidence/mvp2-eclipses-local-reference-receipt-2026-11-16.json`. Every
eclipse from 1900-03 to 2026-01 compared in sequence:

| | Eclipses | Kind or date mismatches | Maximum (rms) | Contacts (rms) |
|---|---|---|---|---|
| Lunar | 289 | **0** | 1.1 s | penumbral 1.4 s, partial 2.5 s, total 5 s (max 43 s, shallow totals) |
| Solar | 285 | **0** | 2.9 s | partial begin/end 21 s (max 175 s), umbral 5.5–6.7 s, central line 8.8 s, `noon` 0.7 s |

- Type filters (`ifltype` total, annular, partial, hybrid; lunar total, partial, penumbral)
  and backward searches (40 eclipses from 2020) give the **same sequences** as the reference.
- Key sets identical to the reference's. Known events agree with the published values to
  seconds: 2024-04-08 total (18:17:20 UT here, 18:17:16 published), 2023-10-14 annular,
  2023-04-20 hybrid, 2025-03-14 and 2022-11-08 total lunar, 2024-03-25 penumbral.
- The maximum of a solar eclipse is limited by the reference's own ΔT; the contacts on the
  globe by the Earth model. The reference's central line fits a slightly smaller Earth (with an
  equatorial radius of 6,375 km it agrees to 3 s), but that is not a standard radius, so WGS84
  is kept and the difference documented.
- Core: 9 offline tests with synthetic bodies (classification by impact distance, contact
  order and symmetry, annular/partial/total kinds, a central-line miss, no eclipse, bad input,
  search direction). Node: 6 tests in `eclipses.test.mjs` (1 without data, 4 with the kernel,
  1 against the real `swisseph`); the whole Node suite is 70 tests with all data.

## Behaviour to know

- **The backend solar routine reads `risultato.partialBegin` / `partialEnd`.** The real result
  has `begin` and `end` (the partial phase) and **no** `partialBegin`, so `inizio_parziale_utc`
  and `fine_parziale_utc` of its JSON are always `null`, with Swiss as with this engine.
  Observed, not changed (the backend was only read).
- Time range: UT from 1900-01-01 to 2027-06-27 (same limit as `swe_calc_ut` without a ΔT table).
  The 400-day search window can run into the end of that range for the last eclipse.
- The reference returns no result for a centrality bit (`SE_ECL_CENTRAL`, `SE_ECL_NONCENTRAL`)
  alone as `ifltype`; this engine filters by it.
- `noon` is `0` when the point under the axis never crosses the Sun's meridian within half a
  day: about one in five of the near-polar eclipses (|gamma| above 0.95); the reference gives a
  value there.
- For the most grazing partial eclipses (|gamma| above 1.5) `begin`/`end` can differ by minutes;
  for one of them (1946) the reference itself returns `begin = end = 0`.
- The hybrid/total/annular decision samples the central line at its ends and middle; a hybrid
  whose annular ends last only seconds could be classified total. None of 285 was.

## Not done

- **Local circumstances**: `swe_sol_eclipse_when_loc`, `swe_sol_eclipse_how`,
  `swe_lun_eclipse_when_loc`, `swe_lun_eclipse_how`, the geographic `swe_sol_eclipse_where`, and
  occultations of planets and stars.
- Saros numbers and the magnitudes of solar eclipses. The lunar magnitudes are computed in the
  core and exposed on `Engine.lunarEclipseUt`, not in the Swiss-shaped result.
- Dates outside 1900 – mid 2027 (as for every UT entry point without a loaded ΔT table).
