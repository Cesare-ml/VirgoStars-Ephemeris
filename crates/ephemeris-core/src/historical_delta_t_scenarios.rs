// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
//! Opt-in sensitivity samples at fixed UT1 using caller-provided Delta T.
//! Three values are not an uncertainty bound, confidence interval, or local
//! civil-time conversion. The long-term frame and TT/TDB model remain experimental.
use crate::apparent_ecliptic_of_date::{
    experimental_long_term_apparent_direct_obliquity, experimental_long_term_apparent_ecliptic,
    ApparentError,
};
use crate::de441::{Body, De441};
use crate::explicit_delta_t::{
    ut1_to_tdb_delta_t_scenarios, ExplicitDeltaTEpoch, ExplicitDeltaTError,
};
use crate::six_field_units::FixedSixFieldsDegreesAu;
use std::{
    error::Error,
    fmt,
    io::{Read, Seek},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenarioFrame {
    PoleBased,
    DirectObliquity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenarioSample {
    Lower,
    Nominal,
    Upper,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HistoricalScenario {
    pub epoch: ExplicitDeltaTEpoch,
    pub fields: FixedSixFieldsDegreesAu,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HistoricalDeltaTScenarios {
    pub lower: HistoricalScenario,
    pub nominal: HistoricalScenario,
    pub upper: HistoricalScenario,
    pub frame: ScenarioFrame,
}

#[derive(Debug)]
pub enum HistoricalScenarioError {
    Time(ExplicitDeltaTError),
    Calculation {
        sample: ScenarioSample,
        source: ApparentError,
    },
}
impl fmt::Display for HistoricalScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Time(e) => write!(f, "scenario time: {e}"),
            Self::Calculation { sample, source } => {
                write!(f, "{sample:?} Delta T scenario: {source}")
            }
        }
    }
}
impl Error for HistoricalScenarioError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Time(e) => e,
            Self::Calculation { source, .. } => source,
        })
    }
}

/// Calculate the nominal and +/- caller-provided Delta T perturbations.
/// All three must succeed. Earth-centered apparent ecliptic of date, output
/// degrees/AU and per TDB day, for the explicit experimental frame. UT1 is
/// held fixed and is neither UTC nor historical local time.
pub fn historical_delta_t_scenarios<R: Read + Seek>(
    eph: &mut De441<R>,
    target: Body,
    jd_ut1: f64,
    delta_t_seconds: f64,
    uncertainty_seconds: f64,
    frame: ScenarioFrame,
) -> Result<HistoricalDeltaTScenarios, HistoricalScenarioError> {
    let epochs = ut1_to_tdb_delta_t_scenarios(jd_ut1, delta_t_seconds, uncertainty_seconds)
        .map_err(HistoricalScenarioError::Time)?;
    let mut calculate = |epoch: ExplicitDeltaTEpoch, sample| {
        let fields = match frame {
            ScenarioFrame::PoleBased => {
                experimental_long_term_apparent_ecliptic(eph, target, epoch.jd_tdb)
            }
            ScenarioFrame::DirectObliquity => {
                experimental_long_term_apparent_direct_obliquity(eph, target, epoch.jd_tdb)
            }
        }
        .map_err(|source| HistoricalScenarioError::Calculation { sample, source })?;
        Ok(HistoricalScenario { epoch, fields })
    };
    Ok(HistoricalDeltaTScenarios {
        lower: calculate(epochs.lower, ScenarioSample::Lower)?,
        nominal: calculate(epochs.nominal, ScenarioSample::Nominal)?,
        upper: calculate(epochs.upper, ScenarioSample::Upper)?,
        frame,
    })
}
