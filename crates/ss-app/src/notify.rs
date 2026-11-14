//! Wake-up hook for the UI: services call it after changing shared state.

use std::sync::Arc;

/// Called by services after they changed a state snapshot (e.g. to request a UI repaint).
pub type Notifier = Arc<dyn Fn() + Send + Sync>;
