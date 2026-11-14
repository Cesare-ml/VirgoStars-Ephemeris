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

//! Fault injection: an SPK I/O error must remain isolated to its pair.
//!
//! Modified 2026-11-14: synthetic mixed-endian kernel read failure and
//! recovery checks for diagnostic probes and the atomic batch contract.
//! These are not real JPL/NAIF numerical accuracy tests.

use std::cell::Cell;
use std::error::Error;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;

use virgostars_ephemeris_core::spk_batch::{relative_states_at, SpkBatchError};
use virgostars_ephemeris_core::spk_direct_state::SpkStateError;
use virgostars_ephemeris_core::spk_kernel_set::SpkKernelSet;
use virgostars_ephemeris_core::spk_pair_diagnostics::probe_relative_states_at;
use virgostars_ephemeris_core::spk_relative_state::SpkRelativeError;

/// One caller-owned source that can fail *after* its DAF index has loaded.
struct SwitchableReader {
    cursor: Cursor<Vec<u8>>,
    fail_reads: Rc<Cell<bool>>,
}

impl SwitchableReader {
    fn new(bytes: Vec<u8>, fail_reads: Rc<Cell<bool>>) -> Self {
        Self {
            cursor: Cursor::new(bytes),
            fail_reads,
        }
    }
}

impl Read for SwitchableReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.fail_reads.get() {
            return Err(io::Error::other("synthetic SPK read failure"));
        }
        self.cursor.read(buf)
    }
}

impl Seek for SwitchableReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.cursor.seek(pos)
    }
}

fn put_i32(bytes: &mut [u8], at: usize, n: i32, little: bool) {
    let word = if little {
        n.to_le_bytes()
    } else {
        n.to_be_bytes()
    };
    bytes[at..at + 4].copy_from_slice(&word);
}

fn put_f64(bytes: &mut [u8], at: usize, n: f64, little: bool) {
    let word = if little {
        n.to_le_bytes()
    } else {
        n.to_be_bytes()
    };
    bytes[at..at + 8].copy_from_slice(&word);
}

fn synthetic_spk(
    target: i32,
    center: i32,
    position: [f64; 2],
    velocity: [f64; 2],
    little: bool,
) -> Vec<u8> {
    // Invented Type-2 DAF/SPK, one J2000/ICRF record covering ET [0, 10].
    // The record midpoint is 5 TDB seconds and its radius is 5 seconds.
    let mut bytes = vec![0u8; 5 * 1024];
    bytes[..8].copy_from_slice(b"DAF/SPK ");
    bytes[88..96].copy_from_slice(if little { b"LTL-IEEE" } else { b"BIG-IEEE" });
    for (at, value) in [(8, 2), (12, 6), (76, 3), (80, 3), (84, 525)] {
        put_i32(&mut bytes, at, value, little);
    }
    let summary = 2 * 1024;
    put_f64(&mut bytes, summary + 16, 1.0, little);
    put_f64(&mut bytes, summary + 24, 0.0, little);
    put_f64(&mut bytes, summary + 32, 10.0, little);
    for (i, value) in [target, center, 1, 2, 513, 524].into_iter().enumerate() {
        put_i32(&mut bytes, summary + 40 + i * 4, value, little);
    }
    let record = 4 * 1024;
    put_f64(&mut bytes, record, 5.0, little);
    put_f64(&mut bytes, record + 8, 5.0, little);
    for (i, value) in [
        position[0],
        velocity[0] * 5.0,
        position[1],
        velocity[1] * 5.0,
        0.0,
        0.0,
    ]
    .into_iter()
    .enumerate()
    {
        put_f64(&mut bytes, record + 16 + i * 8, value, little);
    }
    for (i, value) in [0.0, 10.0, 8.0, 1.0].into_iter().enumerate() {
        put_f64(&mut bytes, record + 64 + i * 8, value, little);
    }
    bytes
}

fn loaded() -> (SpkKernelSet<SwitchableReader>, Rc<Cell<bool>>) {
    let healthy = Rc::new(Cell::new(false));
    let affected = Rc::new(Cell::new(false));
    let mut kernels = SpkKernelSet::new();
    kernels
        .load(SwitchableReader::new(
            synthetic_spk(399, 0, [100.0, 5.0], [3.0, -1.0], true),
            healthy,
        ))
        .unwrap();
    kernels
        .load(SwitchableReader::new(
            synthetic_spk(301, 399, [2.0, 4.0], [0.1, 0.2], false),
            affected.clone(),
        ))
        .unwrap();
    (kernels, affected)
}

#[test]
fn diagnostic_probes_keep_good_pairs_after_a_selected_kernel_read_failure() {
    let (mut kernels, affected) = loaded();
    affected.set(true);
    let requests = [(399, 0), (301, 0), (399, 0), (301, 399), (399, 0)];
    let probes = probe_relative_states_at(&mut kernels, &requests, 5.0).unwrap();

    assert_eq!(probes.len(), requests.len());
    for (index, &(target, center)) in requests.iter().enumerate() {
        assert_eq!(probes[index].request_index, index);
        assert_eq!(
            (probes[index].target_id, probes[index].center_id),
            (target, center)
        );
    }
    for index in [0, 2, 4] {
        let state = probes[index].result.as_ref().unwrap();
        assert_eq!(state.position_km, [100.0, 5.0, 0.0]);
        assert_eq!(state.velocity_km_per_second, [3.0, -1.0, 0.0]);
    }
    for index in [1, 3] {
        let error = probes[index].result.as_ref().unwrap_err();
        assert!(matches!(
            error,
            SpkRelativeError::Segment(SpkStateError::Io(_))
        ));
        assert!(
            error.source().is_some(),
            "preserve the underlying I/O error"
        );
    }
    assert_eq!(kernels.kernel_count(), 2);
}

#[test]
fn atomic_batch_still_fails_then_diagnostic_probe_recovers_without_reloading() {
    let (mut kernels, affected) = loaded();
    affected.set(true);
    let requests = [(399, 0), (301, 0), (399, 0)];
    assert!(matches!(
        relative_states_at(&mut kernels, &requests, 5.0),
        Err(SpkBatchError::Request {
            index: 1,
            target_id: 301,
            center_id: 0,
            ..
        })
    ));

    affected.set(false);
    let recovered = probe_relative_states_at(&mut kernels, &requests, 5.0).unwrap();
    assert!(recovered.iter().all(|probe| probe.result.is_ok()));
    assert_eq!(
        recovered[1].result.as_ref().unwrap().position_km,
        [102.0, 9.0, 0.0]
    );
    assert_eq!(
        recovered[1].result.as_ref().unwrap().velocity_km_per_second,
        [3.1, -0.8, 0.0]
    );
    let atomic = relative_states_at(&mut kernels, &requests, 5.0).unwrap();
    assert_eq!(atomic.len(), requests.len());
    assert_eq!(atomic[0].position_km, [100.0, 5.0, 0.0]);
    assert_eq!(atomic[1].position_km, [102.0, 9.0, 0.0]);
    assert_eq!(atomic[2].position_km, [100.0, 5.0, 0.0]);
    assert_eq!(kernels.kernel_count(), 2);
}
