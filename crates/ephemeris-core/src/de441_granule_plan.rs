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

//! Classic DE441 per-series granule coverage planner (not a kernel evaluator).
//!
//! Uses only the validated header's record grid and per-series granule counts.
//! Earth-center geometry needs EMB + geocentric Moon, while other bodies may
//! have different granule counts. This planner partitions exact TDB Julian-day
//! coverage at every required series boundary before any rate/enclosure work.
//!
//! The plan is a metadata/selection aid, not proof of continuity, polynomial
//! correctness, numeric rounding, or actual NASA/JPL astronomy accuracy.
//! Original VirgoStars contribution 2026-11-12, CPAL-1.0.

use core::fmt;
use std::collections::HashMap;
use crate::de441::Body;

/// Classic DE441 header layout: 13 optional series in official pointer order.
/// Caller should obtain this value via De441::granule_layout() after opening a
/// validated binary; manually fabricated metadata is NOT independently checked.
#[derive(Clone, Copy, Debug)]
pub struct ClassicDe441GranuleLayout {
    pub start_jd_tdb: f64,
    pub end_jd_tdb: f64,
    pub record_step_days: f64,
    pub record_count: usize,
    pub series_granules_per_record: [Option<usize>; 13],
}

/// Each partition slice uses the same record/granule index for its entire
/// strictly positive interior, and ends before any next series boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveSeriesGranule {
    pub series_index: usize,
    pub principal_record_index: usize,
    pub subrecord_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassicDe441CoverageSlice {
    pub start_jd_tdb: f64,
    pub end_jd_tdb: f64,
    pub active_granules: Vec<ActiveSeriesGranule>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassicDe441GranulePlan {
    pub start_jd_tdb: f64,
    pub end_jd_tdb: f64,
    /// Indices required to reconstruct both input body states relative to
    /// the Solar System barycenter. This may overpartition special pairs.
    pub required_series_indices: Vec<usize>,
    pub slices: Vec<ClassicDe441CoverageSlice>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum De441GranulePlanError {
    SameBodyNoLongitude,
    InvalidHeaderLayout,
    MissingRequiredSeries,
    InvalidRequestedInterval,
    OutOfCoverage,
    ExcessiveCoverage,
    InsufficientTimePrecision,
}
impl fmt::Display for De441GranulePlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameBodyNoLongitude => write!(f, "a body relative to itself has undefined longitude"),
            Self::InvalidHeaderLayout => write!(f, "invalid classic DE441 header granule metadata"),
            Self::MissingRequiredSeries => write!(f, "required source series is absent from classic DE441"),
            Self::InvalidRequestedInterval => write!(f, "TDB interval must be finite and strictly increasing"),
            Self::OutOfCoverage => write!(f, "requested interval lies outside DE441 header coverage"),
            Self::ExcessiveCoverage => write!(f, "too many principal records or subintervals for a single plan"),
            Self::InsufficientTimePrecision => write!(f, "TDB f64 cannot distinguish required record boundaries"),
        }
    }
}
impl std::error::Error for De441GranulePlanError {}

fn append_series(body: Body, series: &mut Vec<usize>) {
    let indices: &[usize] = match body {
        Body::SolarSystemBarycenter => &[],
        Body::Mercury => &[0],
        Body::Venus => &[1],
        Body::EarthMoonBarycenter => &[2],
        Body::Earth | Body::Moon => &[2,9],
        Body::MarsBarycenter => &[3],
        Body::JupiterBarycenter => &[4],
        Body::SaturnBarycenter => &[5],
        Body::UranusBarycenter => &[6],
        Body::NeptuneBarycenter => &[7],
        Body::PlutoBarycenter => &[8],
        Body::Sun => &[10],
    };
    for &i in indices {
        if !series.contains(&i) { series.push(i); }
    }
}

fn valid_layout(layout: &ClassicDe441GranuleLayout) -> bool {
    let s=layout.start_jd_tdb;
    let e=layout.end_jd_tdb;
    let dt=layout.record_step_days;
    if !s.is_finite() || !e.is_finite() || !dt.is_finite() ||
        s>=e || dt<=0.0 || layout.record_count==0 {
        return false;
    }
    let inferred=s+(layout.record_count as f64)*dt;
    if !inferred.is_finite() || inferred!=e || s+dt<=s {
        return false;
    }
    layout.series_granules_per_record.iter().all(|v|
        v.map(|n| (1..=100).contains(&n) && dt/(n as f64)>0.0).unwrap_or(true)
    )
}

fn active_at(
    layout:&ClassicDe441GranuleLayout, series_index:usize, epoch:f64,
) -> ActiveSeriesGranule {
    let count=layout.series_granules_per_record[series_index].expect("series checked");
    let principal=(((epoch-layout.start_jd_tdb)/layout.record_step_days).floor() as usize)
        .min(layout.record_count-1);
    let principal_start=layout.start_jd_tdb+(principal as f64)*layout.record_step_days;
    let width=layout.record_step_days/(count as f64);
    let sub=(((epoch-principal_start)/width).floor() as usize).min(count-1);
    ActiveSeriesGranule{
        series_index,principal_record_index:principal,subrecord_index:sub,
    }
}

/// Plan the exact common coefficient-grid partition for a classic DE441 pair.
///
/// No coordinate interpolation is performed. The result identifies *which*
/// principal record and per-series Chebyshev subrecord must be read for each
/// mathematical slice. It does not read or verify those coefficient bytes.
///
/// This is a strict header-domain planner: near-collapsed f64 boundaries
/// refuse rather than silently merge distinct subrecords. A future layer
/// must prove endpoint continuity/errors and physical center-chain validity.
pub fn plan_de441_relative_granules(
    layout: &ClassicDe441GranuleLayout,
    target: Body, observer: Body,
    start_jd_tdb: f64, end_jd_tdb: f64,
) -> Result<ClassicDe441GranulePlan,De441GranulePlanError> {
    if target==observer { return Err(De441GranulePlanError::SameBodyNoLongitude); }
    if !valid_layout(layout) { return Err(De441GranulePlanError::InvalidHeaderLayout); }
    if !start_jd_tdb.is_finite() || !end_jd_tdb.is_finite() ||
        start_jd_tdb>=end_jd_tdb {
        return Err(De441GranulePlanError::InvalidRequestedInterval);
    }
    if start_jd_tdb<layout.start_jd_tdb || end_jd_tdb>layout.end_jd_tdb {
        return Err(De441GranulePlanError::OutOfCoverage);
    }

    let mut series=Vec::new();
    append_series(target,&mut series);
    append_series(observer,&mut series);
    series.sort_unstable();
    if series.iter().any(|&i|layout.series_granules_per_record[i].is_none()) {
        return Err(De441GranulePlanError::MissingRequiredSeries);
    }
    // At most 512 principal records per request; large historical ranges must
    // be processed as separately validated pieces, not silently truncated.
    let first=(((start_jd_tdb-layout.start_jd_tdb)/layout.record_step_days).floor() as usize)
        .min(layout.record_count-1);
    let last=(((end_jd_tdb-layout.start_jd_tdb)/layout.record_step_days).floor() as usize)
        .min(layout.record_count-1);
    if last<first || last-first>=512 {
        return Err(De441GranulePlanError::ExcessiveCoverage);
    }
    let mut boundaries=vec![start_jd_tdb,end_jd_tdb];
    // Same rounded f64 date can accidentally represent two DIFFERENT
    // rational granule boundaries at large Julian dates. Never deduplicate
    // those as though the polynomial selection were proven unambiguous.
    let mut seen = HashMap::<(usize,u64),(usize,usize)>::new();
    for principal in first..=last {
        let principal_start=layout.start_jd_tdb+(principal as f64)*layout.record_step_days;
        for &index in &series {
            let count=layout.series_granules_per_record[index].expect("series checked");
            let width=layout.record_step_days/(count as f64);
            let mut previous=f64::NEG_INFINITY;
            for k in 0..=count {
                let boundary=principal_start+(k as f64)*width;
                if !boundary.is_finite() || boundary<=previous {
                    return Err(De441GranulePlanError::InsufficientTimePrecision);
                }
                previous=boundary;
                if boundary>start_jd_tdb && boundary<end_jd_tdb {
                    let key=(principal,boundary.to_bits());
                    if let Some(&(other_k,other_count))=seen.get(&key) {
                        // k/count and other_k/other_count are exact rational
                        // subrecord fractions; cross products fit usize
                        // because classic DE441 permits at most 100 granules.
                        if k*other_count!=other_k*count {
                            return Err(De441GranulePlanError::InsufficientTimePrecision);
                        }
                    } else {
                        seen.insert(key,(k,count));
                    }
                    boundaries.push(boundary);
                }
            }
        }
        // All required source series share the principal record boundary.
        // If both bodies are SSB, target==observer would have been rejected,
        // so this also partitions single-series directions correctly.
        let principal_end=principal_start+layout.record_step_days;
        if principal_end>start_jd_tdb && principal_end<end_jd_tdb {
            boundaries.push(principal_end);
        }
    }
    boundaries.sort_by(f64::total_cmp);
    boundaries.dedup();
    if boundaries.len()<2 || boundaries.len()>4097 {
        return Err(De441GranulePlanError::ExcessiveCoverage);
    }
    let mut slices=Vec::with_capacity(boundaries.len()-1);
    for pair in boundaries.windows(2) {
        let [left,right]=[pair[0],pair[1]];
        let midpoint=left+(right-left)*0.5;
        if !(left<midpoint && midpoint<right) {
            return Err(De441GranulePlanError::InsufficientTimePrecision);
        }
        // If rounding placed a mathematical boundary in the interior of a
        // slice, refuse rather than claiming the whole slice uses one record.
        let selections=series.iter().map(|&index|active_at(layout,index,midpoint)).collect();
        slices.push(ClassicDe441CoverageSlice{
            start_jd_tdb:left,end_jd_tdb:right,active_granules:selections,
        });
    }
    Ok(ClassicDe441GranulePlan{
        start_jd_tdb,end_jd_tdb,required_series_indices:series,slices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> ClassicDe441GranuleLayout {
        let mut g=[None;13];
        g[2]=Some(2); // EMB (2 subrecords/record)
        g[9]=Some(4); // geocentric Moon (4 subrecords/record)
        g[3]=Some(1); // Mars system barycenter (1 subrecord)
        g[10]=Some(1); // Sun
        ClassicDe441GranuleLayout{
            start_jd_tdb:100.0,end_jd_tdb:104.0,
            record_step_days:2.0,record_count:2,
            series_granules_per_record:g,
        }
    }
    #[test]
    fn earth_requires_emb_and_lunar_series_and_merges_granule_grids() {
        let p=plan_de441_relative_granules(
            &layout(),Body::MarsBarycenter,Body::Earth,100.0,102.0,
        ).unwrap();
        assert_eq!(p.required_series_indices,vec![2,3,9]);
        assert_eq!(p.slices.len(),4);
        assert_eq!(p.slices[0].start_jd_tdb,100.0);
        assert_eq!(p.slices[0].end_jd_tdb,100.5);
        assert_eq!(p.slices[0].active_granules.iter().map(|g|g.subrecord_index)
            .collect::<Vec<_>>(),vec![0,0,0]);
        assert_eq!(p.slices[3].active_granules[2].subrecord_index,3);
    }
    #[test]
    fn crossing_main_record_boundary_selects_new_record_without_gap() {
        let p=plan_de441_relative_granules(
            &layout(),Body::MarsBarycenter,Body::Earth,101.5,102.5,
        ).unwrap();
        assert_eq!(p.slices.len(),2);
        assert_eq!(p.slices[0].end_jd_tdb,p.slices[1].start_jd_tdb);
        assert_eq!(p.slices[0].active_granules[0].principal_record_index,0);
        assert_eq!(p.slices[1].active_granules[0].principal_record_index,1);
    }
    #[test]
    fn different_required_granule_counts_union_without_interpolation() {
        let p=plan_de441_relative_granules(
            &layout(),Body::Earth,Body::Sun,100.0,101.0,
        ).unwrap();
        assert_eq!(p.required_series_indices,vec![2,9,10]);
        assert_eq!(p.slices.len(),2);
        assert_eq!(p.slices[0].end_jd_tdb,100.5);
    }
    #[test]
    fn out_of_coverage_and_missing_series_fail_closed() {
        assert_eq!(plan_de441_relative_granules(
            &layout(),Body::MarsBarycenter,Body::Earth,99.0,102.0,
        ),Err(De441GranulePlanError::OutOfCoverage));
        assert_eq!(plan_de441_relative_granules(
            &layout(),Body::Venus,Body::Earth,100.0,102.0,
        ),Err(De441GranulePlanError::MissingRequiredSeries));
    }
    #[test]
    fn same_body_and_invalid_requests_do_not_create_false_longitude() {
        assert_eq!(plan_de441_relative_granules(
            &layout(),Body::Earth,Body::Earth,100.0,101.0,
        ),Err(De441GranulePlanError::SameBodyNoLongitude));
        for (start,end) in [(100.0,100.0),(101.0,100.0),(f64::NAN,101.0)] {
            assert_eq!(plan_de441_relative_granules(
                &layout(),Body::MarsBarycenter,Body::Earth,start,end,
            ),Err(De441GranulePlanError::InvalidRequestedInterval));
        }
    }
    #[test]
    fn incorrect_header_layout_refuses_planning() {
        let mut bad=layout();
        bad.record_step_days=0.0;
        assert_eq!(plan_de441_relative_granules(
            &bad,Body::MarsBarycenter,Body::Earth,100.0,102.0,
        ),Err(De441GranulePlanError::InvalidHeaderLayout));
        let mut bad=layout();
        bad.series_granules_per_record[9]=Some(0);
        assert_eq!(plan_de441_relative_granules(
            &bad,Body::MarsBarycenter,Body::Earth,100.0,102.0,
        ),Err(De441GranulePlanError::InvalidHeaderLayout));
    }
    #[test]
    fn fractional_requested_windows_are_preserved_exactly() {
        let p=plan_de441_relative_granules(
            &layout(),Body::MarsBarycenter,Body::Earth,100.25,100.75,
        ).unwrap();
        assert_eq!(p.slices.len(),2);
        assert_eq!(p.slices[0].start_jd_tdb,100.25);
        assert_eq!(p.slices[1].end_jd_tdb,100.75);
    }
    #[test]
    fn unresolvable_granule_subdivisions_fail_closed() {
        // 1e16 has 2-second-sized ulps: 100 subdivisions across a 4-day
        // synthetic JD step collapse in f64, even though the header step fits.
        let mut bad=layout();
        bad.start_jd_tdb=1.0e16;
        bad.end_jd_tdb=1.0e16+4.0;
        bad.record_step_days=4.0;
        bad.record_count=1;
        assert_eq!(plan_de441_relative_granules(
            &bad,Body::MarsBarycenter,Body::Earth,1.0e16,1.0e16+4.0,
        ),Err(De441GranulePlanError::InsufficientTimePrecision));
    }

    #[test]
    fn distinct_rational_boundaries_rounded_to_one_float_are_rejected() {
        // At JD 1e13, 1/99 and 1/100 subrecord grids contain distinct
        // rational cut points that can collapse to an identical f64.
        let mut bad=layout();
        bad.start_jd_tdb=1.0e13;
        bad.end_jd_tdb=1.0e13+1.0;
        bad.record_step_days=1.0;
        bad.record_count=1;
        bad.series_granules_per_record[2]=Some(99);
        bad.series_granules_per_record[9]=Some(100);
        assert_eq!(plan_de441_relative_granules(
            &bad,Body::MarsBarycenter,Body::Earth,1.0e13,1.0e13+1.0,
        ),Err(De441GranulePlanError::InsufficientTimePrecision));
    }

    #[test]
    fn no_required_series_direction_still_uses_nonempty_plan() {
        let p=plan_de441_relative_granules(
            &layout(),Body::SolarSystemBarycenter,Body::MarsBarycenter,100.0,102.0,
        ).unwrap();
        assert_eq!(p.required_series_indices,vec![3]);
        assert_eq!(p.slices.len(),1);
    }
}
