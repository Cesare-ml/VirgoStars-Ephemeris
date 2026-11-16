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

//! Small-body ephemeris from a NAIF SPK file of **segment type 21** (Extended
//! Modified Difference Arrays, the type JPL uses for asteroid and comet SPKs).
//!
//! The caller supplies the file bytes; nothing is bundled or downloaded. The reader
//! accepts little-endian (`LTL-IEEE`) DAF/SPK files whose segments are all type 21,
//! in the J2000 (ICRF-aligned) frame, for one target relative to one center
//! (JPL small-body files use the Sun, body 10). Segments must be ordered in time.
//! Evaluation follows the published Shampine–Gordon difference-table interpolation
//! of NAIF's SPK required reading; it was checked against JPL Horizons vector
//! tables (see the tests and `docs/MVP2_CHIRON_2026-11-16.md`).
//!
//! # Record layout (per difference line, `4·MAXDIM + 11` doubles)
//!
//! `TL, G(1..MAXDIM), REFPOS(1), REFVEL(1), REFPOS(2), REFVEL(2), REFPOS(3),
//! REFVEL(3), DT(MAXDIM, 3), KQMAX1, KQ(1..3)`. A segment holds `N` such lines, `N`
//! final epochs, a directory of every 100th epoch, then `MAXDIM` and `N`.
//!
//! Times are TDB seconds past J2000 (`ET`); states are km and km/s.

use std::error::Error;
use std::fmt;

use crate::de441::JulianDateTdb;

const RECORD_BYTES: usize = 1024;
const J2000_JD: f64 = 2_451_545.0;
const SECONDS_PER_DAY: f64 = 86_400.0;

#[derive(Debug, PartialEq, Eq)]
pub enum SpkType21Error {
    /// Not a little-endian DAF/SPK file, or truncated or inconsistent.
    InvalidFile(&'static str),
    /// A segment is not type 21, not in the J2000 frame, or targets differ.
    UnsupportedSegment(&'static str),
    /// The epoch is outside every segment (no extrapolation).
    OutsideCoverage,
    /// A difference line is internally inconsistent.
    CorruptRecord(&'static str),
    NonFinite,
}

impl fmt::Display for SpkType21Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFile(m) => write!(f, "invalid SPK file: {m}"),
            Self::UnsupportedSegment(m) => write!(f, "unsupported SPK segment: {m}"),
            Self::OutsideCoverage => write!(f, "epoch outside the SPK coverage"),
            Self::CorruptRecord(m) => write!(f, "corrupt type 21 record: {m}"),
            Self::NonFinite => write!(f, "non-finite epoch"),
        }
    }
}
impl Error for SpkType21Error {}

#[derive(Clone, Copy, Debug)]
struct Segment {
    et_start: f64,
    et_end: f64,
    /// First data word (1-based double-word address) and record geometry.
    begin: usize,
    count: usize,
    maxdim: usize,
}

/// A validated type 21 SPK for one target.
#[derive(Clone, Debug)]
pub struct SmallBodySpk {
    bytes: Vec<u8>,
    segments: Vec<Segment>,
    target_id: i32,
    center_id: i32,
}

fn read_i32(bytes: &[u8], at: usize) -> Result<i32, SpkType21Error> {
    let slice = bytes
        .get(at..at + 4)
        .ok_or(SpkType21Error::InvalidFile("truncated"))?;
    Ok(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_f64(bytes: &[u8], at: usize) -> Result<f64, SpkType21Error> {
    let slice = bytes
        .get(at..at + 8)
        .ok_or(SpkType21Error::InvalidFile("truncated"))?;
    let mut raw = [0u8; 8];
    raw.copy_from_slice(slice);
    Ok(f64::from_le_bytes(raw))
}

impl SmallBodySpk {
    /// Parse and validate the file. All segments must be type 21 for one target.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, SpkType21Error> {
        if bytes.len() < 2 * RECORD_BYTES || &bytes[0..8] != b"DAF/SPK " {
            return Err(SpkType21Error::InvalidFile("not a DAF/SPK file"));
        }
        if &bytes[88..96] != b"LTL-IEEE" {
            return Err(SpkType21Error::InvalidFile(
                "only little-endian files are supported",
            ));
        }
        let nd = read_i32(&bytes, 8)?;
        let ni = read_i32(&bytes, 12)?;
        if nd != 2 || ni != 6 {
            return Err(SpkType21Error::InvalidFile("an SPK has ND=2 and NI=6"));
        }
        let mut record = read_i32(&bytes, 76)?;
        let summary_doubles = 5usize; // ND + (NI + 1) / 2
        let mut segments = Vec::new();
        let (mut target, mut center) = (None, None);
        let mut guard = 0;
        while record > 0 {
            guard += 1;
            if guard > 10_000 {
                return Err(SpkType21Error::InvalidFile("summary records loop"));
            }
            let base = (record as usize - 1) * RECORD_BYTES;
            if base + RECORD_BYTES > bytes.len() {
                return Err(SpkType21Error::InvalidFile(
                    "summary record beyond the file",
                ));
            }
            let next = read_f64(&bytes, base)?;
            let count = read_f64(&bytes, base + 16)?;
            if !(0.0..=25.0).contains(&count) || count.fract() != 0.0 {
                return Err(SpkType21Error::InvalidFile("bad summary count"));
            }
            for i in 0..count as usize {
                let at = base + 24 + i * summary_doubles * 8;
                let et_start = read_f64(&bytes, at)?;
                let et_end = read_f64(&bytes, at + 8)?;
                let t = read_i32(&bytes, at + 16)?;
                let c = read_i32(&bytes, at + 20)?;
                let frame = read_i32(&bytes, at + 24)?;
                let kind = read_i32(&bytes, at + 28)?;
                let begin = read_i32(&bytes, at + 32)?;
                let end = read_i32(&bytes, at + 36)?;
                if kind != 21 {
                    return Err(SpkType21Error::UnsupportedSegment("not type 21"));
                }
                if frame != 1 {
                    return Err(SpkType21Error::UnsupportedSegment("not the J2000 frame"));
                }
                if *target.get_or_insert(t) != t || *center.get_or_insert(c) != c {
                    return Err(SpkType21Error::UnsupportedSegment(
                        "several targets or centers",
                    ));
                }
                if !(et_start.is_finite() && et_end.is_finite() && et_start < et_end)
                    || begin < 1
                    || end <= begin
                {
                    return Err(SpkType21Error::InvalidFile("bad segment summary"));
                }
                let (begin, end) = (begin as usize, end as usize);
                if end * 8 > bytes.len() {
                    return Err(SpkType21Error::InvalidFile("segment beyond the file"));
                }
                let count = read_f64(&bytes, (end - 1) * 8)?;
                let maxdim = read_f64(&bytes, (end - 2) * 8)?;
                if count.fract() != 0.0
                    || maxdim.fract() != 0.0
                    || !(1.0..=1.0e6).contains(&count)
                    || !(2.0..=200.0).contains(&maxdim)
                {
                    return Err(SpkType21Error::InvalidFile("bad type 21 tail"));
                }
                let (count, maxdim) = (count as usize, maxdim as usize);
                let line = 4 * maxdim + 11;
                let directory = count / 100;
                if end - begin + 1 != count * line + count + directory + 2 {
                    return Err(SpkType21Error::InvalidFile(
                        "segment size does not match its tail",
                    ));
                }
                segments.push(Segment {
                    et_start,
                    et_end,
                    begin,
                    count,
                    maxdim,
                });
            }
            record = next as i32;
        }
        segments.sort_by(|a, b| a.et_start.total_cmp(&b.et_start));
        if segments.is_empty() {
            return Err(SpkType21Error::InvalidFile("no segments"));
        }
        if segments
            .windows(2)
            .any(|w| w[1].et_start < w[0].et_end - 1e-6)
        {
            return Err(SpkType21Error::UnsupportedSegment("overlapping segments"));
        }
        Ok(Self {
            bytes,
            segments,
            target_id: target.unwrap_or(0),
            center_id: center.unwrap_or(0),
        })
    }

    pub fn target_id(&self) -> i32 {
        self.target_id
    }
    pub fn center_id(&self) -> i32 {
        self.center_id
    }
    /// Coverage as TDB Julian dates.
    pub fn coverage_jd_tdb(&self) -> (f64, f64) {
        let first = self.segments[0].et_start;
        let last = self.segments[self.segments.len() - 1].et_end;
        (
            J2000_JD + first / SECONDS_PER_DAY,
            J2000_JD + last / SECONDS_PER_DAY,
        )
    }

    fn word(&self, address: usize) -> Result<f64, SpkType21Error> {
        read_f64(&self.bytes, (address - 1) * 8)
    }

    /// State of the target relative to the center at `et` (TDB seconds past
    /// J2000): position in km, velocity in km/s, J2000 axes.
    pub fn state_at_et(&self, et: f64) -> Result<[f64; 6], SpkType21Error> {
        if !et.is_finite() {
            return Err(SpkType21Error::NonFinite);
        }
        let segment = self
            .segments
            .iter()
            .find(|s| et >= s.et_start && et <= s.et_end)
            .ok_or(SpkType21Error::OutsideCoverage)?;
        let line = 4 * segment.maxdim + 11;
        let epochs_at = segment.begin + segment.count * line;
        // First line whose final epoch is not before `et`.
        let (mut low, mut high) = (0usize, segment.count);
        while low < high {
            let mid = (low + high) / 2;
            if self.word(epochs_at + mid)? < et {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        let index = low.min(segment.count - 1);
        let base = segment.begin + index * line;
        let md = segment.maxdim;
        let tl = self.word(base)?;
        let g: Vec<f64> = (0..md)
            .map(|j| self.word(base + 1 + j))
            .collect::<Result<_, _>>()?;
        let mut inter = [0.0; 6];
        for (k, slot) in inter.iter_mut().enumerate() {
            *slot = self.word(base + 1 + md + k)?;
        }
        let refpos = [inter[0], inter[2], inter[4]];
        let refvel = [inter[1], inter[3], inter[5]];
        let dt_at = |i: usize, j: usize| self.word(base + 7 + md + i * md + j);
        let kqmax1 = self.word(base + 7 + 4 * md)?;
        let mut kq = [0usize; 3];
        for (i, slot) in kq.iter_mut().enumerate() {
            let v = self.word(base + 8 + 4 * md + i)?;
            if v.fract() != 0.0 || v < 1.0 || v > md as f64 {
                return Err(SpkType21Error::CorruptRecord("integration order"));
            }
            *slot = v as usize;
        }
        if kqmax1.fract() != 0.0 || kqmax1 < 3.0 || kqmax1 > (md + 1) as f64 {
            return Err(SpkType21Error::CorruptRecord("KQMAX1"));
        }
        let kqmax = kqmax1 as usize - 1;
        let delta = et - tl;
        let mut tp = delta;
        let mq2 = kqmax - 2;
        let mut ks = kqmax - 1;
        let mut fc = vec![0.0; md + 8];
        let mut wc = vec![0.0; md + 8];
        let mut w = vec![0.0; md + 8];
        fc[0] = 1.0;
        for j in 0..mq2 {
            if !g[j].is_finite() || g[j] == 0.0 {
                return Err(SpkType21Error::CorruptRecord("step size"));
            }
            fc[j + 1] = tp / g[j];
            wc[j] = delta / g[j];
            tp = delta + g[j];
        }
        for (j, slot) in w.iter_mut().enumerate().take(kqmax) {
            *slot = 1.0 / (j as f64 + 1.0);
        }
        let mut jx = 0usize;
        let mut ks1 = ks - 1;
        while ks >= 2 {
            jx += 1;
            for j in 0..jx {
                w[j + ks] = fc[j + 1] * w[j + ks1] - wc[j] * w[j + ks];
            }
            ks = ks1;
            ks1 = ks1.saturating_sub(1);
        }
        let mut state = [0.0; 6];
        for i in 0..3 {
            let mut sum = 0.0;
            for j in (0..kq[i]).rev() {
                sum += dt_at(i, j)? * w[j + ks];
            }
            state[i] = refpos[i] + delta * (refvel[i] + delta * sum);
        }
        for j in 0..mq2 {
            w[j + 1] = fc[j + 1] * w[j] - wc[j] * w[j + 1];
        }
        ks -= 1;
        for i in 0..3 {
            let mut sum = 0.0;
            for j in (0..kq[i]).rev() {
                sum += dt_at(i, j)? * w[j + ks];
            }
            state[i + 3] = refvel[i] + delta * sum;
        }
        if state.iter().any(|v| !v.is_finite()) {
            return Err(SpkType21Error::CorruptRecord("non-finite state"));
        }
        Ok(state)
    }

    /// State at a TDB Julian date: position km, velocity km per TDB day.
    pub fn state_at(&self, epoch: JulianDateTdb) -> Result<([f64; 3], [f64; 3]), SpkType21Error> {
        let s = self.state_at_et((epoch.0 - J2000_JD) * SECONDS_PER_DAY)?;
        Ok((
            [s[0], s[1], s[2]],
            [
                s[3] * SECONDS_PER_DAY,
                s[4] * SECONDS_PER_DAY,
                s[5] * SECONDS_PER_DAY,
            ],
        ))
    }
}
