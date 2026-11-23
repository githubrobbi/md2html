// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Noticing that the files a page is made from changed, or are gone.
//!
//! `--watch` and `--serve` look at the files' times of change a few times a
//! second. Watching ends when none of the Markdown inputs has been there
//! for [`GONE_AFTER`]: a save that replaces a file is over long before.

use core::time::Duration;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

/// How long every input must be missing before watching ends.
pub(crate) const GONE_AFTER: Duration = Duration::from_secs(5);

/// One file watched, and when it last changed.
#[derive(Debug)]
struct Watched {
    /// The file.
    path: PathBuf,
    /// It is one of the Markdown inputs, not a stylesheet or a glossary.
    input: bool,
    /// Its time of change when last looked at; `None` while it is missing.
    stamp: Option<SystemTime>,
}

impl Watched {
    /// A file, looked at now.
    fn new(path: &Path, input: bool) -> Self {
        Self {
            path: path.to_path_buf(),
            input,
            stamp: stamp(path),
        }
    }
}

/// What a look at the files found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Change {
    /// Nothing changed.
    Same,
    /// A file changed, appeared or went away.
    Changed,
    /// None of the inputs has been there for [`GONE_AFTER`].
    Gone,
}

/// The files watched.
#[derive(Debug)]
pub(crate) struct Watch {
    /// Each file and its last time of change.
    files: Vec<Watched>,
    /// How long to wait between looks.
    interval: Duration,
    /// Since when every input has been missing.
    missing_since: Option<Instant>,
}

impl Watch {
    /// Watches `inputs` and the `others` they are made with, looking every
    /// `interval`.
    pub(crate) fn new(inputs: &[PathBuf], others: &[PathBuf], interval: Duration) -> Self {
        let files = inputs
            .iter()
            .map(|path| Watched::new(path, true))
            .chain(others.iter().map(|path| Watched::new(path, false)))
            .collect();
        Self {
            files,
            interval,
            missing_since: None,
        }
    }

    /// How many files are watched.
    pub(crate) fn len(&self) -> usize {
        self.files.len()
    }

    /// Waits one interval, then looks.
    pub(crate) fn wait(&mut self) -> Change {
        std::thread::sleep(self.interval);
        self.look(Instant::now())
    }

    /// Looks at every file at `now`.
    fn look(&mut self, now: Instant) -> Change {
        let mut changed = false;
        for file in &mut self.files {
            let current = stamp(&file.path);
            if current != file.stamp {
                file.stamp = current;
                changed = true;
            }
        }
        let all_missing = self
            .files
            .iter()
            .filter(|file| file.input)
            .all(|file| file.stamp.is_none());
        if !all_missing {
            self.missing_since = None;
        } else if self.missing_since.is_none() {
            self.missing_since = Some(now);
        }
        let gone = self
            .missing_since
            .is_some_and(|since| now.saturating_duration_since(since) >= GONE_AFTER);
        if gone {
            Change::Gone
        } else if changed {
            Change::Changed
        } else {
            Change::Same
        }
    }
}

/// A file's time of change, or `None` when it cannot be read.
fn stamp(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::time::Instant;

    use super::{Change, GONE_AFTER, Watch};

    /// Watching sees a file go and come back, and ends when it stays gone.
    ///
    /// # Panics
    ///
    /// When a stylesheet that does not exist yet is not watched beside the
    /// input; when nothing changed and a change is seen; when an input removed
    /// is not a change at once, or is gone after a moment; when it is not gone
    /// after [`GONE_AFTER`]; when its return is not a change; or when the
    /// scratch folder or the input cannot be written or removed.
    #[test]
    fn watching_sees_a_change_and_ends_when_the_inputs_are_gone() {
        let dir = std::env::temp_dir().join(format!("md2html-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("a.md");
        std::fs::write(&input, "# A\n").unwrap();
        let mut watch = Watch::new(
            core::slice::from_ref(&input),
            &[dir.join("absent.css")],
            Duration::ZERO,
        );
        assert_eq!(watch.len(), 2, "the input and the stylesheet");
        let start = Instant::now();
        assert_eq!(watch.look(start), Change::Same, "nothing happened");
        std::fs::remove_file(&input).unwrap();
        assert_eq!(watch.look(start), Change::Changed, "the input went away");
        assert_eq!(watch.look(start), Change::Same, "a moment is not gone");
        assert_eq!(
            watch.look(start + GONE_AFTER),
            Change::Gone,
            "gone for long enough"
        );
        std::fs::write(&input, "# A again\n").unwrap();
        assert_eq!(
            watch.look(start + GONE_AFTER),
            Change::Changed,
            "it is back"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
