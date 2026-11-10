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

//! VirgoStars Ephemeris: independent astronomical numerical foundations.
//!
//! Experimental pre-release crate: the classic DE441 reader is not independently validated.
//! No SPK file parsing, Earth orientation, UTC conversion, aberration, or
//! validated real-world astronomical positions are provided at this stage.
//!
//! The core is intentionally free of external runtime dependencies so the
//! mathematical primitives can be audited independently.

#![forbid(unsafe_code)]

pub mod calendar;
pub mod chebyshev;
pub mod coordinates;
pub mod de441;
pub mod error;
pub mod first_sky;
pub mod light_time;
pub mod mean_obliquity;
pub mod state;
pub mod time;

pub use chebyshev::{evaluate_chebyshev, ChebyshevSample};
pub use error::{EphemerisError, Result};
pub use state::{ChebyshevPositionRecord, StateVectorKm, Vector3};
pub use time::{JulianDayTdb, TdbSecondsPastJ2000, J2000_JD_TDB, SECONDS_PER_DAY};
