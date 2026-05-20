//! Cross-platform Bun binary discovery.
//!
//! This module is the single source of truth for locating the `bun` executable
//! on Linux, macOS, and Windows. It is designed to be robust, debuggable,
//! and to degrade gracefully (returning `None` when nothing can be found).
//!
//! The function is intentionally side-effect free except for filesystem
//! and PATH lookups. All decisions are logged via `tracing` so users and
//! developers can understand why a particular binary was (or was not) chosen.

use std::path::PathBuf;

use dirs::home_dir;
use tracing::{debug, warn};
use which::which;

/// Attempts to locate the `bun` binary using a well-defined search order
/// that works across Linux, macOS, and Windows.
///
/// Search order:
/// 1. `BUN_INSTALL` environment variable (official Bun installer location)
/// 2. `~/.bun/bin/bun` (default user installation)
/// 3. `bun` found in `PATH` (via the `which` crate)
/// 4. A small set of well-known platform-specific locations
///
/// Returns `Some(path)` if a usable binary is found, otherwise `None`.
/// Callers are expected to fall back to the Python harness when this returns `None`.
pub fn find_bun() -> Option<PathBuf> {
    debug!("Starting cross-platform Bun binary discovery");

    // 1. BUN_INSTALL (highest priority – official installer)
    if let Ok(bun_install) = std::env::var("BUN_INSTALL") {
        let bun_install = PathBuf::from(bun_install);

        // Most common layout: $BUN_INSTALL/bin/bun(.exe)
        let candidate = if cfg!(windows) {
            bun_install.join("bin").join("bun.exe")
        } else {
            bun_install.join("bin").join("bun")
        };

        if candidate.exists() {
            debug!("Found bun via BUN_INSTALL: {:?}", candidate);
            return Some(candidate);
        }

        // Some installations place it directly under BUN_INSTALL
        let direct = if cfg!(windows) {
            bun_install.join("bun.exe")
        } else {
            bun_install.join("bun")
        };

        if direct.exists() {
            debug!("Found bun directly under BUN_INSTALL: {:?}", direct);
            return Some(direct);
        }
    }

    // 2. Default user installation: ~/.bun/bin/bun(.exe)
    if let Some(home) = home_dir() {
        let candidate = if cfg!(windows) {
            home.join(".bun").join("bin").join("bun.exe")
        } else {
            home.join(".bun").join("bin").join("bun")
        };

        if candidate.exists() {
            debug!("Found bun in default user location: {:?}", candidate);
            return Some(candidate);
        }
    }

    // 3. PATH lookup (works on all platforms, respects .exe on Windows)
    if let Ok(path) = which("bun") {
        debug!("Found bun via PATH: {:?}", path);
        return Some(path);
    }

    // 4. Well-known platform-specific locations
    let candidates = platform_specific_candidates();

    for candidate in candidates {
        if candidate.exists() {
            debug!("Found bun in platform-specific location: {:?}", candidate);
            return Some(candidate);
        }
    }

    warn!("Could not locate the `bun` binary on this system");
    None
}

/// Returns a list of well-known installation locations for the current platform.
/// These are intentionally conservative and only include locations that have
/// historically been reliable.
fn platform_specific_candidates() -> Vec<PathBuf> {
    let mut locations = Vec::new();

    if cfg!(target_os = "windows") {
        // Windows common locations
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let profile = PathBuf::from(user_profile);

            // Official installer default
            locations.push(profile.join(".bun").join("bin").join("bun.exe"));

            // Scoop (very common on Windows)
            locations.push(profile.join("scoop").join("apps").join("bun").join("current").join("bun.exe"));
        }

        // Global scoop (less common but possible)
        locations.push(PathBuf::from("C:\\ProgramData\\scoop\\apps\\bun\\current\\bun.exe"));
    } else if cfg!(target_os = "macos") {
        // macOS common locations
        locations.push(PathBuf::from("/opt/homebrew/bin/bun"));      // Apple Silicon Homebrew (new)
        locations.push(PathBuf::from("/usr/local/bin/bun"));         // Intel Homebrew / manual
        locations.push(PathBuf::from("/usr/bin/bun"));

        // Homebrew on Apple Silicon can also live here in some setups
        if let Some(home) = home_dir() {
            locations.push(home.join("homebrew").join("bin").join("bun"));
        }
    } else {
        // Linux and other Unix-like systems
        locations.push(PathBuf::from("/usr/local/bin/bun"));
        locations.push(PathBuf::from("/usr/bin/bun"));
        locations.push(PathBuf::from("/usr/local/bun/bin/bun"));
        locations.push(PathBuf::from("/snap/bin/bun"));
    }

    locations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_bun_does_not_panic() {
        // This test only verifies that the function runs without crashing.
        // It may return None on CI machines that don't have Bun installed.
        let _ = find_bun();
    }
}