//! One StealSeries at a time (`docs/architecture/app.md` §9): a newly started app asks the running
//! one to quit (so it restores app volumes and saves its config), then takes over. Covers a
//! second start from the Start menu, a development build next to the installed app, and the
//! installer starting the new version after an update.
//!
//! The running instance listens on a random localhost port and records `{pid, port, token}` in
//! `instance.json` next to `config.json`. A new instance connects, checks the greeting (so a
//! stale port now used by another program is never mistaken for StealSeries), sends
//! `quit <token>` and waits until the connection closes, i.e. the old process has exited. Only a
//! verified instance that does not exit in time is terminated.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher as _, Hasher as _};
use std::io::{BufRead as _, BufReader, ErrorKind, Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Weak;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::core::AppCore;

/// Record of the running instance, next to `config.json`.
const FILE: &str = "instance.json";
/// Greeting the running instance sends to every connection.
const GREETING: &str = "stealseries";
/// Connecting to a stale port fails fast on localhost.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
/// How long the old instance gets for a clean shutdown before it is terminated.
const QUIT_TIMEOUT: Duration = Duration::from_secs(10);
/// Grace period after the old process ended, so the OS releases the HID device.
const RELEASE_DELAY: Duration = Duration::from_millis(300);
/// Longest accepted request line.
const MAX_LINE: u64 = 128;

#[derive(Debug, Serialize, Deserialize)]
struct Record {
    pid: u32,
    port: u16,
    token: String,
}

/// What happened to a previously running instance.
#[derive(Debug, PartialEq, Eq)]
pub enum Takeover {
    /// None was running.
    NoneRunning,
    /// It quit cleanly.
    Replaced,
    /// It did not quit in time and was terminated.
    Terminated,
}

/// Makes a running StealSeries (same config directory) quit before this one starts.
#[must_use]
pub fn replace_running(config_dir: &Path) -> Takeover {
    let Some(record) = read(config_dir).filter(|r| r.pid != std::process::id()) else {
        return Takeover::NoneRunning;
    };
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, record.port));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) else {
        return Takeover::NoneRunning; // stale record
    };
    if !greets_with(&mut stream, &record.token) {
        return Takeover::NoneRunning; // the port belongs to something else now
    }
    let _ = stream.set_read_timeout(Some(QUIT_TIMEOUT));
    let _ = writeln!(stream, "quit {}", record.token);
    let mut byte = [0_u8; 1];
    let exited = match stream.read(&mut byte) {
        Ok(0) => true,
        Err(error) => matches!(
            error.kind(),
            ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::BrokenPipe
        ),
        Ok(_) => false,
    };
    let outcome = if exited {
        Takeover::Replaced
    } else {
        crate::platform::terminate_process(record.pid);
        Takeover::Terminated
    };
    std::thread::sleep(RELEASE_DELAY);
    outcome
}

/// Registers this process as the running instance and quits it (cleanly, through
/// [`crate::quit`]) when a newer start asks for it.
///
/// # Errors
/// If the listener cannot be created or the record not written.
pub fn listen(config_dir: &Path, core: Weak<AppCore>) -> std::io::Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let token = random_token();
    let record = Record {
        pid: std::process::id(),
        port: listener.local_addr()?.port(),
        token: token.clone(),
    };
    std::fs::create_dir_all(config_dir)?;
    let json = serde_json::to_string(&record).map_err(std::io::Error::other)?;
    std::fs::write(config_dir.join(FILE), json)?;
    std::thread::Builder::new()
        .name("instance".to_owned())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                // `stream` stays open until the process exits: the new instance waits for it
                // to close.
                if is_quit_request(&stream, &token) {
                    match core.upgrade() {
                        Some(core) => crate::quit(&core),
                        None => std::process::exit(0),
                    }
                }
            }
        })?;
    Ok(())
}

/// Removes the record if it is ours (on quit).
pub fn unregister(config_dir: &Path) {
    if read(config_dir).is_some_and(|r| r.pid == std::process::id()) {
        let _ = std::fs::remove_file(config_dir.join(FILE));
    }
}

/// The config directory that scopes "one instance" (development runs with another `APPDATA`
/// or `HOME` get their own).
#[must_use]
pub fn scope(config_file: &Path) -> PathBuf {
    config_file
        .parent()
        .map_or_else(std::env::temp_dir, Path::to_path_buf)
}

fn read(config_dir: &Path) -> Option<Record> {
    let text = std::fs::read_to_string(config_dir.join(FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Reads the greeting line and checks it carries `token`.
fn greets_with(stream: &mut TcpStream, token: &str) -> bool {
    let _ = stream.set_read_timeout(Some(CONNECT_TIMEOUT));
    read_line(stream).is_some_and(|line| line == format!("{GREETING} {token}"))
}

/// Greets the caller and reads one request line.
fn is_quit_request(mut stream: &TcpStream, token: &str) -> bool {
    let _ = stream.set_read_timeout(Some(CONNECT_TIMEOUT));
    if writeln!(stream, "{GREETING} {token}").is_err() {
        return false;
    }
    read_line(stream).is_some_and(|line| line == format!("quit {token}"))
}

fn read_line(stream: &TcpStream) -> Option<String> {
    let mut line = String::new();
    // One byte at a time (no read-ahead buffer), so nothing after the line is consumed.
    BufReader::with_capacity(1, stream.take(MAX_LINE))
        .read_line(&mut line)
        .ok()?;
    Some(line.trim().to_owned())
}

/// A per-start random token (std's `RandomState` is randomly keyed per process).
fn random_token() -> String {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u32(std::process::id());
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_running_without_a_record() {
        let dir = std::env::temp_dir().join(format!("ss-instance-none-{}", random_token()));
        assert_eq!(replace_running(&dir), Takeover::NoneRunning);
    }

    #[test]
    fn stale_record_and_foreign_port_are_ignored() {
        let dir = std::env::temp_dir().join(format!("ss-instance-stale-{}", random_token()));
        std::fs::create_dir_all(&dir).unwrap();
        // A port that belongs to something that is not StealSeries: it greets differently.
        let foreign = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = foreign.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for mut stream in foreign.incoming().flatten() {
                let _ = writeln!(stream, "HTTP/1.1 400 Bad Request");
            }
        });
        let record = Record {
            pid: u32::MAX, // never terminated: the greeting check fails first
            port,
            token: "abc".to_owned(),
        };
        std::fs::write(dir.join(FILE), serde_json::to_string(&record).unwrap()).unwrap();
        assert_eq!(replace_running(&dir), Takeover::NoneRunning);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn quit_request_needs_the_token() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let mut results = Vec::new();
            for stream in listener.incoming().take(2).flatten() {
                results.push(is_quit_request(&stream, "secret"));
            }
            results
        });
        for request in ["quit wrong", "quit secret"] {
            let mut client = TcpStream::connect(address).unwrap();
            assert!(greets_with(&mut client, "secret"));
            writeln!(client, "{request}").unwrap();
        }
        assert_eq!(server.join().unwrap(), vec![false, true]);
    }
}
