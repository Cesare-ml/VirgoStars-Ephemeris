//! VirgoStars Ephemeris: independent astronomical numerical foundations.
//!
//! Experimental pre-release crate: **not** a planetary ephemeris yet.
//! No SPK file parsing, Earth orientation, UTC conversion, aberration, or
//! validated real-world astronomical positions are provided at this stage.
//!
//! The core is intentionally free of external runtime dependencies so the
//! mathematical primitives can be audited independently.

#![forbid(unsafe_code)]

pub mod chebyshev;
pub mod error;
pub mod state;
pub mod time;

pub use chebyshev::{evaluate_chebyshev, ChebyshevSample};
pub use error::{EphemerisError, Result};
pub use state::{ChebyshevPositionRecord, StateVectorKm, Vector3};
pub use time::{JulianDayTdb, TdbSecondsPastJ2000, J2000_JD_TDB, SECONDS_PER_DAY};
