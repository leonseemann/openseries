//! [`AudioBackend`] for platforms without an audio adapter yet (Windows until M4).

use super::plan::{DeviceSet, MicPlan, MixPlan, ProcessInfo};
use super::supervisor::{AudioBackend, MicHandle, MixHandle};

/// Explains why AudioMix and the virtual mic are missing.
const NOT_YET: &str = "not available on this operating system yet (Windows support: milestone M4)";

/// A backend that offers nothing and explains why.
pub struct UnsupportedBackend;

impl AudioBackend for UnsupportedBackend {
    fn processes(&mut self) -> Result<Vec<ProcessInfo>, String> {
        Ok(Vec::new())
    }

    fn devices(&mut self) -> DeviceSet {
        DeviceSet::default()
    }

    fn start_mix(&mut self, _plan: &MixPlan) -> Result<Box<dyn MixHandle>, String> {
        Err(format!("AudioMix is {NOT_YET}"))
    }

    fn start_mic(&mut self, _plan: &MicPlan) -> Result<Box<dyn MicHandle>, String> {
        Err(format!("The virtual microphone is {NOT_YET}"))
    }
}
