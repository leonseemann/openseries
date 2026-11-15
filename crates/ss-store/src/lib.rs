//! Configuration of the OpenSeries app (`docs/architecture/app.md` §3).
//!
//! * [`config`] — versioned serde DTOs of `config.json`, with defaults for every field.
//! * [`convert`] — mapping to the serde-free domain types (`ss-core`, `ss-dsp`).
//! * [`file`] — atomic load/save in the per-user config directory.
//! * [`presets`] — named sound presets and their export/import format.

pub mod config;
pub mod convert;
pub mod file;
pub mod presets;

pub use config::{CONFIG_VERSION, Config};
pub use convert::ConvertError;
pub use file::{Loaded, Store, StoreError};
