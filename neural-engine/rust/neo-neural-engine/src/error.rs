use std::fmt;

use neo_core::error::NeoError;

/// Neural engine specific error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum NeuralErrorCode {
    ShapeMismatch = 2000,
    DtypeMismatch = 2001,
    DeviceMismatch = 2002,
    BroadcastingError = 2003,
    OutOfBounds = 2004,
    GraphCycle = 2005,
    GraphValidation = 2006,
    OpNotRegistered = 2007,
    MemoryAllocation = 2008,
    ExecutionFailed = 2009,
    AutodiffError = 2010,
    SerializationError = 2011,
    DeviceNotAvailable = 2012,
    KernelError = 2013,
    GradientError = 2014,
}

impl fmt::Display for NeuralErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Errors specific to the neural engine.
#[derive(Debug)]
pub enum NeuralError {
    ShapeMismatch {
        expected: Vec<usize>,
        actual: Vec<usize>,
        context: String,
    },
    DtypeMismatch {
        expected: &'static str,
        actual: &'static str,
        context: String,
    },
    DeviceMismatch {
        expected: String,
        actual: String,
        context: String,
    },
    BroadcastingError {
        left_shape: Vec<usize>,
        right_shape: Vec<usize>,
    },
    OutOfBounds {
        index: usize,
        bound: usize,
        context: String,
    },
    GraphCycle {
        path: Vec<String>,
    },
    GraphValidation {
        message: String,
    },
    OpNotRegistered {
        op_name: String,
    },
    MemoryAllocation {
        requested: usize,
        available: usize,
        context: String,
    },
    ExecutionFailed {
        message: String,
    },
    AutodiffError {
        message: String,
    },
    SerializationError {
        message: String,
    },
    DeviceNotAvailable {
        device: String,
    },
    KernelError {
        message: String,
    },
    GradientError {
        message: String,
    },
}

impl fmt::Display for NeuralError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShapeMismatch {
                expected,
                actual,
                context,
            } => write!(
                f,
                "[shape mismatch in {}] expected {:?}, got {:?}",
                context, expected, actual
            ),
            Self::DtypeMismatch {
                expected,
                actual,
                context,
            } => write!(
                f,
                "[dtype mismatch in {}] expected {}, got {}",
                context, expected, actual
            ),
            Self::DeviceMismatch {
                expected,
                actual,
                context,
            } => write!(
                f,
                "[device mismatch in {}] expected {}, got {}",
                context, expected, actual
            ),
            Self::BroadcastingError {
                left_shape,
                right_shape,
            } => write!(
                f,
                "[broadcasting] cannot broadcast shapes {:?} and {:?}",
                left_shape, right_shape
            ),
            Self::OutOfBounds {
                index,
                bound,
                context,
            } => write!(
                f,
                "[out of bounds in {}] index {} >= bound {}",
                context, index, bound
            ),
            Self::GraphCycle { path } => write!(
                f,
                "[graph cycle] {}",
                path.join(" -> ")
            ),
            Self::GraphValidation { message } => {
                write!(f, "[graph validation] {}", message)
            }
            Self::OpNotRegistered { op_name } => {
                write!(f, "[op not registered] '{}'", op_name)
            }
            Self::MemoryAllocation {
                requested,
                available,
                context,
            } => write!(
                f,
                "[memory allocation in {}] requested {} bytes, available {} bytes",
                context, requested, available
            ),
            Self::ExecutionFailed { message } => {
                write!(f, "[execution failed] {}", message)
            }
            Self::AutodiffError { message } => {
                write!(f, "[autodiff] {}", message)
            }
            Self::SerializationError { message } => {
                write!(f, "[serialization] {}", message)
            }
            Self::DeviceNotAvailable { device } => {
                write!(f, "[device not available] {}", device)
            }
            Self::KernelError { message } => {
                write!(f, "[kernel error] {}", message)
            }
            Self::GradientError { message } => {
                write!(f, "[gradient error] {}", message)
            }
        }
    }
}

impl std::error::Error for NeuralError {}

impl From<NeuralError> for NeoError {
    fn from(e: NeuralError) -> Self {
        NeoError::Internal(e.to_string())
    }
}

impl NeuralError {
    /// Returns the error code for this error.
    #[must_use]
    pub fn code(&self) -> NeuralErrorCode {
        match self {
            Self::ShapeMismatch { .. } => NeuralErrorCode::ShapeMismatch,
            Self::DtypeMismatch { .. } => NeuralErrorCode::DtypeMismatch,
            Self::DeviceMismatch { .. } => NeuralErrorCode::DeviceMismatch,
            Self::BroadcastingError { .. } => NeuralErrorCode::BroadcastingError,
            Self::OutOfBounds { .. } => NeuralErrorCode::OutOfBounds,
            Self::GraphCycle { .. } => NeuralErrorCode::GraphCycle,
            Self::GraphValidation { .. } => NeuralErrorCode::GraphValidation,
            Self::OpNotRegistered { .. } => NeuralErrorCode::OpNotRegistered,
            Self::MemoryAllocation { .. } => NeuralErrorCode::MemoryAllocation,
            Self::ExecutionFailed { .. } => NeuralErrorCode::ExecutionFailed,
            Self::AutodiffError { .. } => NeuralErrorCode::AutodiffError,
            Self::SerializationError { .. } => NeuralErrorCode::SerializationError,
            Self::DeviceNotAvailable { .. } => NeuralErrorCode::DeviceNotAvailable,
            Self::KernelError { .. } => NeuralErrorCode::KernelError,
            Self::GradientError { .. } => NeuralErrorCode::GradientError,
        }
    }
}

/// Convenience result type for neural engine operations.
pub type NeuralResult<T> = Result<T, NeuralError>;
