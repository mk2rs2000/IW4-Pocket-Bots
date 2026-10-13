//! Opens a web page for "Send logs" in IW4 Pocket options (iOS: Safari).
//!
//! Only links into this project's own GitHub repository are opened, so a message cannot
//! be used to send the player somewhere else. Other platforms just log the link.
use bevy::prelude::*;

/// The only place the options menu may send the player.
const ALLOWED_PREFIX: &str = "https://github.com/MarkusSela/IW4-Pocket/";

pub(crate) fn allowed(url: &str) -> bool {
    url.starts_with(ALLOWED_PREFIX) && !url.chars().any(|c| c.is_control() || c.is_whitespace())
}

pub(crate) fn register(app: &mut App) {
    // Idempotent: ConsolePlugin registers it too.
    app.add_message::<console::OpenExternalUrl>();
    #[cfg(target_os = "ios")]
    app.insert_non_send(ios::MainThread)
        .add_systems(Last, ios::open_requested);
    #[cfg(not(target_os = "ios"))]
    app.add_systems(Last, log_requested);
}

#[cfg(not(target_os = "ios"))]
fn log_requested(mut requests: MessageReader<console::OpenExternalUrl>) {
    for request in requests.read() {
        if allowed(&request.0) {
            diag::info!(Launch, "send logs: would open {}", request.0);
        }
    }
}

#[cfg(target_os = "ios")]
mod ios {
    use std::ffi::CString;

    use bevy::prelude::*;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    /// NonSend marker: UIKit is touched on the main thread only.
    pub(super) struct MainThread;

    pub(super) fn open_requested(
        _main: NonSend<MainThread>,
        mut requests: MessageReader<console::OpenExternalUrl>,
    ) {
        for request in requests.read() {
            if !super::allowed(&request.0) {
                diag::warn!(
                    Launch,
                    "send logs: refused a link outside the project repository"
                );
                continue;
            }
            let opened = open(&request.0);
            diag::info!(
                Launch,
                "send logs: {}",
                if opened {
                    "opened the GitHub page"
                } else {
                    "could not open the GitHub page"
                }
            );
        }
    }

    fn open(url: &str) -> bool {
        let Ok(c_url) = CString::new(url) else {
            return false;
        };
        // SAFETY: plain UIKit/Foundation messages with their documented signatures, on
        // the main thread (NonSend). `c_url` outlives the call that copies it.
        unsafe {
            let string: Option<Retained<AnyObject>> =
                msg_send![class!(NSString), stringWithUTF8String: c_url.as_ptr()];
            let Some(string) = string else {
                return false;
            };
            let nsurl: Option<Retained<AnyObject>> =
                msg_send![class!(NSURL), URLWithString: &*string];
            let Some(nsurl) = nsurl else {
                return false;
            };
            let app: Option<Retained<AnyObject>> =
                msg_send![class!(UIApplication), sharedApplication];
            let Some(app) = app else {
                return false;
            };
            let none: Option<&AnyObject> = None;
            let _: () = msg_send![
                &*app,
                openURL: &*nsurl,
                options: none,
                completionHandler: none
            ];
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::allowed;

    #[test]
    fn only_the_project_repository_is_allowed() {
        assert!(allowed(
            "https://github.com/MarkusSela/IW4-Pocket/issues/new?title=a&body=b%20c"
        ));
        assert!(!allowed("https://github.com/someone/else/issues/new"));
        assert!(!allowed("http://github.com/MarkusSela/IW4-Pocket/issues"));
        assert!(!allowed(
            "https://github.com.evil.example/MarkusSela/IW4-Pocket/"
        ));
        assert!(!allowed("https://github.com/MarkusSela/IW4-Pocket/\nx"));
        assert!(!allowed("https://github.com/MarkusSela/IW4-Pocket/ x"));
        assert!(!allowed("javascript:alert(1)"));
    }
}
