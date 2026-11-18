# Draft inquiry to JPL Horizons — not sent

**Suggested recipient:** Jon.D.Giorgini@jpl.nasa.gov, listed for inquiries in the [Horizons manual](https://ssd.jpl.nasa.gov/horizons/manual.html). This is an inquiry address, not the automated Horizons email command interface. The owner should review and send the message if desired.

**Subject:** Clarification of Horizons Q46 versus the time derivative of Q45 apparent ICRF coordinates

Dear Horizons team,

We would appreciate clarification of the conventions used for observer-table quantity 46, inertial apparent RA and declination rates. In a few tests, the printed Q46 values differ from centered time differences of the printed quantity 45 apparent coordinates. We may be misunderstanding a convention, so we would be grateful for your guidance.

One reproducible case is the Moon, observed from the geocenter, ICRF, airless, using Horizons' DE441. We queried the API on 2026-11-18 with these parameters:

```text
format=json
COMMAND='301'
OBJ_DATA='NO'
MAKE_EPHEM='YES'
EPHEM_TYPE='OBSERVER'
CENTER='500@399'
TLIST='2451673.994999984 2451673.999999984 2451674.004999984'
TLIST_TYPE='JD'
TIME_TYPE='TT'
CAL_FORMAT='JD'
QUANTITIES='1,45,46'
REF_SYSTEM='ICRF'
ANG_FORMAT='DEG'
CSV_FORMAT='YES'
EXTRA_PREC='YES'
APPARENT='AIRLESS'
```

The three Q45 coordinates returned were approximately:

```text
JD TT              RA apparent deg    Dec apparent deg
2451673.994999984  124.025016252       20.184341034
2451673.999999984  124.100179731       20.174659812
2451674.004999984  124.175326866       20.164947611
```

Using the outer points over approximately 0.01 TT day, scaling the RA derivative by cos(center declination), and converting to arcseconds/hour gives approximately `(2116.3261, -290.90135)`. Q46 at the center prints `(2116.140, -290.877)` arcseconds/hour. A second half-step of 0.01 day yields a similar discrepancy. The finite difference has rounding and truncation error, and its TT rate may differ slightly from a TDB rate; neither should be ignored.

For a contrasting near-Sun case, set `COMMAND='199'` at the same epochs and center. The Sun–Mercury apparent separation is about 0.416 degrees, so we treat it only as a numerical stress case, not an observable or validated limb result. Our ERFA-based decomposition suggests that the time dependence of light deflection makes a measurable contribution there, whereas the Moon case is dominated by stellar aberration. Both our local and Horizons computations use DE441; this is not an independent dynamical-ephemeris comparison.

Could you please clarify:

1. Is Q46 intended to be the full time derivative, with respect to reception time, of Q45, or is it a projected relative velocity using a different convention?
2. Are the derivatives of the target emission epoch/light time, stellar aberration (including observer acceleration), and solar light deflection included in Q46? If only some terms are included, which ones and at which epochs?
3. Is Q46 expressed per TT, TDB or another time parameter, and are there accuracy or rounding limits that could explain the example? Is there a preferred Horizons quantity or method for the complete time derivative of Q45?

We have a related Q20 range-rate question. For `COMMAND='199'`, `CENTER='500@399'`, at JD TDB 2442595.0 (approximate JD TT 2442594.999999999), Q20 `deldot` is about 33.7223560 km/s, while a four-point centered derivative of its `delta` range gives about 33.7173940 km/s at both 0.02- and 0.04-day half-steps. The queried JD TT values were `2442594.919999999, 2442594.959999999, 2442594.979999999, 2442594.999999999, 2442595.019999999, 2442595.039999999, 2442595.079999999`, with `QUANTITIES='20,23,45'` and the other settings above. In a local diagnostic the former is close to the line-of-sight projection of `V_target(emission) − V_observer(reception)`, and the latter to the projection including the derivative of emission time. Is that distinction intended in Q20, and which Horizons output is recommended for the full reception-time derivative of apparent range? We can supply the raw tables.

We can supply the full API query, response, the exact finite-difference calculation and further cases if helpful. Thank you for your assistance.

Kind regards,

[Name and affiliation]
