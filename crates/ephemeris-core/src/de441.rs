/*
EXHIBIT A. Common Public Attribution License Version 1.0.

The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.

Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.

The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.

The Original Developer is the Initial Developer.

The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.

Contributors retain applicable rights in their own contributions under the terms of this License.

[NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
SPDX-License-Identifier: CPAL-1.0
*/

//! Reader for the classic little-endian JPL DE441 binary format, not SPK or SE1.
//!
//! Binary header layout: <https://www.imcce.fr/content/medias/recherche/equipes/asd/inpop/inpop_file_format_2_0.pdf>.
//! JPL data and format context: <https://ssd.jpl.nasa.gov/planets/eph_export.html>.

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

const HEADER_SIZE: usize = 2856;
const SERIES_COUNT: usize = 13;

/// Julian date in Barycentric Dynamical Time. No UTC/TT conversion is performed.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct JulianDateTdb(pub f64);

/// Names that distinguish planetary system barycenters from planetary centers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    SolarSystemBarycenter,
    Mercury,
    Venus,
    EarthMoonBarycenter,
    Earth,
    Moon,
    MarsBarycenter,
    JupiterBarycenter,
    SaturnBarycenter,
    UranusBarycenter,
    NeptuneBarycenter,
    PlutoBarycenter,
    Sun,
}

/// Geometric position and velocity of `target` relative to `center` in ICRF axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateVector {
    pub target: Body,
    pub center: Body,
    pub epoch: JulianDateTdb,
    pub position_km: [f64; 3],
    pub velocity_km_per_day: [f64; 3],
}

#[derive(Debug)]
pub enum De441Error {
    Io(io::Error),
    InvalidFile(&'static str),
    OutsideCoverage {
        epoch: JulianDateTdb,
        start: JulianDateTdb,
        end: JulianDateTdb,
    },
    MissingSeries(&'static str),
}

impl fmt::Display for De441Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "DE441 I/O error: {e}"),
            Self::InvalidFile(message) => write!(f, "invalid DE441 file: {message}"),
            Self::OutsideCoverage { epoch, start, end } => write!(
                f,
                "TDB Julian date {} is outside [{}, {}]",
                epoch.0, start.0, end.0
            ),
            Self::MissingSeries(name) => write!(f, "DE441 series not present: {name}"),
        }
    }
}

impl Error for De441Error {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}
impl From<io::Error> for De441Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Pointer {
    first: usize,
    coefficients: usize,
    granules: usize,
}

#[derive(Clone, Copy, Default)]
struct Components {
    position: [f64; 3],
    velocity: [f64; 3],
}
impl Components {
    fn add_scaled(self, other: Self, scale: f64) -> Self {
        let mut result = self;
        for i in 0..3 {
            result.position[i] += other.position[i] * scale;
            result.velocity[i] += other.velocity[i] * scale;
        }
        result
    }
}

/// Reads on demand from a caller-supplied seekable source; the 2.8 GB file is not loaded in memory.
pub struct De441<R> {
    source: R,
    start: f64,
    end: f64,
    step: f64,
    earth_moon_mass_ratio: f64,
    pointers: [Pointer; SERIES_COUNT],
    record_size: usize,
    record_count: usize,
    // Reuse the last complete coefficient record for repeated epochs in the
    // same 32-day interval; a failed read must never populate the cache.
    cached_record_index: Option<usize>,
    cached_record: Vec<u8>,
}

impl De441<File> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, De441Error> {
        Self::new(File::open(path)?)
    }
}

fn f64_at(bytes: &[u8], offset: usize) -> f64 {
    f64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("fixed-size field"),
    )
}
fn i32_at(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("fixed-size field"),
    )
}

impl<R: Read + Seek> De441<R> {
    /// Opens a seekable classic JPL DE441 binary. Checks its header and declared file length.
    pub fn new(mut source: R) -> Result<Self, De441Error> {
        source.seek(SeekFrom::Start(0))?;
        let mut header = [0u8; HEADER_SIZE];
        source.read_exact(&mut header)?;
        if !header.starts_with(b"JPL Planetary Ephemeris DE441") || i32_at(&header, 2840) != 441 {
            return Err(De441Error::InvalidFile(
                "expected classic little-endian JPL DE441",
            ));
        }
        let start = f64_at(&header, 2652);
        let end = f64_at(&header, 2660);
        let step = f64_at(&header, 2668);
        let ratio = f64_at(&header, 2688);
        if ![start, end, step, ratio].iter().all(|v| v.is_finite())
            || end <= start
            || step <= 0.0
            || ratio <= 0.0
        {
            return Err(De441Error::InvalidFile(
                "invalid coverage or Earth/Moon mass ratio",
            ));
        }
        let mut pointers = [Pointer::default(); SERIES_COUNT];
        let mut words = 2usize;
        for (i, pointer) in pointers.iter_mut().enumerate() {
            let offset = if i == 12 { 2844 } else { 2696 + i * 12 };
            let (first, coefficients, granules) = (
                i32_at(&header, offset),
                i32_at(&header, offset + 4),
                i32_at(&header, offset + 8),
            );
            if first == 0 && coefficients == 0 && granules == 0 {
                continue;
            }
            if first < 3 || !(1..=100).contains(&coefficients) || !(1..=100).contains(&granules) {
                return Err(De441Error::InvalidFile("invalid coefficient pointer"));
            }
            *pointer = Pointer {
                first: first as usize,
                coefficients: coefficients as usize,
                granules: granules as usize,
            };
            words = words.max(
                pointer.first - 1
                    + pointer.coefficients * pointer.granules * if i == 11 { 2 } else { 3 },
            );
        }
        // Every series occupies a disjoint interval of 1-based coefficient words.
        // Reject overlapping pointers instead of interpreting another series' data
        // as valid coefficients. Words 1 and 2 are reserved for record dates.
        for (i, current) in pointers.iter().enumerate() {
            if current.first == 0 {
                continue;
            }
            let current_components = if i == 11 { 2 } else { 3 };
            let current_end =
                current.first + current.coefficients * current.granules * current_components;
            for (j, previous) in pointers[..i].iter().enumerate() {
                if previous.first == 0 {
                    continue;
                }
                let previous_components = if j == 11 { 2 } else { 3 };
                let previous_end = previous.first
                    + previous.coefficients * previous.granules * previous_components;
                if current.first < previous_end && previous.first < current_end {
                    return Err(De441Error::InvalidFile("overlapping coefficient pointers"));
                }
            }
        }
        let record_size = words
            .checked_mul(8)
            .ok_or(De441Error::InvalidFile("record size overflow"))?;
        let count = (end - start) / step;
        if !count.is_finite()
            || count < 1.0
            || count.fract() != 0.0
            || count > (usize::MAX - 2) as f64
            || record_size < HEADER_SIZE
        {
            return Err(De441Error::InvalidFile("invalid record count or size"));
        }
        let record_count = count as usize;
        let expected = (record_count as u64)
            .checked_add(2)
            .and_then(|n| n.checked_mul(record_size as u64))
            .ok_or(De441Error::InvalidFile("file size overflow"))?;
        if source.seek(SeekFrom::End(0))? != expected {
            return Err(De441Error::InvalidFile("file length does not match header"));
        }
        Ok(Self {
            source,
            start,
            end,
            step,
            earth_moon_mass_ratio: ratio,
            pointers,
            record_size,
            record_count,
            cached_record_index: None,
            cached_record: vec![0u8; record_size],
        })
    }

    /// Inclusive interval as specified by the binary header, in TDB Julian days.
    pub fn coverage(&self) -> (JulianDateTdb, JulianDateTdb) {
        (JulianDateTdb(self.start), JulianDateTdb(self.end))
    }

    /// Validated classic-header series/granule grid for read-only planning.
    ///
    /// Exposes the actual granule counts accepted during De441::new(), not
    /// polynomial coefficients, kernel contents, or proof of boundary
    /// continuity. Plan results must never be labeled JPL-validated.
    pub fn granule_layout(&self) -> crate::de441_granule_plan::ClassicDe441GranuleLayout {
        crate::de441_granule_plan::ClassicDe441GranuleLayout {
            start_jd_tdb: self.start,
            end_jd_tdb: self.end,
            record_step_days: self.step,
            record_count: self.record_count,
            series_granules_per_record: self.pointers.map(|p| {
                if p.first == 0 { None } else { Some(p.granules) }
            }),
        }
    }

    /// Returns a geometric target-relative-to-center state at a TDB epoch.
    pub fn state(
        &mut self,
        target: Body,
        center: Body,
        epoch: JulianDateTdb,
    ) -> Result<StateVector, De441Error> {
        let jd = epoch.0;
        if !jd.is_finite() || jd < self.start || jd > self.end {
            return Err(De441Error::OutsideCoverage {
                epoch,
                start: JulianDateTdb(self.start),
                end: JulianDateTdb(self.end),
            });
        }
        let components = if target == center {
            Components::default()
        } else {
            let record_index =
                (((jd - self.start) / self.step).floor() as usize).min(self.record_count - 1);
            if self.cached_record_index != Some(record_index) {
                // Invalidate before I/O: a short read may partially overwrite
                // the buffer, including the previously cached record.
                self.cached_record_index = None;
                let offset = (record_index as u64 + 2) * self.record_size as u64;
                self.source.seek(SeekFrom::Start(offset))?;
                self.source.read_exact(&mut self.cached_record)?;
                let record_start = f64_at(&self.cached_record, 0);
                if record_start != self.start + record_index as f64 * self.step
                    || f64_at(&self.cached_record, 8) != record_start + self.step
                {
                    return Err(De441Error::InvalidFile(
                        "coefficient record has unexpected dates",
                    ));
                }
                self.cached_record_index = Some(record_index);
            }
            let record = &self.cached_record;
            let record_start = f64_at(record, 0);
            // The Moon series is geocentric. Compute Earth/Moon/EMB-relative
            // vectors directly from it, without subtracting large barycentric
            // positions that can erase small lunar offsets through cancellation.
            let lunar_scale = match (target, center) {
                (Body::Moon, Body::Earth) => Some(1.0),
                (Body::Earth, Body::Moon) => Some(-1.0),
                (Body::Earth, Body::EarthMoonBarycenter) => {
                    Some(-1.0 / (1.0 + self.earth_moon_mass_ratio))
                }
                (Body::EarthMoonBarycenter, Body::Earth) => {
                    Some(1.0 / (1.0 + self.earth_moon_mass_ratio))
                }
                (Body::Moon, Body::EarthMoonBarycenter) => {
                    Some(self.earth_moon_mass_ratio / (1.0 + self.earth_moon_mass_ratio))
                }
                (Body::EarthMoonBarycenter, Body::Moon) => {
                    Some(-self.earth_moon_mass_ratio / (1.0 + self.earth_moon_mass_ratio))
                }
                _ => None,
            };
            if let Some(scale) = lunar_scale {
                Components::default().add_scaled(self.series(9, jd, record_start, record)?, scale)
            } else {
                self.barycentric(target, jd, record_start, record)?
                    .add_scaled(self.barycentric(center, jd, record_start, record)?, -1.0)
            }
        };
        if components
            .position
            .iter()
            .chain(components.velocity.iter())
            .any(|value| !value.is_finite())
        {
            return Err(De441Error::InvalidFile("non-finite relative state"));
        }
        Ok(StateVector {
            target,
            center,
            epoch,
            position_km: components.position,
            velocity_km_per_day: components.velocity,
        })
    }

    fn barycentric(
        &self,
        body: Body,
        jd: f64,
        record_start: f64,
        record: &[u8],
    ) -> Result<Components, De441Error> {
        let index = match body {
            Body::SolarSystemBarycenter => return Ok(Components::default()),
            Body::Mercury => 0,
            Body::Venus => 1,
            Body::EarthMoonBarycenter => 2,
            Body::MarsBarycenter => 3,
            Body::JupiterBarycenter => 4,
            Body::SaturnBarycenter => 5,
            Body::UranusBarycenter => 6,
            Body::NeptuneBarycenter => 7,
            Body::PlutoBarycenter => 8,
            Body::Sun => 10,
            Body::Earth | Body::Moon => {
                let emb = self.series(2, jd, record_start, record)?;
                let moon_geo = self.series(9, jd, record_start, record)?;
                let earth = emb.add_scaled(moon_geo, -1.0 / (1.0 + self.earth_moon_mass_ratio));
                return Ok(if body == Body::Earth {
                    earth
                } else {
                    earth.add_scaled(moon_geo, 1.0)
                });
            }
        };
        self.series(index, jd, record_start, record)
    }

    fn series(
        &self,
        index: usize,
        jd: f64,
        record_start: f64,
        record: &[u8],
    ) -> Result<Components, De441Error> {
        let p = self.pointers[index];
        if p.first == 0 {
            return Err(De441Error::MissingSeries(match index {
                2 => "Earth-Moon barycenter",
                9 => "geocentric Moon",
                _ => "body",
            }));
        }
        let width = self.step / p.granules as f64;
        let granule = (((jd - record_start) / width).floor() as usize).min(p.granules - 1);
        let x = 2.0 * (jd - (record_start + granule as f64 * width)) / width - 1.0;
        let mut result = Components::default();
        for axis in 0..3 {
            let offset = (p.first - 1 + (granule * 3 + axis) * p.coefficients) * 8;
            let (mut t0, mut t1, mut d0, mut d1) = (1.0, x, 0.0, 1.0);
            let mut position = f64_at(record, offset);
            if !position.is_finite() {
                return Err(De441Error::InvalidFile("non-finite Chebyshev coefficient"));
            }
            let mut derivative = 0.0;
            if p.coefficients > 1 {
                let c = f64_at(record, offset + 8);
                if !c.is_finite() {
                    return Err(De441Error::InvalidFile("non-finite Chebyshev coefficient"));
                }
                position += c * x;
                derivative += c;
            }
            for k in 2..p.coefficients {
                let t = 2.0 * x * t1 - t0;
                let d = 2.0 * t1 + 2.0 * x * d1 - d0;
                let c = f64_at(record, offset + k * 8);
                if !c.is_finite() {
                    return Err(De441Error::InvalidFile("non-finite Chebyshev coefficient"));
                }
                position += c * t;
                derivative += c * d;
                (t0, t1, d0, d1) = (t1, t, d1, d);
            }
            let velocity = derivative * 2.0 / width;
            if !position.is_finite() || !velocity.is_finite() {
                return Err(De441Error::InvalidFile("non-finite interpolated state"));
            }
            result.position[axis] = position;
            result.velocity[axis] = velocity;
        }
        Ok(result)
    }
}
