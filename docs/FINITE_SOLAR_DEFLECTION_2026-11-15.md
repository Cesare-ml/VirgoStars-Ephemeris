# Experimental solar deflection for finite sources

`solar_deflection::deflect_finite_source` calculates a first-order point-mass
solar light-deflection direction for a source at finite distance. It is a
standalone position-only Rust primitive: it does not calculate light time,
aberration, rates, a date frame, time conversion, or an apparent position.
The primitive itself is not an apparent-place API. The opt-in planetary
composition described below uses it as one stage.

The caller supplies source-from-observer and Sun-from-observer vectors in AU,
in common fixed ICRF axes and at epochs consistent with the caller's model.
The source direction from the Sun is reconstructed as their vector difference.
With unit source direction `p`, unit Sun-to-source direction `q`, unit
Sun-to-observer direction `e`, and observer-to-Sun distance `R`, the correction
uses the finite-source point-mass vector in equation (1) of
[Kaplan (1998), USNO](https://aa.usno.navy.mil/downloads/reports/Kaplan1998.pdf).
It uses nominal solar GM 132712440000 km³/s², radius 695700 km and exact SI
speed of light, normalizing the output direction after the first-order step.

The observer and source must be outside the adopted solar sphere; a ray that
passes through the solar disk **before reaching the source** is an error.
This distinction permits a nearby foreground source on the solar sightline.
Zero/nonfinite vectors and singular geometry also fail. The source may not be
the Sun itself. Solar multipoles, higher-order bending, other gravitating
bodies, observed solar limb, atmospheric refraction and finite-source light
travel are not included. Any finite-source apparent pipeline must compute the
appropriate source and deflector epochs, handle velocities and document its
time scales and frames before using this result.

Four geometric synthetic samples (foreground, distant behind the Sun, near
observer and quadrature) were compared against independently generated
`erfa.ld` output from pyerfa 2.0.1.5. The component tolerance is 1e-12. Solar
occultation, foreground alignment and invalid geometry have separate tests.
ERFA is used externally for reference values and is not a runtime dependency;
no third-party code or ephemeris files are included. These tests do not
validate a complete apparent planetary pipeline or provide an accuracy bound.

## Experimental planetary composition

`planetary_solar_deflection::calculate_deflected_planet` composes DE441
Earth-to-target reception light time with the finite-source correction above,
annual aberration and true ecliptic or equatorial projection. Sun-to-observer
is sampled at reception and the target at the iterated emission epoch; a
Mars–Pluto physical center additionally requires an explicit satellite SPK
sampled at each emission epoch. Self-deflection is not applied when the Sun is
the target. No higher-order or planetary deflectors are included.

The corrected ICRF velocity is a centered difference of the entire corrected
position at ±0.005 TDB day, including the retarded target and moving Sun and
Earth. Projection adds the existing numerical frame motion. Both samples must
be available and unobscured; the method fails if any required kernel does not
cover them or if the source passes behind the adopted solar disk. Numerical
rates, approximate TT/TDB derivatives, the single-f64 epoch representation,
and mixed DE441/satellite solution provenance limit precision. This is not
Swiss parity or a certified complete apparent-place model.

Node callers can opt in with `planetSolarDeflection: 'POINT_MASS'`; the
default is `'OMIT'`. The option applies to geocentric ecliptic and equatorial
`swe_calc_ut` output and has no effect on the existing heliocentric mode.
Warnings indicate the chosen correction and numerical rates, and identify
the Sun's self-deflection exception. The integration tests exercise all ten
targets with real DE441 and supplemental SPKs, both frames for selected
targets, numerical rate consistency, missing coverage, and the unchanged
heliocentric behavior. The finite-source ERFA comparison validates only the
isolated correction, not the entire planetary composition.

An independent end-to-end diagnostic used [NASA Horizons](https://ssd-api.jpl.nasa.gov/doc/horizons.html)
observer quantity `31`, target IDs 199, 299 and 499, geocenter `500@399`,
`TIME_TYPE=TT`, TT JD `2461354.500800741`, airless, on 2026-11-15. The
reported ecliptic longitude/latitude in degrees were Mercury
`216.3231960, 1.3285547`; Venus `203.1924041, -2.6211221`; Mars
`143.0142629, 1.7953367`. This implementation at TT `2461354.500800741` and
TDB `2461354.500800725` differed by less than 0.00003° in each component.
Horizons uses IAU 1976/1980 ecliptic-of-date and mar099 for Mars; this branch
uses IAU 2006/2000B and mar097. The model and precision differences preclude
direct high-precision parity claims. This single epoch is not a general bound,
nor does it independently validate velocities, all bodies, or occultations.
