//! Configuration of the StealSeries app (`docs/architecture/app.md` §3).
//!
//! * [`config`] — versioned serde DTOs of `config.json`, with defaults for every field.
//! * [`convert`] — mapping to the serde-free domain types (`ss-core`, `ss-dsp`).
//! * [`file`] — atomic load/save in the per-user config directory.

pub mod config;
pub mod convert;
pub mod file;

pub use config::{CONFIG_VERSION, Config};
pub use convert::ConvertError;
pub use file::{Loaded, Store, StoreError};
