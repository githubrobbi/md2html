// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! The platforms md2html is released for, and how the machine at hand
//! builds each: `release-local`'s table.

/// One platform md2html is released for.
///
/// ```
/// use md2html_dev_tools::platform;
///
/// let mac = platform("macos-x64").expect("a platform md2html is built for");
/// assert_eq!((mac.os, mac.arch), ("macos", "x86_64"));
/// ```
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
    ///
    /// ```
    /// use md2html_dev_tools::platform;
    ///
    /// let windows = platform("windows-x64").expect("a platform md2html is built for");
    /// assert_eq!(windows.binary(), "md2html.exe");
    /// ```
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
    ///
    /// ```
    /// use md2html_dev_tools::platform;
    ///
    /// let linux = platform("linux-arm64").expect("a platform md2html is built for");
    /// assert_eq!(linux.kind(), "ELF");
    /// ```
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
///
/// ```
/// use md2html_dev_tools::PLATFORMS;
///
/// let names: Vec<&str> = PLATFORMS.iter().map(|platform| platform.name).collect();
/// assert_eq!(names.first(), Some(&"macos-arm64"));
/// assert!(PLATFORMS.iter().all(|platform| !platform.target.is_empty()));
/// ```
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
///
/// ```
/// use md2html_dev_tools::platform;
///
/// assert_eq!(
///     platform("linux-x64").map(|found| found.target),
///     Some("x86_64-unknown-linux-musl")
/// );
/// assert_eq!(platform("amiga"), None);
/// ```
#[must_use]
pub fn platform(name: &str) -> Option<Platform> {
    PLATFORMS
        .iter()
        .copied()
        .find(|platform| platform.name == name)
}

/// The cargo subcommand that builds a platform.
///
/// ```
/// use md2html_dev_tools::{Driver, driver, platform};
///
/// let windows = platform("windows-arm64").expect("a platform md2html is built for");
/// assert_eq!(driver("linux", &windows), Some(Driver::Xwin));
/// ```
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
    ///
    /// ```
    /// use md2html_dev_tools::Driver;
    ///
    /// assert_eq!(Driver::Xwin.words(), ["xwin", "build"]);
    /// ```
    #[must_use]
    pub const fn words(self) -> &'static [&'static str] {
        match self {
            Self::Cargo => &["build"],
            Self::Zig => &["zigbuild"],
            Self::Xwin => &["xwin", "build"],
        }
    }

    /// What to install when the driver is missing.
    ///
    /// ```
    /// use md2html_dev_tools::Driver;
    ///
    /// assert!(Driver::Zig.needs().contains("just install-dev-tools"));
    /// ```
    #[must_use]
    pub const fn needs(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::Zig => "cargo-zigbuild and zig 0.17.1 (`just install-dev-tools`)",
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
///
/// ```
/// use md2html_dev_tools::executable_kind;
///
/// assert_eq!(executable_kind(b"\x7fELF\x02\x01"), Some("ELF"));
/// assert_eq!(executable_kind(b"MZ\x90\x00"), Some("PE"));
/// assert_eq!(executable_kind(b"#!/bin/sh"), None);
/// ```
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

#[cfg(test)]
mod tests {
    use super::{Driver, PLATFORMS, driver, executable_kind, platform};

    /// Every platform has a name and a target no other platform has.
    ///
    /// # Panics
    ///
    /// When two platforms share a name or a target, when `platform` does not
    /// find a platform by its own name or finds the unknown `amiga-68k`, or
    /// when a target does not start with its platform's processor.
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

    /// A Mac has a driver for every platform; Linux and Windows for all but
    /// macOS.
    ///
    /// # Panics
    ///
    /// When a Mac has no driver for a platform, when Linux or Windows has one
    /// for macOS or none for another platform, when `windows-x64` is not built
    /// with plain cargo on Windows and with cargo-xwin on a Mac, when
    /// `windows-x64` or `linux-arm64` is not a platform, or when their binaries
    /// are not named `md2html.exe` and `md2html`.
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

    /// ELF, Mach-O and PE files are told apart by their first bytes.
    ///
    /// # Panics
    ///
    /// When the first bytes of one of the three are read as another or as none,
    /// when a shell script is read as an executable, or when a platform's kind
    /// is none of the three.
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
}
