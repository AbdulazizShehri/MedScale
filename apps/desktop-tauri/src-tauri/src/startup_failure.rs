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
pub fn message(_detail: &str) -> String {
    // Startup failures can include absolute home paths, filenames and panic
    // payloads from OS/WebView APIs. Both stderr and native error dialogs use
    // this message, so never serialize the raw diagnostic at this boundary.
    format!(
        "MedScale could not open its window.\n\n\
         On Windows this usually means the Microsoft Edge WebView2 Runtime is missing \
         or damaged. Install or repair the WebView2 Evergreen Runtime from Microsoft, \
         then start MedScale again. MedScale does not download it for you.\n\n\
         No workspace was opened by this startup attempt. An application data \
         directory may have been created.\n\n\
         Diagnostic details withheld to protect local paths and data. \
         Exit code: {STARTUP_FAILURE_EXIT_CODE}"
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
            report_and_exit("startup panic");
        }
        previous(info);
    }));
}

/// Reports the failure to the user and exits.
pub fn report_and_exit(detail: &str) -> ! {
    let text = message(detail);
    eprintln!("{TITLE}: {text}");
    native::show(TITLE, &text);
    std::process::exit(STARTUP_FAILURE_EXIT_CODE);
}

/// Exit code when another MedScale process already uses this data
/// directory (Spec 108 step 3). Distinct from a failed startup (2).
pub const ALREADY_RUNNING_EXIT_CODE: i32 = 3;

pub const ALREADY_RUNNING_TITLE: &str = "MedScale is already running";

pub const ALREADY_RUNNING_TEXT: &str = "MedScale is already open. Switch to the open MedScale window.\n\n\
     If no MedScale window is visible, it may still be closing. Wait a moment and try again, \
     or end MedScale in Task Manager.";

pub const INSTANCE_UNCONFIRMED_TEXT: &str = "MedScale could not confirm that it is the only running copy, \
     so it did not open. This protects your workspace from being changed by two copies at once.\n\n\
     Check that MedScale's data folder is on a local drive you can write to, then try again.";

/// Test-only: the native E2E launches a second copy deliberately and must
/// not leave a blocking dialog on the runner. Release builds always show it.
fn dialog_suppressed() -> bool {
    cfg!(feature = "e2e-webdriver")
        && std::env::var_os("MEDSCALE_E2E_HEADLESS_SECOND_INSTANCE").is_some_and(|v| v == "1")
}

/// Another process holds the instance lock: tell the user and exit 3
/// without opening a window or touching any workspace file.
pub fn exit_already_running() -> ! {
    eprintln!("{ALREADY_RUNNING_TITLE}");
    if !dialog_suppressed() {
        native::show_info(ALREADY_RUNNING_TITLE, ALREADY_RUNNING_TEXT);
    }
    std::process::exit(ALREADY_RUNNING_EXIT_CODE);
}

/// Exclusivity could not be confirmed: fail closed with exit 2.
pub fn exit_instance_unconfirmed() -> ! {
    eprintln!("{TITLE}: {INSTANCE_UNCONFIRMED_TEXT}");
    native::show(TITLE, INSTANCE_UNCONFIRMED_TEXT);
    std::process::exit(STARTUP_FAILURE_EXIT_CODE);
}

#[cfg(windows)]
mod native {
    //! Workspace `unsafe_code` is `deny`; this module allows one confined call
    //! to `MessageBoxW` in user32 (already linked by every Windows GUI app), so
    //! no new dependency is needed.
    #![allow(unsafe_code)]

    const MB_ICONERROR: u32 = 0x0000_0010;
    const MB_ICONINFORMATION: u32 = 0x0000_0040;
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
        message_box(title, text, MB_ICONERROR);
    }

    pub fn show_info(title: &str, text: &str) {
        message_box(title, text, MB_ICONINFORMATION);
    }

    fn message_box(title: &str, text: &str, icon: u32) {
        let (title, text) = (wide(title), wide(text));
        // SAFETY: both buffers are NUL-terminated UTF-16 that outlive the call;
        // a null owner window is permitted; the call is synchronous. Style = OK
        // button (0) + icon + foreground.
        unsafe {
            MessageBoxW(
                core::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                icon | MB_SETFOREGROUND,
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
    pub fn show_info(_title: &str, _text: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_messages_are_actionable_and_path_free() {
        assert_ne!(ALREADY_RUNNING_EXIT_CODE, STARTUP_FAILURE_EXIT_CODE);
        assert!(ALREADY_RUNNING_TEXT.contains("Switch to the open MedScale window"));
        assert!(ALREADY_RUNNING_TEXT.contains("Task Manager"));
        assert!(INSTANCE_UNCONFIRMED_TEXT.contains("did not open"));
        for text in [ALREADY_RUNNING_TEXT, INSTANCE_UNCONFIRMED_TEXT] {
            assert!(!text.contains(":\\") && !text.contains('/'), "no paths");
            assert!(!text.contains("WebView2"), "not the WebView2 message");
        }
    }

    #[test]
    fn message_names_webview2_and_refuses_download() {
        let m = message("WebView2 error 0x80070002");
        assert!(m.contains("WebView2 Evergreen Runtime"));
        assert!(m.contains("does not download"));
        assert!(m.contains("An application data directory may have been created"));
        assert!(m.contains("Diagnostic details withheld"));
        assert!(m.ends_with("Exit code: 2"));
        assert!(!m.contains("0x80070002"));
    }

    #[test]
    fn empty_detail_is_explicit() {
        assert!(message("  ").contains("Diagnostic details withheld"));
    }

    #[test]
    fn startup_error_never_prints_private_path_or_secret() {
        let raw = "panic at /Users/example/PrivateNotes/synthetic-secret-token";
        let shown = message(raw);
        assert_eq!(shown, message("some different OS error"));
        for fragment in [
            "/Users/",
            "PrivateNotes",
            "synthetic-secret-token",
            "panic at",
        ] {
            assert!(!shown.contains(fragment));
        }
        assert!(shown.contains("WebView2 Evergreen Runtime"));
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
