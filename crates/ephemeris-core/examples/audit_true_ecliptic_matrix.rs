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
// Modification 2026-11-18: export existing TT rotation for external ERFA audit.
// SPDX-License-Identifier: CPAL-1.0

//! Export the existing ICRF-to-true-ecliptic-of-date rotation for an offline
//! independent comparison. Arguments are caller-supplied JD TT values.
use std::error::Error;
use virgostars_ephemeris_core::earth_orientation::true_ecliptic_of_date_matrix;

fn main() -> Result<(), Box<dyn Error>> {
    let epochs: Vec<f64> = std::env::args()
        .skip(1)
        .map(|s| s.parse())
        .collect::<Result<_, _>>()?;
    if epochs.is_empty() {
        return Err("supply at least one JD TT epoch".into());
    }
    for epoch in epochs {
        let matrix = true_ecliptic_of_date_matrix(epoch)?;
        print!("{epoch:.9}");
        for value in matrix.into_iter().flatten() {
            print!(",{value:.17e}");
        }
        println!();
    }
    Ok(())
}
