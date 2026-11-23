use std::{
    fmt::Display,
    sync::atomic::{AtomicBool, Ordering},
};

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(verbose: bool) {
    VERBOSE.store(verbose, Ordering::Relaxed);
}

/// Errors and state changes the user should see, always on stderr.
pub fn log(message: impl Display) {
    eprintln!("{}: {message}", crate::APP);
}

/// Diagnostics, only with `--verbose`.
pub fn debug(message: impl Display) {
    if VERBOSE.load(Ordering::Relaxed) {
        log(message);
    }
}
