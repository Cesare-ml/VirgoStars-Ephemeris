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
// 2026-11-14: Bound a caller-controlled record cache allocation. Real classic
// DE441 records are far smaller; this is a defensive implementation limit.
const MAX_CLASSIC_RECORD_BYTES: usize = 16 * 1024 * 1024;
// 2026-11-16: Bound materialized DE441 batch results before evaluation/allocation.
// The byte budget measures Vec<StateVector> element storage, not allocator metadata.
pub const MAX_DE441_BATCH_OUTPUT_BYTES: usize = 1024 * 1024;

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
    /// A small body (asteroid, comet) identified by its NAIF id, supplied by a
    /// separate SPK; it is never in DE441.
    SmallBody(i32),
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

/// Original DE441 interpolation result with an **analytic second
/// time derivative** of the same Chebyshev series, km/TDB day².
///
/// Does not represent a numerical acceleration from three separately
/// sampled velocities or an independent real-JPL accuracy certification.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateVectorWithAcceleration {
    pub target: Body,
    pub center: Body,
    pub epoch: JulianDateTdb,
    pub position_km: [f64; 3],
    pub velocity_km_per_day: [f64; 3],
    pub acceleration_km_per_day2: [f64; 3],
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
    BatchOutputTooLarge {
        requested_states: usize,
        max_states: usize,
    },
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
            Self::BatchOutputTooLarge { requested_states, max_states } => write!(
                f,
                "DE441 batch requests {requested_states} states; maximum is {max_states} ({}-byte output budget)",
                MAX_DE441_BATCH_OUTPUT_BYTES
            ),
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
    acceleration: [f64; 3],
}
impl Components {
    fn add_scaled(self, other: Self, scale: f64) -> Self {
        let mut result = self;
        for i in 0..3 {
            result.position[i] += other.position[i] * scale;
            result.velocity[i] += other.velocity[i] * scale;
            result.acceleration[i] += other.acceleration[i] * scale;
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
        // Reject malicious sparse/seekable inputs before allocating a record cache.
        // Pointer fields are untrusted and can otherwise request multi-GB buffers.
        if record_size > MAX_CLASSIC_RECORD_BYTES {
            return Err(De441Error::InvalidFile(
                "coefficient record exceeds supported size",
            ));
        }
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

        // Modification 2026-11-14: every declared Chebyshev subrecord must
        // occupy distinguishable TDB Julian dates in the f64 reader API.
        // Test the first and last principal record, where the binary64 ULP
        // can be largest over the declared interval. Checking each boundary
        // (rather than only granule width) also catches adjacent collisions.
        let last_record_start = start + (record_count - 1) as f64 * step;
        for pointer in pointers.iter().filter(|pointer| pointer.first != 0) {
            let width = step / pointer.granules as f64;
            if !width.is_finite() || width <= 0.0 {
                return Err(De441Error::InvalidFile(
                    "coefficient granule dates exceed f64 JD resolution",
                ));
            }
            for record_start in [start, last_record_start] {
                let mut previous = record_start;
                for granule in 1..pointer.granules {
                    let boundary = record_start + granule as f64 * width;
                    if !boundary.is_finite() || boundary <= previous {
                        return Err(De441Error::InvalidFile(
                            "coefficient granule dates exceed f64 JD resolution",
                        ));
                    }
                    previous = boundary;
                }
                let record_end = record_start + step;
                if !record_end.is_finite() || record_end <= previous {
                    return Err(De441Error::InvalidFile(
                        "coefficient granule dates exceed f64 JD resolution",
                    ));
                }
            }
        }
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

    /// Returns a geometric target-relative-to-center state at a TDB epoch.
    ///
    /// This original velocity-only API avoids computing second derivatives
    /// when a caller does not need them.
    pub fn state(
        &mut self,
        target: Body,
        center: Body,
        epoch: JulianDateTdb,
    ) -> Result<StateVector, De441Error> {
        let result = self.state_components(target, center, epoch, false)?;
        Ok(StateVector {
            target,
            center,
            epoch,
            position_km: result.position,
            velocity_km_per_day: result.velocity,
        })
    }

    /// Evaluate position, velocity **and analytic Chebyshev acceleration**
    /// in the same fixed ICRF frame and at one TDB Julian epoch.
    ///
    /// Uses identical file, center/barycenter and Earth-Moon handling as
    /// `state`; no step-size choice or extra observer-time samples.
    /// At a record/granule boundary the second derivative is one-sided
    /// according to the selected Chebyshev segment.
    pub fn state_with_acceleration(
        &mut self,
        target: Body,
        center: Body,
        epoch: JulianDateTdb,
    ) -> Result<StateVectorWithAcceleration, De441Error> {
        let result = self.state_components(target, center, epoch, true)?;
        Ok(StateVectorWithAcceleration {
            target,
            center,
            epoch,
            position_km: result.position,
            velocity_km_per_day: result.velocity,
            acceleration_km_per_day2: result.acceleration,
        })
    }

    fn state_components(
        &mut self,
        target: Body,
        center: Body,
        epoch: JulianDateTdb,
        with_acceleration: bool,
    ) -> Result<Components, De441Error> {
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
            // Modification 2026-11-14: subtracting the epoch from a distant
            // file start can round a value just *before* a record boundary
            // onto that boundary. Compare the original TDB JD against the
            // header-defined boundaries instead of dividing a rounded delta.
            // A boundary selects the following record; the final endpoint
            // remains in the last record.
            let (mut low, mut high) = (0usize, self.record_count);
            while low + 1 < high {
                let mid = low + (high - low) / 2;
                if jd < self.start + mid as f64 * self.step {
                    high = mid;
                } else {
                    low = mid;
                }
            }
            let record_index = low;
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
                Components::default().add_scaled(
                    self.series(9, jd, record_start, record, with_acceleration)?,
                    scale,
                )
            } else {
                self.barycentric(target, jd, record_start, record, with_acceleration)?
                    .add_scaled(
                        self.barycentric(center, jd, record_start, record, with_acceleration)?,
                        -1.0,
                    )
            }
        };
        if components
            .position
            .iter()
            .chain(components.velocity.iter())
            .chain(components.acceleration.iter())
            .any(|value| !value.is_finite())
        {
            return Err(De441Error::InvalidFile("non-finite relative state"));
        }
        Ok(components)
    }

    /// Return geometric ICRF states for multiple targets at one TDB epoch.
    ///
    /// Results follow the input target order, including duplicates. All
    /// states share the caller's center and epoch; positions are km and
    /// velocities are km/day. This delegates to the same validated scalar
    /// reader and its one-record cache, so repeated targets in the current
    /// record do not trigger additional file reads. No new numerical model,
    /// apparent corrections or frame conversion are applied.
    ///
    /// Fail-closed: any unavailable series, invalid epoch or I/O failure
    /// returns an error rather than exposing a partial vector. An empty
    /// target list still validates the epoch but does not read coefficients.
    ///
    /// The returned `Vec<StateVector>` payload is capped at
    /// [`MAX_DE441_BATCH_OUTPUT_BYTES`] bytes. This is checked with overflow-safe
    /// multiplication before state evaluation, coefficient I/O or allocation.
    /// Requests over the derived state-count limit return
    /// [`De441Error::BatchOutputTooLarge`]; results are never truncated.
    ///
    /// Modified 2026-11-11: add batch state convenience interface.
    /// Modified 2026-11-16: bound the materialized result to 1 MiB.
    pub fn states_at(
        &mut self,
        targets: &[Body],
        center: Body,
        epoch: JulianDateTdb,
    ) -> Result<Vec<StateVector>, De441Error> {
        let state_bytes = std::mem::size_of::<StateVector>();
        let max_states = MAX_DE441_BATCH_OUTPUT_BYTES / state_bytes;
        let requested_bytes = targets.len().checked_mul(state_bytes);
        if match requested_bytes {
            None => true,
            Some(bytes) => bytes > MAX_DE441_BATCH_OUTPUT_BYTES,
        } {
            return Err(De441Error::BatchOutputTooLarge {
                requested_states: targets.len(),
                max_states,
            });
        }
        if targets.is_empty() {
            self.state(center, center, epoch)?;
            return Ok(Vec::new());
        }
        let mut states = Vec::with_capacity(targets.len());
        for target in targets.iter().copied() {
            states.push(self.state(target, center, epoch)?);
        }
        Ok(states)
    }

    fn barycentric(
        &self,
        body: Body,
        jd: f64,
        record_start: f64,
        record: &[u8],
        with_acceleration: bool,
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
            Body::SmallBody(_) => {
                return Err(De441Error::MissingSeries("small bodies are not in DE441"))
            }
            Body::Earth | Body::Moon => {
                let emb = self.series(2, jd, record_start, record, with_acceleration)?;
                let moon_geo = self.series(9, jd, record_start, record, with_acceleration)?;
                let earth = emb.add_scaled(moon_geo, -1.0 / (1.0 + self.earth_moon_mass_ratio));
                return Ok(if body == Body::Earth {
                    earth
                } else {
                    earth.add_scaled(moon_geo, 1.0)
                });
            }
        };
        self.series(index, jd, record_start, record, with_acceleration)
    }

    fn series(
        &self,
        index: usize,
        jd: f64,
        record_start: f64,
        record: &[u8],
        with_acceleration: bool,
    ) -> Result<Components, De441Error> {
        let p = self.pointers[index];
        if p.first == 0 {
            // Report the actual missing series to keep incomplete or damaged
            // classic kernels diagnosable; barycenters are not planet centers.
            return Err(De441Error::MissingSeries(match index {
                0 => "Mercury",
                1 => "Venus",
                2 => "Earth-Moon barycenter",
                3 => "Mars system barycenter",
                4 => "Jupiter system barycenter",
                5 => "Saturn system barycenter",
                6 => "Uranus system barycenter",
                7 => "Neptune system barycenter",
                8 => "Pluto system barycenter",
                9 => "geocentric Moon",
                10 => "Sun",
                _ => "unknown DE441 series",
            }));
        }
        let width = self.step / p.granules as f64;
        // Modification 2026-11-14: division/floor can round an exact
        // representable granule boundary just below its integer index.
        // Binary-search the same rounded boundaries validated by new().
        // At a boundary choose the following granule; at the inclusive
        // final coverage endpoint choose the last one.
        let (mut low, mut high) = (0usize, p.granules);
        while low + 1 < high {
            let mid = low + (high - low) / 2;
            let boundary = record_start + mid as f64 * width;
            if jd < boundary {
                high = mid;
            } else {
                low = mid;
            }
        }
        let granule = low;
        // Modification 2026-11-14: divide before doubling the normalized
        // record offset. The old 2 * offset / width could overflow an
        // intermediate even when the final Chebyshev abscissa is in [-1, 1].
        let x = 2.0 * ((jd - (record_start + granule as f64 * width)) / width) - 1.0;
        let mut result = Components::default();
        for axis in 0..3 {
            let offset = (p.first - 1 + (granule * 3 + axis) * p.coefficients) * 8;
            let (mut t0, mut t1, mut d0, mut d1) = (1.0, x, 0.0, 1.0);
            let (mut dd0, mut dd1) = (0.0, 0.0);
            let mut position = f64_at(record, offset);
            if !position.is_finite() {
                return Err(De441Error::InvalidFile("non-finite Chebyshev coefficient"));
            }
            let mut derivative = 0.0;
            let mut second_derivative = 0.0;
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
                // d²T_k/dx² = 4*dT_(k-1)/dx +
                // 2*x*d²T_(k-1)/dx² - d²T_(k-2)/dx².
                let dd = if with_acceleration {
                    4.0 * d1 + 2.0 * x * dd1 - dd0
                } else {
                    0.0
                };
                let c = f64_at(record, offset + k * 8);
                if !c.is_finite() {
                    return Err(De441Error::InvalidFile("non-finite Chebyshev coefficient"));
                }
                position += c * t;
                derivative += c * d;
                if with_acceleration {
                    // Modification (2026-11-16): fail closed on a nonzero
                    // contribution rounded all the way to zero.
                    let contribution = c * dd;
                    if c != 0.0 && dd != 0.0 && contribution == 0.0 {
                        return Err(De441Error::InvalidFile("unrepresentable acceleration"));
                    }
                    second_derivative += contribution;
                    (dd0, dd1) = (dd1, dd);
                }
                (t0, t1, d0, d1) = (t1, t, d1, d);
            }
            // Modification 2026-11-14: avoid spurious intermediate overflow
            // (multiply by 2 first) and subnormal underflow (divide first).
            let velocity = if width >= 2.0 {
                derivative * (2.0 / width)
            } else {
                (derivative / width) * 2.0
            };
            // Scale in stages: squaring 2/width first may overflow for a
            // constant series or underflow even when the final result exists.
            let acceleration = if !with_acceleration || second_derivative == 0.0 {
                0.0
            } else if width >= 1.0 {
                let factor = 2.0 / width;
                (second_derivative * factor) * factor
            } else {
                ((second_derivative / width) * 2.0 / width) * 2.0
            };
            if with_acceleration && second_derivative != 0.0 && acceleration == 0.0 {
                return Err(De441Error::InvalidFile("unrepresentable acceleration"));
            }
            if !position.is_finite() || !velocity.is_finite() || !acceleration.is_finite() {
                return Err(De441Error::InvalidFile("non-finite interpolated state"));
            }
            result.position[axis] = position;
            result.velocity[axis] = velocity;
            result.acceleration[axis] = acceleration;
        }
        Ok(result)
    }
}
