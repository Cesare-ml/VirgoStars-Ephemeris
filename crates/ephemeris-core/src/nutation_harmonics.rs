// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

//! Numerically stable accumulation of **caller-supplied** nutation harmonics.
//!
//! No official IAU 2000A coefficient table is bundled here. Evaluating a
//! partial synthetic/user-supplied table does NOT produce IAU 2000A nutation
//! or prove compliance with NASA/JPL, SOFA/ERFA or Swiss Ephemeris.
//! Scientific series shape: USNO Circular 179, §5.4.2 and Appendix E.

use std::error::Error;
use std::f64::consts::TAU;
use std::fmt;

use crate::calendar::{JulianDate, TimeScale};

const J2000_TDB_JD: f64 = 2_451_545.0;
const TDB_DAYS_PER_CENTURY: f64 = 36_525.0;
const MAX_TESTED_CENTURIES: f64 = 2.0;

/// The six trigonometric amplitude coefficients of one nutation term.
///
/// Multipliers are ordered `[l,l',F,D,Omega]` for `N=5`, or
/// `[l,l',F,D,Omega,Mercury,Venus,Earth,Mars,Jupiter,Saturn,Uranus,
/// Neptune,general precession]` for `N=14`.
///
/// **All coefficients are in radians**, not 0.1 microarcseconds nor degrees.
/// The two *_slope_* coefficients are radians per **TDB Julian century**.
/// No automatic scale-factor conversion or coefficient-set provenance check.
///
/// The mathematical contribution is:
/// `dpsi=(s+sd*T)*sin(phi)+c*cos(phi)`;
/// `deps=(e+ed*T)*cos(phi)+f*sin(phi)`.
/// For the historical planetary IAU 2000A table the slope fields are zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NutationHarmonicTerm<const N: usize> {
    pub multipliers: [i8; N],
    pub longitude_sin_rad: f64,
    pub longitude_sin_slope_rad_per_century: f64,
    pub longitude_cos_rad: f64,
    pub obliquity_cos_rad: f64,
    pub obliquity_cos_slope_rad_per_century: f64,
    pub obliquity_sin_rad: f64,
}

/// Five-argument lunar-solar term, for future audited 678-term tables.
pub type LuniSolarHarmonicTerm = NutationHarmonicTerm<5>;
/// Fourteen-argument planetary term, for future audited 687-term tables.
pub type PlanetaryHarmonicTerm = NutationHarmonicTerm<14>;

/// A sum over **only the supplied terms**, not a certified full series.
///
/// Rates are with respect to **TDB day**: they are *not* a TT derivative
/// and cannot be passed to a TT-frame model without explicit reconciliation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NutationHarmonicSum {
    pub longitude_rad: f64,
    pub obliquity_rad: f64,
    pub longitude_rate_rad_per_tdb_day: f64,
    pub obliquity_rate_rad_per_tdb_day: f64,
    pub terms_evaluated: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NutationHarmonicError {
    UnsupportedArgumentCount,
    EmptyTerms,
    TooManyTerms,
    WrongTimeScale,
    UnrepresentableEpoch,
    OutsideTestedInterval,
    NonFiniteArgument,
    UnnormalizedArgument,
    NonFiniteCoefficient,
    ArithmeticOverflow,
}

impl fmt::Display for NutationHarmonicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedArgumentCount => write!(f, "nutation terms require 5 or 14 arguments"),
            Self::EmptyTerms => write!(f, "no nutation harmonic terms were supplied"),
            Self::TooManyTerms => write!(f, "harmonic table exceeds its IAU 2000A family size"),
            Self::WrongTimeScale => write!(f, "harmonic evaluation requires TDB Julian date"),
            Self::UnrepresentableEpoch => write!(f, "TDB epoch cannot be represented safely"),
            Self::OutsideTestedInterval => write!(f, "epoch outside tested J2000 +/-2 centuries"),
            Self::NonFiniteArgument => write!(f, "non-finite phase or phase rate"),
            Self::UnnormalizedArgument => write!(f, "fundamental angles must be in [0,2pi)"),
            Self::NonFiniteCoefficient => write!(f, "non-finite harmonic coefficient"),
            Self::ArithmeticOverflow => write!(f, "harmonic arithmetic is not representable"),
        }
    }
}

impl Error for NutationHarmonicError {}

/// Neumaier compensated sum: preserves small periodic contributions in the
/// presence of large positive/negative terms. No heap allocation required.
#[derive(Default)]
struct CompensatedSum {
    total: f64,
    residual: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) -> Result<(), NutationHarmonicError> {
        if !value.is_finite() {
            return Err(NutationHarmonicError::ArithmeticOverflow);
        }
        let next = self.total + value;
        if !next.is_finite() {
            return Err(NutationHarmonicError::ArithmeticOverflow);
        }
        let correction = if self.total.abs() >= value.abs() {
            (self.total - next) + value
        } else {
            (value - next) + self.total
        };
        self.residual += correction;
        if !self.residual.is_finite() {
            return Err(NutationHarmonicError::ArithmeticOverflow);
        }
        self.total = next;
        Ok(())
    }

    fn finish(self) -> Result<f64, NutationHarmonicError> {
        let final_sum = self.total + self.residual;
        if !final_sum.is_finite() {
            return Err(NutationHarmonicError::ArithmeticOverflow);
        }
        Ok(final_sum)
    }
}

/// Evaluate a **partial or complete caller-supplied** harmonic table.
///
/// The caller supplies `angles_rad` normalized to [0,2pi) and the matching
/// derivatives of the **unwrapped** fundamental arguments in rad/TDB day,
/// **all at the same supplied TDB epoch**. The API does not generate those
/// arguments, and cannot independently detect if their epochs differ.
/// The caller must also supply amplitude coefficients already in radians.
///
/// `N=5` is the lunar-solar family (at most 678 terms); `N=14` is the
/// planetary family (at most 687 terms). Unlike a full astronomy model,
/// any nonempty partial term slice is accepted and explicitly reported by
/// `terms_evaluated`. No automatic zero-nutation fallback is allowed.
///
/// The exact analytical derivatives include the derivative of each harmonic
/// phase and time-varying amplitude. They are per **TDB** observation day,
/// not TT. A later IAU 2006/2000A pipeline must reconcile this with the TT
/// conventions of its precession/nutation frame model.
pub fn evaluate_nutation_harmonics<const N: usize>(
    tdb_epoch: JulianDate,
    angles_rad: [f64; N],
    rates_rad_per_tdb_day: [f64; N],
    terms: &[NutationHarmonicTerm<N>],
) -> Result<NutationHarmonicSum, NutationHarmonicError> {
    let maximum = match N {
        5 => 678,
        14 => 687,
        _ => return Err(NutationHarmonicError::UnsupportedArgumentCount),
    };
    if terms.is_empty() {
        return Err(NutationHarmonicError::EmptyTerms);
    }
    if terms.len() > maximum {
        return Err(NutationHarmonicError::TooManyTerms);
    }
    if tdb_epoch.scale() != TimeScale::Tdb {
        return Err(NutationHarmonicError::WrongTimeScale);
    }
    let (whole, fraction) = tdb_epoch
        .parts()
        .map_err(|_| NutationHarmonicError::UnrepresentableEpoch)?;
    let centuries = ((whole - J2000_TDB_JD) + fraction) / TDB_DAYS_PER_CENTURY;
    if !centuries.is_finite() {
        return Err(NutationHarmonicError::UnrepresentableEpoch);
    }
    if centuries.abs() > MAX_TESTED_CENTURIES {
        return Err(NutationHarmonicError::OutsideTestedInterval);
    }
    for (angle, rate) in angles_rad.iter().zip(rates_rad_per_tdb_day) {
        if !angle.is_finite() || !rate.is_finite() {
            return Err(NutationHarmonicError::NonFiniteArgument);
        }
        if !(0.0..TAU).contains(angle) {
            return Err(NutationHarmonicError::UnnormalizedArgument);
        }
    }

    let mut longitude = CompensatedSum::default();
    let mut obliquity = CompensatedSum::default();
    let mut longitude_rate = CompensatedSum::default();
    let mut obliquity_rate = CompensatedSum::default();

    for term in terms {
        let coefficients = [
            term.longitude_sin_rad,
            term.longitude_sin_slope_rad_per_century,
            term.longitude_cos_rad,
            term.obliquity_cos_rad,
            term.obliquity_cos_slope_rad_per_century,
            term.obliquity_sin_rad,
        ];
        if coefficients.iter().any(|value| !value.is_finite()) {
            return Err(NutationHarmonicError::NonFiniteCoefficient);
        }
        let mut phase = 0.0;
        let mut phase_rate = 0.0;
        for (i, multiplier) in term.multipliers.iter().enumerate() {
            phase += f64::from(*multiplier) * angles_rad[i];
            phase_rate += f64::from(*multiplier) * rates_rad_per_tdb_day[i];
        }
        if !phase.is_finite() || !phase_rate.is_finite() {
            return Err(NutationHarmonicError::ArithmeticOverflow);
        }
        let (sin, cos) = phase.rem_euclid(TAU).sin_cos();
        let sin_amplitude =
            term.longitude_sin_rad + centuries * term.longitude_sin_slope_rad_per_century;
        let cos_amplitude =
            term.obliquity_cos_rad + centuries * term.obliquity_cos_slope_rad_per_century;

        longitude.add(sin_amplitude * sin + term.longitude_cos_rad * cos)?;
        obliquity.add(cos_amplitude * cos + term.obliquity_sin_rad * sin)?;
        longitude_rate.add(
            (term.longitude_sin_slope_rad_per_century / TDB_DAYS_PER_CENTURY) * sin
                + (sin_amplitude * cos - term.longitude_cos_rad * sin) * phase_rate,
        )?;
        obliquity_rate.add(
            (term.obliquity_cos_slope_rad_per_century / TDB_DAYS_PER_CENTURY) * cos
                + (term.obliquity_sin_rad * cos - cos_amplitude * sin) * phase_rate,
        )?;
    }

    Ok(NutationHarmonicSum {
        longitude_rad: longitude.finish()?,
        obliquity_rad: obliquity.finish()?,
        longitude_rate_rad_per_tdb_day: longitude_rate.finish()?,
        obliquity_rate_rad_per_tdb_day: obliquity_rate.finish()?,
        terms_evaluated: terms.len(),
    })
}
