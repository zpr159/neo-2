use std::fmt;

use neo_neural_engine::error::NeuralError;

/// Error type for the neural network framework.
#[derive(Debug)]
pub enum NnError {
    /// Shape mismatch between tensors.
    ShapeMismatch {
        expected: Vec<usize>,
        actual: Vec<usize>,
        context: String,
    },
    /// Invalid input to an operation.
    InvalidInput(String),
    /// Gradient computation error.
    GradientError(String),
    /// Module not found in the model.
    ModuleNotFound(String),
    /// Parameter not found.
    ParameterNotFound(String),
    /// Configuration error.
    ConfigError(String),
    /// Checkpoint error.
    CheckpointError(String),
    /// Serialization error.
    SerializationError(String),
    /// I/O error.
    IoError(std::io::Error),
    /// Training error.
    TrainingError(String),
    /// Model zoo error.
    ModelZooError(String),
    /// Dataset error.
    DatasetError(String),
    /// Initialization error.
    InitializationError(String),
    /// Numerical error (NaN, Inf).
    NumericalError(String),
    /// Backend error propagated from neural engine.
    NeuralEngine(NeuralError),
}

impl fmt::Display for NnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NnError::ShapeMismatch {
                expected,
                actual,
                context,
            } => write!(
                f,
                "[shape mismatch] expected {:?}, got {:?} in {}",
                expected, actual, context
            ),
            NnError::InvalidInput(msg) => write!(f, "[invalid input] {}", msg),
            NnError::GradientError(msg) => write!(f, "[gradient error] {}", msg),
            NnError::ModuleNotFound(name) => write!(f, "[module not found] {}", name),
            NnError::ParameterNotFound(name) => write!(f, "[parameter not found] {}", name),
            NnError::ConfigError(msg) => write!(f, "[config error] {}", msg),
            NnError::CheckpointError(msg) => write!(f, "[checkpoint error] {}", msg),
            NnError::SerializationError(msg) => write!(f, "[serialization error] {}", msg),
            NnError::IoError(err) => write!(f, "[io error] {}", err),
            NnError::TrainingError(msg) => write!(f, "[training error] {}", msg),
            NnError::ModelZooError(msg) => write!(f, "[model zoo error] {}", msg),
            NnError::DatasetError(msg) => write!(f, "[dataset error] {}", msg),
            NnError::InitializationError(msg) => write!(f, "[initialization error] {}", msg),
            NnError::NumericalError(msg) => write!(f, "[numerical error] {}", msg),
            NnError::NeuralEngine(err) => write!(f, "[neural engine] {}", err),
        }
    }
}

impl std::error::Error for NnError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NnError::IoError(err) => Some(err),
            NnError::NeuralEngine(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for NnError {
    fn from(err: std::io::Error) -> Self {
        NnError::IoError(err)
    }
}

impl From<NeuralError> for NnError {
    fn from(err: NeuralError) -> Self {
        NnError::NeuralEngine(err)
    }
}

impl From<serde_json::Error> for NnError {
    fn from(err: serde_json::Error) -> Self {
        NnError::SerializationError(err.to_string())
    }
}

impl From<bincode::Error> for NnError {
    fn from(err: bincode::Error) -> Self {
        NnError::SerializationError(err.to_string())
    }
}

/// Convenience result alias for NN framework operations.
pub type NnResult<T> = Result<T, NnError>;
