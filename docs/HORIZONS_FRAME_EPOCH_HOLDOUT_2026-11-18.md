# Ancient frame comparison and rotation-fit correction

**Status: exploratory comparison of conventions, not physical validation.**
This continues the [earlier two-era frame analysis](HORIZONS_DENSE_RATE_HOLDOUT_2026-11-18.md).
It corrects the interpretation of that analysis's one-pass fit residuals and
extends the comparison across epochs. Production calculations are unchanged.

## Correction to the earlier fit

The earlier diagnostic solved for a rotation using one linearized step, then
evaluated the residual using an exact rotation. At ancient epochs, the
rotation is large enough that the neglected second-order terms contribute to
the reported residual. Replaying the **same 26 stored epochs and six stored
Horizons directions**, with iterative linearized steps composed as exact
rotations, gives:

| Astronomical year | Largest old one-step RMS | Largest iterated RMS | Largest change in rotation vector |
| --- | ---: | ---: | ---: |
| −9998 | 0.017292″ | 0.00000247″ | 0.02454″ |
| 2000 | 0.00000203″ | 0.00000203″ | 0.000000003″ |

These residuals describe the fit between the particular **coordinate frames**
and sampled directions. In particular, the earlier 0.0173″ value should not
be treated as an astronomical discrepancy: it is dominated by the numerical
linearization of that fit. The roughly 214″ frame rotation itself remains.
The refined residual is near the printed-coordinate precision and is not a
measurement of physical Earth-orientation accuracy.

## More epochs and held-out directions

Request Horizons apparent airless geocentric ICRF and true equatorial/ecliptic
coordinates at four proleptic-Gregorian dates per year in 15 astronomical
years: −9997, −9000, −8000, −7000, −6000, −5000, −4000, −3000, −2000,
−1000, 0, 1000, 2000, 3000 and 3999. That is 60 TT epochs and six bodies
per epoch. January of astronomical year −9998 was rejected by Horizons as
before its DE441 service limit; the local kernel's longer interval is not
tested by that service here. Astronomical numbering includes year zero.

Project each Horizons ICRF direction through the local experimental
Vondrák-based true equator, including the existing extrapolated nutation.
Fit the rotation to Sun, Venus, Jupiter barycenter and Saturn barycenter;
apply it to the **held-out Moon and Mars barycenter** at the same epoch.
The held-out bodies were not used to estimate the rotation, but they are
correlated outputs from the same Horizons solution, not independent
astronomical observations. The local ephemeris does not supply the fitted
directions. Separately compare local pole-based and mixed direct-obliquity
ecliptic projections of the same directions to Horizons ecliptic coordinates.

The JPL [Horizons manual](https://ssd.jpl.nasa.gov/horizons/manual.html)
describes its true-of-date conventions and long-term Owen model; they are
not identical to the local long-term model. Both conventions are calculations,
not direct observations of the ancient Earth. Frame rotation combines model,
precession, nutation and convention differences; it does not by itself tell
which is more physically accurate.

The mean below is the magnitude of the mean rotation vector for the four
dates; seasonal deviation is measured from that vector.

| Astronomical year | Mean fitted frame rotation | Largest seasonal deviation | Largest held-out RMS |
| --- | ---: | ---: | ---: |
| −9997 | 214.415″ | 0.132″ | 0.0000033″ |
| −5000 | 46.966″ | 0.023″ | 0.0000042″ |
| 0 | 3.048″ | 0.0055″ | 0.0000029″ |
| 2000 | 0.0527″ | 0.00047″ | 0.0000024″ |
| 3999 | 7.934″ | 0.0059″ | 0.0000034″ |

Across all 60 epochs the largest training RMS is 0.00000258″ and largest
held-out RMS is 0.00000524″. The smallest elimination pivot is 0.163; a pivot
is not a formal condition-number bound. The tiny residuals, close to the
coordinate output precision, are consistent with a common rotation for these
directions. They do not prove the frame or positions accurate to microarcseconds.
The seasonal spread is a four-date sample, not a bound over each year or a
measurement of the true historical pole.

At year −9997, maximum angular separations of local versus Horizons
ecliptic coordinates across these six bodies and four dates are about 211″
for the pole-based convention and 214″ for the mixed direct-obliquity
convention. This is a comparison with Horizons, not the smaller residual
previously observed against Swiss; the references use different conventions.
It is not sufficient to choose a production model.

## Reproduction and evidence

The projection example uses local orientation code and needs no DE441 file.
The first command samples Horizons over the network and records the raw
responses locally. The optional source/hash form re-parses and validates
those responses offline, rather than trusting cached coordinates. Outputs
must be new paths; the executable and any supplied input-report hashes are
checked before and after.

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example diagnose_horizons_orientation_components
node bindings/node/scripts/sample-horizons-frame-epochs.cjs EXAMPLE NEW_REPORT
node bindings/node/scripts/sample-horizons-frame-epochs.cjs EXAMPLE NEW_REPORT RAW_REPORT RAW_SHA256
node bindings/node/scripts/audit-horizons-frame-linearization.cjs OLD_REPORT OLD_SHA256 EXAMPLE NEW_REPORT
```

- New raw report SHA-256: `06d6e9f20559bfc784d7242e40e072ae1939fa33b9ff9125542bd486da12c70e`.
- Iterated 60-epoch report SHA-256: `e89a316982636771ae817afebd2a603a635efd70041f7ee305617830b285f688`.
- Earlier two-era report SHA-256: `405baf381ba1fa8e5f1edff4ff36e66f428ebca51f4685f3b33058f5de213ecd`.
- Same-data linearization audit SHA-256: `8cadbffa280083fbb2044be191ddd089f5b7c2a9571d54fa071d7fbd72f22070`.
- Projection executable SHA-256 for this Windows run: `88ff68f242899aed553f9c7cd2e641020c9cf54bdb6487018f1fd7df0b338a4e`.

Reports and external responses remain local and untracked. Synthetic tests
cover known rotations, held-out discrepancies, convergence and rejection of
directions outside the diagnostic's small-angle range. Further work requires
physically motivated orientation model selection, documented tolerances and
validation beyond the Horizons service limit; no automatic correction has
been introduced.
