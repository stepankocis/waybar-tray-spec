mod error;
mod log;
mod paths;
mod tray;

use std::{
    fs::{File, OpenOptions},
    os::unix::fs::OpenOptionsExt,
    process::ExitCode,
};

use ksni::TrayMethods;
use tokio::{
    signal::unix::{SignalKind, signal},
    sync::mpsc,
};

use error::Error;
use log::log;
use paths::{absolute_path, xdg_home};
use tray::{Action, AppTray};

pub const APP: &str = env!("CARGO_PKG_NAME");
const USAGE: &str = concat!(
    "Usage: ",
    env!("CARGO_PKG_NAME"),
    " [--verbose | -v]\n\n",
    env!("CARGO_PKG_DESCRIPTION"),
);
const TRAY_STOPPED: &str = "tray service stopped unexpectedly";

// No runtime and no D-Bus before the arguments are parsed.
fn main() -> ExitCode {
    let mut verbose = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-v" | "--verbose" => verbose = true,
            _ => {
                log(format_args!("unknown argument: {arg}"));
                return ExitCode::from(2);
            }
        }
    }
    log::set_verbose(verbose);

    // One thread serves the tray's single D-Bus connection.
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(Error::from)
        .and_then(|runtime| runtime.block_on(run()));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log(error);
            ExitCode::FAILURE
        }
    }
}

/// Returns `None` when another instance holds the lock.
fn lock_instance() -> Result<Option<File>, Error> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR");
    let dir = if let Some(path) = absolute_path(runtime.as_deref()) {
        path.to_path_buf()
    } else {
        xdg_home("XDG_STATE_HOME", ".local/state")
            .map_err(|error| format!("XDG_RUNTIME_DIR lacks an absolute path, and {error}"))?
            .join(APP)
    };
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("could not create {}: {error}", dir.display()))?;
    let path = dir.join(format!("{APP}.lock"));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)
        .map_err(|error| format!("could not open {}: {error}", path.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(std::fs::TryLockError::WouldBlock) => {
            log("already running");
            Ok(None)
        }
        Err(std::fs::TryLockError::Error(error)) => {
            Err(format!("could not lock {}: {error}", path.display()).into())
        }
    }
}

async fn run() -> Result<(), Error> {
    // The lock comes first, so a second instance exits before it touches D-Bus.
    let Some(_instance_lock) = lock_instance()? else {
        return Ok(());
    };
    let mut terminate = signal(SignalKind::terminate())
        .map_err(|error| format!("could not install SIGTERM handler: {error}"))?;
    let mut interrupt = signal(SignalKind::interrupt())
        .map_err(|error| format!("could not install SIGINT handler: {error}"))?;
    let shutdown = async {
        tokio::select! {
            _ = terminate.recv() => {},
            _ = interrupt.recv() => {},
        }
    };
    tokio::pin!(shutdown);

    let (actions, mut events) = mpsc::unbounded_channel();
    // assume_sni_available: start before Waybar; ksni registers again
    // whenever the StatusNotifierWatcher appears or restarts.
    let handle = tokio::select! {
        biased;
        () = &mut shutdown => return Ok(()),
        result = AppTray::new(actions).assume_sni_available(true).spawn() => result?,
    };
    log::debug("tray registered");

    // Add the app's event sources (D-Bus signal streams, child exit, timers
    // until the next visible change) as further branches here.
    let result = tokio::select! {
        biased;
        () = &mut shutdown => Ok(()),
        action = events.recv() => match action {
            Some(Action::Quit) => Ok(()),
            // The tray owns the only sender, so None means the tray service died.
            None => Err(TRAY_STOPPED.into()),
        },
    };
    handle.shutdown().await;
    result
}
