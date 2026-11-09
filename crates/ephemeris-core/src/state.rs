//! Geometric XYZ state interpolation for a single SPK-type-2-style
//! Chebyshev position record. This is not yet a binary SPK parser.
//!
//! Position polynomial coefficients must represent kilometers. Velocity is
//! obtained analytically by differentiating the position polynomial and
//! dividing by the record radius in TDB seconds.

use crate::chebyshev::evaluate_chebyshev;
use crate::error::{EphemerisError, Result};
use crate::time::TdbSecondsPastJ2000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateVectorKm {
    pub position_km: Vector3,
    pub velocity_km_per_second: Vector3,
}

/// One validated Cartesian Chebyshev position record.
///
/// The record has midpoint and radius in TDB seconds past J2000, and three
/// equal-length coefficient arrays expressed in kilometers. Do not confuse
/// this representation with SPK type 3, whose velocity is independently
/// interpolated rather than differentiated from position.
#[derive(Clone, Debug)]
pub struct ChebyshevPositionRecord {
    midpoint_et_seconds: f64,
    radius_seconds: f64,
    coefficients_km: [Vec<f64>; 3],
}

impl ChebyshevPositionRecord {
    pub fn new(
        midpoint: TdbSecondsPastJ2000,
        radius_seconds: f64,
        coefficients_km: [Vec<f64>; 3],
    ) -> Result<Self> {
        if !radius_seconds.is_finite() || radius_seconds <= 0.0 {
            return Err(EphemerisError::InvalidRadius);
        }
        let len = coefficients_km[0].len();
        if len == 0 {
            return Err(EphemerisError::EmptyCoefficients);
        }
        if coefficients_km.iter().any(|axis| axis.len() != len) {
            return Err(EphemerisError::MismatchedCoefficientLengths);
        }
        if coefficients_km.iter().flatten().any(|c| !c.is_finite()) {
            return Err(EphemerisError::NonFinite {
                field: "position Chebyshev coefficient",
            });
        }
        if !(midpoint.value() - radius_seconds).is_finite()
            || !(midpoint.value() + radius_seconds).is_finite()
        {
            return Err(EphemerisError::ArithmeticOverflow);
        }

        Ok(Self {
            midpoint_et_seconds: midpoint.value(),
            radius_seconds,
            coefficients_km,
        })
    }

    pub fn state_at(&self, epoch: TdbSecondsPastJ2000) -> Result<StateVectorKm> {
        let x = (epoch.value() - self.midpoint_et_seconds) / self.radius_seconds;
        let sx = evaluate_chebyshev(&self.coefficients_km[0], x)?;
        let sy = evaluate_chebyshev(&self.coefficients_km[1], x)?;
        let sz = evaluate_chebyshev(&self.coefficients_km[2], x)?;

        let state = StateVectorKm {
            position_km: Vector3 {
                x: sx.value,
                y: sy.value,
                z: sz.value,
            },
            velocity_km_per_second: Vector3 {
                x: sx.derivative_dx / self.radius_seconds,
                y: sy.derivative_dx / self.radius_seconds,
                z: sz.derivative_dx / self.radius_seconds,
            },
        };
        if [
            state.velocity_km_per_second.x,
            state.velocity_km_per_second.y,
            state.velocity_km_per_second.z,
        ].iter().any(|v| !v.is_finite()) {
            return Err(EphemerisError::ArithmeticOverflow);
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1.0e-12, "expected {b}, got {a}");
    }

    #[test]
    fn synthetic_record_has_correct_positions_and_velocities() {
        // Midpoint=100s; radius=10s; sample time=105s => x=0.5.
        // x-position: 1 + 2*T1(x) = 2km, vx=2/10 = 0.2km/s.
        // y-position: 3km, vy=0km/s.
        // z-position: T2(x) = -0.5km, vz=4*x/10 = 0.2km/s.
        let record = ChebyshevPositionRecord::new(
            TdbSecondsPastJ2000::new(100.0).unwrap(),
            10.0,
            [vec![1.0, 2.0, 0.0], vec![3.0, 0.0, 0.0], vec![0.0, 0.0, 1.0]],
        ).unwrap();
        let s = record.state_at(TdbSecondsPastJ2000::new(105.0).unwrap()).unwrap();
        near(s.position_km.x, 2.0);
        near(s.position_km.y, 3.0);
        near(s.position_km.z, -0.5);
        near(s.velocity_km_per_second.x, 0.2);
        near(s.velocity_km_per_second.y, 0.0);
        near(s.velocity_km_per_second.z, 0.2);
    }

    #[test]
    fn refuses_epochs_outside_record() {
        let record = ChebyshevPositionRecord::new(
            TdbSecondsPastJ2000::new(0.0).unwrap(),
            10.0,
            [vec![1.0], vec![2.0], vec![3.0]],
        ).unwrap();
        assert!(record.state_at(TdbSecondsPastJ2000::new(-10.0).unwrap()).is_ok());
        assert!(record.state_at(TdbSecondsPastJ2000::new(10.0).unwrap()).is_ok());
        assert!(record.state_at(TdbSecondsPastJ2000::new(11.0).unwrap()).is_err());
    }

    #[test]
    fn rejects_invalid_record() {
        let midpoint = TdbSecondsPastJ2000::new(0.0).unwrap();
        assert!(ChebyshevPositionRecord::new(midpoint, 0.0, [vec![1.0],vec![2.0],vec![3.0]]).is_err());
        assert!(ChebyshevPositionRecord::new(midpoint, 1.0, [vec![1.0],vec![2.0,3.0],vec![3.0]]).is_err());
        assert!(ChebyshevPositionRecord::new(midpoint, 1.0, [vec![1.0],vec![f64::NAN],vec![3.0]]).is_err());
    }
}
