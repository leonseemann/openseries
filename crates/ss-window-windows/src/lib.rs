//! Win32 window helpers for `ss-app` (`docs/architecture/app.md` §6).
//!
//! Why this exists: eframe repaints through `winit`'s `request_redraw`, which Windows delivers as
//! `WM_PAINT` — and Windows sends no `WM_PAINT` to a hidden window. Once the window is hidden
//! (closing it keeps the app in the tray), eframe stops running the UI frame, so a queued
//! `ViewportCommand::Visible(true)` is never applied. **Observed** 2026-11-14 on Windows 11:
//! "Open StealSeries…" in the tray did nothing. [`show_window`] shows the window directly instead.
//! On other platforms the functions are no-ops.

// FFI adapter crate: `unsafe` is allowed here only (AGENTS.md). Every block has a SAFETY note.
#![allow(unsafe_code)]

use std::num::NonZeroIsize;

/// Shows, restores and focuses the top-level window `hwnd` (a raw Win32 `HWND`).
///
/// Safe to call from any thread and with a handle of a window that no longer exists: Win32 then
/// just reports an invalid handle, which is ignored.
#[cfg(windows)]
pub fn show_window(hwnd: NonZeroIsize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsIconic, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow,
    };
    let hwnd = hwnd.get() as windows_sys::Win32::Foundation::HWND;
    // SAFETY: plain Win32 calls on a window handle; they validate the handle themselves and have
    // no memory-safety preconditions beyond receiving a pointer-sized value.
    unsafe {
        ShowWindow(hwnd, SW_SHOW);
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd);
    }
}

/// No-op outside Windows.
#[cfg(not(windows))]
pub fn show_window(_hwnd: NonZeroIsize) {}
