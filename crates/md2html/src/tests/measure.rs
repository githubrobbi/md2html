// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Where a render spends its time, on the repository's own documents.
//!
//! Not a check: a measurement, run on request (`just measure`, which
//! builds it with the release profile) and printed as a table. Each
//! document is read once, then rendered in three stages, each timed on its
//! own: the blocks read (`front_matter` and `block::parse`), the bare HTML
//! of `--fragment` (the blocks read again, the inline text read and
//! written), and the whole page (`render_doc`: the article with its
//! glossary links, the images beside it embedded, the page around it, its
//! checks), each document from its own file as the command line renders
//! it, with the shared glossary left out. A stage is run until
//! it has taken a second or ten times, whichever comes first, and the
//! fastest run is the one shown: the others measured the machine as much
//! as md2html.
//!
//! The documents are the README, the showcase, the capabilities page and
//! its translations, all of them as one document (read as if it were the
//! capabilities page, beside the same images), and that one twice over,
//! then a made-up handbook with a glossary of [`TERMS`] terms. All as one
//! and twice say how the time grows with the length of a text: twice the
//! text should take twice the time. The handbook says what a glossary
//! costs when it is large: every text of the page is searched for its
//! terms.

use core::time::Duration;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::own_glossary_only;
use crate::config::front_matter;
use crate::document::{Shape, render_doc};
use crate::inline::HtmlMode;
use crate::text::put;
use crate::{block, render};

/// How long a stage is run for, at most, before its fastest run is taken.
const BUDGET: Duration = Duration::from_secs(1);

/// How many runs a stage gets, at most.
const RUNS: usize = 10;

/// How many terms the made-up handbook's glossary has.
const TERMS: usize = 2000;

/// A document the measurement renders.
struct Sample {
    /// What the table calls it.
    name: String,
    /// Its Markdown.
    text: String,
    /// The file it is read as: the files it names are found beside it.
    source: PathBuf,
}

/// The fastest of the runs of `stage` that fit in [`BUDGET`] and [`RUNS`];
/// one run at least.
fn fastest(mut stage: impl FnMut() -> usize) -> Duration {
    let mut best = Duration::MAX;
    let started = Instant::now();
    for _ in 0..RUNS {
        let run = Instant::now();
        // The length of what the stage made, so it cannot be left unmade.
        core::hint::black_box(stage());
        best = best.min(run.elapsed());
        if started.elapsed() >= BUDGET {
            break;
        }
    }
    best
}

/// The repository's documents, then all of them as one, then that twice.
///
/// # Panics
///
/// When the repository cannot be found from the crate's folder, or one of
/// its documents cannot be read.
fn samples() -> Vec<Sample> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let docs = root.join("docs");
    // `capabilities.<language>.md`: a stem with a dot in it.
    let mut translations: Vec<PathBuf> = std::fs::read_dir(&docs)
        .expect("the docs folder")
        .map(|entry| entry.expect("a file of the docs folder").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .filter(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| stem.starts_with("capabilities."))
        })
        .collect();
    translations.sort();
    let read = |path: &Path| std::fs::read_to_string(path).expect("a document of the repository");
    let mut out: Vec<Sample> = [
        ("README", root.join("README.md")),
        ("showcase", docs.join("showcase.md")),
        ("capabilities", docs.join("capabilities.md")),
    ]
    .into_iter()
    .map(|(name, path)| Sample {
        name: name.to_owned(),
        text: read(&path),
        source: path,
    })
    .collect();
    out.extend(translations.into_iter().map(|path| {
        Sample {
            name: path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("a translation's name")
                .to_owned(),
            text: read(&path),
            source: path,
        }
    }));
    let all: String = out
        .iter()
        .map(|sample| sample.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let twice = format!("{all}\n\n{all}");
    let beside = docs.join("capabilities.md");
    out.push(Sample {
        name: "all as one".to_owned(),
        text: all,
        source: beside.clone(),
    });
    out.push(Sample {
        name: "all, twice".to_owned(),
        text: twice,
        source: beside,
    });
    out.push(Sample {
        name: format!("glossary of {TERMS}"),
        text: handbook(),
        source: docs.join("showcase.md"),
    });
    out
}

/// A handbook of a hundred chapters whose paragraphs each mention two of
/// the [`TERMS`] terms of its glossary, among words that are none.
fn handbook() -> String {
    let mut text = String::from("# Handbook\n\n");
    for chapter in 0..100 {
        put!(text, "## {chapter}. Chapter\n\n");
        for paragraph in 0..TERMS / 100 {
            let term = chapter * (TERMS / 100) + paragraph;
            put!(
                text,
                "The gizmo{term} sits beside gizmo{} on the bench, and the operator \
                 checks both before the shift ends.\n\n",
                (term * 7) % TERMS
            );
        }
    }
    text.push_str("## Glossary\n\n| Term | Meaning |\n|---|---|\n");
    for term in 0..TERMS {
        put!(text, "| Gizmo{term} | A part of the machine. |\n");
    }
    text
}

/// Milliseconds with three decimals, from a duration.
fn ms(time: Duration) -> String {
    let micros = time.as_micros();
    format!("{}.{:03}", micros / 1000, micros % 1000)
}

/// The table: one row per document, the time of each stage.
///
/// # Panics
///
/// When a document cannot be read or the table cannot be written.
#[test]
#[ignore = "a measurement, not a check: `just measure` runs it"]
fn where_the_time_goes() {
    let settings = own_glossary_only();
    let mut stdout = std::io::stdout().lock();
    writeln!(
        stdout,
        "\n{:<20} {:>9} {:>11} {:>13} {:>11} {:>12}",
        "document", "KiB", "blocks ms", "fragment ms", "page ms", "page KiB"
    )
    .expect("the table");
    for sample in samples() {
        let text = sample.text.as_str();
        let source = Some(sample.source.as_path());
        let page_file = sample.source.with_extension("html");
        let blocks = fastest(|| {
            let read = front_matter(text);
            block::parse(read.body, read.first_line, true, block::Reading::Author)
                .0
                .len()
        });
        let fragment = fastest(|| {
            let read = front_matter(text);
            let (parsed, defs) =
                block::parse(read.body, read.first_line, true, block::Reading::Author);
            render::fragment(&parsed, &defs, HtmlMode::Safe, true).len()
        });
        let mut made = 0;
        let page = fastest(|| {
            made = render_doc(text, source, Some(&page_file), &settings, Shape::Page, None)
                .html
                .len();
            made
        });
        writeln!(
            stdout,
            "{:<20} {:>9} {:>11} {:>13} {:>11} {:>12}",
            sample.name,
            text.len() / 1024,
            ms(blocks),
            ms(fragment),
            ms(page),
            made / 1024
        )
        .expect("the table");
    }
}
