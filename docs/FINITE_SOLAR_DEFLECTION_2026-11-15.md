# Experimental solar deflection for finite sources

`solar_deflection::deflect_finite_source` calculates a first-order point-mass
solar light-deflection direction for a source at finite distance. It is a
standalone position-only Rust primitive: it does not calculate light time,
aberration, rates, a date frame, time conversion, or an apparent position.
It is not yet connected to `swe_calc_ut` or planetary output.

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
