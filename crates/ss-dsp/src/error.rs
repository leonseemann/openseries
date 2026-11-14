//! Construction errors.

/// Errors raised when creating processors (never during processing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DspError {
    /// Sample rate is not a positive finite number.
    InvalidSampleRate(f32),
    /// Channel count outside `1..=MAX_CHANNELS`.
    UnsupportedChannels(usize),
}

impl std::fmt::Display for DspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSampleRate(rate) => write!(f, "invalid sample rate {rate}"),
            Self::UnsupportedChannels(n) => {
                write!(
                    f,
                    "unsupported channel count {n} (1..={})",
                    crate::MAX_CHANNELS
                )
            }
        }
    }
}

impl std::error::Error for DspError {}
