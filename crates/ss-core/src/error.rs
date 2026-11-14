//! Errors raised when constructing domain values.

use thiserror::Error;

/// A value was outside the range accepted by a domain type.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{what} must be within {min}..={max}, got {actual}")]
pub struct RangeError {
    /// Human-readable name of the value (e.g. "transparency level").
    pub what: &'static str,
    /// Smallest accepted value.
    pub min: f32,
    /// Largest accepted value.
    pub max: f32,
    /// The rejected value.
    pub actual: f32,
}
