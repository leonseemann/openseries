//! Errors of the macOS audio backend.

use thiserror::Error;

/// Failure while talking to CoreAudio.
#[derive(Debug, Error)]
pub enum AudioError {
    /// A CoreAudio call returned a non-zero `OSStatus`.
    #[error("{context} failed with OSStatus {status} ({})", fourcc(*status))]
    Os {
        /// What was being attempted.
        context: &'static str,
        /// The returned status code.
        status: i32,
    },
    /// A requested device or process does not exist (any more).
    #[error("not found: {0}")]
    NotFound(String),
    /// The device/stream layout is not supported by this version.
    #[error("unsupported: {0}")]
    Unsupported(String),
}

impl AudioError {
    /// Converts an `OSStatus` into `Ok(())` or an [`AudioError::Os`].
    ///
    /// # Errors
    /// Returns [`AudioError::Os`] for any non-zero status.
    pub fn check(status: i32, context: &'static str) -> Result<(), Self> {
        if status == 0 {
            Ok(())
        } else {
            Err(Self::Os { context, status })
        }
    }
}

/// Renders an `OSStatus` as a four-character code when printable (e.g. `'!obj'`).
fn fourcc(status: i32) -> String {
    let bytes = status.to_be_bytes();
    if bytes.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
        format!("'{}'", String::from_utf8_lossy(&bytes))
    } else {
        "-".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_check_and_fourcc() {
        assert!(AudioError::check(0, "x").is_ok());
        let err = AudioError::check(0x216F_626A, "AudioObjectGetPropertyData").unwrap_err();
        assert!(err.to_string().contains("'!obj'"), "{err}");
        assert_eq!(fourcc(-50), "-");
    }
}
