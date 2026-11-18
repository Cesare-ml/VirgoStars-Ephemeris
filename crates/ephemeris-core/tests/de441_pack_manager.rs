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

//! Synthetic-only package selection; does not establish independent JPL parity.

use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use virgostars_ephemeris_core::de441::{Body, De441, De441Error, JulianDateTdb};
use virgostars_ephemeris_core::de441_pack_manager::{
    De441PackError, De441PackManager, De441PackSpec,
};

const RECORD_SIZE: usize = 3000;
const START: f64 = 1000.0;
const STEP: f64 = 32.0;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn create() -> Self {
        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "virgostars-pack-test-{}-{token}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("owned synthetic test directory is removable");
    }
}

fn fixture(records: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; (records + 2) * RECORD_SIZE];
    let title = b"JPL Planetary Ephemeris DE441";
    bytes[..title.len()].copy_from_slice(title);
    bytes[2652..2660].copy_from_slice(&START.to_le_bytes());
    bytes[2660..2668].copy_from_slice(&(START + records as f64 * STEP).to_le_bytes());
    bytes[2668..2676].copy_from_slice(&STEP.to_le_bytes());
    bytes[2688..2696].copy_from_slice(&81.0f64.to_le_bytes());
    bytes[2840..2844].copy_from_slice(&441i32.to_le_bytes());
    for (index, first) in [(0, 3i32), (12, 373i32)] {
        let offset = if index == 12 { 2844 } else { 2696 + index * 12 };
        for (at, value) in [(offset, first), (offset + 4, 1), (offset + 8, 1)] {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    for index in 0..records {
        let offset = (index + 2) * RECORD_SIZE;
        bytes[offset..offset + 8].copy_from_slice(&(START + index as f64 * STEP).to_le_bytes());
        bytes[offset + 8..offset + 16]
            .copy_from_slice(&(START + (index + 1) as f64 * STEP).to_le_bytes());
        for axis in 0..3 {
            let value = (index * 100 + axis * 3) as f64;
            let at = offset + (2 + axis) * 8;
            bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

fn write_file(path: &Path, bytes: &[u8]) {
    let mut output = File::create(path).unwrap();
    output.write_all(bytes).unwrap();
    output.flush().unwrap();
}

fn make_pack(_source: &mut De441<Cursor<Vec<u8>>>, path: &Path, start: f64, end: f64) {
    // Test-only fixture extraction: this branch does not contain the exporter.
    let all = fixture(4);
    let first = ((start - START) / STEP) as usize;
    let count = ((end - start) / STEP) as usize;
    let mut bytes = all[..2 * RECORD_SIZE].to_vec();
    bytes[2652..2660].copy_from_slice(&start.to_le_bytes());
    bytes[2660..2668].copy_from_slice(&end.to_le_bytes());
    bytes.extend_from_slice(&all[(first + 2) * RECORD_SIZE..(first + 2 + count) * RECORD_SIZE]);
    write_file(path, &bytes);
}

fn spec(id: &str, path: &Path, start: f64, end: f64) -> De441PackSpec {
    De441PackSpec::new(id, path, JulianDateTdb(start), JulianDateTdb(end), true)
}

#[test]
fn mobile_catalog_matches_all_ten_manifest_intervals_and_exact_boundaries() {
    let directory = TempDirectory::create();
    let manager = De441PackManager::from_mobile_directory(&directory.0).unwrap();
    let inventory = manager.inventory();
    assert_eq!(inventory.len(), 10);
    assert_eq!(
        inventory.iter().filter(|item| item.spec.required).count(),
        1
    );
    assert!(inventory.iter().all(|item| !item.installed));
    let years_and_jds = [
        ("history-0000-0199", 1721059.5),
        ("history-0200-0399", 1794108.5),
        ("history-0400-0599", 1867156.5),
        ("history-0600-0799", 1940205.5),
        ("history-0800-0999", 2013253.5),
        ("history-1000-1199", 2086302.5),
        ("history-1200-1399", 2159350.5),
        ("history-1400-1599", 2232399.5),
        ("core", 2305447.5),
        ("future-2101-2300", 2488434.5),
    ];
    for &(id, jd) in &years_and_jds {
        assert_eq!(manager.select_pack(JulianDateTdb(jd)).unwrap().id, id);
    }
    assert_eq!(
        manager
            .select_pack(JulianDateTdb(2561482.5 - 1e-5))
            .unwrap()
            .id,
        "future-2101-2300"
    );
    assert!(matches!(
        manager.select_pack(JulianDateTdb(2561482.5)),
        Err(De441PackError::NoPackForEpoch(_))
    ));
}

#[test]
fn chooses_correct_slice_at_boundary_and_preserves_source_vectors() {
    let directory = TempDirectory::create();
    let source_data = fixture(5);
    let mut source = De441::new(Cursor::new(source_data)).unwrap();
    let previous = directory.file("previous.eph");
    let next = directory.file("next.eph");
    make_pack(&mut source, &previous, 1000.0, 1064.0);
    make_pack(&mut source, &next, 1064.0, 1128.0);
    let mut manager = De441PackManager::from_specs(vec![
        spec("next", &next, 1064.0, 1128.0),
        spec("previous", &previous, 1000.0, 1064.0),
    ])
    .unwrap();
    assert_eq!(
        manager.select_pack(JulianDateTdb(1064.0)).unwrap().id,
        "next"
    );
    for jd in [1000.0, 1032.0, 1063.9, 1064.0, 1095.9, 1096.0, 1127.9] {
        let epoch = JulianDateTdb(jd);
        let expected = source
            .state(Body::Mercury, Body::SolarSystemBarycenter, epoch)
            .unwrap();
        let actual = manager
            .state(Body::Mercury, Body::SolarSystemBarycenter, epoch)
            .unwrap();
        assert_eq!(actual, expected);
        let batch = manager
            .states_at(
                &[Body::Mercury, Body::Mercury],
                Body::SolarSystemBarycenter,
                epoch,
            )
            .unwrap();
        assert_eq!(batch, vec![expected, expected]);
        let acceleration = manager
            .state_with_acceleration(Body::Mercury, Body::SolarSystemBarycenter, epoch)
            .unwrap();
        assert_eq!(acceleration.position_km, expected.position_km);
        assert_eq!(
            acceleration.velocity_km_per_day,
            expected.velocity_km_per_day
        );
    }
    assert!(matches!(
        manager.state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1128.0)
        ),
        Err(De441PackError::NoPackForEpoch(_))
    ));
}

#[test]
fn missing_optional_pack_can_be_added_and_removed_without_restarting() {
    let directory = TempDirectory::create();
    let mut source = De441::new(Cursor::new(fixture(5))).unwrap();
    let core = directory.file("core.eph");
    let optional = directory.file("optional.eph");
    make_pack(&mut source, &core, 1000.0, 1064.0);
    let mut manager = De441PackManager::from_specs(vec![
        spec("core", &core, 1000.0, 1064.0),
        spec("history", &optional, 1064.0, 1128.0),
    ])
    .unwrap();
    assert_eq!(
        manager.inventory().iter().filter(|p| p.installed).count(),
        1
    );
    assert!(matches!(
        manager.state(Body::Mercury, Body::SolarSystemBarycenter, JulianDateTdb(1064.0)),
        Err(De441PackError::MissingPack { id, .. }) if id == "history"
    ));
    make_pack(&mut source, &optional, 1064.0, 1128.0);
    assert_eq!(
        manager.inventory().iter().filter(|p| p.installed).count(),
        2
    );
    manager
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1064.0),
        )
        .unwrap();
    manager.unload();
    fs::remove_file(&optional).unwrap();
    assert_eq!(
        manager.inventory().iter().filter(|p| p.installed).count(),
        1
    );
    assert!(matches!(
        manager.state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1064.0)
        ),
        Err(De441PackError::MissingPack { .. })
    ));
    // No implicit fallback from missing history to an available core file.
    manager
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1000.0),
        )
        .unwrap();
}

#[test]
fn invalid_headers_and_inadequate_coverage_fail_explicitly() {
    let directory = TempDirectory::create();
    let corrupted = directory.file("corrupted.eph");
    write_file(&corrupted, b"not a JPL kernel");
    let mut manager =
        De441PackManager::from_specs(vec![spec("bad", &corrupted, 1000.0, 1032.0)]).unwrap();
    assert!(matches!(
        manager.state(Body::Mercury, Body::SolarSystemBarycenter, JulianDateTdb(1000.0)),
        Err(De441PackError::Reader { id, source: De441Error::Io(_), .. }) if id == "bad"
    ));
    let inadequate = directory.file("inadequate.eph");
    let mut source = De441::new(Cursor::new(fixture(5))).unwrap();
    make_pack(&mut source, &inadequate, 1032.0, 1064.0);
    let mut manager =
        De441PackManager::from_specs(vec![spec("wrong", &inadequate, 1000.0, 1064.0)]).unwrap();
    assert!(matches!(
        manager.state(Body::Mercury, Body::SolarSystemBarycenter, JulianDateTdb(1000.0)),
        Err(De441PackError::InvalidPackCoverage { id, .. }) if id == "wrong"
    ));
}

#[test]
fn rejects_ambiguous_catalogs_and_unsupported_epochs() {
    let path = Path::new("unused.eph");
    for invalid in [
        vec![],
        vec![spec("bad", path, 1000.0, 1000.0)],
        vec![spec("bad", path, f64::NAN, 1010.0)],
        vec![
            spec("x", path, 1000.0, 1065.0),
            spec("y", path, 1064.0, 1100.0),
        ],
        vec![
            spec("dupe", path, 1000.0, 1064.0),
            spec("dupe", path, 1064.0, 1100.0),
        ],
    ] {
        assert!(matches!(
            De441PackManager::from_specs(invalid),
            Err(De441PackError::InvalidCatalog(_))
        ));
    }
    let manager = De441PackManager::from_specs(vec![spec("first", path, 1000.0, 1032.0)]).unwrap();
    assert!(matches!(
        manager.select_pack(JulianDateTdb(f64::NAN)),
        Err(De441PackError::InvalidEpoch(_))
    ));
    assert!(matches!(
        manager.select_pack(JulianDateTdb(f64::INFINITY)),
        Err(De441PackError::InvalidEpoch(_))
    ));
    assert!(matches!(
        manager.select_pack(JulianDateTdb(999.0)),
        Err(De441PackError::NoPackForEpoch(_))
    ));
}

#[test]
fn complete_classic_file_uses_original_inclusive_final_endpoint() {
    let directory = TempDirectory::create();
    let path = directory.file("de441.eph");
    write_file(&path, &fixture(5));
    let mut manager = De441PackManager::from_full_file(&path).unwrap();
    assert_eq!(manager.inventory().len(), 1);
    assert_eq!(
        manager.select_pack(JulianDateTdb(1160.0)).unwrap().id,
        "full-de441"
    );
    let state = manager
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1160.0),
        )
        .unwrap();
    let reference = De441::open(&path)
        .unwrap()
        .state(
            Body::Mercury,
            Body::SolarSystemBarycenter,
            JulianDateTdb(1160.0),
        )
        .unwrap();
    assert_eq!(state, reference);
}
