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

//! Opt-in batch geometric DE441 states at one checked split TDB epoch.
//!
//! Modified 2026-11-14: validate precision once for a group of target-center
//! requests, then reuse the classic reader's coefficient-record cache.
//! This adapter does not perform time-scale conversion, apparent corrections
//! or independent NASA/JPL validation.

use std::io::{Read, Seek};

use crate::calendar::JulianDate;
use crate::de441::{Body, De441};
use crate::de441_checked_state::{CheckedDe441State, CheckedStateError};
use crate::de441_epoch_bridge::checked_de441_epoch;

/// Read target-minus-center geometric states at a single already-TDB epoch.
///
/// Each request is a `(target, center)` pair. All states share the same
/// rounded TDB Julian date and carry the *same* measured input-epoch
/// quantization error in seconds. The precision budget is checked before
/// reading **any** coefficient record, including when requests is empty.
///
/// For each pair, the returned position is in fixed ICRF km, and velocity
/// is in fixed ICRF km per TDB day. Planetary system barycenters remain
/// distinct from their planet centers.
///
/// Results preserve caller order, including duplicate requests. The classic
/// reader caches the last coefficient record; this batch API introduces no
/// second cache or implicit kernel downloads. If any state fails (e.g.
/// missing series or outside coverage), the batch returns the reader error
/// without a partial result or an analytic fallback. Previously read
/// coefficient records may remain cached, as with the scalar API.
///
/// An empty request list yields an empty vector **only after** validating
/// the TDB time scale and finite, nonnegative precision budget.
///
/// This conversion does not preserve split precision inside the classic
/// reader, perform TT/UTC/TDB conversion, apply light-time or aberration,
/// or provide an astronomical position-error bound.
pub fn checked_states_at_split_tdb<R: Read + Seek>(
    ephemeris: &mut De441<R>,
    requests: &[(Body, Body)],
    epoch: JulianDate,
    max_quantization_error_seconds: f64,
) -> Result<Vec<CheckedDe441State>, CheckedStateError> {
    let checked = checked_de441_epoch(epoch, max_quantization_error_seconds)?;
    let mut states = Vec::with_capacity(requests.len());
    for &(target, center) in requests {
        let state = ephemeris.state(target, center, checked.epoch)?;
        states.push(CheckedDe441State {
            state,
            quantization_error_seconds: checked.quantization_error_seconds,
        });
    }
    Ok(states)
}
