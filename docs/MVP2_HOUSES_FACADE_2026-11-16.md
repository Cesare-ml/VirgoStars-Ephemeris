# MVP 2 — `swe_houses` facade and the extra house points (2026-11-16)

**Status: Locally tested against the installed `swisseph` 0.5.17. Systems V, P, K, R, O
only. Not integrated.** The `swe_houses` and `swe_houses_pos` calls of the backend's
`func.js`.

## Where things live

| Piece | Repository |
|---|---|
| Vertex, East Point, Koch and Munkasey co-ascendants (astronomy from ARMC, obliquity, latitude) | ephemeris core, `house_angles::ExtraAngles` |
| Cusps of Vehlow, Placidus, Koch, Regiomontanus, Porphyry, with polar warnings | `astrological-functions` (`calculateHouseSetWithWarnings`, branch `feat/houses-reference-validation-20261116`) |
| `swe_houses`, `swe_houses_pos` in the Swiss-shaped facade | `bindings/node/index.js` |

House systems are astrological methods, so the ephemeris package does **not** depend on
`astrological-functions`: the caller injects its functions,

```js
const facade = createSwissShapedFacade(engine, {
  houseMethods: { calculateHouseSetWithWarnings, calculatePlacidusHousePosition },
});
facade.swe_houses(jdUt, lat, lon, 'P'); // { house[12], ascendant, mc, armc, vertex,
                                        //   equatorialAscendant, kochCoAscendant,
                                        //   munkaseyCoAscendant, munkaseyPolarAscendant }
```

Without `houseMethods` the call returns an explicit error naming what to inject.

## The five extra points

`asc(a, φ)` is the ecliptic point rising at sidereal angle `a` for latitude `φ` (the
Ascendant formula already in the core). Their construction was **found by fitting** the
reference over 2,496 sites and dates (every combination of a quarter-turn shift, six
latitude forms and a half-turn flip), then confirmed over 19,000 comparisons:

| Point | Construction |
|---|---|
| Vertex | `asc(ARMC + 180°, 90° − φ)`; when `|φ|` ≤ the obliquity (two crossings of the prime vertical) the one west of the Midheaven |
| Equatorial ascendant (East Point) | `asc(ARMC, 0)` |
| Koch co-ascendant | `asc(ARMC, −φ)` |
| Munkasey co-ascendant | `asc(ARMC, 90° − φ)` |
| Munkasey polar ascendant | Koch co-ascendant + 180° |

They are definitions of points; the core tests check the geometry itself (the vertex lies
on the prime vertical, in the west north of the tropics; the East Point has right ascension
ARMC + 90°).

## Behaviour that matters to the backend

- **Exactly the reference keys by default.** The backend loops over every property of the
  result and mirrors it for southern latitudes, so an extra key (a `warnings` array) would
  break it. `{ provenance: true }` adds `warnings`, `status` (`OK`/`FALLBACK_PORPHYRY`) and
  `timeSource`; the default does not.
- **Polar latitudes return values.** The reference answers `{ error: "Can't calculate
  houses." }` for Placidus and Koch beyond the polar circle (948 of the 4,000 compared
  latitudes). The facade returns Porphyry cusps from the same angles and says so in the
  warnings (maintainer decision of 2026-11-16: values with a warning, never an interruption).
- **Unknown system letters are errors.** The reference silently uses Placidus for any unknown
  letter; the facade returns `E_UNSUPPORTED_FLAG`.
- **Time:** `tjd_ut` is taken as UT1; TT follows the engine's time policy, so a loaded
  measured-ΔT table is used.
- `swe_houses_pos` supports `'P'` only (the backend's use). Other letters are errors.

## Evidence

Receipt: `docs/evidence/mvp2-houses-facade-local-reference-receipt-2026-11-16.json`.
4,000 random dates (1950–2026) × five systems, latitudes −60…60 plus 15 % polar, any
longitude:

- result keys identical in every case; ascendant, MC, ARMC, vertex, East Point and the
  three co-ascendants agree to **0.04″** (rms 0.002″);
- cusps of Vehlow, Placidus, Koch, Porphyry agree to **0.07″**; Placidus house positions
  to 1e-8 of a house;
- Regiomontanus agrees except in **150 cases at |latitude| ≥ 67°**, where the reference puts
  the Midheaven (and the cusps tied to it) 180° away: its known polar mislabelling, not
  reproduced on purpose;
- engine errors where the reference succeeds: none.

Tests: core 1 new (extra points' geometry), Node 8 in `houses-facade.test.mjs` (1 without data, 6 with
the kernel and an `astrological-functions` checkout through `ASTROLOGICAL_FUNCTIONS_SRC`, 1
also with the real `swisseph`). The whole Node suite is 64 tests with all data.

## Not done

- Other house systems (equal `E`, whole sign `W`, Campanus `C`, Alcabitius `B`, …): they need
  implementations in `astrological-functions` first.
- `swe_houses_ex` with flags (sidereal houses), `swe_houses_armc`, position methods for
  systems other than Placidus.
- The Ascendant convention near the poles follows `astrological-functions` (rising point
  reflected to the east of the Midheaven where needed); no case differed from the reference
  in the comparison.
