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
