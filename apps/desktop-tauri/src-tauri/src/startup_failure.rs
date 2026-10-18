//! Controlled startup failure (Spec 101 finding F101-02).
//!
//! When the window or its WebView cannot be created (on Windows most often a
//! missing or broken Microsoft Edge WebView2 Runtime), MedScale shows a native
//! message and exits with a defined code instead of panicking. It never
//! downloads or installs anything.
//!
//! Two paths reach it: an error returned while building the app, and a panic
//! raised before the main page has finished loading for the first time (on a
//! hosted Windows runner WebView2 initialization panics inside the event loop,
//! run 37799770502). After the first finished page load, panics keep the
//! default behavior.

use std::sync::atomic::{AtomicBool, Ordering};

static READY: AtomicBool = AtomicBool::new(false);

/// Process exit code for a failed startup (a panic would exit with 101).
pub const STARTUP_FAILURE_EXIT_CODE: i32 = 2;

pub const TITLE: &str = "MedScale could not start";

/// User-facing text. `detail` is the runtime's own error, kept for support.
pub fn message(detail: &str) -> String {
    let detail = detail.trim();
    let detail = if detail.is_empty() {
        "no further detail"
    } else {
        detail
    };
    format!(
        "MedScale could not open its window.\n\n\
         On Windows this usually means the Microsoft Edge WebView2 Runtime is missing \
         or damaged. Install or repair the WebView2 Evergreen Runtime from Microsoft, \
         then start MedScale again. MedScale does not download it for you.\n\n\
         No workspace was opened and nothing was changed.\n\n\
         Detail: {detail}"
    )
}

/// Called on the main window's first finished page load.
pub fn mark_ready() {
    READY.store(true, Ordering::SeqCst);
}

/// True until the first finished page load.
pub fn starting() -> bool {
    !READY.load(Ordering::SeqCst)
}

/// Installs a panic hook that reports startup-phase panics and exits with
/// [`STARTUP_FAILURE_EXIT_CODE`]; later panics use the previous hook.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if starting() {
            report_and_exit(&panic_detail(info));
        }
        previous(info);
    }));
}

fn panic_detail(info: &std::panic::PanicHookInfo<'_>) -> String {
    let payload = info
        .payload()
        .downcast_ref::<&str>()
        .map(ToString::to_string)
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "startup panic".to_string());
    match info.location() {
        Some(at) => format!("{payload} ({}:{})", at.file(), at.line()),
        None => payload,
    }
}

/// Reports the failure to the user and exits.
pub fn report_and_exit(detail: &str) -> ! {
    let text = message(detail);
    eprintln!("{TITLE}: {text}");
    native::show(TITLE, &text);
    std::process::exit(STARTUP_FAILURE_EXIT_CODE);
}

#[cfg(windows)]
mod native {
    //! Workspace `unsafe_code` is `deny`; this module allows one confined call
    //! to `MessageBoxW` in user32 (already linked by every Windows GUI app), so
    //! no new dependency is needed.
    #![allow(unsafe_code)]

    const MB_ICONERROR: u32 = 0x0000_0010;
    const MB_SETFOREGROUND: u32 = 0x0001_0000;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(
            hwnd: *mut core::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            utype: u32,
        ) -> i32;
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16()
            .filter(|&c| c != 0)
            .chain(std::iter::once(0))
            .collect()
    }

    pub fn show(title: &str, text: &str) {
        let (title, text) = (wide(title), wide(text));
        // SAFETY: both buffers are NUL-terminated UTF-16 that outlive the call;
        // a null owner window is permitted; the call is synchronous. Style = OK
        // button (0) + error icon + foreground.
        unsafe {
            MessageBoxW(
                core::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_ICONERROR | MB_SETFOREGROUND,
            );
        }
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn wide_strings_are_nul_terminated_without_interior_nul() {
            let w = super::wide("a\0b");
            assert_eq!(w, vec![u16::from(b'a'), u16::from(b'b'), 0]);
        }
    }
}

#[cfg(not(windows))]
mod native {
    // macOS and Linux: the message is written to stderr; no new dependency.
    pub fn show(_title: &str, _text: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_names_webview2_and_refuses_download() {
        let m = message("WebView2 error 0x80070002");
        assert!(m.contains("WebView2 Evergreen Runtime"));
        assert!(m.contains("does not download"));
        assert!(m.contains("nothing was changed"));
        assert!(m.ends_with("Detail: WebView2 error 0x80070002"));
    }

    #[test]
    fn empty_detail_is_explicit() {
        assert!(message("  ").ends_with("Detail: no further detail"));
    }

    #[test]
    fn startup_phase_ends_on_mark_ready() {
        assert!(starting());
        mark_ready();
        assert!(!starting());
    }

    #[test]
    fn exit_code_is_not_the_panic_code() {
        assert_ne!(STARTUP_FAILURE_EXIT_CODE, 101);
        assert_ne!(STARTUP_FAILURE_EXIT_CODE, 0);
    }
}
