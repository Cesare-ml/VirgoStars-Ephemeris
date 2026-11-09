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

//! Illustrative mathematics only: synthetic coefficients, not a real planet.

use virgostars_ephemeris_core::{ChebyshevPositionRecord, TdbSecondsPastJ2000};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let record = ChebyshevPositionRecord::new(
        TdbSecondsPastJ2000::new(100.0)?,
        10.0,
        [
            vec![1.0, 2.0, 0.0], // x = 1 + 2*T1(t)
            vec![3.0, 0.0, 0.0], // y = 3
            vec![0.0, 0.0, 1.0], // z = T2(t)
        ],
    )?;
    let state = record.state_at(TdbSecondsPastJ2000::new(105.0)?)?;
    println!("SYNTHETIC POSITION (km): {:?}", state.position_km);
    println!("SYNTHETIC VELOCITY (km/s): {:?}", state.velocity_km_per_second);
    Ok(())
}
