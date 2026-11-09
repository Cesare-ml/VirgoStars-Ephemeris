use core::fmt;

/// Errors returned by the numerical foundations. No silent NaN fallbacks.
#[derive(Clone, Debug, PartialEq)]
pub enum EphemerisError {
    NonFinite { field: &'static str },
    EmptyCoefficients,
    MismatchedCoefficientLengths,
    InvalidRadius,
    OutsideRecord { normalized_time: f64 },
    ArithmeticOverflow,
}

pub type Result<T> = core::result::Result<T, EphemerisError>;

impl fmt::Display for EphemerisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => write!(f, "non-finite numeric input: {field}"),
            Self::EmptyCoefficients => write!(f, "Chebyshev coefficients cannot be empty"),
            Self::MismatchedCoefficientLengths => {
                write!(f, "three coordinate axes must have the same polynomial degree")
            }
            Self::InvalidRadius => {
                write!(f, "Chebyshev record radius must be finite and positive")
            }
            Self::OutsideRecord { normalized_time } => write!(
                f,
                "time outside Chebyshev record domain [-1, 1]: {normalized_time}"
            ),
            Self::ArithmeticOverflow => write!(f, "numeric result is not finite"),
        }
    }
}

impl std::error::Error for EphemerisError {}
