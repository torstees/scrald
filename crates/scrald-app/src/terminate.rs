//! Quitting from outside Scrald on macOS (DESIGN.md §11.1, #197): Quit in
//! the Dock's menu, logging out, and shutting down. They send a quit Apple
//! Event, which AppKit turns into `[NSApp terminate:]`. AppKit then asks the
//! app delegate's `applicationShouldTerminate:` whether it may quit, but tao
//! (Tauri's windowing layer) doesn't implement that method, so the app ended
//! at once and no window could ask about unsaved changes.
//!
//! Scrald adds the method to tao's delegate class at startup. It answers
//! "later" and asks every window to close, as Quit Scrald does
//! (`menu::quit`). When the last window has closed, it tells AppKit to go
//! ahead (`finish`); when a window's unsaved-changes question is cancelled,
//! it tells AppKit not to quit (`cancel`), so a logout is cancelled rather
//! than left waiting.

// The only unsafe code in Scrald (AGENTS.md): two calls into the Objective-C
// runtime below, each with the promise it makes written beside it.
#![allow(unsafe_code)]

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use objc2::MainThreadMarker;
use objc2::ffi::NSUInteger;
use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
use objc2::sel;
use objc2_app_kit::{NSApplication, NSApplicationTerminateReply};
use tauri::{AppHandle, Manager};

/// The app, for the delegate method, which AppKit calls without any context.
static APP: OnceLock<AppHandle> = OnceLock::new();

/// Whether AppKit is waiting for the answer to a quit (we said "later").
static WAITING: AtomicBool = AtomicBool::new(false);

/// The name tao registers its app delegate class under.
const TAO_DELEGATE_CLASS: &std::ffi::CStr = c"TaoAppDelegateParent";

/// Adds `applicationShouldTerminate:` to tao's app delegate class. Call once,
/// after the event loop exists (tao creates the delegate with it). If the
/// class isn't found or already has the method (a newer tao), Scrald logs it
/// and leaves quitting as it was.
pub fn install(app: &AppHandle) {
    let _ = APP.set(app.clone());
    let Some(class) = AnyClass::get(TAO_DELEGATE_CLASS) else {
        tracing::warn!("tao's app delegate class not found; quitting from the Dock won't ask");
        return;
    };
    // Rust note: `transmute` reinterprets the bits of a value as another
    // type, like `reinterpret_cast` in C++. The Objective-C runtime stores
    // every method as the same untyped function pointer (`Imp`); the type
    // encoding below tells it the real signature.
    let imp: Imp = unsafe {
        std::mem::transmute::<
            extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject) -> NSUInteger,
            Imp,
        >(should_terminate)
    };
    // "Q@:@": returns an unsigned long (NSApplicationTerminateReply), takes
    // self, the selector, and the sender.
    // Rust note: `unsafe` marks code whose safety the compiler can't check.
    // Here: the class pointer is a registered class, and the function has
    // the signature the encoding says.
    let added = unsafe {
        objc2::ffi::class_addMethod(
            class as *const AnyClass as *mut AnyClass,
            sel!(applicationShouldTerminate:),
            imp,
            c"Q@:@".as_ptr(),
        )
    };
    if added.as_bool() {
        tracing::debug!("quitting from the Dock, logout, and shutdown now asks each window");
    } else {
        tracing::warn!("tao already answers applicationShouldTerminate:; left as it is");
    }
}

/// AppKit's question before quitting. With no windows, quit now; otherwise
/// ask each window to close and answer later.
extern "C-unwind" fn should_terminate(
    _this: *mut AnyObject,
    _cmd: Sel,
    _sender: *mut AnyObject,
) -> NSUInteger {
    let Some(app) = APP.get() else {
        return NSApplicationTerminateReply::TerminateNow.0;
    };
    if app.webview_windows().is_empty() {
        return NSApplicationTerminateReply::TerminateNow.0;
    }
    tracing::info!("quit requested by the system (Dock, logout, or shutdown)");
    WAITING.store(true, Ordering::SeqCst);
    crate::menu::quit(app);
    NSApplicationTerminateReply::TerminateLater.0
}

/// The last window has closed: if AppKit is waiting, let it quit.
pub fn finish() {
    reply(true);
}

/// A window's unsaved-changes question was cancelled (or its save failed):
/// if AppKit is waiting, the quit (or logout) is called off.
pub fn cancel(app: &AppHandle) {
    if !WAITING.load(Ordering::SeqCst) {
        return;
    }
    // Commands run on a worker thread; AppKit must be answered on the main one.
    if let Err(error) = app.run_on_main_thread(|| reply(false)) {
        tracing::warn!(error = %error, "could not call off the quit");
    }
}

/// Answers AppKit, once per quit. Must run on the main thread.
fn reply(quit: bool) {
    if !WAITING.swap(false, Ordering::SeqCst) {
        return;
    }
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("not on the main thread; can't answer the quit");
        return;
    };
    tracing::info!(quit, "answering the system's quit");
    NSApplication::sharedApplication(mtm).replyToApplicationShouldTerminate(quit);
}
