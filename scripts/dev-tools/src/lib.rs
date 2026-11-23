// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! What the developer tools of this workspace share: where the workspace is,
//! how a step is run, the platforms md2html is built for and how each is built
//! from the machine at hand, and the checksum of an archive.
//!
//! The tools themselves are the two binaries beside this file:
//! `install-bins` (`just use-local`) and `release-local` (`just
//! release-local`).

use std::path::{Path, PathBuf};
use std::process::Command;

mod sha256;

pub use sha256::sha256_hex;

/// The version every crate of the workspace carries, md2html's own.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One platform md2html is released for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    /// The name in the archive's file name (`macos-arm64`).
    pub name: &'static str,
    /// The Rust target it is built for.
    pub target: &'static str,
    /// The operating system, as `std::env::consts::OS` spells it.
    pub os: &'static str,
    /// The processor, as `std::env::consts::ARCH` spells it.
    pub arch: &'static str,
}

impl Platform {
    /// The binary's file name on this platform.
    #[must_use]
    pub fn binary(&self) -> &'static str {
        if self.os == "windows" {
            "md2html.exe"
        } else {
            "md2html"
        }
    }

    /// The kind of executable file this platform runs, as [`executable_kind`]
    /// names it.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self.os {
            "macos" => "Mach-O",
            "windows" => "PE",
            _ => "ELF",
        }
    }
}

/// Every platform md2html is built for, in the order they are reported.
///
/// Linux is built against musl, so the binary is static and runs on any
/// distribution, whatever its C library.
pub const PLATFORMS: [Platform; 6] = [
    Platform {
        name: "macos-arm64",
        target: "aarch64-apple-darwin",
        os: "macos",
        arch: "aarch64",
    },
    Platform {
        name: "macos-x64",
        target: "x86_64-apple-darwin",
        os: "macos",
        arch: "x86_64",
    },
    Platform {
        name: "linux-x64",
        target: "x86_64-unknown-linux-musl",
        os: "linux",
        arch: "x86_64",
    },
    Platform {
        name: "linux-arm64",
        target: "aarch64-unknown-linux-musl",
        os: "linux",
        arch: "aarch64",
    },
    Platform {
        name: "windows-x64",
        target: "x86_64-pc-windows-msvc",
        os: "windows",
        arch: "x86_64",
    },
    Platform {
        name: "windows-arm64",
        target: "aarch64-pc-windows-msvc",
        os: "windows",
        arch: "aarch64",
    },
];

/// The platform of that name, if md2html is built for it.
#[must_use]
pub fn platform(name: &str) -> Option<Platform> {
    PLATFORMS
        .iter()
        .copied()
        .find(|platform| platform.name == name)
}

/// The cargo subcommand that builds a platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Driver {
    /// `cargo build`: the machine's own linker serves the target.
    Cargo,
    /// `cargo zigbuild`: zig is the C toolchain and the linker.
    Zig,
    /// `cargo xwin build`: the Windows SDK, fetched and cached by cargo-xwin.
    Xwin,
}

impl Driver {
    /// The words after `cargo` that start the build.
    #[must_use]
    pub const fn words(self) -> &'static [&'static str] {
        match self {
            Self::Cargo => &["build"],
            Self::Zig => &["zigbuild"],
            Self::Xwin => &["xwin", "build"],
        }
    }

    /// What to install when the driver is missing.
    #[must_use]
    pub const fn needs(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::Zig => "cargo-zigbuild and zig (`just install-dev-tools`)",
            Self::Xwin => "cargo-xwin (`just install-dev-tools`)",
        }
    }
}

/// How a platform is built on a machine of this operating system, or `None`
/// where it cannot be: only a Mac holds Apple's SDK, so only a Mac builds for
/// macOS.
///
/// ```
/// use md2html_dev_tools::{Driver, driver, platform};
///
/// let linux = platform("linux-x64").expect("a platform md2html is built for");
/// assert_eq!(driver("macos", &linux), Some(Driver::Zig));
/// let mac = platform("macos-arm64").expect("a platform md2html is built for");
/// assert_eq!(driver("linux", &mac), None);
/// ```
#[must_use]
pub fn driver(host_os: &str, platform: &Platform) -> Option<Driver> {
    match platform.os {
        "macos" => (host_os == "macos").then_some(Driver::Cargo),
        "windows" if host_os == "windows" => Some(Driver::Cargo),
        "windows" => Some(Driver::Xwin),
        _ => Some(Driver::Zig),
    }
}

/// The kind of executable these bytes begin as: `ELF`, `Mach-O` or `PE`.
#[must_use]
pub fn executable_kind(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x7fELF") {
        Some("ELF")
    } else if bytes.starts_with(&[0xCF, 0xFA, 0xED, 0xFE]) {
        Some("Mach-O")
    } else if bytes.starts_with(b"MZ") {
        Some("PE")
    } else {
        None
    }
}

/// The workspace's folder: two above this crate's manifest.
#[must_use]
pub fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2)
        .unwrap_or(manifest)
        .to_path_buf()
}

/// Where cargo puts what it builds: `CARGO_TARGET_DIR`, or `target` in the
/// workspace.
#[must_use]
pub fn target_dir() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace_root().join("target"), PathBuf::from)
}

/// Runs one step in the workspace's folder, its output going where this tool's
/// goes.
///
/// # Errors
///
/// The step could not be started (the program is not installed), or it ended
/// with a failure; the message names the step.
pub fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let step = format!("{program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(workspace_root())
        .status()
        .map_err(|error| format!("`{step}` could not be started: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`{step}` failed ({status})"))
    }
}

/// A size in bytes the way a person reads it (`1.9 MB`).
#[must_use]
pub fn human_size(bytes: u64) -> String {
    let tenths_of_mb = bytes.saturating_mul(10) / (1024 * 1024);
    if tenths_of_mb >= 10 {
        format!("{}.{} MB", tenths_of_mb / 10, tenths_of_mb % 10)
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_platform_has_its_own_name_and_target() {
        for (at, one) in PLATFORMS.iter().enumerate() {
            for other in PLATFORMS.iter().skip(at + 1) {
                assert_ne!(one.name, other.name);
                assert_ne!(one.target, other.target);
            }
            assert_eq!(platform(one.name), Some(*one));
            assert!(
                one.target.starts_with(one.arch),
                "{} names its processor",
                one.target
            );
        }
        assert_eq!(platform("amiga-68k"), None);
    }

    #[test]
    fn a_mac_builds_everything_and_the_others_everything_but_macos() {
        for one in &PLATFORMS {
            assert!(
                driver("macos", one).is_some(),
                "{} builds on a Mac",
                one.name
            );
            assert_eq!(driver("linux", one).is_none(), one.os == "macos");
            assert_eq!(driver("windows", one).is_none(), one.os == "macos");
        }
        let windows = platform("windows-x64").unwrap();
        assert_eq!(driver("windows", &windows), Some(Driver::Cargo));
        assert_eq!(driver("macos", &windows), Some(Driver::Xwin));
        assert_eq!(windows.binary(), "md2html.exe");
        assert_eq!(platform("linux-arm64").unwrap().binary(), "md2html");
    }

    #[test]
    fn executables_are_told_apart_by_their_first_bytes() {
        assert_eq!(executable_kind(b"\x7fELF\x02\x01"), Some("ELF"));
        assert_eq!(
            executable_kind(&[0xCF, 0xFA, 0xED, 0xFE, 0x0C]),
            Some("Mach-O")
        );
        assert_eq!(executable_kind(b"MZ\x90\x00"), Some("PE"));
        assert_eq!(executable_kind(b"#!/bin/sh"), None);
        for one in &PLATFORMS {
            assert!(["ELF", "Mach-O", "PE"].contains(&one.kind()));
        }
    }

    #[test]
    fn sizes_read_as_a_person_says_them() {
        assert_eq!(human_size(1), "1 KB");
        assert_eq!(human_size(900 * 1024), "900 KB");
        assert_eq!(human_size(1024 * 1024), "1.0 MB");
        assert_eq!(human_size(1024 * 1024 * 19 / 10), "1.8 MB");
    }

    #[test]
    fn the_workspace_root_holds_the_workspace_manifest() {
        assert!(workspace_root().join("Cargo.toml").is_file());
        assert!(workspace_root().join("crates/md2html/Cargo.toml").is_file());
    }
}
