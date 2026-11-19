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

// Modification (2026-11-19): same-source Desktop multipack boundary validation.

use std::path::PathBuf;
use virgostars_ephemeris_core::apparent_ecliptic_of_date::{
    experimental_geocentric_apparent_with_provider, experimental_long_term_apparent_ecliptic,
    experimental_long_term_apparent_with_provider,
    geocentric_apparent_six_fields_many_with_options, ApparentError, ApparentFrame,
    ApparentOptions, ExperimentalProviderApparentError,
};
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::de441_pack_manager::{De441PackManager, De441PackSpec};
use virgostars_ephemeris_core::earth_orientation::OrientationError;
use virgostars_ephemeris_core::six_field_units::FixedSixFieldsDegreesAu;

fn fields(f: FixedSixFieldsDegreesAu) -> [f64; 6] {
    [
        f.longitude_deg,
        f.latitude_deg,
        f.distance_au,
        f.longitude_speed_deg_per_tdb_day,
        f.latitude_speed_deg_per_tdb_day,
        f.distance_speed_au_per_tdb_day,
    ]
}

#[test]
#[ignore = "requires locally verified classic DE441 and three private Desktop slices"]
fn apparent_provider_matches_source_at_desktop_boundaries() {
    let source = PathBuf::from(std::env::var_os("DE441_PATH").expect("DE441_PATH"));
    let directory = PathBuf::from(
        std::env::var_os("DE441_DESKTOP_TEST_PACKS").expect("DE441_DESKTOP_TEST_PACKS"),
    );
    let specs = [
        (
            "history",
            "de441-v2-history-00800-00999.eph",
            2013253.5,
            2086302.5,
            false,
        ),
        (
            "base",
            "de441-v2-desktop-core-1000-2200.eph",
            2086302.5,
            2524958.5,
            true,
        ),
        (
            "future",
            "de441-v2-future-02201-02400.eph",
            2524958.5,
            2598007.5,
            false,
        ),
    ]
    .into_iter()
    .map(|(id, file, start, end, required)| {
        De441PackSpec::new(
            id,
            directory.join(file),
            JulianDateTdb(start),
            JulianDateTdb(end),
            required,
        )
    })
    .collect();
    let mut manager = De441PackManager::from_specs(specs).unwrap();
    let mut original = De441::open(source).unwrap();
    let bodies = [
        Body::Sun,
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::MarsBarycenter,
        Body::JupiterBarycenter,
        Body::SaturnBarycenter,
        Body::UranusBarycenter,
        Body::NeptuneBarycenter,
        Body::PlutoBarycenter,
    ];
    let limits = [1e-8, 1e-8, 1e-12, 1e-8, 1e-8, 1e-12];
    let mut supported = 0;
    let mut unsupported = 0;
    for boundary in [2086302.5, 2451545.0, 2524958.5] {
        for offset in [-0.08, -0.02, -1.0 / 86400.0, 0.0, 1.0 / 86400.0, 0.02, 0.08] {
            let epoch = JulianDateTdb(boundary + offset);
            for body in bodies {
                let actual = fields(
                    experimental_long_term_apparent_with_provider(&mut manager, body, epoch)
                        .unwrap(),
                );
                let expected = fields(
                    experimental_long_term_apparent_ecliptic(&mut original, body, epoch).unwrap(),
                );
                for i in 0..6 {
                    let diff = if i == 0 {
                        ((actual[i] - expected[i] + 180.0).rem_euclid(360.0) - 180.0).abs()
                    } else {
                        (actual[i] - expected[i]).abs()
                    };
                    assert!(
                        diff <= limits[i],
                        "boundary={boundary} offset={offset} body={body:?} field={i} diff={diff}"
                    );
                }
                for frame in [
                    ApparentFrame::TrueEclipticOfDate,
                    ApparentFrame::TrueEquatorOfDate,
                ] {
                    let result = experimental_geocentric_apparent_with_provider(
                        &mut manager,
                        body,
                        epoch,
                        frame,
                    );
                    let standard = geocentric_apparent_six_fields_many_with_options(
                        &mut original,
                        &[body],
                        epoch,
                        ApparentOptions {
                            frame,
                            ..ApparentOptions::default()
                        },
                    );
                    let (result, standard) = match (result, standard) {
                        (Ok(result), Ok(standard)) => {
                            supported += 1;
                            (result, standard)
                        }
                        (
                            Err(ExperimentalProviderApparentError::Apparent(
                                ApparentError::Orientation(OrientationError::UnsupportedEpoch {
                                    ..
                                }),
                            )),
                            Err(ApparentError::Orientation(OrientationError::UnsupportedEpoch {
                                ..
                            })),
                        ) => {
                            unsupported += 1;
                            continue;
                        }
                        other => panic!("unexpected provider/source results: {other:?}"),
                    };
                    let actual = fields(result.fields);
                    let expected = fields(standard[0].fields);
                    for i in 0..6 {
                        let diff = if i == 0 {
                            ((actual[i] - expected[i] + 180.0).rem_euclid(360.0) - 180.0).abs()
                        } else {
                            (actual[i] - expected[i]).abs()
                        };
                        assert!(diff <= limits[i], "frame={frame:?} boundary={boundary} offset={offset} body={body:?} field={i} diff={diff}");
                    }
                }
            }
        }
    }
    assert!(supported > 0 && unsupported > 0);
}
