use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use crate::error::Error;

/// Empty and relative values count as unset.
pub fn absolute_path(value: Option<&OsStr>) -> Option<&Path> {
    value.map(Path::new).filter(|path| path.is_absolute())
}

fn xdg_base_dir(value: Option<&OsStr>, home: Option<&OsStr>, default: &str) -> Option<PathBuf> {
    absolute_path(value)
        .map(Path::to_path_buf)
        .or_else(|| absolute_path(home).map(|home| home.join(default)))
}

/// `xdg_home("XDG_STATE_HOME", ".local/state")` and so on.
pub fn xdg_home(variable: &str, default: &str) -> Result<PathBuf, Error> {
    xdg_base_dir(
        std::env::var_os(variable).as_deref(),
        std::env::var_os("HOME").as_deref(),
        default,
    )
    .ok_or_else(|| format!("neither {variable} nor HOME contains an absolute path").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_base_dir_prefers_an_absolute_override() {
        assert_eq!(
            xdg_base_dir(Some(OsStr::new("/override")), None, ".cache"),
            Some(PathBuf::from("/override"))
        );
    }

    #[test]
    fn xdg_base_dir_uses_home_for_missing_empty_or_relative_overrides() {
        for value in [None, Some(OsStr::new("")), Some(OsStr::new("relative"))] {
            assert_eq!(
                xdg_base_dir(value, Some(OsStr::new("/home/user")), ".local/state"),
                Some(PathBuf::from("/home/user/.local/state"))
            );
        }
    }

    #[test]
    fn xdg_base_dir_requires_an_absolute_home_when_falling_back() {
        for home in [None, Some(OsStr::new("")), Some(OsStr::new("relative"))] {
            assert_eq!(xdg_base_dir(None, home, ".cache"), None);
        }
    }
}
