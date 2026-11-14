//! Non-real-time side of the control queue.

use rtrb::Producer;

use crate::Command;

/// The command queue is full; the audio callback has not drained it yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineBusy;

impl std::fmt::Display for EngineBusy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("audio engine command queue is full")
    }
}

impl std::error::Error for EngineBusy {}

/// Sends [`Command`]s to a running [`crate::MixEngine`]. Never blocks.
pub struct EngineController {
    producer: Producer<Command>,
}

impl EngineController {
    pub(crate) fn new(producer: Producer<Command>) -> Self {
        Self { producer }
    }

    /// Queues a command for the next audio callback.
    ///
    /// # Errors
    /// [`EngineBusy`] if the queue is full (e.g. the stream is stopped).
    pub fn send(&mut self, command: Command) -> Result<(), EngineBusy> {
        self.producer.push(command).map_err(|_| EngineBusy)
    }
}
