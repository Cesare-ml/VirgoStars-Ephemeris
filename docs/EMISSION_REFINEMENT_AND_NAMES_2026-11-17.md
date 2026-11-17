# Experimental emission refinement and Swiss-name leads — 2026-11-17

**EXPERIMENTAL / LOCAL ONLY.** This follows
[the 929-candidate and ICRF checkpoint](HIP2_LEADS_AND_ICRF_2026-11-17.md).
The normal apparent pipeline and production provider remain unchanged.

## Light-time emission rounding

The existing DE441 reader takes a single f64 TDB Julian date. During the
light-time iteration, the emitted epoch is rounded to a representable Julian
date. At modern epochs this rounding is on the order of tens of microseconds;
it introduces small steps in position, especially noticeable when estimating
the Moon's rate by differencing closely spaced positions.

Only the experimental long-term mean/apparent and apparent-ICRF paths now
apply a first-order correction: the residual between the iterated light time
and the rounded emission epoch is evaluated with the difference of the
nearby Julian dates, and the DE441 target's barycentric velocity advances its
position by the residual. Acceleration adjusts the target velocity; the
light-time range and rate are recomputed. The emission epoch field is still
rounded. The correction checks its bound and kernel coverage and refuses
invalid geometry. The ordinary `calcTdb`/`calcUt` paths are not switched.
This does not turn the reader into a full split-date implementation; the
solar-deflection geometry and other computations still use single-f64 epochs.

With the same local DE441 and the same 21 epochs, Sun/Moon and eight planets
(Mars–Pluto **system barycenters**), the JPL Horizons apparent ICRF direction
comparison changed as follows:

| Diagnostic | Before | After |
| --- | ---: | ---: |
| Maximum lunar angular difference | 0.0011052″ | 0.0001102″ |
| Maximum other-body angular difference | 0.000004205″ | 0.000003689″ |
| Maximum Moon-involving separation difference | 0.001105″ | 0.0001112″ |

The ICRF samples compare 210 airless, geocentric apparent directions,
including light-time, solar deflection and annual aberration. The separations
compare 945 body pairs in the existing ecliptic diagnostic. These compare
implementations that both use DE441, not independent orbital truth. Horizons
and this engine also use different ecliptic-of-date models: the maximum
longitude difference remains approximately 12.154″ on the sampled grid.

In the local Swiss JPL-mode comparison, the maximum difference between this
engine's lunar longitude rate and its own centered finite difference at ±0.001
TDB day decreased from 0.11981″/day to 0.000101″/day on the same 21-epoch
grid. This is a numerical consistency diagnostic with a finite-difference
step, not an uncertainty bound or a measurement of real-world accuracy.
The experimental Node regression uses a looser 0.003″/day limit for both
longitude and latitude across those 21 epochs.

The new `compare-swiss-long-range.cjs` separately compares 210 apparent
geocentric positions (years 0–5000 at 250-year steps) with caller-installed
Swiss using the same DE441. It checks the JPL flag for every reference call.
Input is TDB here and approximately corresponding TT in Swiss. Maximum
differences were 0.0627″ longitude and 0.6838″ latitude; these are cross-model
diagnostics with no accepted release tolerances. Swiss's reported rates and
its own ±0.001-day finite differences also differ on this sample. Neither
cross-model rate difference is sufficient evidence to attribute an error to
one implementation. Local reports:
`horizons-apparent-icrf-refined.json`, `horizons-grid-250y-refined.json`
and `swiss-long-range-250y-refined.json`.

## Five additional lookup names

`scripts/attach_verified_swiss_names.mjs` consumes the prior hash-checked
three-epoch Swiss-name gap report. It only proposes a name when Swiss returned
the exact requested spelling at every epoch, one existing catalog candidate
matched, the candidate already has the same unambiguous Swiss designation,
and either the group has identical source astrometry or the new name matches
an already-selected named anchor's exact source astrometry. The local IAU
snapshot contains no conflicting HIP identity. These are still **candidates**,
not individually approved identities. Tests reject conflicting IAU
identifications, multiple designations and stale or incomplete reports.

The resulting local manifest adds **Marfak**, **Alrai**, **Hyadum II**,
**Arundhati** and **Saidak** as lookup aliases for existing stars. Six other leads remain withheld,
including ambiguous multiple systems and the Swiss/IAU conflict for Fafnir.
The 929 candidates now cover 560 of 729 unique Swiss names and 936 of 1,113
designations: 169 names and 177 designations remain unresolved. All 567
aliases passed 1,701 comparisons against Swiss at the same three epochs;
the local loopback backend test loaded the updated manifest with Chiron.
The manifest and receipts are ignored local data:
`manifest-name-929-v2.json`, `name-929-extension-v2.json`, and
`aliases-name-929-v2.json`.

Remaining work includes the omitted stellar identities, multiple systems,
long-range ecliptic reference and accepted error budgets, date/time conversion
and representative full-backend parity. No production activation is authorized.
