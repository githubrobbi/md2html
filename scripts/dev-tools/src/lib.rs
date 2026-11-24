// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! What the developer tools of this workspace share, in four parts:
//!
//! - where the workspace is and how a step is run in it ([`workspace_root`],
//!   [`target_dir`], [`run`], [`stdout_text`]);
//! - what a tool tells the person running it ([`say`], [`fail`],
//!   [`human_size`]);
//! - the platforms md2html is released for and how this machine builds each
//!   ([`PLATFORMS`], [`platform`], [`driver`], [`executable_kind`]);
//! - the checksum of an archive ([`sha256_hex`]).
//!
//! The tools themselves are the two binaries beside this file:
//! `install-bins` (`just use-local`) uses the first two parts, and
//! `release-local` (`just release-local`) all four. Everything is imported
//! from this root; the modules behind it are private.
//!
//! Each failure is a typed error that names what failed and keeps its cause
//! as its `source` ([`StepError`], [`NotUtf8`], [`Unsaid`]); the tools add
//! what they were doing with `anyhow`'s context.

extern crate alloc;

mod platforms;
mod report;
mod sha256;
mod workspace;

pub use platforms::{Driver, PLATFORMS, Platform, driver, executable_kind, platform};
pub use report::{Unsaid, fail, human_size, say};
pub use sha256::sha256_hex;
pub use workspace::{NotUtf8, StepError, run, stdout_text, target_dir, workspace_root};

/// The version every crate of the workspace carries, md2html's own.
///
/// ```
/// assert_eq!(md2html_dev_tools::VERSION, env!("CARGO_PKG_VERSION"));
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
