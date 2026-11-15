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
//! Experimental pre-release crate. The classic DE441 reader, bounded civil
//! time conversion, light-time, aberration and date-frame projection do not
//! yet constitute a fully validated astronomical or Swiss-compatible engine.
//! A separate experimental SPK reader handles selected J2000 type 2/3
//! geometric states. No mode silently falls back to lite data.
//!
//! The core is intentionally free of external runtime dependencies so the
//! mathematical primitives can be audited independently.

#![forbid(unsafe_code)]

pub mod aberrated_spherical;
pub mod aberration;
pub mod aberration_state;
pub mod calendar;
pub mod calendar_decimal_hours;
pub mod chebyshev;
pub mod civil_aberrated;
pub mod coordinates;
pub mod de441;
pub mod earth_orientation;
pub mod equatorial_horizontal;
pub mod error;
pub mod first_sky;
pub mod fixed_star;
pub mod horizon_events;
pub mod house_angles;
pub mod light_time;
pub mod lunar_points;
pub mod physical_center;
pub mod planetary_solar_deflection;
pub mod six_field_units;
pub mod solar_deflection;
pub mod spk_direct_state;
pub mod spk_index;
pub mod spk_kernel_set;
pub mod spk_relative_state;
pub mod state;
pub mod time;
pub mod true_ecliptic;

pub use chebyshev::{evaluate_chebyshev, ChebyshevSample};
pub use error::{EphemerisError, Result};
pub use state::{ChebyshevPositionRecord, StateVectorKm, Vector3};
pub use time::{JulianDayTdb, TdbSecondsPastJ2000, J2000_JD_TDB, SECONDS_PER_DAY};
