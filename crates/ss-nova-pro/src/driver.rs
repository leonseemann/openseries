//! Session with one base station over an abstract HID transport.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use ss_core::battery::BatteryReport;
use ss_core::transport::{HidTransport, TransportError};
use thiserror::Error;

use crate::protocol::{OPCODE_OFFSET, REPORT_ID_COMMAND, REPORT_LEN};
use crate::{Command, DecodeError, Display, Event, Settings, Status};

/// How long a query waits for its reply.
const QUERY_TIMEOUT: Duration = Duration::from_millis(500);

/// Errors of [`NovaProWireless`].
#[derive(Debug, Error)]
pub enum DriverError {
    /// Transport failure (I/O, disconnect).
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The device sent a malformed report.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// No reply to a query arrived in time.
    #[error("no reply to opcode {opcode:#04x} within {timeout:?}")]
    Timeout {
        /// Opcode of the unanswered query.
        opcode: u8,
        /// Time waited.
        timeout: Duration,
    },
}

/// A connected Arctis Nova Pro Wireless base station.
///
/// Unsolicited events that arrive while waiting for a query reply are buffered and
/// returned by [`NovaProWireless::next_event`] in arrival order.
pub struct NovaProWireless<T: HidTransport> {
    transport: T,
    pending: VecDeque<Event>,
}

impl<T: HidTransport> NovaProWireless<T> {
    /// Wraps an opened transport.
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            pending: VecDeque::new(),
        }
    }

    /// Sends a command without waiting for a reply.
    ///
    /// # Errors
    /// Returns [`DriverError::Transport`] if the write fails.
    pub fn send(&mut self, command: Command) -> Result<(), DriverError> {
        self.transport.write(&command.encode())?;
        Ok(())
    }

    /// Reads the general status block.
    ///
    /// # Errors
    /// Fails on transport errors, malformed replies or timeout.
    pub fn status(&mut self) -> Result<Status, DriverError> {
        let command = Command::QueryStatus;
        self.query(command, |event| match event {
            Event::Status(status) => Some(status),
            _ => None,
        })
    }

    /// Reads the audio settings block.
    ///
    /// # Errors
    /// Fails on transport errors, malformed replies or timeout.
    pub fn settings(&mut self) -> Result<Settings, DriverError> {
        self.query(Command::QuerySettings, |event| match event {
            Event::Settings(settings) => Some(settings),
            _ => None,
        })
    }

    /// Reads the display settings block.
    ///
    /// # Errors
    /// Fails on transport errors, malformed replies or timeout.
    pub fn display(&mut self) -> Result<Display, DriverError> {
        self.query(Command::QueryDisplay, |event| match event {
            Event::Display(display) => Some(display),
            _ => None,
        })
    }

    /// Reads the firmware versions (`None` if the reply has an unknown layout).
    ///
    /// # Errors
    /// Fails on transport errors, malformed replies or timeout.
    pub fn firmware(&mut self) -> Result<Option<crate::FirmwareVersions>, DriverError> {
        self.query(Command::QueryFirmware, |event| match event {
            Event::Firmware(versions) => Some(versions),
            _ => None,
        })
    }

    /// Reads the battery levels.
    ///
    /// # Errors
    /// Fails on transport errors, malformed replies or timeout.
    pub fn battery(&mut self) -> Result<BatteryReport, DriverError> {
        self.query(Command::QueryBattery, |event| match event {
            Event::Battery(report) => Some(report),
            _ => None,
        })
    }

    /// Sends a query and returns its undecoded reply (for protocol exploration).
    ///
    /// Unrelated reports received meanwhile are decoded and buffered as events.
    ///
    /// # Errors
    /// Fails on transport errors, malformed reports or timeout.
    pub fn query_raw(&mut self, command: Command) -> Result<[u8; REPORT_LEN], DriverError> {
        self.send(command)?;
        let deadline = Instant::now() + QUERY_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(DriverError::Timeout {
                    opcode: command.opcode(),
                    timeout: QUERY_TIMEOUT,
                });
            }
            let mut buf = [0u8; REPORT_LEN];
            let len = self.transport.read_timeout(&mut buf, remaining)?;
            if len == 0 {
                continue;
            }
            if buf[0] == REPORT_ID_COMMAND && buf[OPCODE_OFFSET] == command.opcode() {
                return Ok(buf);
            }
            self.pending.push_back(Event::decode(&buf[..len])?);
        }
    }

    /// Returns the next event, waiting at most `timeout`. `Ok(None)` on timeout.
    ///
    /// # Errors
    /// Fails on transport errors or malformed reports.
    pub fn next_event(&mut self, timeout: Duration) -> Result<Option<Event>, DriverError> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(Some(event));
        }
        self.read_event(timeout)
    }

    /// Gives back the transport (e.g. to close it explicitly).
    pub fn into_transport(self) -> T {
        self.transport
    }

    fn query<R>(
        &mut self,
        command: Command,
        mut extract: impl FnMut(Event) -> Option<R>,
    ) -> Result<R, DriverError> {
        self.send(command)?;
        let deadline = Instant::now() + QUERY_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(DriverError::Timeout {
                    opcode: command.opcode(),
                    timeout: QUERY_TIMEOUT,
                });
            }
            if let Some(event) = self.read_event(remaining)? {
                match extract(event) {
                    Some(result) => return Ok(result),
                    None => self.pending.push_back(event),
                }
            }
        }
    }

    fn read_event(&mut self, timeout: Duration) -> Result<Option<Event>, DriverError> {
        let mut buf = [0u8; REPORT_LEN];
        let len = self.transport.read_timeout(&mut buf, timeout)?;
        if len == 0 {
            return Ok(None);
        }
        Ok(Some(Event::decode(&buf[..len])?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss_core::headset::SidetoneLevel;

    /// In-memory transport: records writes, replays queued input reports.
    #[derive(Default)]
    struct FakeTransport {
        written: Vec<Vec<u8>>,
        inbox: VecDeque<Vec<u8>>,
    }

    impl HidTransport for FakeTransport {
        fn write(&mut self, report: &[u8]) -> Result<(), TransportError> {
            self.written.push(report.to_vec());
            Ok(())
        }

        fn read_timeout(&mut self, buf: &mut [u8], _: Duration) -> Result<usize, TransportError> {
            Ok(self.inbox.pop_front().map_or(0, |r| {
                buf[..r.len()].copy_from_slice(&r);
                r.len()
            }))
        }
    }

    #[test]
    fn query_buffers_unrelated_events() {
        let mut fake = FakeTransport::default();
        fake.inbox.push_back(vec![0x07, 0x39, 1]); // sidetone event arrives first
        fake.inbox.push_back(vec![0x06, 0xB7, 8, 0, 8]); // battery reply
        let mut dev = NovaProWireless::new(fake);

        let battery = dev.battery().unwrap();
        assert_eq!(battery.headset.percent(), 100);
        assert_eq!(
            dev.next_event(Duration::ZERO).unwrap(),
            Some(Event::Sidetone(SidetoneLevel::Low))
        );
        assert_eq!(dev.next_event(Duration::ZERO).unwrap(), None);
        assert_eq!(dev.into_transport().written[0][..2], [0x06, 0xB7]);
    }

    #[test]
    fn send_writes_encoded_command() {
        let mut dev = NovaProWireless::new(FakeTransport::default());
        dev.send(Command::Save).unwrap();
        let written = dev.into_transport().written;
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].len(), REPORT_LEN);
        assert_eq!(written[0][..2], [0x06, 0x09]);
    }
}
