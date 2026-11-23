// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The same document makes the same bytes.
//!
//! It is rendered twice in one run, and in two runs from two different
//! working folders, with the moment of rendering pinned by
//! `SOURCE_DATE_EPOCH` as a reproducible build pins it.
//!
//! A run's environment and working folder belong to its process, so the
//! runs are this test program started again on its `one_run` test, which writes
//! what it rendered where [`OUT_VAR`] says.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{Scratch, own_glossary_only};
use crate::document::render_doc;

/// Where a run writes its pages; set only for the runs this test starts.
const OUT_VAR: &str = "MD2HTML_TEST_RENDERS";

/// The moment every run gives as "now".
const EPOCH: &str = "1700000000";

/// The name of the test that is one run.
const ONE_RUN: &str = "one_run";

/// A document and what it is called in a run's folder.
struct Document {
    /// Its name there.
    name: &'static str,
    /// Its Markdown.
    text: String,
    /// Its file, for a document that has one: its folder is where it
    /// reads from.
    source: Option<PathBuf>,
    /// The page it becomes; an EPUB is made beside it when asked for.
    out: PathBuf,
}

/// The repository's own pages, and a few that draw, set maths and make a
/// book.
///
/// # Panics
///
/// When the repository's root cannot be resolved from the crate's folder,
/// or one of its pages (`README.md`, `docs/showcase.md`,
/// `docs/capabilities.md`, `docs/capabilities.he.md`) cannot be read.
fn documents() -> Vec<Document> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository");
    let mut out: Vec<Document> = [
        ("readme", "README.md"),
        ("showcase", "docs/showcase.md"),
        ("capabilities", "docs/capabilities.md"),
        ("capabilities-he", "docs/capabilities.he.md"),
    ]
    .into_iter()
    .map(|(name, file)| {
        let path = root.join(file);
        let page = path.with_extension("html");
        Document {
            name,
            text: std::fs::read_to_string(&path).expect("a page of the repository"),
            source: Some(path),
            out: page,
        }
    })
    .collect();
    let written = [
        (
            "diagrams",
            "# Drawn\n\n```mermaid\nflowchart TD\n  A[Start] --> B{Ok?}\n  B -->|yes| C[Done]\n  B -->|no| A\n```\n\n```mermaid\nsequenceDiagram\n  Alice->>Bob: Hello\n  Bob-->>Alice: Hi\n```\n",
        ),
        (
            "maths",
            "# Set\n\nEuler: $e^{i\\pi} + 1 = 0$.\n\n$$\n\\sum_{k=1}^{n} k = \\frac{n(n+1)}{2}\n$$\n\n## Glossary\n\n| Term | Meaning |\n|---|---|\n| sum | What adding makes. |\n",
        ),
        (
            "book",
            "---\nepub: true\nlang: de\n---\n# Ein Buch\n\nText mit[^1] Fußnote.\n\n## 1. Kapitel\n\n```rust\nlet x = 1 < 2;\n```\n\n[^1]: Die Fußnote.\n",
        ),
    ];
    out.extend(written.into_iter().map(|(name, text)| Document {
        name,
        text: String::from(text),
        source: None,
        out: PathBuf::from(format!("{name}.html")),
    }));
    out
}

/// One run: every document rendered twice, the two the same.
///
/// The pages are written to the folder [`OUT_VAR`] names. Started by
/// [`the_same_documents_render_the_same_bytes_anywhere`].
///
/// # Panics
///
/// When a document renders to a different page or a different book the
/// second time, when [`OUT_VAR`] is not set, or when a page or a book cannot
/// be written to its folder.
#[test]
#[ignore = "one run of the_same_documents_render_the_same_bytes_anywhere, which starts it"]
fn one_run() {
    let folder = PathBuf::from(std::env::var_os(OUT_VAR).expect("the folder to write to"));
    for document in documents() {
        let render = || {
            render_doc(
                &document.text,
                document.source.as_deref(),
                Some(&document.out),
                &own_glossary_only(),
                false,
                None,
            )
        };
        let first = render();
        let second = render();
        assert!(first.html == second.html, "{}: two pages", document.name);
        assert!(first.epub == second.epub, "{}: two books", document.name);
        std::fs::write(folder.join(format!("{}.html", document.name)), first.html)
            .expect("written");
        if let Some(book) = first.epub {
            std::fs::write(folder.join(format!("{}.epub", document.name)), book).expect("written");
        }
    }
}

/// Starts the `one_run` test from the working folder `cwd`, writing to `out`.
///
/// # Panics
///
/// When this test program cannot be started again, when the run fails (the
/// message carries what it printed), or when it does not report its one
/// test as passed: the filter found no test to run.
fn run_from(cwd: &Path, out: &Path) {
    let program = std::env::current_exe().expect("this test program");
    let module = module_path!()
        .split_once("::")
        .map_or(module_path!(), |(_, inside)| inside);
    let ran = Command::new(program)
        .args([
            "--exact",
            &format!("{module}::{ONE_RUN}"),
            "--ignored",
            "--test-threads=1",
        ])
        .env(OUT_VAR, out)
        .env("SOURCE_DATE_EPOCH", EPOCH)
        .current_dir(cwd)
        .output()
        .expect("the run starts");
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(
        ran.status.success(),
        "the run from {} failed:\n{said}",
        cwd.display()
    );
    assert!(said.contains("1 passed"), "the run ran its test:\n{said}");
}

/// The files a run wrote, by name, sorted.
///
/// # Panics
///
/// When the run's folder or a file in it cannot be read.
fn written(folder: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(folder)
        .expect("the run's folder")
        .map(|entry| {
            let path = entry.expect("a file the run wrote").path();
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            (name, std::fs::read(&path).expect("readable"))
        })
        .collect();
    files.sort();
    files
}

/// A document renders to the same bytes from any working folder.
///
/// # Panics
///
/// When a run from either folder fails or renders a document differently
/// twice; when the runs did not write a page for every document and the
/// one book; when the two folders' runs wrote different files, or one of
/// them different bytes; or when the folders cannot be made.
#[test]
fn the_same_documents_render_the_same_bytes_anywhere() {
    let scratch = Scratch::new("determinism");
    let here = scratch.path().join("here");
    let elsewhere = scratch.path().join("some/where/else");
    let (out_here, out_elsewhere) = (
        scratch.path().join("out-here"),
        scratch.path().join("out-elsewhere"),
    );
    for folder in [&here, &elsewhere, &out_here, &out_elsewhere] {
        std::fs::create_dir_all(folder).expect("a folder");
    }
    run_from(&here, &out_here);
    run_from(&elsewhere, &out_elsewhere);
    let first = written(&out_here);
    let second = written(&out_elsewhere);
    let names: Vec<&str> = first.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names.len(),
        documents().len() + 1,
        "a page for each document and the book: {}",
        names.join(", ")
    );
    assert_eq!(
        first.len(),
        second.len(),
        "as many files from either folder"
    );
    for ((name, bytes), (other_name, other_bytes)) in first.iter().zip(&second) {
        assert_eq!(name, other_name);
        assert!(
            bytes == other_bytes,
            "{name} differs between the two folders"
        );
    }
}
